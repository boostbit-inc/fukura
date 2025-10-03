# Multi-stage build for Fukura CLI
FROM rust:1.90-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev pkgconfig

# Create app directory
WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build for release
RUN cargo build --release

# Runtime stage
FROM alpine:latest

# Install runtime dependencies
RUN apk add --no-cache ca-certificates

# Create non-root user
RUN addgroup -g 1001 -S fukura && \
    adduser -u 1001 -S fukura -G fukura

# Copy binary from builder stage
COPY --from=builder /app/target/release/fukura /usr/local/bin/fukura
COPY --from=builder /app/target/release/fuku /usr/local/bin/fuku

# Make binaries executable
RUN chmod +x /usr/local/bin/fukura /usr/local/bin/fuku

# Switch to non-root user
USER fukura

# Set working directory
WORKDIR /home/fukura

# Default command
ENTRYPOINT ["fukura"]
CMD ["--help"]

