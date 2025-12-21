# syntax=docker/dockerfile:1
# Multi-stage build for minimal sloc-guard Docker image
# Features:
# - cargo-chef for optimal dependency caching
# - non-root user for security
# - alpine-based for small size (~10MB)

# Stage 1: Chef - Prepare cargo-chef
FROM rust:1-alpine AS chef
RUN apk add --no-cache musl-dev
RUN cargo install cargo-chef
WORKDIR /app

# Stage 2: Planner - Generate recipe
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Builder - Build dependencies and binary
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the cached layer
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --locked --bin sloc-guard && \
    strip target/release/sloc-guard

# Stage 4: Runtime - Minimal runtime image
FROM alpine:3.21 AS runtime

# Install dependencies and create non-root user
RUN apk add --no-cache ca-certificates && \
    adduser -D -g '' appuser

# Copy binary from builder
COPY --from=builder /app/target/release/sloc-guard /usr/local/bin/sloc-guard

# Set user and working directory
USER appuser
WORKDIR /home/appuser

# Verify binary works
RUN sloc-guard --version

ENTRYPOINT ["sloc-guard"]
CMD ["--help"]
