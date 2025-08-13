use anyhow::Result;
use ordered_float::NotNan;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Index {
    path: PathBuf,
    dim: usize,
}

impl Index {
    pub fn open_or_create<P: AsRef<Path>>(path: P) -> Result<Self> {
        let p = path.as_ref();
        if !p.exists() {
            File::create(p)?;
        }
        Ok(Self {
            path: p.to_path_buf(),
            dim: 768,
        })
    }

    pub fn append(&mut self, vector: &[f32]) -> Result<()> {
        assert_eq!(vector.len(), self.dim, "vector dim mismatch");
        let mut file = OpenOptions::new().append(true).open(&self.path)?;
        let bytes = bytemuck::cast_slice(vector);
        file.write_all(bytes)?;
        Ok(())
    }

    pub fn search(&self, query: &[f32], top_k: usize) -> Result<Vec<(usize, f32)>> {
        assert_eq!(query.len(), self.dim, "query dim mismatch");
        let data = std::fs::read(&self.path)?;
        if data.is_empty() {
            return Ok(Vec::new());
        }
        let total_f32: &[f32] = bytemuck::cast_slice(&data);
        let num_vecs = total_f32.len() / self.dim;
        let mut heap: std::collections::BinaryHeap<(NotNan<f32>, usize)> =
            std::collections::BinaryHeap::with_capacity(top_k + 1);
        for i in 0..num_vecs {
            let start = i * self.dim;
            let end = start + self.dim;
            let vec_slice = &total_f32[start..end];
            let mut score = 0.0f32;
            for (a, b) in query.iter().zip(vec_slice.iter()) {
                score += a * b;
            }
            if let Ok(not_nan) = NotNan::new(score) {
                heap.push((not_nan, i));
                if heap.len() > top_k {
                    heap.pop();
                }
            }
        }
        let mut results: Vec<(usize, f32)> =
            heap.into_iter().map(|(s, i)| (i, s.into_inner())).collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        Ok(results)
    }
}
