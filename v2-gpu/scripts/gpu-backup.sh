#!/bin/bash

# GPU Backup Script for Protein Docking v2
# AGENTE 4: DevOps & Infrastructure Specialist
# Backup de datos, configuraciones y kernels GPU

set -euo pipefail

# Colores para output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BACKUP_DIR="${PROJECT_DIR}/backups/gpu"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")

echo -e "${BLUE}💾 GPU Backup - Protein Docking v2${NC}"
echo -e "${BLUE}=================================${NC}"

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

# Crear directorio de backup
create_backup_directory() {
    log_info "Creando directorio de backup..."
    
    mkdir -p "${BACKUP_DIR}/${TIMESTAMP}"
    mkdir -p "${BACKUP_DIR}/${TIMESTAMP}"/{data,configs,logs,kernels}
    
    log_success "Directorio de backup creado: ${BACKUP_DIR}/${TIMESTAMP}"
}

# Backup de configuraciones GPU
backup_gpu_configs() {
    log_info "Respaldando configuraciones GPU..."
    
    # Docker Compose GPU
    cp "${PROJECT_DIR}/docker-compose.gpu.yml" "${BACKUP_DIR}/${TIMESTAMP}/configs/"
    
    # Dockerfiles GPU
    cp -r "${PROJECT_DIR}/backend/Dockerfile.gpu" "${BACKUP_DIR}/${TIMESTAMP}/configs/"
    cp -r "${PROJECT_DIR}/infrastructure/gpu/" "${BACKUP_DIR}/${TIMESTAMP}/configs/" 2>/dev/null || true
    
    # Variables de entorno GPU
    cp "${PROJECT_DIR}/.env.gpu" "${BACKUP_DIR}/${TIMESTAMP}/configs/" 2>/dev/null || true
    
    # Configuraciones de monitoreo GPU
    cp -r "${PROJECT_DIR}/infrastructure/monitoring/prometheus-gpu.yml" "${BACKUP_DIR}/${TIMESTAMP}/configs/" 2>/dev/null || true
    cp -r "${PROJECT_DIR}/infrastructure/monitoring/rules-gpu/" "${BACKUP_DIR}/${TIMESTAMP}/configs/" 2>/dev/null || true
    
    # Configuraciones de PostgreSQL y Redis GPU
    cp -r "${PROJECT_DIR}/infrastructure/postgres/postgresql-gpu.conf" "${BACKUP_DIR}/${TIMESTAMP}/configs/" 2>/dev/null || true
    cp -r "${PROJECT_DIR}/infrastructure/redis/redis-gpu-node-"*.conf "${BACKUP_DIR}/${TIMESTAMP}/configs/" 2>/dev/null || true
    
    log_success "Configuraciones GPU respaldadas"
}

# Backup de datos GPU
backup_gpu_data() {
    log_info "Respaldando datos GPU..."
    
    # GPU cache y kernels (si existen)
    if [ -d "${PROJECT_DIR}/data/gpu-cache" ]; then
        tar -czf "${BACKUP_DIR}/${TIMESTAMP}/data/gpu-cache.tar.gz" -C "${PROJECT_DIR}/data" gpu-cache
        log_success "GPU cache respaldado"
    fi
    
    if [ -d "${PROJECT_DIR}/data/gpu-kernels" ]; then
        tar -czf "${BACKUP_DIR}/${TIMESTAMP}/kernels/gpu-kernels.tar.gz" -C "${PROJECT_DIR}/data" gpu-kernels
        log_success "GPU kernels respaldados"
    fi
    
    # Datos optimizados para GPU
    if [ -d "${PROJECT_DIR}/data/postgres-gpu" ]; then
        tar -czf "${BACKUP_DIR}/${TIMESTAMP}/data/postgres-gpu.tar.gz" -C "${PROJECT_DIR}/data" postgres-gpu
        log_success "PostgreSQL GPU data respaldado"
    fi
    
    if [ -d "${PROJECT_DIR}/data/redis-gpu" ]; then
        tar -czf "${BACKUP_DIR}/${TIMESTAMP}/data/redis-gpu.tar.gz" -C "${PROJECT_DIR}/data" redis-gpu
        log_success "Redis GPU data respaldado"
    fi
    
    # Resultados GPU (NVMe)
    if [ -d "${PROJECT_DIR}/data/results-nvme" ]; then
        tar -czf "${BACKUP_DIR}/${TIMESTAMP}/data/results-nvme.tar.gz" -C "${PROJECT_DIR}/data" results-nvme
        log_success "Resultados NVMe respaldados"
    fi
    
    # Uploads SSD
    if [ -d "${PROJECT_DIR}/data/uploads-ssd" ]; then
        tar -czf "${BACKUP_DIR}/${TIMESTAMP}/data/uploads-ssd.tar.gz" -C "${PROJECT_DIR}/data" uploads-ssd
        log_success "Uploads SSD respaldados"
    fi
}

