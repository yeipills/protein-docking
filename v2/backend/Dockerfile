# Multi-stage Dockerfile ultra-optimizado para máximo rendimiento
# Optimizado para protein docking con GPU, SIMD y paralelización masiva

# Stage 1: Build environment con optimizaciones extremas
FROM rust:1.84-bullseye AS builder

# Configurar variables de optimización compatible
ENV RUSTFLAGS="-C target-cpu=native -C opt-level=3 -C link-arg=-s -C codegen-units=1"
ENV CARGO_BUILD_RUSTFLAGS="-C target-feature=+avx2,+fma"

# Install system dependencies for compilation + GPU support
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    cmake \
    build-essential \
    libnuma-dev \
    libopenblas-dev \
    liblapack-dev \
    libeigen3-dev \
    libtbb-dev \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy dependency files first for better caching
COPY Cargo.toml Cargo.lock ./

# Create dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies (cached layer)
RUN cargo build --release
RUN rm -f target/release/deps/protein_docking_backend*

# Copy source code
COPY src ./src

# Build application with maximum optimizations
ENV RUSTFLAGS="-C target-cpu=native -C opt-level=3 -C link-arg=-s"
RUN cargo build --release --bin protein-docking-server

# Stage 2: Runtime environment
FROM debian:bullseye-slim AS runtime

# Install runtime dependencies including curl
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl1.1 \
    libpq5 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN groupadd -r app && useradd -r -g app app

# Set working directory
WORKDIR /app

# Copy binary from builder with execute permissions
COPY --from=builder --chmod=755 /app/target/release/protein-docking-server /usr/local/bin/protein-docking-server

# Create necessary directories
RUN mkdir -p /app/uploads /app/results /app/logs \
    && chown -R app:app /app

# Switch to app user
USER app

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run the application
CMD ["protein-docking-server"]

# Stage 3: Development environment
FROM builder AS development

# Install development tools
RUN cargo install cargo-watch cargo-edit

# Copy source for development
COPY . .

# Set development environment
ENV RUST_LOG=debug
ENV RUST_BACKTRACE=1

# Expose port for development
EXPOSE 8080

# Development command
CMD ["cargo", "watch", "-x", "run"]

# Stage 4: Production with optimizations
FROM runtime AS production

# Production-specific optimizations
ENV RUST_LOG=info
ENV TOKIO_WORKER_THREADS=16
ENV RAYON_NUM_THREADS=16

# Production command
CMD ["protein-docking-server"]
