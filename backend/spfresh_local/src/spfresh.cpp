#include <vector>
#include <string>
#include <fstream>
#include <iostream>
#include <sstream>
#include <cmath>
#include <algorithm>
#include <cstring>
#include <unordered_map>
#include <mutex>
#include <random>

const float MIN_VAL = -2.0f;
const float MAX_VAL = 2.0f;
const uint8_t QUANTIZATION_LEVELS = 127; // Use 7 bits instead of 8 bits to reduce file size

struct SPFreshIndex {
    std::string path;
    std::vector<std::vector<uint8_t>> quantized_vectors;
    std::vector<float> norms;
    size_t dimension;
    std::mutex mtx;
    
    // Buffer for incremental writes
    std::vector<std::vector<uint8_t>> write_buffer_vectors;
    std::vector<float> write_buffer_norms;
    static const size_t WRITE_BUFFER_SIZE = 1000; // Write to disk every 1000 vectors
    
    SPFreshIndex(const std::string& p) : path(p), dimension(768) {
        // Strip ".index" extension if present to get base path
        if (path.size() > 6 && path.substr(path.size() - 6) == ".index") {
            path = path.substr(0, path.size() - 6);
        }
        load_index();
    }
    
    ~SPFreshIndex() {
        flush_write_buffer();
        save_index();
    }
    
    void flush_write_buffer() {
        if (write_buffer_vectors.empty()) return;
        
        std::string vec_path = path + ".vectors";
        std::string meta_path = path + ".metadata";
        
        {
            std::ofstream vec_file(vec_path, std::ios::binary | std::ios::app);
            if (!vec_file) {
                std::cerr << "Failed to open vector file for writing: " << vec_path << std::endl;
                return;
            }
            for (const auto& qv : write_buffer_vectors) {
                vec_file.write(reinterpret_cast<const char*>(qv.data()), dimension);
            }
        }
        
        // Append norms to file
        {
            std::ofstream meta_file(meta_path, std::ios::binary | std::ios::app);
            if (!meta_file) {
                std::cerr << "Failed to open metadata file for writing: " << meta_path << std::endl;
                return;
            }
            meta_file.write(reinterpret_cast<const char*>(write_buffer_norms.data()),
                           write_buffer_norms.size() * sizeof(float));
        }
        
        // Clear buffers
        write_buffer_vectors.clear();
        write_buffer_norms.clear();
    }
    
    uint8_t quantize(float val) {
        float clamped = std::max(MIN_VAL, std::min(MAX_VAL, val));
        float normalized = (clamped - MIN_VAL) / (MAX_VAL - MIN_VAL);
        return static_cast<uint8_t>(normalized * QUANTIZATION_LEVELS);
    }
    
    float dequantize(uint8_t val) {
        float normalized = static_cast<float>(val) / QUANTIZATION_LEVELS;
        return normalized * (MAX_VAL - MIN_VAL) + MIN_VAL;
    }
    
    std::vector<uint8_t> quantize_vector(const std::vector<float>& vec) {
        std::vector<uint8_t> quantized;
        quantized.reserve(vec.size());
        for (float val : vec) {
            quantized.push_back(quantize(val));
        }
        return quantized;
    }
    
    std::vector<float> dequantize_vector(const std::vector<uint8_t>& quantized) {
        std::vector<float> vec;
        vec.reserve(quantized.size());
        for (uint8_t val : quantized) {
            vec.push_back(dequantize(val));
        }
        return vec;
    }
    
    float calculate_norm(const std::vector<float>& vec) {
        float sum_sq = 0.0f;
        for (float val : vec) {
            sum_sq += val * val;
        }
        float norm = std::sqrt(sum_sq);
        // Debug: Log norm calculation
        std::cout << "Vector norm: " << norm << " (sum_sq: " << sum_sq << ")" << std::endl;
        return norm;
    }
    
    bool load_index() {
        std::lock_guard<std::mutex> lock(mtx);
        quantized_vectors.clear();
        norms.clear();

        std::string vec_path = path + ".vectors";
        std::string meta_path = path + ".metadata";

        std::ifstream vec_file(vec_path, std::ios::binary);
        if (!vec_file) {
            // No vectors yet; that's fine
            return true;
        }

        // Determine file size to compute number of vectors
        vec_file.seekg(0, std::ios::end);
        std::streampos vec_size = vec_file.tellg();
        if (vec_size <= 0 || (vec_size % static_cast<std::streampos>(dimension)) != 0) {
            return false;
        }
        size_t num_vectors = static_cast<size_t>(vec_size) / dimension;
        vec_file.seekg(0, std::ios::beg);

        quantized_vectors.resize(num_vectors, std::vector<uint8_t>(dimension));
        for (size_t i = 0; i < num_vectors; i++) {
            vec_file.read(reinterpret_cast<char*>(quantized_vectors[i].data()), dimension);
        }

        // Load norms if available
        std::ifstream meta_file(meta_path, std::ios::binary);
        if (meta_file) {
            norms.resize(num_vectors);
            meta_file.read(reinterpret_cast<char*>(norms.data()), num_vectors * sizeof(float));
            if (meta_file.gcount() != static_cast<std::streamsize>(num_vectors * sizeof(float))) {
                // Corrupt metadata; recompute norms
                norms.clear();
            }
        }

        if (norms.size() != num_vectors) {
            norms.resize(num_vectors);
            for (size_t i = 0; i < num_vectors; i++) {
                std::vector<float> deq = dequantize_vector(quantized_vectors[i]);
                norms[i] = calculate_norm(deq);
            }
            save_index();
        }
        return true;
    }
    
