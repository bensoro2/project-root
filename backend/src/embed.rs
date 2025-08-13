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
    
    #[allow(dead_code)]
pub fn embedding_size(&self) -> usize {
        self.embedding_size
    }
}
