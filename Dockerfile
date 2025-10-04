# Multi-stage build for Fukura CLI
FROM rust:1.90 AS builder

WORKDIR /app

# Install system dependencies for cross-compilation (if needed)
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy Cargo.toml and Cargo.lock to leverage Docker cache
COPY Cargo.toml Cargo.lock ./

# Create dummy files to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN mkdir benches && echo "fn main() {}" > benches/search_benchmark.rs
RUN cargo build --release
RUN rm -rf src benches

# Copy the actual source code
COPY src ./src
COPY benches ./benches
COPY tests ./tests
COPY dist.toml ./dist.toml
COPY deny.toml ./deny.toml
COPY scripts ./scripts
COPY installers ./installers

# Build the application
RUN cargo build --release

# --- Final stage ---
FROM debian:stable-slim

# Install runtime dependencies if any
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the built binaries from the builder stage
COPY --from=builder /app/target/release/fukura ./fukura
COPY --from=builder /app/target/release/fuku ./fuku

# Run the application
ENTRYPOINT ["./fukura"]
CMD ["--help"]