# Backup de logs GPU
backup_gpu_logs() {
    log_info "Respaldando logs GPU..."
    
    # Logs de servicios GPU
    for service in backend-gpu frontend-gpu postgres-gpu nginx-gpu grafana-gpu redis-gpu; do
        if [ -d "${PROJECT_DIR}/logs/${service}" ]; then
            tar -czf "${BACKUP_DIR}/${TIMESTAMP}/logs/${service}.tar.gz" -C "${PROJECT_DIR}/logs" "${service}"
            log_success "Logs ${service} respaldados"
        fi
    done
}

# Backup de métricas GPU
backup_gpu_metrics() {
    log_info "Respaldando métricas GPU..."
    
    # Datos de Prometheus GPU
    if [ -d "${PROJECT_DIR}/data/prometheus-gpu" ]; then
        tar -czf "${BACKUP_DIR}/${TIMESTAMP}/data/prometheus-gpu.tar.gz" -C "${PROJECT_DIR}/data" prometheus-gpu
        log_success "Métricas Prometheus GPU respaldadas"
    fi
    
    # Dashboards Grafana GPU
    if [ -d "${PROJECT_DIR}/data/grafana-gpu" ]; then
        tar -czf "${BACKUP_DIR}/${TIMESTAMP}/data/grafana-gpu.tar.gz" -C "${PROJECT_DIR}/data" grafana-gpu
        log_success "Dashboards Grafana GPU respaldados"
    fi
}

# Crear manifiesto del backup
create_backup_manifest() {
    log_info "Creando manifiesto del backup..."
    
    local manifest_file="${BACKUP_DIR}/${TIMESTAMP}/MANIFEST.txt"
    
    cat > "$manifest_file" << EOF
# GPU Backup Manifest - Protein Docking v2
# Timestamp: ${TIMESTAMP}
# Date: $(date)
# Host: $(hostname)

## System Information
GPU_INFO: $(nvidia-smi --query-gpu=name,driver_version,memory.total --format=csv,noheader,nounits)
CUDA_VERSION: $(nvidia-smi --query-gpu=cuda_version --format=csv,noheader,nounits 2>/dev/null || echo "N/A")
DOCKER_VERSION: $(docker --version)

## Backup Contents
CONFIGS: $(ls -la "${BACKUP_DIR}/${TIMESTAMP}/configs/" 2>/dev/null | wc -l) files
DATA: $(ls -la "${BACKUP_DIR}/${TIMESTAMP}/data/" 2>/dev/null | wc -l) archives
LOGS: $(ls -la "${BACKUP_DIR}/${TIMESTAMP}/logs/" 2>/dev/null | wc -l) archives
KERNELS: $(ls -la "${BACKUP_DIR}/${TIMESTAMP}/kernels/" 2>/dev/null | wc -l) archives

## File Checksums
EOF
    
    # Añadir checksums de archivos importantes
    find "${BACKUP_DIR}/${TIMESTAMP}" -type f -name "*.tar.gz" -exec sha256sum {} \; >> "$manifest_file"
    find "${BACKUP_DIR}/${TIMESTAMP}" -type f -name "*.yml" -exec sha256sum {} \; >> "$manifest_file"
    find "${BACKUP_DIR}/${TIMESTAMP}" -type f -name "*.conf" -exec sha256sum {} \; >> "$manifest_file"
    
    log_success "Manifiesto del backup creado"
}

