#[cfg(feature = "spfresh")]
use spfresh_sys as sys;

#[cfg(feature = "spfresh")]
use anyhow::Result;
use std::path::Path;

#[cfg(feature = "spfresh")]
#[derive(Debug)]
pub struct Index {
    inner: sys::Index,
}

unsafe impl Send for Index {}
unsafe impl Sync for Index {}

#[cfg(feature = "spfresh")]
impl Index {
    pub fn open_or_create<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_str = path.as_ref().to_string_lossy();
        println!("Creating SPFresh index with path: {}", path_str);
        let inner = sys::Index::new(&path_str)
            .map_err(|e| anyhow::anyhow!("Failed to create SPFresh index: {}", e))?;
        println!("SPFresh index created successfully");
        
        Ok(Self { inner })
    }

    pub fn append(&mut self, vector: &[f32]) -> Result<()> {
        let result = self.inner.append(vector);
        if let Err(ref e) = result {
            eprintln!("Failed to append vector: {}", e);
        }
        result.map_err(|e| anyhow::anyhow!("Failed to append vector: {}", e))
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn search(&self, query: &[f32], top_k: usize) -> Result<Vec<(usize, f32)>> {
        self.inner.search(query, top_k)
            .map_err(|e| anyhow::anyhow!("Search failed: {}", e))
    }
}

#[cfg(not(feature = "spfresh"))]
mod fallback {
    use anyhow::Result;
    use ordered_float::NotNan;
    use std::fs::{File, OpenOptions};
    use std::io::{Seek, SeekFrom, Write};
    use std::path::PathBuf;
    use bytemuck;
    use std::collections::BinaryHeap;
    use std::cmp::Reverse;

    #[derive(Debug)]
    pub struct Index {
        path: PathBuf,
        dim: usize,
    }

    impl Index {
        pub fn open_or_create<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
            let p = path.as_ref();
            if !p.exists() {
                File::create(p)?;
            }
            Ok(Self {
                path: p.to_path_buf(),
                dim: 128,
            })
        }

        pub fn append(&mut self, vector: &[f32]) -> Result<()> {
            assert_eq!(vector.len(), self.dim, "vector dim mismatch");
            let mut file = OpenOptions::new()
                .append(true)
                .create(true)
                .open(&self.path)
                .map_err(|e| anyhow::anyhow!("Failed to open vector store file: {}", e))?;
            
            let bytes = bytemuck::cast_slice(vector);
            file.write_all(bytes)
                .map_err(|e| anyhow::anyhow!("Failed to write vector to file: {}", e))?;
            
            file.sync_all()
                .map_err(|e| anyhow::anyhow!("Failed to sync vector store to disk: {}", e))?;
                
            Ok(())
        }

        pub fn len(&self) -> Result<usize> {
            let mut file = File::open(&self.path)
                .map_err(|e| anyhow::anyhow!("Failed to open vector store file: {}", e))?;
            let size = file.seek(SeekFrom::End(0))?
                as usize;
            Ok(size / (self.dim * std::mem::size_of::<f32>()))
        }

        pub fn search(&self, query: &[f32], top_k: usize) -> Result<Vec<(usize, f32)>> {
            assert!(top_k > 0, "top_k must be > 0");
            assert_eq!(query.len(), self.dim, "query dim mismatch");
            
            let data = std::fs::read(&self.path)
                .map_err(|e| anyhow::anyhow!("Failed to read vector store file: {}", e))?;
                
            if data.is_empty() {
                return Ok(Vec::new());
            }
            
            let total_f32: &[f32] = bytemuck::cast_slice(&data);
            let num_vecs = total_f32.len() / self.dim;
            
            let mut heap: BinaryHeap<Reverse<(NotNan<f32>, usize)>> =
                BinaryHeap::with_capacity(top_k + 1);
                
            for i in 0..num_vecs {
                let start = i * self.dim;
                let end = start + self.dim;
                let vec_slice = &total_f32[start..end];
                
                let mut score = 0.0f32;
                for (a, b) in query.iter().zip(vec_slice.iter()) {
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
pub use fallback::Index;
