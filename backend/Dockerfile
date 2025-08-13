# ---------- Backend Dockerfile (Rust + fastembed) ----------
# Build stage
FROM rust:1.77 as builder
WORKDIR /app
# Speed up incremental builds by pre-copying Cargo manifests
COPY Cargo.toml Cargo.lock ./
COPY src ./src
# Enable fastembed at build; comment out if you want dummy embedder
RUN cargo build --release --features fastembed

# ---------- Runtime stage ----------
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/backend ./backend
# Create data directory inside container (mounted by docker-compose)
RUN mkdir /app/data
EXPOSE 8000
ENTRYPOINT ["/app/backend"]
