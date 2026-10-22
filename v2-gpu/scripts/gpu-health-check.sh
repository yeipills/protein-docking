#!/bin/bash

# GPU Health Check Script for Protein Docking v2
# AGENTE 4: DevOps & Infrastructure Specialist
# Verifica el estado y salud de los componentes GPU

set -euo pipefail

# Colores para output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo -e "${BLUE}🔍 GPU Health Check - Protein Docking v2${NC}"
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

# Verificar estado de GPU hardware
check_gpu_hardware() {
    log_info "Verificando hardware GPU..."
    
    if ! command -v nvidia-smi &> /dev/null; then
        log_error "nvidia-smi no encontrado"
        return 1
    fi
    
    if ! nvidia-smi > /dev/null 2>&1; then
        log_error "GPU no accesible"
        return 1
    fi
    
    local temp=$(nvidia-smi --query-gpu=temperature.gpu --format=csv,noheader,nounits)
    local util=$(nvidia-smi --query-gpu=utilization.gpu --format=csv,noheader,nounits)
    local mem_used=$(nvidia-smi --query-gpu=memory.used --format=csv,noheader,nounits)
    local mem_total=$(nvidia-smi --query-gpu=memory.total --format=csv,noheader,nounits)
    
    log_success "GPU Hardware OK"
    echo -e "  • Temperatura: ${temp}°C"
    echo -e "  • Utilización: ${util}%"
    echo -e "  • Memoria: ${mem_used}MB/${mem_total}MB"
    
    # Verificar alertas
    if [ "$temp" -gt 80 ]; then
        log_warning "Temperatura GPU alta: ${temp}°C"
    fi
    
    if [ "$util" -gt 90 ]; then
        log_warning "Utilización GPU alta: ${util}%"
    fi
    
    local mem_percent=$((mem_used * 100 / mem_total))
    if [ "$mem_percent" -gt 90 ]; then
        log_warning "Memoria GPU alta: ${mem_percent}%"
    fi
}

# Verificar contenedores GPU
check_gpu_containers() {
    log_info "Verificando contenedores GPU..."
    
    local containers=(
        "protein-docking-backend-gpu"
        "protein-docking-nvidia-exporter" 
        "protein-docking-dcgm-exporter"
    )
    
    for container in "${containers[@]}"; do
        if docker ps --filter "name=${container}" --filter "status=running" | grep -q "${container}"; then
            log_success "Contenedor ${container} está corriendo"
        else
            log_error "Contenedor ${container} no está corriendo"
        fi
    done
}

# Verificar servicios GPU
check_gpu_services() {
    log_info "Verificando servicios GPU..."
    
    # Backend GPU
    if curl -f -s http://localhost:8082/metrics > /dev/null 2>&1; then
        log_success "Backend GPU metrics OK"
    else
        log_warning "Backend GPU metrics no accesible"
    fi
    
    # NVIDIA Exporter
    if curl -f -s http://localhost:9445/metrics > /dev/null 2>&1; then
        log_success "NVIDIA Exporter OK"
    else
        log_warning "NVIDIA Exporter no accesible"
    fi
    
    # DCGM Exporter
    if curl -f -s http://localhost:9400/metrics > /dev/null 2>&1; then
        log_success "DCGM Exporter OK"
    else
        log_warning "DCGM Exporter no accesible"
    fi
}

# Verificar métricas GPU
check_gpu_metrics() {
    log_info "Verificando métricas GPU..."
    
    # Verificar Prometheus GPU
    if curl -f -s http://localhost:9090/-/healthy > /dev/null 2>&1; then
        log_success "Prometheus GPU OK"
        
        # Verificar si las métricas GPU están siendo recolectadas
        local gpu_metrics=$(curl -s "http://localhost:9090/api/v1/query?query=up{job=\"nvidia-gpu\"}" | grep '"result"')
        if [[ "$gpu_metrics" == *'"value":["'*',"1"]'* ]]; then
            log_success "Métricas GPU siendo recolectadas"
        else
            log_warning "Métricas GPU no están siendo recolectadas"
        fi
    else
        log_warning "Prometheus GPU no accesible"
    fi
    
    # Verificar Grafana GPU
    if curl -f -s http://localhost:3001/api/health > /dev/null 2>&1; then
        log_success "Grafana GPU OK"
    else
        log_warning "Grafana GPU no accesible"
    fi
}

