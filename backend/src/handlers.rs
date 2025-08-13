use std::sync::{Arc, Mutex};

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

use crate::embed::Embedder;
use crate::storage::{metadata::MetadataStore, vector_store::VectorStore};


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

pub async fn insert_review(State(state): State<AppState>, Json(review): Json<Review>) -> Result<impl IntoResponse, (StatusCode, String)> {
    let embedding = state.embedder.embed(&format!("{} {}", review.review_title, review.review_body));
    {
        let mut vs = state.vector_store.lock().unwrap();
        vs.append(&embedding).map_err(internal_error)?;
    }
    {
        let mut ms = state.metadata_store.lock().unwrap();
        ms.append(&review).map_err(internal_error)?;
    }

    Ok(StatusCode::CREATED)
}

pub async fn bulk_insert_reviews(State(state): State<AppState>, Json(reviews): Json<Vec<Review>>) -> Result<impl IntoResponse, (StatusCode, String)> {
    for review in &reviews {
        let embedding = state.embedder.embed(&format!("{} {}", review.review_title, review.review_body));
        {
            let mut vs = state.vector_store.lock().unwrap();
            vs.append(&embedding).map_err(internal_error)?;
        }
        {
            let mut ms = state.metadata_store.lock().unwrap();
            ms.append(review).map_err(internal_error)?;
        }
    }
    Ok(StatusCode::CREATED)
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub top_k: Option<usize>,
}

pub async fn search_reviews(State(state): State<AppState>, Json(query): Json<SearchQuery>) -> Result<impl IntoResponse, (StatusCode, String)> {
    let embedding = state.embedder.embed(&query.query);
    let top_k = query.top_k.unwrap_or(5);

    let ids_scores = {
        let vs = state.vector_store.lock().unwrap();
        vs.search(&embedding, top_k).map_err(internal_error)?
    };

    let mut results = Vec::new();
    {
        let ms = state.metadata_store.lock().unwrap();
        for (idx, score) in ids_scores {
            let review: Review = ms.get_by_index(idx).map_err(internal_error)?;
            results.push(SearchResult { score, review });
        }
    }

    Ok(Json(results))
}

fn internal_error<E: std::fmt::Display>(err: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
