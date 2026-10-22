#!/bin/bash

# Deployment Script - Ultra-optimized protein docking platform
# Script de deployment automatizado para máximo rendimiento

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PROJECT_NAME="protein-docking-v2"
DOCKER_COMPOSE_FILE="infrastructure/docker-compose.yml"
BACKEND_DOCKERFILE="backend/Dockerfile"
FRONTEND_DOCKERFILE="frontend/Dockerfile"

# Functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_prerequisites() {
    log_info "Verificando prerequisitos..."
    
    # Check Docker
    if ! command -v docker &> /dev/null; then
        log_error "Docker no está instalado"
        exit 1
    fi
    
    # Check Docker Compose
    if ! command -v docker compose &> /dev/null; then
        log_error "Docker Compose no está instalado"
        exit 1
    fi
    
    # Check if Docker is running
    if ! docker info &> /dev/null; then
        log_error "Docker no está ejecutándose"
        exit 1
    fi
    
    log_success "Todos los prerequisitos están disponibles"
}

cleanup_previous_deployment() {
    log_info "Limpiando deployment anterior..."
    
    # Stop and remove containers
    docker-compose -f $DOCKER_COMPOSE_FILE down -v --remove-orphans 2>/dev/null || true
    
    # Remove unused images
    docker image prune -f 2>/dev/null || true
    
    # Remove unused volumes (be careful in production)
    if [[ "${ENVIRONMENT:-development}" == "development" ]]; then
        docker volume prune -f 2>/dev/null || true
    fi
    
    log_success "Cleanup completado"
}

build_images() {
    log_info "Construyendo imágenes Docker..."
    
    # Build backend with optimizations
    log_info "Construyendo backend Rust..."
    docker build \
        --target production \
        --build-arg RUSTFLAGS="-C target-cpu=native -C opt-level=3" \
        -t protein-docking-backend:latest \
        -f $BACKEND_DOCKERFILE \
        ./backend
    
    # Build frontend
    log_info "Construyendo frontend Svelte..."
    docker build \
        --target production \
        -t protein-docking-frontend:latest \
        -f $FRONTEND_DOCKERFILE \
        ./frontend
    
    log_success "Imágenes construidas exitosamente"
}

setup_redis_cluster() {
    log_info "Configurando Redis Cluster..."
    
    # Wait for Redis nodes to be ready
    sleep 10
    
    # Create Redis cluster
    docker exec protein-docking-redis-1 redis-cli \
        --cluster create \
        redis-1:6379 redis-2:6380 redis-3:6381 \
        --cluster-replicas 0 \
        --cluster-yes 2>/dev/null || log_warning "Redis cluster ya configurado o error en configuración"
    
    log_success "Redis Cluster configurado"
}

initialize_databases() {
    log_info "Inicializando bases de datos..."
    
    # Wait for databases to be ready
    sleep 15
    
    # Initialize ClickHouse schema
    log_info "Inicializando esquema ClickHouse..."
    docker exec protein-docking-clickhouse clickhouse-client \
        --query "CREATE DATABASE IF NOT EXISTS protein_docking_analytics" 2>/dev/null || true
    
    # Apply ClickHouse schema
    docker exec -i protein-docking-clickhouse clickhouse-client \
        --database protein_docking_analytics < infrastructure/clickhouse/protein_docking_schema.sql || true
    
    log_success "Bases de datos inicializadas"
}

verify_deployment() {
    log_info "Verificando deployment..."
    
    # Check service health
    local services=("backend" "frontend" "postgres" "redis-1" "redis-2" "redis-3" "clickhouse" "nginx" "prometheus" "grafana")
    
    for service in "${services[@]}"; do
        if docker-compose -f $DOCKER_COMPOSE_FILE ps | grep -q "protein-docking-$service.*Up"; then
            log_success "✓ $service está ejecutándose"
        else
            log_error "✗ $service NO está ejecutándose"
        fi
    done
    
    # Check endpoints
    log_info "Verificando endpoints..."
    sleep 30  # Wait for services to fully start
    
    # Check backend health
    if curl -f http://localhost:8080/health &>/dev/null; then
        log_success "✓ Backend health check OK"
    else
        log_warning "✗ Backend health check falló"
    fi
    
    # Check frontend
    if curl -f http://localhost:3000 &>/dev/null; then
        log_success "✓ Frontend accesible"
    else
        log_warning "✗ Frontend no accesible"
    fi
    
    # Check Prometheus
    if curl -f http://localhost:9090/-/healthy &>/dev/null; then
        log_success "✓ Prometheus activo"
    else
        log_warning "✗ Prometheus no activo"
    fi
    
    # Check Grafana
    if curl -f http://localhost:3001/api/health &>/dev/null; then
        log_success "✓ Grafana activo"
    else
        log_warning "✗ Grafana no activo"
    fi
}

show_deployment_info() {
    log_success "=== DEPLOYMENT COMPLETADO ==="
    echo
    log_info "URLs de acceso:"
    echo "  🚀 Frontend:     http://localhost:3000"
    echo "  ⚡ Backend API:  http://localhost:8080"
    echo "  📊 Grafana:     http://localhost:3001 (admin:protein_admin)"
    echo "  📈 Prometheus:  http://localhost:9090"
    echo "  🗄️  ClickHouse:  http://localhost:8123"
    echo "  💾 Redis:       localhost:6379,6380,6381"
    echo "  🗃️  PostgreSQL:  localhost:5432"
    echo
    log_info "Métricas y monitoreo:"
    echo "  📊 Dashboard principal en Grafana"
    echo "  📈 Métricas en tiempo real cada 2-5 segundos"
    echo "  ⚡ Performance ultra-optimizado activado"
    echo
    log_info "Para ver logs en tiempo real:"
    echo "  docker-compose -f $DOCKER_COMPOSE_FILE logs -f [service_name]"
    echo
    log_info "Para detener todos los servicios:"
    echo "  docker-compose -f $DOCKER_COMPOSE_FILE down"
}

# Main deployment process
main() {
    log_info "Iniciando deployment de $PROJECT_NAME..."
    echo
    
    # Set environment if not provided
    export ENVIRONMENT=${ENVIRONMENT:-development}
    log_info "Entorno: $ENVIRONMENT"
    
    # Execute deployment steps
    check_prerequisites
    cleanup_previous_deployment
    build_images
    
    # Start services
    log_info "Iniciando servicios..."
    docker-compose -f $DOCKER_COMPOSE_FILE up -d
    
    # Post-deployment setup
    setup_redis_cluster
    initialize_databases
    
    # Verification and info
    verify_deployment
    show_deployment_info
    
    log_success "🎉 ¡Deployment completado exitosamente!"
}

# Script execution
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi