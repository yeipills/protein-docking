#!/bin/bash

# GPU Setup Script for Protein Docking v2
# AGENTE 4: DevOps & Infrastructure Specialist
# Configura soporte completo de GPU con CUDA para docking molecular

set -euo pipefail

# Colores para output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo -e "${BLUE}🚀 Protein Docking v2 - GPU Setup (CUDA)${NC}"
echo -e "${BLUE}=========================================${NC}"

# Funciones de utilidad
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

# Verificar si se está ejecutando con privilegios adecuados
check_privileges() {
    if [ "$EUID" -eq 0 ]; then
        log_warning "Este script no debe ejecutarse como root. Usa tu usuario normal."
        exit 1
    fi
    
    # Verificar si el usuario está en el grupo docker
    if ! groups $USER | grep -q docker; then
        log_error "El usuario $USER no está en el grupo docker. Ejecuta: sudo usermod -aG docker $USER"
        exit 1
    fi
}

# Verificar hardware GPU
check_gpu_hardware() {
    log_info "Verificando hardware GPU..."
    
    # Verificar si NVIDIA GPU está presente
    if ! lspci | grep -i nvidia > /dev/null 2>&1; then
        log_error "No se detectó hardware NVIDIA GPU en el sistema"
        exit 1
    fi
    
    # Obtener información de GPUs
    local gpu_count=$(lspci | grep -i nvidia | grep -i vga | wc -l)
    local gpu_info=$(lspci | grep -i nvidia | grep -i vga | head -3)
    
    log_success "Hardware GPU detectado:"
    echo "$gpu_info" | while read line; do
        echo -e "  🎯 $line"
    done
    
    log_info "Número de GPUs detectadas: $gpu_count"
    
    if [ "$gpu_count" -eq 0 ]; then
        log_error "No se detectaron GPUs NVIDIA"
        exit 1
    elif [ "$gpu_count" -eq 1 ]; then
        log_info "Configuración single-GPU detectada"
        export GPU_MODE="single"
    else
        log_success "Configuración multi-GPU detectada ($gpu_count GPUs)"
        export GPU_MODE="multi"
    fi
}

# Verificar drivers NVIDIA
check_nvidia_drivers() {
    log_info "Verificando drivers NVIDIA..."
    
    # Verificar si nvidia-smi está disponible
    if ! command -v nvidia-smi &> /dev/null; then
        log_error "nvidia-smi no encontrado. Instala los drivers NVIDIA primero."
        echo ""
        echo "Para Ubuntu/Debian:"
        echo "  sudo apt update"
        echo "  sudo apt install nvidia-driver-535"
        echo "  sudo reboot"
        exit 1
    fi
    
    # Verificar que los drivers funcionen
    if ! nvidia-smi > /dev/null 2>&1; then
        log_error "Los drivers NVIDIA no están funcionando correctamente"
        log_info "Intenta: sudo modprobe nvidia"
        exit 1
    fi
    
    # Mostrar información de GPU
    local driver_version=$(nvidia-smi --query-gpu=driver_version --format=csv,noheader,nounits | head -1)
    local cuda_version=$(nvidia-smi --query-gpu=cuda_version --format=csv,noheader,nounits | head -1)
    
    log_success "Driver NVIDIA: $driver_version"
    log_success "CUDA Version: $cuda_version"
    
    # Verificar temperatura y estado
    echo ""
    log_info "Estado actual de GPUs:"
    nvidia-smi --query-gpu=index,name,temperature.gpu,utilization.gpu,memory.used,memory.total --format=csv,table
}

