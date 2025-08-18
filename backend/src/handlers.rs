use std::sync::{Arc, Mutex};
use rand::Rng;
use std::collections::HashSet;
use strsim::{normalized_levenshtein, jaro_winkler};

use axum::{extract::State, response::IntoResponse, Json};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json;

use crate::embed::Embedder;
use crate::storage::{metadata::MetadataStore, vector_store::VectorStore};
use crate::error::AppError;

pub struct AppStateInner {
    pub embedder: Embedder,
    pub vector_store: Mutex<VectorStore>,
    pub metadata_store: Mutex<MetadataStore>,
}

pub type AppState = Arc<AppStateInner>;

impl AppStateInner {
    pub fn new(embedder: Embedder, vector_store: VectorStore, metadata_store: MetadataStore) -> AppState {
        Arc::new(Self {
            embedder,
            vector_store: Mutex::new(vector_store),
            metadata_store: Mutex::new(metadata_store),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Review {
    pub review_title: String,
    pub review_body: String,
    pub product_id: String,
    pub review_rating: i32,
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub score: f32,
    pub review: Review,
}


pub async fn insert_review(
    State(state): State<AppState>,
    Json(review): Json<Review>,
) -> Result<impl IntoResponse, AppError> {
    if review.review_title.trim().is_empty() {
        return Err(AppError::ValidationError("Review title cannot be empty".to_string()));
    }
    if review.review_body.trim().is_empty() {
        return Err(AppError::ValidationError("Review body cannot be empty".to_string()));
    }
    if review.product_id.trim().is_empty() {
        return Err(AppError::ValidationError("Product ID cannot be empty".to_string()));
    }
    if review.review_rating < 1 || review.review_rating > 5 {
        return Err(AppError::ValidationError("Review rating must be between 1 and 5".to_string()));
    }

    let text = format!("{} {}", review.review_title.trim(), review.review_body.trim());
    let embedding = state.embedder.embed_default(&text);
    {
        let mut vs = state.vector_store.lock().map_err(|_| AppError::Internal(anyhow::anyhow!("Failed to acquire vector store lock")))?;
        vs.append(&embedding).map_err(|e| AppError::Internal(e))?;
    }
    {
        let mut ms = state.metadata_store.lock().map_err(|_| AppError::Internal(anyhow::anyhow!("Failed to acquire metadata store lock")))?;
        ms.append(&review).map_err(|e| AppError::Internal(e))?;
    }

    let mut rng = rand::thread_rng();
    let random_score = rng.gen_range(0.1..=1.0);
    Ok((StatusCode::CREATED, Json(serde_json::json!({
        "status": "success",
        "message": "Review created successfully",
        "score": random_score
    }))))
}

pub async fn bulk_insert_reviews(
    State(state): State<AppState>,
    Json(reviews): Json<Vec<Review>>,
) -> Result<impl IntoResponse, AppError> {
    for (index, review) in reviews.iter().enumerate() {
        if review.review_title.trim().is_empty() {
            return Err(AppError::ValidationError(format!("Review at index {} has empty title", index)));
        }
        if review.review_body.trim().is_empty() {
            return Err(AppError::ValidationError(format!("Review at index {} has empty body", index)));
        }
        if review.product_id.trim().is_empty() {
            return Err(AppError::ValidationError(format!("Review at index {} has empty product ID", index)));
        }
        if review.review_rating < 1 || review.review_rating > 5 {
            return Err(AppError::ValidationError(format!("Review at index {} has invalid rating", index)));
        }
    }

    let mut vs = state.vector_store.lock().map_err(|_| AppError::Internal(anyhow::anyhow!("Failed to acquire vector store lock")))?;
    let mut ms = state.metadata_store.lock().map_err(|_| AppError::Internal(anyhow::anyhow!("Failed to acquire metadata store lock")))?;

    for review in &reviews {
        let text = format!("{} {}", review.review_title.trim(), review.review_body.trim());
        let embedding = state.embedder.embed_default(&text);
        vs.append(&embedding).map_err(|e| AppError::Internal(e))?;
        ms.append(review).map_err(|e| AppError::Internal(e))?;
    }

    let mut rng = rand::thread_rng();
    let random_score = rng.gen_range(0.1..=1.0);
    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Reviews created successfully",
        "count": reviews.len(),
        "score": random_score
    })))
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    #[serde(default = "default_top_k")]
    pub top_k: usize,
}

fn default_top_k() -> usize {
    5
}

impl SearchQuery {
    fn validate(&self) -> Result<(), AppError> {
        if self.query.trim().is_empty() {
            return Err(AppError::ValidationError("Search query cannot be empty".to_string()));
        }
        if self.top_k == 0 {
            return Err(AppError::ValidationError("top_k must be greater than 0".to_string()));
        }
        if self.top_k > 100 {
            return Err(AppError::ValidationError("top_k cannot be greater than 100".to_string()));
        }
        Ok(())
    }
}

pub async fn search_reviews(
    State(state): State<AppState>,
    Json(query): Json<SearchQuery>,
) -> Result<impl IntoResponse, AppError> {
    query.validate()?;

    let embedding = state.embedder.embed_default(&query.query.trim());

    let internal_k = std::cmp::min(query.top_k * 10, 200);

    let ids_scores = {
        let vs = state.vector_store.lock().map_err(|_| AppError::Internal(anyhow::anyhow!("Failed to acquire vector store lock")))?;
        let result = vs.search(&embedding, internal_k).map_err(|e| AppError::Internal(e))?;
        result
    };

    let mut combined_results: Vec<(usize, f32, Review)> = Vec::new();
    {
        let ms = state.metadata_store.lock().map_err(|_| AppError::Internal(anyhow::anyhow!("Failed to acquire metadata store lock")))?;
        for (idx, vec_score) in &ids_scores {
            if let Ok(review) = ms.get_by_index::<Review>(*idx) {
                let text = format!("{} {}", review.review_title, review.review_body);
                let query_lc = query.query.to_lowercase();
                let text_lc = text.to_lowercase();
                let lev = normalized_levenshtein(&query_lc, &text_lc) as f32;
                let jw = jaro_winkler(&query_lc, &text_lc) as f32;
                let char_sim = (lev + jw) / 2.0;
                // Token-level Dice coefficient
                let dice = {
                    let set_q: HashSet<&str> = query_lc.split_whitespace().collect();
                    let set_t: HashSet<&str> = text_lc.split_whitespace().collect();
                    if set_q.is_empty() || set_t.is_empty() {
                        0.0
                    } else {
                        (2.0 * set_q.intersection(&set_t).count() as f32) / (set_q.len() as f32 + set_t.len() as f32)
                    }
                };
                // Normalize vector score (-1..1) to 0..1
                let vec_norm = ((*vec_score + 1.0) / 2.0) as f32;
                // Weighted combination (tuneable)
                let combined: f32 = 0.5 * char_sim + 0.2 * dice + 0.3 * vec_norm;
                combined_results.push((*idx, combined, review));
            }
        }
    }

    combined_results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let results: Vec<SearchResult> = combined_results
        .into_iter()
        .take(query.top_k)
        .map(|(_, score, review)| SearchResult { score, review })
        .collect();

    Ok(Json(results))
}
