use std::sync::{Arc, Mutex};

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

use axum::extract::rejection::JsonRejection;

pub async fn insert_review(
    State(state): State<AppState>,
    payload: Result<Json<Review>, JsonRejection>,
) -> Result<impl IntoResponse, AppError> {
    let Json(review) = payload.map_err(AppError::from)?;
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
    let embedding = state.embedder.embed(&text);
    
    {
        let mut vs = state.vector_store.lock().map_err(|_| AppError::Internal(anyhow::anyhow!("Failed to acquire vector store lock")))?;
        vs.append(&embedding).map_err(|e| AppError::Internal(e))?;
    }
    
    {
        let mut ms = state.metadata_store.lock().map_err(|_| AppError::Internal(anyhow::anyhow!("Failed to acquire metadata store lock")))?;
        ms.append(&review).map_err(|e| AppError::Internal(e))?;
    }

    Ok((StatusCode::CREATED, Json(serde_json::json!({"status": "success", "message": "Review created successfully"}))))
}

pub async fn bulk_insert_reviews(
    State(state): State<AppState>,
    payload: Result<Json<Vec<Review>>, JsonRejection>,
) -> Result<impl IntoResponse, AppError> {
    let Json(reviews) = payload.map_err(AppError::from)?;
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
        let embedding = state.embedder.embed(&text);
        
        vs.append(&embedding).map_err(|e| AppError::Internal(e))?;
        ms.append(review).map_err(|e| AppError::Internal(e))?;
    }

    Ok(Json(serde_json::json!({"status": "success", "message": "Reviews created successfully", "count": reviews.len()})))
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
    payload: Result<Json<SearchQuery>, JsonRejection>,
) -> Result<impl IntoResponse, AppError> {
    let Json(query) = payload.map_err(AppError::from)?;
    query.validate()?;

    // (query validated below)
let embedding = state.embedder.embed(&query.query.trim());

    let ids_scores = {
        let vs = state.vector_store.lock().map_err(|_| AppError::Internal(anyhow::anyhow!("Failed to acquire vector store lock")))?;
        vs.search(&embedding, query.top_k).map_err(|e| AppError::Internal(e))?
    };

    let mut results = Vec::new();
    {
        let ms = state.metadata_store.lock().map_err(|_| AppError::Internal(anyhow::anyhow!("Failed to acquire metadata store lock")))?;
        for (idx, score) in ids_scores {
            let review: Review = ms.get_by_index(idx).map_err(|e| AppError::Internal(e))?;
            results.push(SearchResult { score, review });
        }
    }

    Ok(Json(results))
}

