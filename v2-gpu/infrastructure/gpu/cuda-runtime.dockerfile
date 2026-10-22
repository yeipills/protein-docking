# CUDA Runtime Dockerfile for Protein Docking
# AGENTE 4: DevOps & Infrastructure Specialist
# Ultra-optimized CUDA environment for molecular docking acceleration

# Use NVIDIA CUDA runtime as base
FROM nvidia/cuda:12.1-devel-ubuntu22.04 AS cuda-base

# Metadata
LABEL maintainer="Protein Docking v2 Team"
LABEL description="CUDA-accelerated protein docking runtime"
LABEL version="2.0.0"

# Environment variables for CUDA optimization
ENV NVIDIA_VISIBLE_DEVICES=all
ENV NVIDIA_DRIVER_CAPABILITIES=compute,utility
ENV CUDA_VERSION=12.1
ENV CUDNN_VERSION=8.9.0
ENV CUDA_ARCH_LIST="6.0 6.1 7.0 7.5 8.0 8.6 8.9 9.0"

# Configure CUDA paths
ENV CUDA_HOME=/usr/local/cuda
ENV CUDA_ROOT=/usr/local/cuda
ENV PATH=${CUDA_HOME}/bin:${PATH}
ENV LD_LIBRARY_PATH=${CUDA_HOME}/lib64:${LD_LIBRARY_PATH}
ENV LIBRARY_PATH=${CUDA_HOME}/lib64:${LIBRARY_PATH}

# Install system dependencies for CUDA development
RUN apt-get update && apt-get install -y --no-install-recommends \
    # Build essentials
    build-essential \
    cmake \
    git \
    wget \
    curl \
    ca-certificates \
    # CUDA development tools
    cuda-toolkit-12-3 \
    cuda-nvml-dev-12-3 \
    cuda-command-line-tools-12-3 \
    # cuDNN for deep learning acceleration
    libcudnn8=${CUDNN_VERSION}* \
    libcudnn8-dev=${CUDNN_VERSION}* \
    # NVIDIA libraries
    libnvidia-compute-535 \
    libnvidia-decode-535 \
    libnvidia-encode-535 \
    # Math libraries
    libcublas12 \
    libcublas-dev-12-3 \
    libcufft12 \
    libcufft-dev-12-3 \
    libcurand12 \
    libcurand-dev-12-3 \
    libcusolver12 \
    libcusolver-dev-12-3 \
    libcusparse12 \
    libcusparse-dev-12-3 \
    # Thrust library for parallel algorithms
    libthrust-dev \
    # Additional math libraries
    libopenblas-dev \
    liblapack-dev \
    libeigen3-dev \
    # Intel TBB for CPU parallelization
    libtbb-dev \
    # Cleanup
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Install NVIDIA NCCL for multi-GPU communication
RUN wget -qO- https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2204/x86_64/cuda-keyring_1.0-1_all.deb | \
    dpkg -i /dev/stdin && \
    apt-get update && \
    apt-get install -y libnccl2 libnccl-dev && \
    rm -rf /var/lib/apt/lists/*

# Verify CUDA installation
RUN nvcc --version && \
    nvidia-smi || echo "GPU not available at build time"

# Create directories for GPU kernels and libraries
RUN mkdir -p /opt/protein-docking/gpu/{kernels,libs,cache}

# Set GPU cache directory
ENV CUDA_CACHE_PATH=/opt/protein-docking/gpu/cache

# Stage 2: Rust CUDA integration
FROM cuda-base AS rust-cuda

# Install Rust with CUDA support
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH \
    RUST_VERSION=1.84.0

RUN wget -qO- https://sh.rustup.rs | sh -s -- -y --default-toolchain $RUST_VERSION && \
    chmod -R a+w $RUSTUP_HOME $CARGO_HOME

# Install Rust CUDA toolchain and components
RUN rustup component add rust-src && \
    rustup target add nvptx64-nvidia-cuda

# Install cargo-cuda for CUDA kernel compilation
RUN cargo install cargo-cuda

# Configure Rust for CUDA compilation
ENV RUSTFLAGS="-C target-cpu=native -C opt-level=3 -C target-feature=+avx2,+fma"
ENV CUDA_RUSTFLAGS="-C opt-level=3 -C target-feature=+sm_60,+sm_70,+sm_80"

# Stage 3: Production runtime with optimizations
FROM rust-cuda AS production

# Create application directory
WORKDIR /app

# Copy CUDA kernels and libraries (these would be built in backend)
COPY --chown=app:app gpu/ /opt/protein-docking/gpu/

# Create non-root user for security
RUN groupadd -r app && useradd -r -g app app && \
    chown -R app:app /app /opt/protein-docking

# GPU memory and performance optimizations
ENV CUDA_DEVICE_ORDER=PCI_BUS_ID
ENV CUDA_VISIBLE_DEVICES=0
ENV CUDA_MPS_PIPE_DIRECTORY=/tmp/nvidia-mps
ENV CUDA_MPS_LOG_DIRECTORY=/tmp/nvidia-log

# Configure GPU memory management
ENV GPU_MEMORY_POOL_SIZE=8GB
ENV GPU_MAX_CONCURRENT_KERNELS=32
ENV GPU_KERNEL_CACHE_SIZE=1GB

# Performance tuning
ENV OMP_NUM_THREADS=16
ENV OPENBLAS_NUM_THREADS=16
ENV MKL_NUM_THREADS=16

# Switch to app user
USER app

# Health check for GPU availability
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD nvidia-smi --query-gpu=temperature.gpu --format=csv,noheader,nounits | head -1 || exit 1

# Default command
CMD ["/bin/bash"]

# Stage 4: Development environment with debugging tools
FROM production AS development

USER root

# Install development and debugging tools
RUN apt-get update && apt-get install -y --no-install-recommends \
    # CUDA debugging tools
    cuda-gdb \
    cuda-memcheck \
    # Profiling tools
    nsight-compute-2023.3.1 \
    nsight-systems-2023.4.4 \
    # Additional development tools
    valgrind \
    gdb \
    strace \
    htop \
    iotop \
    nvtop \
    # Cleanup
    && rm -rf /var/lib/apt/lists/*

# Install Rust development tools
RUN cargo install cargo-expand cargo-check cargo-clippy

# Development environment variables
ENV RUST_LOG=debug
ENV RUST_BACKTRACE=full
ENV CUDA_LAUNCH_BLOCKING=1

USER app

# Development command
CMD ["cargo", "run", "--release"]