# Multi-stage build for Fukura CLI
FROM rust:1.90 AS builder

WORKDIR /app

# Install system dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy Cargo files first for better caching
COPY Cargo.toml Cargo.lock ./

# Create dummy source for dependency caching
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN mkdir benches && echo "fn main() {}" > benches/search_benchmark.rs

# Build dependencies only (this layer will be cached)
RUN cargo build --release --offline || cargo build --release
RUN rm -rf src benches

# Copy actual source code
COPY src ./src
COPY benches ./benches
COPY tests ./tests
COPY dist-workspace.toml ./dist-workspace.toml
COPY deny.toml ./deny.toml
COPY scripts ./scripts
COPY installers ./installers

# Build the application (only rebuild if source changed)
RUN cargo build --release --offline

# --- Final stage ---
FROM debian:stable-slim

# Install runtime dependencies if any
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the built binaries from the builder stage
COPY --from=builder /app/target/release/fukura ./fukura
# Create fuku symlink
RUN ln -sf fukura fuku

# Run the application
ENTRYPOINT ["./fukura"]
CMD ["--help"]