# Verificar logs de GPU
check_gpu_logs() {
    log_info "Verificando logs GPU..."
    
    # Verificar logs del backend GPU
    if [ -d "${PROJECT_DIR}/logs/backend-gpu" ]; then
        local error_count=$(find "${PROJECT_DIR}/logs/backend-gpu" -name "*.log" -exec grep -l "ERROR\|FATAL" {} \; 2>/dev/null | wc -l)
        if [ "$error_count" -eq 0 ]; then
            log_success "Logs backend GPU sin errores críticos"
        else
            log_warning "Encontrados $error_count archivos de log con errores"
        fi
    else
        log_warning "Directorio de logs backend GPU no encontrado"
    fi
}

# Test rápido de GPU
quick_gpu_test() {
    log_info "Ejecutando test rápido de GPU..."
    
    # Test simple con nvidia-smi en contenedor
    if docker run --rm --gpus all --name gpu-test-quick nvidia/cuda:12.1-runtime-ubuntu22.04 nvidia-smi > /dev/null 2>&1; then
        log_success "Test rápido GPU: PASSED"
    else
        log_error "Test rápido GPU: FAILED"
        return 1
    fi
}

# Mostrar resumen del health check
show_health_summary() {
    echo ""
    echo -e "${GREEN}📊 Resumen del Health Check GPU${NC}"
    echo -e "${BLUE}================================${NC}"
    
    # Estado general
    local gpu_temp=$(nvidia-smi --query-gpu=temperature.gpu --format=csv,noheader,nounits)
    local gpu_util=$(nvidia-smi --query-gpu=utilization.gpu --format=csv,noheader,nounits) 
    local gpu_power=$(nvidia-smi --query-gpu=power.draw --format=csv,noheader,nounits)
    
    echo -e "${BLUE}Estado GPU:${NC}"
    echo -e "  • Temperatura: ${gpu_temp}°C"
    echo -e "  • Utilización: ${gpu_util}%"
    echo -e "  • Consumo: ${gpu_power}W"
    
    # Contenedores
    local running_containers=$(docker ps --filter "label=gpu=true" | wc -l)
    echo -e "  • Contenedores GPU: $((running_containers - 1)) corriendo"
    
    # URLs útiles
    echo ""
    echo -e "${BLUE}URLs de monitoreo:${NC}"
    echo -e "  • GPU Metrics: http://localhost:8082/metrics"
    echo -e "  • NVIDIA Exporter: http://localhost:9445/metrics"
    echo -e "  • Prometheus: http://localhost:9090"
    echo -e "  • Grafana: http://localhost:3001"
    
    echo ""
    echo -e "${GREEN}✅ Health Check GPU completado${NC}"
}

# Script principal
main() {
    log_info "Iniciando health check GPU..."
    echo ""
    
    # Ejecutar verificaciones
    check_gpu_hardware
    echo ""
    
    check_gpu_containers
    echo ""
    
    check_gpu_services  
    echo ""
    
    check_gpu_metrics
    echo ""
    
    check_gpu_logs
    echo ""
    
    quick_gpu_test
    echo ""
    
    # Mostrar resumen
    show_health_summary
    
    log_success "Health check GPU completo"
}

# Manejo de argumentos
if [ "${1:-}" = "--help" ] || [ "${1:-}" = "-h" ]; then
    echo "GPU Health Check para Protein Docking v2"
    echo ""
    echo "Este script verifica el estado y salud de todos los componentes GPU"
    echo "del sistema de protein docking."
    echo ""
    echo "Uso: $0"
    echo ""
    exit 0
fi

# Manejar señales
trap 'log_error "Health check interrumpido"; exit 1' INT TERM

# Ejecutar health check principal
main "$@"