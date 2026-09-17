# syntax=docker/dockerfile:1
FROM rust:1.97-bookworm AS builder
WORKDIR /app

# Cache dependencies: copy manifests first, build a stub, then the real sources.
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    cargo build --release || true && \
    rm -rf src

COPY migrations ./migrations
COPY src ./src
RUN touch src/main.rs && cargo build --release

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/work-assistant-backend /usr/local/bin/backend
EXPOSE 8080
ENV RUST_LOG=info
CMD ["backend"]
