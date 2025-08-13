use anyhow::Result;

#[cfg(feature = "fastembed")]
use fastembed::{InitOptions, TextEmbedding, EmbeddingModel};

#[cfg(feature = "fastembed")]
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Embedder {
    #[cfg(feature = "fastembed")]
    model: Arc<Mutex<TextEmbedding>>,
}

impl Embedder {
    pub fn new() -> Result<Self> {
        #[cfg(feature = "fastembed")]
        {
            let options = InitOptions::new(EmbeddingModel::AllMiniLML6V2)
                .with_show_download_progress(true);
            let model = TextEmbedding::try_new(options)?;
            Ok(Self { model: Arc::new(Mutex::new(model)) })
        }
        #[cfg(not(feature = "fastembed"))]
        {
            Ok(Self {})
        }
    }

    pub fn embed(&self, text: &str) -> Vec<f32> {
        #[cfg(feature = "fastembed")]
        {
            let mut guard = self.model.lock().unwrap();
            match guard.embed(vec![text], None) {
                Ok(mut v) => v.remove(0),
                Err(_) => vec![0.0_f32; 384],
            }
        }
        #[cfg(not(feature = "fastembed"))]
        {
            vec![0.0_f32; 384]
        }
    }
}
