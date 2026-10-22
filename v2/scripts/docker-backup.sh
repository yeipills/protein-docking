#!/bin/bash

# Docker Backup Script
# AGENTE 4: DevOps & Infrastructure Specialist
# Backup automatizado de datos críticos del sistema

set -euo pipefail

# Colores para output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BACKUP_DIR="${PROJECT_DIR}/backups"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

echo -e "${BLUE}💾 Protein Docking v2 - Backup System${NC}"
echo -e "${BLUE}=====================================${NC}"

# Configuración de backup
BACKUP_TYPE="${1:-daily}"  # daily, weekly, monthly
RETENTION_DAYS=30
RETENTION_WEEKLY=8
RETENTION_MONTHLY=6

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

# Crear directorios de backup
create_backup_structure() {
    mkdir -p "${BACKUP_DIR}"/{daily,weekly,monthly}/{postgres,redis,clickhouse,files,configs}
    log_info "Estructura de backup verificada"
}

# Backup de PostgreSQL
backup_postgresql() {
    log_info "Iniciando backup de PostgreSQL..."
    
    local backup_file="${BACKUP_DIR}/${BACKUP_TYPE}/postgres/postgres_backup_${TIMESTAMP}.sql.gz"
    
    # Crear backup comprimido
    if docker exec protein-docking-postgres pg_dump -U protein_user -d protein_docking | gzip > "$backup_file"; then
        local size=$(du -h "$backup_file" | cut -f1)
        log_success "PostgreSQL backup completado: $size ($backup_file)"
        
        # Verificar integridad del backup
        if gunzip -t "$backup_file" 2>/dev/null; then
            log_success "Backup de PostgreSQL verificado (integridad OK)"
        else
            log_error "Backup de PostgreSQL corrupto"
            return 1
        fi
    else
        log_error "Falló el backup de PostgreSQL"
        return 1
    fi
}

# Backup de Redis Cluster
backup_redis() {
    log_info "Iniciando backup de Redis Cluster..."
    
    local backup_dir="${BACKUP_DIR}/${BACKUP_TYPE}/redis/redis_backup_${TIMESTAMP}"
    mkdir -p "$backup_dir"
    
    # Backup de cada nodo del cluster
    local nodes=("redis-1:6379" "redis-2:6380" "redis-3:6381")
    
    for node in "${nodes[@]}"; do
        local container_name="protein-docking-${node%:*}"
        local port="${node#*:}"
        local node_backup="${backup_dir}/${node%:*}_${port}.rdb"
        
        # Crear snapshot y copiarlo
        if docker exec "$container_name" redis-cli -p "$port" BGSAVE; then
            # Esperar a que termine el background save
            sleep 10
            
            # Copiar el archivo RDB
            if docker cp "${container_name}:/data/dump.rdb" "$node_backup"; then
                local size=$(du -h "$node_backup" | cut -f1)
                log_success "Redis node ${node%:*} backup: $size"
            else
                log_error "Falló copia de Redis node ${node%:*}"
            fi
        else
            log_error "Falló BGSAVE en Redis node ${node%:*}"
        fi
    done
    
    # Comprimir directorio completo
    if tar -czf "${backup_dir}.tar.gz" -C "${BACKUP_DIR}/${BACKUP_TYPE}/redis" "$(basename "$backup_dir")"; then
        rm -rf "$backup_dir"
        local size=$(du -h "${backup_dir}.tar.gz" | cut -f1)
        log_success "Redis cluster backup comprimido: $size"
    fi
}

# Backup de ClickHouse
backup_clickhouse() {
    log_info "Iniciando backup de ClickHouse..."
    
    local backup_file="${BACKUP_DIR}/${BACKUP_TYPE}/clickhouse/clickhouse_backup_${TIMESTAMP}.sql.gz"
    
    # Crear backup de la base de datos de analytics
    if docker exec protein-docking-clickhouse clickhouse-client --query="SHOW CREATE DATABASE protein_analytics" > /tmp/ch_schema.sql && \
       docker exec protein-docking-clickhouse clickhouse-client --database=protein_analytics --query="SHOW TABLES" | \
       while read table; do
           docker exec protein-docking-clickhouse clickhouse-client --database=protein_analytics --query="SELECT * FROM $table FORMAT Native" | gzip >> "$backup_file"
       done; then
        
        local size=$(du -h "$backup_file" | cut -f1)
        log_success "ClickHouse backup completado: $size"
    else
        log_error "Falló el backup de ClickHouse"
        return 1
    fi
}

