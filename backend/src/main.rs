use axum::{routing::post, Router, serve};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use std::env;

mod handlers;
mod embed;
mod storage;
mod error;
mod bulk_insert;

use handlers::{bulk_insert_reviews, insert_review, search_reviews};
use storage::{metadata::MetadataStore, vector_store::VectorStore};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let data_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data");
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| anyhow::anyhow!("Failed to create data directory: {}", e))?;

    let embedder = embed::Embedder::new()
        .map_err(|e| anyhow::anyhow!("Failed to initialize embedder: {}", e))?;
    #[cfg(feature = "spfresh")]
    let vector_store = VectorStore::open_or_create(data_dir.join("reviews.index"))
        .map_err(|e| anyhow::anyhow!("Failed to open or create vector store: {}", e))?;
    #[cfg(not(feature = "spfresh"))]
    let vector_store = VectorStore::open_or_create(data_dir.join("reviews.index"))
        .map_err(|e| anyhow::anyhow!("Failed to open or create vector store: {}", e))?;
    let metadata_store = MetadataStore::open_or_create(data_dir.join("reviews.jsonl"))
        .map_err(|e| anyhow::anyhow!("Failed to open or create metadata store: {}", e))?;

    let app_state = handlers::AppStateInner::new(embedder, vector_store, metadata_store);

    // Build our application with error handling
    let app = Router::new()
        .route("/reviews", post(insert_review))
        .route("/reviews/bulk", post(bulk_insert_reviews))
        .route("/search", post(search_reviews))
        .with_state(app_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    let listener = TcpListener::bind(addr).await
        .map_err(|e| anyhow::anyhow!("Failed to bind to address {}: {}", addr, e))?;
    
    tracing::info!("Listening on http://{}", listener.local_addr()?);
    
    // Check if we should insert bitcoin tweets
    if env::args().any(|arg| arg == "--insert-bitcoin-tweets") {
        tokio::spawn(async {
            if let Err(e) = bulk_insert::insert_bitcoin_tweets().await {
                eprintln!("Error inserting bitcoin tweets: {}", e);
            }
        });
    }
    
    serve(listener, app).await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))
}
