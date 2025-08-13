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
const uint8_t QUANTIZATION_LEVELS = 255;

struct SPFreshIndex {
    std::string path;
    std::vector<std::vector<uint8_t>> quantized_vectors;
    std::vector<float> norms;
    size_t dimension;
    std::mutex mtx;
    
    SPFreshIndex(const std::string& p) : path(p), dimension(768) {
        load_index();
    }
    
    ~SPFreshIndex() {
        save_index();
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
        
        std::ifstream file(path, std::ios::binary);
        if (!file) {
            return true;
        }
        
        uint32_t magic;
        file.read(reinterpret_cast<char*>(&magic), sizeof(magic));
        if (magic != 0x50534648) {
            return false; 
        }
        
        uint32_t version;
        file.read(reinterpret_cast<char*>(&version), sizeof(version));
        
        file.read(reinterpret_cast<char*>(&dimension), sizeof(dimension));
        
        uint32_t num_vectors;
        file.read(reinterpret_cast<char*>(&num_vectors), sizeof(num_vectors));
        
        norms.resize(num_vectors);
        file.read(reinterpret_cast<char*>(norms.data()), num_vectors * sizeof(float));
        
        quantized_vectors.resize(num_vectors);
        for (uint32_t i = 0; i < num_vectors; i++) {
            quantized_vectors[i].resize(dimension);
            file.read(reinterpret_cast<char*>(quantized_vectors[i].data()), dimension * sizeof(uint8_t));
        }
        
        return true;
    }
    
    bool save_index() {
        std::lock_guard<std::mutex> lock(mtx);
        std::ofstream file(path, std::ios::binary | std::ios::trunc);
        if (!file) {
            return false;
        }
        
        uint32_t magic = 0x50534648;
        file.write(reinterpret_cast<const char*>(&magic), sizeof(magic));
        
        uint32_t version = 1;
        file.write(reinterpret_cast<const char*>(&version), sizeof(version));
        
        file.write(reinterpret_cast<const char*>(&dimension), sizeof(dimension));
        
        uint32_t num_vectors = quantized_vectors.size();
        file.write(reinterpret_cast<const char*>(&num_vectors), sizeof(num_vectors));
        
        file.write(reinterpret_cast<const char*>(norms.data()), num_vectors * sizeof(float));
        
        for (const auto& quantized_vec : quantized_vectors) {
            file.write(reinterpret_cast<const char*>(quantized_vec.data()), dimension * sizeof(uint8_t));
        }
        
        return true;
    }
    
    int append(const float* vector, size_t dim) {
        std::lock_guard<std::mutex> lock(mtx);
        
        if (dim != dimension) {
            return -1;
        }
        
        std::vector<float> vec(vector, vector + dim);
        std::vector<uint8_t> quantized_vec = quantize_vector(vec);
        float norm = calculate_norm(vec);

        quantized_vectors.push_back(std::move(quantized_vec));
        norms.push_back(norm);
        
        return save_index() ? 0 : -2;
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