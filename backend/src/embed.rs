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
            let embedding_size = 128;
            Ok(Self {
                model: Arc::new(Mutex::new(model)),
                embedding_size,
            })
        }
        #[cfg(not(feature = "fastembed"))]
        {
            let embedding_size = 128;
            Ok(Self {
                embedding_size,
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

    pub fn embed_reduced(&self, text: &str) -> Vec<f32> {
        let full = self.embed(text);
        let mut reduced = reduce_dim_768_to_128(&full);
        let norm: f32 = reduced.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut reduced {
                *v /= norm;
            }
        }
        reduced
    }

    pub fn embed_default(&self, text: &str) -> Vec<f32> {
        #[cfg(feature = "spfresh")]
        {
            let emb = self.embed_reduced(text);
            emb
        }
        #[cfg(not(feature = "spfresh"))]
        {
            let emb = self.embed_reduced(text);
            emb
        }
    }
    #[allow(dead_code)]
pub fn embedding_size(&self) -> usize {
        self.embedding_size
    }
}

fn reduce_dim_768_to_128(input: &[f32]) -> Vec<f32> {
    const OUT_DIM: usize = 128;
    if input.len() != 768 {
        return input.to_vec();
    }
    let mut output = Vec::with_capacity(OUT_DIM);
    for i in 0..OUT_DIM {
        let start = i * 6;
        let mean = (input[start] + input[start + 1] + input[start + 2] + input[start + 3] + input[start + 4] + input[start + 5]) / 6.0;
        output.push(mean);
    }
    output
}