# Backup de archivos de usuario
backup_user_files() {
    log_info "Iniciando backup de archivos de usuario..."
    
    local files_backup="${BACKUP_DIR}/${BACKUP_TYPE}/files/user_files_${TIMESTAMP}.tar.gz"
    
    # Backup de uploads y results
    local dirs_to_backup=("${PROJECT_DIR}/data/uploads" "${PROJECT_DIR}/data/results")
    local backup_paths=""
    
    for dir in "${dirs_to_backup[@]}"; do
        if [ -d "$dir" ] && [ "$(ls -A "$dir" 2>/dev/null)" ]; then
            backup_paths="$backup_paths $dir"
        fi
    done
    
    if [ -n "$backup_paths" ]; then
        if tar -czf "$files_backup" $backup_paths 2>/dev/null; then
            local size=$(du -h "$files_backup" | cut -f1)
            log_success "Archivos de usuario backup: $size"
        else
            log_error "Falló backup de archivos de usuario"
        fi
    else
        log_info "No hay archivos de usuario para respaldar"
        touch "$files_backup"  # Crear archivo vacío para mantener consistencia
    fi
}

# Backup de configuraciones
backup_configurations() {
    log_info "Iniciando backup de configuraciones..."
    
    local config_backup="${BACKUP_DIR}/${BACKUP_TYPE}/configs/configurations_${TIMESTAMP}.tar.gz"
    
    # Archivos de configuración a respaldar
    local config_files=(
        "${PROJECT_DIR}/docker-compose.yml"
        "${PROJECT_DIR}/.env"
        "${PROJECT_DIR}/infrastructure"
        "${PROJECT_DIR}/scripts"
    )
    
    local existing_files=""
    for file in "${config_files[@]}"; do
        if [ -e "$file" ]; then
            existing_files="$existing_files $file"
        fi
    done
    
    if [ -n "$existing_files" ]; then
        if tar -czf "$config_backup" $existing_files 2>/dev/null; then
            local size=$(du -h "$config_backup" | cut -f1)
            log_success "Configuraciones backup: $size"
        else
            log_error "Falló backup de configuraciones"
        fi
    fi
}

# Limpiar backups antiguos
cleanup_old_backups() {
    log_info "Limpiando backups antiguos..."
    
    # Configurar retención según tipo de backup
    local retention_days
    case "$BACKUP_TYPE" in
        daily)
            retention_days=$RETENTION_DAYS
            ;;
        weekly)
            retention_days=$((RETENTION_WEEKLY * 7))
            ;;
        monthly)
            retention_days=$((RETENTION_MONTHLY * 30))
            ;;
        *)
            retention_days=$RETENTION_DAYS
            ;;
    esac
    
    # Limpiar archivos antiguos
    local cleaned=0
    find "${BACKUP_DIR}/${BACKUP_TYPE}" -type f -mtime +$retention_days -name "*backup*" | while read old_file; do
        rm -f "$old_file"
        ((cleaned++)) || true
        log_info "Eliminado backup antiguo: $(basename "$old_file")"
    done
    
    if [ $cleaned -gt 0 ]; then
        log_success "Limpiados $cleaned backups antiguos (retención: $retention_days días)"
    else
        log_info "No hay backups antiguos para limpiar"
    fi
}

# Verificar integridad de backups
verify_backups() {
    log_info "Verificando integridad de backups..."
    
    local backup_date_dir="${BACKUP_DIR}/${BACKUP_TYPE}"
    local issues=0
    
    # Verificar archivos SQL comprimidos
    find "$backup_date_dir" -name "*.sql.gz" | while read backup_file; do
        if ! gunzip -t "$backup_file" 2>/dev/null; then
            log_error "Backup corrupto: $(basename "$backup_file")"
            ((issues++)) || true
        fi
    done
    
    # Verificar archivos TAR comprimidos
    find "$backup_date_dir" -name "*.tar.gz" | while read backup_file; do
        if ! tar -tzf "$backup_file" >/dev/null 2>&1; then
            log_error "Backup corrupto: $(basename "$backup_file")"
            ((issues++)) || true
        fi
    done
    
    if [ $issues -eq 0 ]; then
        log_success "Todos los backups verificados correctamente"
    else
        log_warning "Se encontraron $issues backups con problemas"
    fi
}

