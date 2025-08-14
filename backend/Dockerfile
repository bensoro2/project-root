# ---------- Backend Dockerfile (Rust + fastembed) ----------
# Build stage
FROM rustlang/rust:nightly-slim AS builder
WORKDIR /app
# Speed up incremental builds by pre-copying Cargo manifests
COPY Cargo.toml Cargo.lock ./
COPY src ./src
# Include local SPFresh bindings
COPY spfresh_local ./spfresh_local
COPY spfresh-sys ./spfresh-sys
# Enable fastembed at build; comment out if you want dummy embedder
# Install build dependencies for crates that rely on OpenSSL
RUN apt-get update && apt-get install -y pkg-config libssl-dev build-essential clang && rm -rf /var/lib/apt/lists/*

RUN cargo build --release --locked --features fastembed

# ---------- Runtime stage ----------
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 libstdc++6 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/backend ./backend
# Create data directory inside container (mounted by docker-compose)
RUN mkdir /app/data
EXPOSE 8000
ENTRYPOINT ["/app/backend"]