    bool save_index() {
        std::lock_guard<std::mutex> lock(mtx);
        std::string vec_path = path + ".vectors";
        std::string meta_path = path + ".metadata";

        // Save vectors
        {
            std::ofstream vec_file(vec_path, std::ios::binary | std::ios::trunc);
            if (!vec_file) {
                return false;
            }
            for (const auto& qv : quantized_vectors) {
                vec_file.write(reinterpret_cast<const char*>(qv.data()), dimension);
            }
        }

        // Save norms
        {
            std::ofstream meta_file(meta_path, std::ios::binary | std::ios::trunc);
            if (!meta_file) {
                return false;
            }
            meta_file.write(reinterpret_cast<const char*>(norms.data()), norms.size() * sizeof(float));
        }
        return true;
    }
    
    int append(const float* vector, size_t dim) {
        std::lock_guard<std::mutex> lock(mtx);
        if (dim != dimension) {
            return -1;
        }

        // Quantize and compute norm
        std::vector<float> vec(vector, vector + dim);
        std::vector<uint8_t> quantized_vec = quantize_vector(vec);
        float norm = calculate_norm(vec);

        // In-memory
        quantized_vectors.push_back(quantized_vec);
        norms.push_back(norm);

        // Add to write buffer
        write_buffer_vectors.push_back(quantized_vec);
        write_buffer_norms.push_back(norm);

        // Flush buffer if it's full
        if (write_buffer_vectors.size() >= WRITE_BUFFER_SIZE) {
            flush_write_buffer();
        }
        
        return 0;
    }
    
    int search(const float* query, size_t dim, size_t top_k,
               size_t* result_indices, float* result_scores) {
        std::lock_guard<std::mutex> lock(mtx);
        
        if (dim != dimension || top_k == 0 || quantized_vectors.empty()) {
            return -1;
        }
        
        std::vector<std::pair<float, size_t>> scores;
        scores.reserve(quantized_vectors.size());
        
        std::vector<float> query_vec(query, query + dim);
        float query_norm = calculate_norm(query_vec);
        
        for (size_t i = 0; i < quantized_vectors.size(); i++) {
            const auto& quantized_vec = quantized_vectors[i];
            float doc_norm = norms[i];
            
            float dot_product = 0.0f;
            for (size_t j = 0; j < dim; j++) {
                float val1 = query[j];
                float val2 = dequantize(quantized_vec[j]);
                dot_product += val1 * val2;
            }
            
            float similarity = 0.0f;
            if (query_norm > 0 && doc_norm > 0) {
                similarity = dot_product / (query_norm * doc_norm);
            }
            
            scores.emplace_back(similarity, i);
        }
        
        std::sort(scores.begin(), scores.end(),
                  [](const std::pair<float, size_t>& a,
                     const std::pair<float, size_t>& b) {
            return a.first > b.first;
        });
        
        size_t result_count = std::min(top_k, scores.size());
        for (size_t i = 0; i < result_count; i++) {
            result_indices[i] = scores[i].second;
            result_scores[i] = scores[i].first;
        }
        
        return 0;
    }
};

std::unordered_map<void*, SPFreshIndex*> index_map;
std::mutex map_mutex;

extern "C" {
    void* spfresh_index_create(const char* path) {
        if (!path) return nullptr;
        
        try {
            SPFreshIndex* index = new SPFreshIndex(std::string(path));
            std::lock_guard<std::mutex> lock(map_mutex);
            index_map[index] = index;
            return static_cast<void*>(index);
        } catch (...) {
            return nullptr;
        }
    }
    
    int spfresh_index_append(void* index_ptr, const float* vector, size_t dim) {
        if (!index_ptr || !vector) return -1;
        
        std::lock_guard<std::mutex> lock(map_mutex);
        auto it = index_map.find(index_ptr);
        if (it == index_map.end()) return -1;
        
        return it->second->append(vector, dim);
    }
    
    int spfresh_index_search(void* index_ptr, const float* query, size_t dim, 
                            size_t top_k, size_t* result_indices, float* result_scores) {
        if (!index_ptr || !query || !result_indices || !result_scores) return -1;
        
        std::lock_guard<std::mutex> lock(map_mutex);
        auto it = index_map.find(index_ptr);
        if (it == index_map.end()) return -1;
        
        return it->second->search(query, dim, top_k, result_indices, result_scores);
    }
    
    size_t spfresh_index_size(void* index_ptr) {
        if (!index_ptr) return 0;
        
        std::lock_guard<std::mutex> lock(map_mutex);
        auto it = index_map.find(index_ptr);
        if (it == index_map.end()) return 0;
        return it->second->quantized_vectors.size();
    }
    
    void spfresh_index_destroy(void* index_ptr) {
        if (!index_ptr) return;
        
        std::lock_guard<std::mutex> lock(map_mutex);
        auto it = index_map.find(index_ptr);
        if (it != index_map.end()) {
            delete it->second;
            index_map.erase(it);
        }
    }
}