# Verificar Docker con soporte GPU
check_docker_gpu_support() {
    log_info "Verificando soporte GPU en Docker..."
    
    # Verificar si Docker está instalado
    if ! command -v docker &> /dev/null; then
        log_error "Docker no está instalado"
        exit 1
    fi
    
    # Verificar NVIDIA Container Toolkit
    if ! docker info | grep -q nvidia > /dev/null 2>&1; then
        log_error "NVIDIA Container Toolkit no está instalado o configurado"
        echo ""
        echo "Para instalar NVIDIA Container Toolkit:"
        echo "  # Add the package repositories"
        echo "  distribution=\$(. /etc/os-release;echo \$ID\$VERSION_ID)"
        echo "  curl -s -L https://nvidia.github.io/nvidia-docker/gpgkey | sudo apt-key add -"
        echo "  curl -s -L https://nvidia.github.io/nvidia-docker/\$distribution/nvidia-docker.list | sudo tee /etc/apt/sources.list.d/nvidia-docker.list"
        echo ""
        echo "  # Install nvidia-container-toolkit"
        echo "  sudo apt-get update && sudo apt-get install -y nvidia-container-toolkit"
        echo "  sudo systemctl restart docker"
        exit 1
    fi
    
    # Test GPU access from Docker
    log_info "Probando acceso GPU desde Docker..."
    if docker run --rm --gpus all nvidia/cuda:12.1-runtime-ubuntu22.04 nvidia-smi > /dev/null 2>&1; then
        log_success "Docker puede acceder a las GPUs correctamente"
    else
        log_error "Docker no puede acceder a las GPUs"
        log_info "Verifica que NVIDIA Container Toolkit esté correctamente configurado"
        exit 1
    fi
}

# Crear estructura de directorios para GPU
create_gpu_directory_structure() {
    log_info "Creando estructura de directorios GPU..."
    
    # Directorios específicos para GPU
    mkdir -p "${PROJECT_DIR}/data"/{gpu-cache,gpu-kernels,postgres-gpu,redis-gpu/{node1,node2,node3},results-nvme,uploads-ssd,prometheus-gpu,grafana-gpu}
    
    # Logs específicos para GPU
    mkdir -p "${PROJECT_DIR}/logs"/{backend-gpu,frontend-gpu,postgres-gpu,nginx-gpu,grafana-gpu,redis-gpu}
    
    # Configuraciones específicas GPU
    mkdir -p "${PROJECT_DIR}/infrastructure"/{monitoring/{prometheus-gpu,grafana-gpu/{dashboards,datasources},rules-gpu},nginx}
    
    # Permisos correctos
    sudo chown -R $USER:$USER "${PROJECT_DIR}/data" "${PROJECT_DIR}/logs" 2>/dev/null || true
    chmod -R 755 "${PROJECT_DIR}/data" "${PROJECT_DIR}/logs"
    
    log_success "Estructura de directorios GPU creada"
}

# Configurar archivos específicos para GPU
setup_gpu_configurations() {
    log_info "Configurando archivos específicos para GPU..."
    
    # PostgreSQL configuración GPU
    cat > "${PROJECT_DIR}/infrastructure/postgres/postgresql-gpu.conf" << 'EOF'
# PostgreSQL GPU-Optimized Configuration
# Optimizado para workloads de GPU con mayor paralelismo

# Connections
max_connections = 500
shared_preload_libraries = 'pg_stat_statements,pg_buffercache'

# Memory (optimizado para sistemas con GPUs que requieren más RAM del sistema)
shared_buffers = 4GB
effective_cache_size = 12GB
maintenance_work_mem = 1GB
work_mem = 32MB
wal_buffers = 128MB

# Parallel processing (mayor paralelismo para complementar GPU)
max_worker_processes = 32
max_parallel_workers_per_gather = 16
max_parallel_workers = 32
max_parallel_maintenance_workers = 16

# I/O optimization para NVMe/SSD
effective_io_concurrency = 1000
random_page_cost = 1.0
seq_page_cost = 1.0

# WAL optimization
wal_level = replica
checkpoint_completion_target = 0.9
checkpoint_timeout = 15min
max_wal_size = 16GB
min_wal_size = 4GB

# Logging
log_min_duration_statement = 500
log_checkpoints = on
log_lock_waits = on
log_temp_files = 10MB

# Statistics
track_io_timing = on
track_functions = all
default_statistics_target = 1000
EOF

    # Redis configuraciones para nodos GPU
    for i in 1 2 3; do
        local port=$((6378 + i))
        local ip="172.31.0.$((9 + i))"
        
        cat > "${PROJECT_DIR}/infrastructure/redis/redis-gpu-node-${i}.conf" << EOF
# Redis GPU Node $i Configuration
bind 0.0.0.0
port $port
protected-mode no

# Memory optimized for GPU workloads
maxmemory 4gb
maxmemory-policy allkeys-lru
maxmemory-samples 10

# Persistence optimized for SSD
save 900 1
save 300 10
save 60 10000
appendonly yes
appendfsync everysec
auto-aof-rewrite-percentage 100
auto-aof-rewrite-min-size 128mb

# Cluster configuration
cluster-enabled yes
cluster-config-file nodes-$port.conf
cluster-node-timeout 15000
cluster-announce-ip $ip
cluster-announce-port $port
cluster-announce-bus-port $((port + 10000))

# Performance tuning
tcp-keepalive 300
timeout 0
tcp-backlog 511
databases 16

# Threading for better performance
io-threads 8
io-threads-do-reads yes

# Security
requirepass protein_redis_gpu_secure_2024
EOF
    done
    
    log_success "Configuraciones GPU creadas"
}

