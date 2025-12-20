# Multi-stage build for minimal sloc-guard Docker image
# Target: ~10MB Alpine-based image with no Rust toolchain

# Stage 1: Build
FROM rust:1-alpine AS builder

# Install musl-dev for static linking
RUN apk add --no-cache musl-dev

WORKDIR /build

# Copy manifests first for better layer caching
COPY Cargo.toml Cargo.lock ./

# Create dummy src for dependency caching
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies only (cached layer)
RUN cargo build --release --locked && rm -rf src target/release/sloc-guard*

# Copy actual source
COPY src ./src

# Build the real binary
RUN cargo build --release --locked && \
    strip target/release/sloc-guard

# Stage 2: Runtime
FROM alpine:3.21

# Add ca-certificates for remote config fetching (HTTPS)
RUN apk add --no-cache ca-certificates

# Copy binary from builder
COPY --from=builder /build/target/release/sloc-guard /usr/local/bin/sloc-guard

# Verify binary works
RUN sloc-guard --version

# Set working directory for volume mounts
WORKDIR /workspace

# Default entrypoint
ENTRYPOINT ["sloc-guard"]
CMD ["--help"]
