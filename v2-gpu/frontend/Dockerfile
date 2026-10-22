# Multi-stage Dockerfile para frontend ultra-optimizado

# Stage 1: Build environment
FROM node:20-alpine AS builder

# Install build dependencies
RUN apk add --no-cache \
    python3 \
    make \
    g++ \
    git

# Set working directory
WORKDIR /app

# Copy package files
COPY package*.json ./

# Install all dependencies (including dev dependencies for build)
RUN npm ci --no-audit --no-fund

# Copy source code
COPY . .

# Build application with maximum optimizations
ENV NODE_ENV=production
ENV VITE_BUILD_ANALYZE=false
RUN npm run build

# Stage 2: Production server
FROM nginx:alpine AS production

# Install runtime dependencies
RUN apk add --no-cache \
    curl \
    ca-certificates

# Copy built application (SvelteKit output)
COPY --from=builder /app/.svelte-kit/output/client /usr/share/nginx/html

# Copy nginx configuration
COPY nginx.conf /etc/nginx/nginx.conf

# Create nginx user and set permissions
RUN addgroup -g 101 -S nginx \
    && adduser -S -D -H -u 101 -h /var/cache/nginx -s /sbin/nologin -G nginx -g nginx nginx \
    && chown -R nginx:nginx /usr/share/nginx/html \
    && chown -R nginx:nginx /var/cache/nginx

# Expose port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

# Start nginx
CMD ["nginx", "-g", "daemon off;"]

# Stage 3: Development environment
FROM node:20-alpine AS development

# Install development dependencies
RUN apk add --no-cache \
    python3 \
    make \
    g++ \
    git

WORKDIR /app

# Copy package files
COPY package*.json ./

# Install all dependencies (including dev)
RUN npm ci

# Copy source
COPY . .

# Expose port
EXPOSE 3000

# Development command
CMD ["npm", "run", "dev"]