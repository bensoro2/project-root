#[cfg(feature = "spfresh")]
pub use spfresh::Index as VectorStore;

#[cfg(not(feature = "spfresh"))]
mod implementation {
    use anyhow::Result;
    use std::fs::{File, OpenOptions};
    use std::io::{Seek, SeekFrom, Write};
    use std::f32;
    use std::mem;
    use std::path::PathBuf;
    use bytemuck;
    use std::collections::BinaryHeap;
use std::cmp::Reverse;
    use ordered_float::NotNan;

    #[derive(Debug)]
    pub struct VectorStore {
        path: PathBuf,
        dim: usize,
        scale: f32, // factor used for quantization
    }

    impl VectorStore {
        pub fn open_or_create(path: PathBuf) -> Result<Self> {
            if !path.exists() {
                File::create(&path)?;
            }
            Ok(Self { path, dim: 768, scale: 127.0 })
        }

        pub fn append(&mut self, vector: &[f32]) -> Result<()> {
            assert_eq!(vector.len(), self.dim, "vector dim mismatch");
            // Normalize vector to unit length to keep values within [-1, 1]
            let norm: f32 = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
            // Avoid division by zero
            let norm = if norm == 0.0 { 1.0 } else { norm };
            
            // Quantize to i8
            let mut quantized: Vec<i8> = Vec::with_capacity(self.dim);
            for &v in vector {
                let val = (v / norm).clamp(-1.0, 1.0) * self.scale;
                quantized.push(val.round() as i8);
            }
            
            let mut file = OpenOptions::new()
                .append(true)
                .create(true)
                .open(&self.path)
                .map_err(|e| anyhow::anyhow!("Failed to open vector store file: {}", e))?;
            
            let bytes: &[u8] = bytemuck::cast_slice(&quantized);
            file.write_all(bytes)
                .map_err(|e| anyhow::anyhow!("Failed to write vector to file: {}", e))?;
            file.sync_all()
                .map_err(|e| anyhow::anyhow!("Failed to sync vector store to disk: {}", e))?;
            Ok(())
        }

        #[allow(dead_code)]
        pub fn len(&self) -> Result<usize> {
            let mut file = File::open(&self.path)
                .map_err(|e| anyhow::anyhow!("Failed to open vector store file: {}", e))?;
            let size = file.seek(SeekFrom::End(0))
                .map_err(|e| anyhow::anyhow!("Failed to seek to end of vector store file: {}", e))? as usize;
            Ok(size / (self.dim * mem::size_of::<i8>()))
        }

        pub fn search(&self, query: &[f32], top_k: usize) -> Result<Vec<(usize, f32)>> {
            assert!(top_k > 0, "top_k must be > 0");
            assert_eq!(query.len(), self.dim, "query dim mismatch");
            
            let data = std::fs::read(&self.path)
                .map_err(|e| anyhow::anyhow!("Failed to read vector store file: {}", e))?;
                
            if data.is_empty() {
                return Ok(Vec::new());
            }
            
            let total_i8: &[i8] = bytemuck::cast_slice(&data);
            let num_vecs = total_i8.len() / self.dim;
            
            let mut heap: BinaryHeap<Reverse<(NotNan<f32>, usize)>> =
                BinaryHeap::with_capacity(top_k + 1);
                
            for i in 0..num_vecs {
                let start = i * self.dim;
                let end = start + self.dim;
                let vec_slice = &total_i8[start..end];
                
                let mut score = 0.0f32;
                for (a, &b_q) in query.iter().zip(vec_slice.iter()) {
                    // dequantize on the fly
                    let b = (b_q as f32) / self.scale;
                    score += a * b;
                }
                
                if let Ok(not_nan) = NotNan::new(score) {
                    heap.push(Reverse((not_nan, i)));
                    if heap.len() > top_k {
                        heap.pop();
                    }
                }
            }
            
            let mut results: Vec<(usize, f32)> = heap.into_iter()
                .map(|Reverse((s, i))| (i, s.into_inner()))
                .collect();
            results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            
            Ok(results)
        }
    }

}

#[cfg(not(feature = "spfresh"))]
pub use implementation::VectorStore;
