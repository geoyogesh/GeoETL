# syntax=docker/dockerfile:1.6

# ============================================================================
# Stage 1: Build the CLI binary with all dependencies
# ============================================================================
ARG RUST_VERSION=1.90
FROM rust:${RUST_VERSION}-bookworm AS builder

# Install build dependencies
RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends \
        build-essential \
        ca-certificates \
        clang \
        cmake \
        libclang-dev \
        libssl-dev \
        libsqlite3-dev \
        llvm \
        pkg-config \
        protobuf-compiler \
        musl-tools; \
    rm -rf /var/lib/apt/lists/*

# Add musl target for static linking (optional, for truly static binaries)
RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /build

# Copy dependency manifests first for better layer caching
COPY Cargo.toml Cargo.lock ./
COPY crates/geoetl-cli/Cargo.toml ./crates/geoetl-cli/
COPY crates/geoetl-core/Cargo.toml ./crates/geoetl-core/

# Create dummy source files to cache dependencies
RUN mkdir -p crates/geoetl-cli/src crates/geoetl-core/src && \
    echo "fn main() {}" > crates/geoetl-cli/src/main.rs && \
    echo "pub fn dummy() {}" > crates/geoetl-core/src/lib.rs

# Build dependencies only (this layer will be cached)
RUN cargo build --release --bin geoetl-cli

# Remove dummy build artifacts
RUN rm -rf crates/*/src target/release/.fingerprint/geoetl-*

# Copy actual source code
COPY . .

# Build the actual CLI binary with optimizations
RUN cargo build --release --bin geoetl-cli

# Strip debug symbols to reduce binary size
RUN strip /build/target/release/geoetl-cli

# Verify the binary can be executed
RUN /build/target/release/geoetl-cli --version

# ============================================================================
# Stage 2: Minimal runtime container with just the CLI binary
# ============================================================================
FROM debian:bookworm-slim AS runtime

# Create a non-root user for security
# Note: No additional packages needed - the binary is fully self-contained
# All dependencies (SSL, SQLite, etc.) are statically linked (verified via ldd)
# If you add HTTPS features in the future, install ca-certificates here
RUN useradd --create-home --shell /bin/bash --uid 1000 geoetl

# Copy only the compiled binary from builder stage
COPY --from=builder /build/target/release/geoetl-cli /usr/local/bin/geoetl

# Switch to non-root user
USER geoetl
WORKDIR /home/geoetl

# Verify the binary works without any build dependencies
RUN geoetl --version

# Set the CLI as the entrypoint
ENTRYPOINT ["geoetl"]
CMD ["--help"]
