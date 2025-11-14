# Multi-stage Dockerfile for Protein Docking Platform Frontend
# Optimized for production with Vite + React + TypeScript

# =====================================
# Stage 1: Builder - Install dependencies and build
# =====================================
FROM node:20-alpine as builder

WORKDIR /app

# Copy package files FIRST for better layer caching
# If package.json hasn't changed, Docker will use cached npm install
COPY package.json package-lock.json ./

# Install dependencies (cached if package files unchanged)
# --silent reduces output noise
# --prefer-offline speeds up repeated builds
RUN npm ci --silent --prefer-offline

# Copy application source (separate layer for better caching)
# Only rebuild if source code changes, not on every Docker build
COPY . .

# Build the application with Vite
# Creates optimized production bundle with code splitting in /app/dist
RUN npm run build

# Verify build output exists
RUN ls -la /app/dist && \
    echo "Build successful - dist size:" && \
    du -sh /app/dist

# =====================================
# Stage 2: Production - Serve with Nginx
# =====================================
FROM nginx:alpine

# Copy built assets from builder stage
COPY --from=builder /app/dist /usr/share/nginx/html

# Copy custom nginx configuration for SPA routing
RUN echo 'server { \
    listen 3000; \
    server_name localhost; \
    root /usr/share/nginx/html; \
    index index.html; \
    \
    location / { \
        try_files $uri $uri/ /index.html; \
    } \
    \
    location /assets/ { \
        expires 1y; \
        add_header Cache-Control "public, immutable"; \
    } \
}' > /etc/nginx/conf.d/default.conf

# Expose port 3000
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --quiet --tries=1 --spider http://localhost:3000 || exit 1

# Start nginx
CMD ["nginx", "-g", "daemon off;"]
