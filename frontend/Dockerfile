# ---------- Frontend Dockerfile (Leptos + Trunk) ----------
# Build stage
FROM rustlang/rust:nightly-slim AS builder
WORKDIR /app

# Install system build dependencies then Trunk and wasm target for WASM build
RUN apt-get update && apt-get install -y pkg-config libssl-dev build-essential && rm -rf /var/lib/apt/lists/*

# Install Trunk and wasm target for WASM build
# Pin trunk version compatible with Rust 1.77
RUN cargo install trunk --version 0.21.5 --locked \
    && cargo install wasm-bindgen-cli --locked \
    && rustup target add wasm32-unknown-unknown

# Copy source
COPY . .

# Build release static assets
RUN trunk build --release --public-url /

# ---------- Runtime stage ----------
FROM nginx:1.25-alpine
# Copy the compiled static files
# Copy static files
COPY --from=builder /app/dist /usr/share/nginx/html
# Copy custom nginx config for API proxy
COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
