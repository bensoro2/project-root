use anyhow::Result;

#[cfg(feature = "fastembed")]
use fastembed::{InitOptions, TextEmbedding, EmbeddingModel};

#[cfg(feature = "fastembed")]
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Embedder {
    #[cfg(feature = "fastembed")]
    model: Arc<Mutex<TextEmbedding>>,
    embedding_size: usize,
}

impl Embedder {
    pub fn new() -> Result<Self> {
        #[cfg(feature = "fastembed")]
        {
            let options = InitOptions::new(EmbeddingModel::MultilingualE5Base)
                .with_show_download_progress(true);
            let model = TextEmbedding::try_new(options)?;
            Ok(Self {
                model: Arc::new(Mutex::new(model)),
                embedding_size: 768,
            })
        }
        #[cfg(not(feature = "fastembed"))]
        {
            Ok(Self {
                embedding_size: 768,
            })
        }
    }

    /// Full 768-dim embedding
    pub fn embed(&self, text: &str) -> Vec<f32> {
        if text.trim().is_empty() {
            return vec![0.0_f32; self.embedding_size];
        }
        
        #[cfg(feature = "fastembed")]
        {
            let mut guard = self.model.lock().expect("Failed to acquire model lock");
            match guard.embed(vec![text], None) {
                Ok(mut embeddings) => {
                    if let Some(embedding) = embeddings.pop() {
                        tracing::debug!(embedding_length = embedding.len(), "Embedding generated successfully");
                        // Debug: Log first few values of embedding
                        if embedding.len() > 5 {
                            tracing::debug!(first_values = ?&embedding[0..5], "First few embedding values");
                        }
                        embedding
                    } else {
                        tracing::warn!("Empty embedding returned from model");
                        vec![0.0_f32; self.embedding_size]
                    }
                },
                Err(e) => {
                    tracing::error!("Failed to generate embedding: {}", e);
                    vec![0.0_f32; self.embedding_size]
                }
            }
        }
        #[cfg(not(feature = "fastembed"))]
        {
            tracing::warn!("Using zero embedding (fastembed feature not enabled)");
            vec![0.0_f32; self.embedding_size]
        }
    }

    /// Return 256-dim embedding by simple mean-pooling every 3 dimensions
    pub fn embed_reduced(&self, text: &str) -> Vec<f32> {
        let full = self.embed(text);
        reduce_dim_768_to_256(&full)
    }

    /// Return embedding appropriate for current index implementation
    pub fn embed_default(&self, text: &str) -> Vec<f32> {
        #[cfg(feature = "spfresh")]
        { self.embed(text) }
        #[cfg(not(feature = "spfresh"))]
        { self.embed_reduced(text) }
    }
    
    #[allow(dead_code)]
pub fn embedding_size(&self) -> usize {
        self.embedding_size
    }
}

/// Downscale a 768-dim vector to 256 dimensions by averaging every consecutive 3 values.
fn reduce_dim_768_to_256(input: &[f32]) -> Vec<f32> {
    const OUT_DIM: usize = 256;
    if input.len() != 768 {
        return input.to_vec();
    }
    let mut output = Vec::with_capacity(OUT_DIM);
    for i in 0..OUT_DIM {
        let start = i * 3;
        let mean = (input[start] + input[start + 1] + input[start + 2]) / 3.0;
        output.push(mean);
    }
    output
}
