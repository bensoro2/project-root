use libc::{c_char, c_void};
use std::ffi::CString;

pub type SPFreshIndex = c_void;

extern "C" {
    pub fn spfresh_index_create(path: *const c_char) -> *mut SPFreshIndex;
    
    pub fn spfresh_index_append(index: *mut SPFreshIndex, vector: *const f32, dim: usize) -> i32;
    
    pub fn spfresh_index_search(
        index: *mut SPFreshIndex,
        query: *const f32,
        dim: usize,
        top_k: usize,
        result_indices: *mut usize,
        result_scores: *mut f32
    ) -> i32;
    
    pub fn spfresh_index_size(index: *mut SPFreshIndex) -> usize;
    pub fn spfresh_index_destroy(index: *mut SPFreshIndex);
}

#[derive(Debug)]
pub struct Index {
    ptr: *mut SPFreshIndex,
}

unsafe impl Send for Index {}
unsafe impl Sync for Index {}

impl Index {
    pub fn new<P: AsRef<str>>(path: P) -> Result<Self, String> {
        let path_str = path.as_ref();
        let c_path = CString::new(path_str)
            .map_err(|e| format!("Invalid path: {}", e))?;
        
        let ptr = unsafe { spfresh_index_create(c_path.as_ptr()) };
        if ptr.is_null() {
            return Err("Failed to create index".to_string());
        }
        
        Ok(Self { ptr })
    }
    
    pub fn append(&mut self, vector: &[f32]) -> Result<(), String> {
        let result = unsafe {
            spfresh_index_append(self.ptr, vector.as_ptr(), vector.len())
        };
        
        if result == 0 {
            Ok(())
        } else {
            Err(format!("Failed to append vector: error code {}", result))
        }
    }
    
    pub fn len(&self) -> usize {
        unsafe { spfresh_index_size(self.ptr) }
    }

    pub fn search(&self, query: &[f32], top_k: usize) -> Result<Vec<(usize, f32)>, String> {
        let available = self.len();
        if top_k == 0 || available == 0 {
            return Ok(Vec::new());
        }
        let k = std::cmp::min(top_k, available);
        
        let mut result_indices: Vec<usize> = vec![0; k];
        let mut result_scores: Vec<f32> = vec![0.0; k];
        
        let result = unsafe {
            spfresh_index_search(
                self.ptr,
                query.as_ptr(),
                query.len(),
                k,
                result_indices.as_mut_ptr(),
                result_scores.as_mut_ptr()
            )
        };
        
        if result == 0 {
            Ok(result_indices.into_iter().zip(result_scores.into_iter()).collect())
        } else if result == -1 {
            Ok(Vec::new())
        } else {
            Err(format!("Search failed: error code {}", result))
        }
    }
}

impl Drop for Index {
    fn drop(&mut self) {
        unsafe {
            spfresh_index_destroy(self.ptr);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_index_lifecycle() {
        let mut index = Index::new("test.index").expect("Failed to create index");
        
        let vector = vec![1.0f32; 768];
        index.append(&vector).expect("Failed to append vector");
        
        let results = index.search(&vector, 1).expect("Search failed");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 0);
    }
}