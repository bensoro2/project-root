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
            let options = InitOptions::new(EmbeddingModel::AllMiniLML6V2)
                .with_show_download_progress(true);
            let model = TextEmbedding::try_new(options)?;
            Ok(Self {
                model: Arc::new(Mutex::new(model)),
                embedding_size: 384,
            })
        }
        #[cfg(not(feature = "fastembed"))]
        {
            Ok(Self {
                embedding_size: 384,
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
                        embedding
                    } else {
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
            vec![0.0_f32; self.embedding_size]
        }
    }
    
    #[allow(dead_code)]
pub fn embedding_size(&self) -> usize {
        self.embedding_size
    }
}