# Comprimir backup completo
compress_backup() {
    log_info "Comprimiendo backup completo..."
    
    cd "${BACKUP_DIR}"
    tar -czf "gpu_backup_${TIMESTAMP}.tar.gz" "${TIMESTAMP}/"
    
    # Calcular tamaño del backup
    local backup_size=$(du -h "gpu_backup_${TIMESTAMP}.tar.gz" | cut -f1)
    
    log_success "Backup comprimido: gpu_backup_${TIMESTAMP}.tar.gz (${backup_size})"
    
    # Opcional: eliminar directorio temporal
    if [ "${KEEP_TEMP_DIR:-false}" != "true" ]; then
        rm -rf "${TIMESTAMP}/"
        log_info "Directorio temporal eliminado"
    fi
}

# Limpiar backups antiguos
cleanup_old_backups() {
    log_info "Limpiando backups antiguos..."
    
    # Mantener solo los últimos 7 backups
    local keep_count=7
    local old_backups=$(ls -t "${BACKUP_DIR}"/gpu_backup_*.tar.gz 2>/dev/null | tail -n +$((keep_count + 1)))
    
    if [ -n "$old_backups" ]; then
        echo "$old_backups" | xargs rm -f
        local removed_count=$(echo "$old_backups" | wc -l)
        log_success "Eliminados $removed_count backups antiguos"
    else
        log_info "No hay backups antiguos para eliminar"
    fi
}

# Mostrar resumen del backup
show_backup_summary() {
    echo ""
    echo -e "${GREEN}📦 Resumen del Backup GPU${NC}"
    echo -e "${BLUE}=========================${NC}"
    
    local backup_file="${BACKUP_DIR}/gpu_backup_${TIMESTAMP}.tar.gz"
    local backup_size=$(du -h "$backup_file" | cut -f1)
    
    echo -e "${BLUE}Backup GPU creado:${NC}"
    echo -e "  • Archivo: gpu_backup_${TIMESTAMP}.tar.gz"
    echo -e "  • Tamaño: ${backup_size}"
    echo -e "  • Ubicación: ${BACKUP_DIR}"
    
    echo ""
    echo -e "${BLUE}Contenido respaldado:${NC}"
    echo -e "  ✅ Configuraciones GPU (Docker Compose, Dockerfiles)"
    echo -e "  ✅ Variables de entorno GPU"
    echo -e "  ✅ Configuraciones de monitoreo"
    echo -e "  ✅ Datos GPU (cache, kernels)"
    echo -e "  ✅ Logs de servicios GPU"
    echo -e "  ✅ Métricas y dashboards"
    
    echo ""
    echo -e "${BLUE}Para restaurar:${NC}"
    echo -e "  tar -xzf gpu_backup_${TIMESTAMP}.tar.gz"
    echo -e "  ./scripts/gpu-restore.sh ${TIMESTAMP}"
    
    echo ""
    echo -e "${GREEN}✅ Backup GPU completado exitosamente${NC}"
}

# Script principal
main() {
    log_info "Iniciando backup GPU..."
    echo ""
    
    # Crear directorio de backup
    create_backup_directory
    echo ""
    
    # Realizar backups
    backup_gpu_configs
    echo ""
    
    backup_gpu_data
    echo ""
    
    backup_gpu_logs
    echo ""
    
    backup_gpu_metrics
    echo ""
    
    # Crear manifiesto y comprimir
    create_backup_manifest
    echo ""
    
    compress_backup
    echo ""
    
    cleanup_old_backups
    echo ""
    
    # Mostrar resumen
    show_backup_summary
    
    log_success "Backup GPU completo"
}

# Manejo de argumentos
if [ "${1:-}" = "--help" ] || [ "${1:-}" = "-h" ]; then
    echo "GPU Backup para Protein Docking v2"
    echo ""
    echo "Este script crea un backup completo de todos los datos, configuraciones"
    echo "y componentes relacionados con GPU del sistema."
    echo ""
    echo "Uso: $0 [--keep-temp]"
    echo ""
    echo "Opciones:"
    echo "  --keep-temp    Mantener directorio temporal después de comprimir"
    echo ""
    exit 0
fi

if [ "${1:-}" = "--keep-temp" ]; then
    export KEEP_TEMP_DIR=true
fi

# Manejar señales
trap 'log_error "Backup interrumpido"; exit 1' INT TERM

# Ejecutar backup principal
main "$@"