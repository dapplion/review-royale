# Build stage
FROM rust:1.83-bookworm as builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build dependencies (this layer will be cached)
RUN cargo build --release --package api

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/review-royale-api /app/review-royale-api

# Copy migrations
COPY migrations ./migrations

EXPOSE 3000

CMD ["/app/review-royale-api"]
