# Review Semantic Search Platform

A lightweight, file-based semantic search engine built entirely in Rust for product reviews. This platform enables semantic search over user reviews without relying on external databases, using append-only files for both vector embeddings and metadata storage.

## Key Features

- **Semantic Search**: Find relevant reviews based on meaning rather than keywords
- **No Database Required**: All data stored in append-only files (`reviews.index` for vectors, `reviews.jsonl` for metadata)
- **Rust Full-Stack**: Backend (Axum), Frontend (Leptos), and Vector Search (spfresh) all in Rust
- **Embedding Support**: Uses [fastembed-rs](https://crates.io/crates/fastembed) for efficient text embeddings
- **Docker Ready**: Full Docker Compose setup for easy deployment
- **Extensible Architecture**: Designed to easily swap vector search implementations

## Architecture

```
project-root/
├── backend/          # Rust + Axum web server, fastembed, spfresh
│   ├── src/
│   │   ├── main.rs           # Server entry point, routes
│   │   ├── handlers.rs       # Request handlers
│   │   ├── embed.rs          # Embedding generation
│   │   └── storage/
│   │       ├── vector_store.rs   # Current naive vector storage
│   │       ├── metadata.rs       # JSONL metadata storage
│   │       └── mod.rs            # Storage module
│   ├── data/                 # Persistent data storage
│   │   ├── reviews.index     # Binary f32 vectors (append-only)
│   │   └── reviews.jsonl     # JSON Lines metadata (1-line per review)
│   ├── spfresh/              # Future vector search implementation
│   │   └── src/lib.rs        # spfresh core logic
│   ├── Cargo.toml            # Dependencies and features
│   ├── Cargo.lock
│   └── Dockerfile            # Containerization
├── frontend/                 # Leptos SPA (Client-Side Rendering)
│   ├── src/
│   │   ├── main.rs           # WASM entry point
│   │   └── lib.rs            # Main application logic
│   ├── index.html            # HTML template
│   ├── style.css             # Styling
│   ├── Cargo.toml            # Frontend dependencies
│   └── Trunk.toml            # Build configuration
└── docker-compose.yml        # Orchestrates backend service
```

## Running Locally (Native)

```bash
# Compile quickly with dummy embedder (for development)
$ cargo run --manifest-path backend/Cargo.toml

# Compile with real embeddings (downloads model ~80 MB first time)
$ cargo run --manifest-path backend/Cargo.toml --features fastembed
```

The server listens on `0.0.0.0:8000` and writes data files into `backend/data/`.

## API Reference

### 1. Insert Single Review
Inserts a single product review into the system.

**Endpoint**: `POST /reviews`  
**Content-Type**: `application/json`

**Request Body**:
```json
{
  "review_title": "Great phone",
  "review_body": "Battery lasts long",
  "product_id": "P123",
  "review_rating": 5
}
```

**Fields**:
- `review_title` (string): Title of the review
- `review_body` (string): Main content of the review
- `product_id` (string): ID of the reviewed product
- `review_rating` (integer): Rating from 1-5

**Response**: `201 Created` (no response body)

### 2. Bulk Insert Reviews
Inserts multiple reviews in a single request.

**Endpoint**: `POST /reviews/bulk`  
**Content-Type**: `application/json`

**Request Body**:
```json
[
  {
    "review_title": "Great phone",
    "review_body": "Battery lasts long",
    "product_id": "P123",
    "review_rating": 5
  },
  {
    "review_title": "Poor camera quality",
    "review_body": "Photos are blurry",
    "product_id": "P123",
    "review_rating": 2
  }
]
```

**Response**: `201 Created` (no response body)

### 3. Semantic Search
Searches for reviews semantically similar to the query.

**Endpoint**: `POST /search`  
**Content-Type**: `application/json`

**Request Body**:
```json
{
  "query": "long lasting battery phone",
  "top_k": 5
}
```

**Fields**:
- `query` (string): Search query in natural language
- `top_k` (integer): Maximum number of results to return

**Response**:
```json
[
  {
    "score": 0.83,
    "review": {
      "review_title": "Great phone",
      "review_body": "Battery lasts long",
      "product_id": "P123",
      "review_rating": 5
    }
  }
]
```

**Score**: Relevance score between 0-1, where 1 is most relevant.

## Docker Compose

```bash
$ docker compose up --build
```

- `backend` image built from `backend/Dockerfile` with fastembed enabled
- `backend/data` directory mounted as a volume for data persistence
- Service exposed on port 8000

## Development Setup

1. Install Rust: https://rust-lang.org/tools/install
2. Clone this repository
3. Run backend: `cargo run --manifest-path backend/Cargo.toml --features fastembed`
4. For frontend development: `trunk serve --open` in the frontend directory

## Replacing VectorStore with spfresh

The current vector storage uses a naive brute-force search implementation. The project is designed to seamlessly replace this with the more efficient `spfresh` implementation:

1. **Implement spfresh binding**:
   - Add C++ binding crate or submodule inside `backend/spfresh/`
   - Ensure it provides `append()` and `search()` methods

2. **Create wrapper API**:
   - Implement a wrapper that matches the current `VectorStore` trait
   - Maintain identical method signatures for seamless integration

3. **Swap implementation**:
   - Replace `use storage::vector_store::VectorStore` with the new spfresh implementation
   - No changes required to handlers or other components

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests.

## License

MIT License
