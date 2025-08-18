use axum::{routing::post, Router, serve};
use tower_http::cors::{CorsLayer, Any};
use axum::http::Method;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use std::env;


use backend::handlers::{bulk_insert_reviews, insert_review, search_reviews};
use backend::handlers as handlers;
#[cfg(feature = "fastembed")]
use backend::bulk_insert;
use backend::embed::Embedder;
use backend::storage::{metadata::MetadataStore, vector_store::VectorStore};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let data_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data");
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| anyhow::anyhow!("Failed to create data directory: {}", e))?;
    let embedder = Embedder::new()
        .map_err(|e| anyhow::anyhow!("Failed to initialize embedder: {}", e))?;
    #[cfg(feature = "spfresh")]
    let index_path = data_dir.join("reviews");
    #[cfg(not(feature = "spfresh"))]
    let index_path = data_dir.join("reviews.vectors");
    let vector_store = VectorStore::open_or_create(index_path)
        .map_err(|e| anyhow::anyhow!("Failed to open or create vector store: {}", e))?;
    let metadata_store = MetadataStore::open_or_create(data_dir.join("reviews.jsonl"))
        .map_err(|e| anyhow::anyhow!("Failed to open or create metadata store: {}", e))?;
    let app_state = handlers::AppStateInner::new(embedder, vector_store, metadata_store);

    let api_routes = Router::new()
        .route("/reviews", post(insert_review))
        .route("/reviews/bulk", post(bulk_insert_reviews))
        .route("/search", post(search_reviews));

    let app = Router::new()
        .nest("/api", api_routes)
        .with_state(app_state)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
                .allow_headers(Any)
        );

    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    let listener = TcpListener::bind(addr).await
        .map_err(|e| anyhow::anyhow!("Failed to bind to address {}: {}", addr, e))?;
    #[cfg(feature = "fastembed")]
    if env::args().any(|arg| arg == "--insert-bitcoin-tweets") {
        tokio::spawn(async {
            if let Err(e) = bulk_insert::insert_bitcoin_tweets().await {
            }
        });
    }
    
    serve(listener, app).await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))
}