# Generar reporte de backup
generate_backup_report() {
    log_info "Generando reporte de backup..."
    
    local report_file="${BACKUP_DIR}/${BACKUP_TYPE}/backup_report_${TIMESTAMP}.txt"
    
    cat > "$report_file" << EOF
Protein Docking v2 - Backup Report
==================================
Fecha: $(date)
Tipo: $BACKUP_TYPE
Timestamp: $TIMESTAMP

Resumen de Backups:
EOF
    
    # Agregar información de cada backup
    local backup_dir="${BACKUP_DIR}/${BACKUP_TYPE}"
    
    echo "" >> "$report_file"
    echo "PostgreSQL:" >> "$report_file"
    find "$backup_dir/postgres" -name "*${TIMESTAMP}*" -exec ls -lh {} \; | awk '{print "  " $9 " (" $5 ")"}' >> "$report_file" 2>/dev/null || echo "  No backup found" >> "$report_file"
    
    echo "" >> "$report_file"
    echo "Redis Cluster:" >> "$report_file"
    find "$backup_dir/redis" -name "*${TIMESTAMP}*" -exec ls -lh {} \; | awk '{print "  " $9 " (" $5 ")"}' >> "$report_file" 2>/dev/null || echo "  No backup found" >> "$report_file"
    
    echo "" >> "$report_file"
    echo "ClickHouse:" >> "$report_file"
    find "$backup_dir/clickhouse" -name "*${TIMESTAMP}*" -exec ls -lh {} \; | awk '{print "  " $9 " (" $5 ")"}' >> "$report_file" 2>/dev/null || echo "  No backup found" >> "$report_file"
    
    echo "" >> "$report_file"
    echo "User Files:" >> "$report_file"
    find "$backup_dir/files" -name "*${TIMESTAMP}*" -exec ls -lh {} \; | awk '{print "  " $9 " (" $5 ")"}' >> "$report_file" 2>/dev/null || echo "  No backup found" >> "$report_file"
    
    echo "" >> "$report_file"
    echo "Configurations:" >> "$report_file"
    find "$backup_dir/configs" -name "*${TIMESTAMP}*" -exec ls -lh {} \; | awk '{print "  " $9 " (" $5 ")"}' >> "$report_file" 2>/dev/null || echo "  No backup found" >> "$report_file"
    
    # Calcular tamaño total
    local total_size=$(du -sh "$backup_dir" 2>/dev/null | cut -f1)
    echo "" >> "$report_file"
    echo "Tamaño total de backups ($BACKUP_TYPE): $total_size" >> "$report_file"
    
    log_success "Reporte generado: $report_file"
}

# Verificar servicios antes del backup
check_services() {
    log_info "Verificando servicios antes del backup..."
    
    local required_services=("protein-docking-postgres" "protein-docking-redis-1" "protein-docking-clickhouse")
    local services_ok=true
    
    for service in "${required_services[@]}"; do
        if ! docker ps --format "{{.Names}}" | grep -q "$service"; then
            log_error "Servicio $service no está corriendo"
            services_ok=false
        fi
    done
    
    if [ "$services_ok" = false ]; then
        log_error "No se puede continuar con el backup - servicios no disponibles"
        exit 1
    fi
    
    log_success "Todos los servicios están disponibles"
}

# Script principal
main() {
    log_info "Iniciando proceso de backup ($BACKUP_TYPE)..."
    
    # Verificaciones preliminares
    check_services
    create_backup_structure
    
    # Realizar backups
    backup_postgresql
    backup_redis
    backup_clickhouse
    backup_user_files
    backup_configurations
    
    # Verificación y limpieza
    verify_backups
    cleanup_old_backups
    generate_backup_report
    
    # Reporte final
    local total_size=$(du -sh "${BACKUP_DIR}/${BACKUP_TYPE}" 2>/dev/null | cut -f1)
    log_success "Backup completado exitosamente!"
    log_info "Tamaño total: $total_size"
    log_info "Ubicación: ${BACKUP_DIR}/${BACKUP_TYPE}"
}

# Manejo de argumentos
case "${1:-daily}" in
    daily|weekly|monthly)
        BACKUP_TYPE="$1"
        ;;
    --help|-h)
        echo "Uso: $0 [tipo]"
        echo ""
        echo "Tipos de backup:"
        echo "  daily    - Backup diario (retención: $RETENTION_DAYS días)"
        echo "  weekly   - Backup semanal (retención: $RETENTION_WEEKLY semanas)"
        echo "  monthly  - Backup mensual (retención: $RETENTION_MONTHLY meses)"
        echo ""
        echo "Ejemplo: $0 weekly"
        exit 0
        ;;
    *)
        log_error "Tipo de backup inválido: $1"
        echo "Tipos válidos: daily, weekly, monthly"
        exit 1
        ;;
esac

# Verificar Docker
if ! command -v docker &> /dev/null; then
    log_error "Docker no está instalado o no está disponible"
    exit 1
fi

# Ejecutar backup
main