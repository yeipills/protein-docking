# Multi-stage Dockerfile for Protein Docking Platform Backend
# Optimized for production with minimal image size

# =====================================
# Stage 1: Builder - Install dependencies and compile Cython
# =====================================
FROM python:3.11-slim as builder

WORKDIR /build

# Install build dependencies for Cython compilation
# Cleaned in same layer to reduce image size
RUN apt-get update && apt-get install -y --no-install-recommends \
    gcc \
    g++ \
    make \
    python3-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy requirements FIRST for better layer caching
# If requirements.txt unchanged, Docker uses cached pip install
COPY requirements.txt .

# Install Python dependencies with optimizations
# --user installs to /root/.local (copied to runtime stage)
# --no-cache-dir reduces image size
# Cached if requirements.txt hasn't changed
RUN pip install --no-cache-dir --user -r requirements.txt && \
    pip list

# Copy setup.py and Cython source files for compilation
COPY setup.py .
COPY app/algorithms/cython_utils.pyx app/algorithms/cython_utils.pyx

# Compile Cython extensions
# This creates .so files that provide 4-6x speedup for layer evaluation
RUN python setup.py build_ext --inplace

# Note: Compiled .so files will be in app/algorithms/ directory

# =====================================
# Stage 2: Runtime - Copy only what's needed
# =====================================
FROM python:3.11-slim

WORKDIR /app

# Install runtime dependencies only (no build tools needed)
# Cleaned in same layer to minimize final image size
RUN apt-get update && apt-get install -y --no-install-recommends \
    libgomp1 \
    wget \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Copy Python dependencies from builder
COPY --from=builder /root/.local /root/.local

# Make sure scripts in .local are usable
ENV PATH=/root/.local/bin:$PATH

# Copy application code
COPY ./app /app/app
COPY ./socket_server /app/socket_server

# Copy compiled Cython extensions from builder
# These .so files provide 4-6x performance boost for layer evaluation
RUN --mount=type=bind,from=builder,source=/build/app/algorithms,target=/tmp/build \
    cp /tmp/build/*.so /app/app/algorithms/ 2>/dev/null || true

# Create directories for uploads and logs
RUN mkdir -p /app/uploads /app/results /app/logs

# Expose ports
EXPOSE 5000 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD wget --quiet --tries=1 --spider http://localhost:5000/health || exit 1

# Default command (can be overridden in docker-compose)
CMD ["uvicorn", "app.main:app", "--host", "0.0.0.0", "--port", "5000"]