# Configurar Prometheus para métricas GPU
setup_prometheus_gpu_config() {
    log_info "Configurando Prometheus para métricas GPU..."
    
    cat > "${PROJECT_DIR}/infrastructure/monitoring/prometheus-gpu.yml" << 'EOF'
global:
  scrape_interval: 15s
  evaluation_interval: 15s

rule_files:
  - "/etc/prometheus/rules/*.yml"

alerting:
  alertmanagers:
    - static_configs:
        - targets: []

scrape_configs:
  # Backend GPU metrics
  - job_name: 'backend-gpu'
    static_configs:
      - targets: ['backend-gpu:8081', 'backend-gpu:8082']
    scrape_interval: 10s
    metrics_path: /metrics

  # NVIDIA GPU metrics via nvidia-exporter
  - job_name: 'nvidia-gpu'
    static_configs:
      - targets: ['nvidia-exporter:9445']
    scrape_interval: 5s

  # DCGM GPU metrics (advanced)
  - job_name: 'dcgm-gpu'
    static_configs:
      - targets: ['dcgm-exporter:9400']
    scrape_interval: 5s

  # System metrics
  - job_name: 'node-exporter'
    static_configs:
      - targets: ['node-exporter:9100']

  # PostgreSQL GPU metrics
  - job_name: 'postgres-gpu'
    static_configs:
      - targets: ['postgres-exporter:9187']

  # Redis GPU cluster metrics
  - job_name: 'redis-gpu'
    static_configs:
      - targets: ['redis-exporter:9121']

  # Nginx GPU metrics
  - job_name: 'nginx-gpu'
    static_configs:
      - targets: ['nginx-gpu:8090']
    metrics_path: /nginx_status
EOF

    # Reglas de alertas para GPU
    mkdir -p "${PROJECT_DIR}/infrastructure/monitoring/rules-gpu"
    
    cat > "${PROJECT_DIR}/infrastructure/monitoring/rules-gpu/gpu-alerts.yml" << 'EOF'
groups:
  - name: gpu-alerts
    rules:
      - alert: GPUHighTemperature
        expr: DCGM_FI_DEV_GPU_TEMP > 85
        for: 2m
        labels:
          severity: warning
        annotations:
          summary: "GPU temperature is high"
          description: "GPU {{ $labels.gpu }} temperature is {{ $value }}°C"

      - alert: GPUHighUtilization
        expr: DCGM_FI_DEV_GPU_UTIL > 95
        for: 5m
        labels:
          severity: info
        annotations:
          summary: "GPU utilization is high"
          description: "GPU {{ $labels.gpu }} utilization is {{ $value }}%"

      - alert: GPUMemoryHigh
        expr: DCGM_FI_DEV_FB_USED / DCGM_FI_DEV_FB_TOTAL > 0.9
        for: 3m
        labels:
          severity: warning
        annotations:
          summary: "GPU memory usage is high"
          description: "GPU {{ $labels.gpu }} memory usage is {{ $value | humanizePercentage }}"

      - alert: GPUDown
        expr: up{job="nvidia-gpu"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "GPU monitoring is down"
          description: "Cannot scrape GPU metrics"
EOF

    log_success "Prometheus GPU configurado"
}

# Configurar variables de entorno para GPU
setup_gpu_environment() {
    log_info "Configurando variables de entorno GPU..."
    
    # Crear o actualizar archivo .env para GPU
    local env_file="${PROJECT_DIR}/.env.gpu"
    
    cat > "$env_file" << EOF
# GPU Configuration for Protein Docking v2
# AGENTE 4: DevOps & Infrastructure Specialist

# CUDA Configuration
CUDA_VISIBLE_DEVICES=0
NVIDIA_VISIBLE_DEVICES=all
NVIDIA_DRIVER_CAPABILITIES=compute,utility
CUDA_CACHE_PATH=/app/gpu-cache

# GPU Performance
GPU_MEMORY_POOL_SIZE=12GB
GPU_MAX_CONCURRENT_KERNELS=128
GPU_MEMORY_FRACTION=0.95
GPU_BATCH_SIZE=2048

# Multi-GPU Configuration
NCCL_DEBUG=WARN
NCCL_SOCKET_IFNAME=eth0
NCCL_IB_DISABLE=1

# Protein Docking GPU Features
PROTEIN_DOCKING_ENABLE_GPU=true
PROTEIN_DOCKING_ENABLE_MULTI_GPU=false
PROTEIN_DOCKING_ENABLE_CUDA=true
PROTEIN_DOCKING_GPU_BATCH_SIZE=2048
PROTEIN_DOCKING_MAX_CONCURRENT_JOBS=16

# Database GPU Optimized
POSTGRES_MAX_CONNECTIONS=500
POSTGRES_SHARED_BUFFERS=4GB
POSTGRES_EFFECTIVE_CACHE_SIZE=12GB

# Redis GPU Optimized
REDIS_MAXMEMORY=4gb
REDIS_IO_THREADS=8

# Monitoring
ENABLE_GPU_METRICS=true
GPU_METRICS_PORT=8082
PROMETHEUS_GPU_SCRAPE_INTERVAL=5s
EOF

    log_success "Variables de entorno GPU configuradas en $env_file"
}

# Verificar configuración GPU de Docker Compose
validate_gpu_compose_config() {
    log_info "Validando configuración GPU de Docker Compose..."
    
    if ! docker compose -f "${PROJECT_DIR}/docker-compose.gpu.yml" config --quiet; then
        log_error "Error en la configuración de docker-compose.gpu.yml"
        docker compose -f "${PROJECT_DIR}/docker-compose.gpu.yml" config
        exit 1
    fi
    
    log_success "Configuración GPU de Docker Compose validada"
}

# Preparar imágenes GPU
prepare_gpu_images() {
    log_info "Preparando imágenes Docker con soporte GPU..."
    
    # Pull de imágenes base CUDA
    log_info "Descargando imágenes base CUDA..."
    docker pull nvidia/cuda:12.1-devel-ubuntu22.04
    docker pull nvidia/cuda:12.1-runtime-ubuntu22.04
    
    # Pull de imágenes de monitoreo GPU
    log_info "Descargando imágenes de monitoreo GPU..."
    docker pull mindprince/nvidia_gpu_prometheus_exporter:0.1
    docker pull nvcr.io/nvidia/k8s/dcgm-exporter:3.3.5-3.4.0-ubuntu22.04
    
    log_success "Imágenes base GPU descargadas"
}

# Test básico de GPU
test_gpu_functionality() {
    log_info "Ejecutando test básico de GPU..."
    
    # Test simple con nvidia-smi
    log_info "Test 1: nvidia-smi en contenedor"
    if docker run --rm --gpus all nvidia/cuda:12.1-runtime-ubuntu22.04 nvidia-smi; then
        log_success "Test nvidia-smi: PASSED"
    else
        log_error "Test nvidia-smi: FAILED"
        return 1
    fi
    
    # Test de compilación CUDA básica
    log_info "Test 2: Compilación CUDA básica"
    cat > /tmp/test_cuda.cu << 'EOF'
#include <stdio.h>
#include <cuda_runtime.h>

__global__ void hello() {
    printf("Hello from GPU thread %d\n", threadIdx.x);
}

int main() {
    int deviceCount;
    cudaGetDeviceCount(&deviceCount);
    printf("Number of CUDA devices: %d\n", deviceCount);
    
    if (deviceCount > 0) {
        hello<<<1, 10>>>();
        cudaDeviceSynchronize();
        printf("CUDA test successful!\n");
    }
    return 0;
}
EOF
    
    if docker run --rm --gpus all -v /tmp/test_cuda.cu:/test.cu nvidia/cuda:12.1-devel-ubuntu22.04 \
        bash -c "nvcc /test.cu -o /test && /test"; then
        log_success "Test compilación CUDA: PASSED"
    else
        log_warning "Test compilación CUDA: FAILED (no crítico)"
    fi
    
    rm -f /tmp/test_cuda.cu
}

# Mostrar información final
show_gpu_setup_summary() {
    echo ""
    echo -e "${GREEN}🎉 GPU Setup completado exitosamente!${NC}"
    echo -e "${BLUE}======================================${NC}"
    echo ""
    echo -e "${BLUE}Configuración GPU:${NC}"
    
    # Información de hardware
    local gpu_count=$(nvidia-smi --list-gpus | wc -l)
    echo -e "  • GPUs detectadas: ${GREEN}$gpu_count${NC}"
    
    local driver_version=$(nvidia-smi --query-gpu=driver_version --format=csv,noheader,nounits | head -1)
    echo -e "  • Driver NVIDIA: ${GREEN}$driver_version${NC}"
    
    local cuda_version=$(nvidia-smi --query-gpu=cuda_version --format=csv,noheader,nounits | head -1)
    echo -e "  • CUDA Version: ${GREEN}$cuda_version${NC}"
    
    echo ""
    echo -e "${BLUE}Archivos creados:${NC}"
    echo -e "  • ${YELLOW}docker-compose.gpu.yml${NC} - Configuración Docker con GPU"
    echo -e "  • ${YELLOW}backend/Dockerfile.gpu${NC} - Backend con soporte CUDA"
    echo -e "  • ${YELLOW}.env.gpu${NC} - Variables de entorno GPU"
    echo -e "  • ${YELLOW}infrastructure/monitoring/prometheus-gpu.yml${NC} - Métricas GPU"
    echo ""
    echo -e "${BLUE}Comandos para usar GPU:${NC}"
    echo -e "  • Iniciar con GPU: ${YELLOW}docker compose -f docker-compose.gpu.yml up -d${NC}"
    echo -e "  • Ver métricas GPU: ${YELLOW}nvidia-smi${NC}"
    echo -e "  • Monitoreo GPU: ${YELLOW}http://localhost:3001${NC} (Grafana)"
    echo -e "  • Test GPU: ${YELLOW}docker run --rm --gpus all nvidia/cuda:12.1-runtime-ubuntu22.04 nvidia-smi${NC}"
    echo ""
    echo -e "${BLUE}Próximos pasos:${NC}"
    echo -e "  1. Ejecutar: ${YELLOW}docker compose -f docker-compose.gpu.yml build${NC}"
    echo -e "  2. Ejecutar: ${YELLOW}docker compose -f docker-compose.gpu.yml up -d${NC}"
    echo -e "  3. Verificar: ${YELLOW}./scripts/gpu-health-check.sh${NC}"
    echo ""
}

# Script principal
main() {
    log_info "Iniciando configuración GPU para Protein Docking v2..."
    echo ""
    
    # Verificaciones previas
    check_privileges
    check_gpu_hardware
    check_nvidia_drivers
    check_docker_gpu_support
    
    # Configuración
    create_gpu_directory_structure
    setup_gpu_configurations
    setup_prometheus_gpu_config
    setup_gpu_environment
    validate_gpu_compose_config
    
    # Preparación
    prepare_gpu_images
    test_gpu_functionality
    
    # Resumen final
    show_gpu_setup_summary
    
    log_success "GPU Setup completo! El sistema está listo para acceleration con CUDA."
}

# Manejo de argumentos
if [ "${1:-}" = "--help" ] || [ "${1:-}" = "-h" ]; then
    echo "GPU Setup para Protein Docking v2"
    echo ""
    echo "Este script configura soporte completo de GPU con CUDA para acelerar"
    echo "las operaciones de docking molecular."
    echo ""
    echo "Requisitos:"
    echo "  • Hardware NVIDIA GPU"
    echo "  • Drivers NVIDIA instalados"
    echo "  • Docker con NVIDIA Container Toolkit"
    echo ""
    echo "Uso: $0"
    echo ""
    exit 0
fi

# Manejar señales
trap 'log_error "GPU setup interrumpido"; exit 1' INT TERM

# Ejecutar setup principal
main "$@"