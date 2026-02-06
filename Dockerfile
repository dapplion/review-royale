# Build stage
FROM rust:1-bookworm as builder

WORKDIR /app

# Copy manifests and migrations
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY migrations ./migrations

# Build
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
COPY --from=builder /app/migrations ./migrations

# Copy static files
COPY --from=builder /app/crates/api/static ./static

EXPOSE 3000

CMD ["/app/review-royale-api"]
