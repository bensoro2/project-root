use axum::{routing::post, Router, serve};
use std::net::SocketAddr;
use tokio::net::TcpListener;

mod handlers;
mod embed;
mod storage;

use handlers::{bulk_insert_reviews, insert_review, search_reviews};
use storage::{metadata::MetadataStore, vector_store::VectorStore};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let data_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data");
    std::fs::create_dir_all(&data_dir)?;

    let embedder = embed::Embedder::new()?;
    let vector_store = VectorStore::open_or_create(data_dir.join("reviews.index"))?;
    let metadata_store = MetadataStore::open_or_create(data_dir.join("reviews.jsonl"))?;

    let app_state = handlers::AppStateInner::new(embedder, vector_store, metadata_store);

    let app = Router::new()
        .route("/reviews", post(insert_review))
        .route("/reviews/bulk", post(bulk_insert_reviews))
        .route("/search", post(search_reviews))
        .with_state(app_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    let listener = TcpListener::bind(addr).await?;
    tracing::info!("Listening on {}", listener.local_addr()?);
    serve(listener, app).await?;
    Ok(())
}
