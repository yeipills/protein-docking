#!/bin/bash

# Docker Setup Script - Resuelve conflictos y prepara el entorno
# AGENTE 4: DevOps & Infrastructure Specialist
# Protein Docking v2 - Ultra Performance Edition

set -euo pipefail

# Colores para output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuración
PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DATA_DIR="${PROJECT_DIR}/data"
LOGS_DIR="${PROJECT_DIR}/logs"
BACKUP_DIR="${PROJECT_DIR}/backups"

echo -e "${BLUE}🧬 Protein Docking v2 - Docker Setup${NC}"
echo -e "${BLUE}=====================================${NC}"

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

# Verificar requisitos del sistema
check_system_requirements() {
    log_info "Verificando requisitos del sistema..."
    
    # Docker
    if ! command -v docker &> /dev/null; then
        log_error "Docker no está instalado. Por favor instala Docker primero."
        exit 1
    fi
    
    # Docker Compose
    if ! command -v docker-compose &> /dev/null; then
        log_error "Docker Compose no está instalado. Por favor instala Docker Compose primero."
        exit 1
    fi
    
    # Verificar versiones
    DOCKER_VERSION=$(docker --version | cut -d' ' -f3 | cut -d',' -f1)
    COMPOSE_VERSION=$(docker-compose --version | cut -d' ' -f3 | cut -d',' -f1)
    
    log_success "Docker $DOCKER_VERSION detectado"
    log_success "Docker Compose $COMPOSE_VERSION detectado"
    
    # Verificar que Docker está corriendo
    if ! docker info &> /dev/null; then
        log_error "Docker daemon no está corriendo. Por favor inicia Docker."
        exit 1
    fi
    
    # Verificar recursos disponibles
    TOTAL_MEM=$(free -g | awk '/^Mem:/{print $2}')
    if [ "$TOTAL_MEM" -lt 16 ]; then
        log_warning "Se recomienda al menos 16GB de RAM para óptimo rendimiento (detectado: ${TOTAL_MEM}GB)"
    fi
    
    # Verificar espacio en disco
    AVAILABLE_SPACE=$(df -BG . | awk 'NR==2 {print $4}' | sed 's/G//')
    if [ "$AVAILABLE_SPACE" -lt 50 ]; then
        log_warning "Se recomienda al menos 50GB de espacio libre (disponible: ${AVAILABLE_SPACE}GB)"
    fi
    
    log_success "Requisitos del sistema verificados"
}

# Crear estructura de directorios
create_directory_structure() {
    log_info "Creando estructura de directorios..."
    
    # Directorios de datos
    mkdir -p "${DATA_DIR}"/{postgres,redis/{node1,node2,node3},clickhouse,prometheus,grafana,uploads,results}
    
    # Directorios de logs
    mkdir -p "${LOGS_DIR}"/{backend,frontend,postgres,nginx,clickhouse,grafana,redis}
    
    # Directorios de backup
    mkdir -p "${BACKUP_DIR}"/{daily,weekly,monthly}
    
    # Directorios de SSL (para futura configuración HTTPS)
    mkdir -p "${PROJECT_DIR}/infrastructure/nginx/ssl"
    
    # Directorios de configuración
    mkdir -p "${PROJECT_DIR}/infrastructure"/{postgres,redis,nginx/conf.d,monitoring/{prometheus,grafana/{dashboards,datasources}}}
    
    # Permisos correctos
    sudo chown -R $USER:$USER "${DATA_DIR}" "${LOGS_DIR}" "${BACKUP_DIR}"
    chmod -R 755 "${DATA_DIR}" "${LOGS_DIR}"
    
    log_success "Estructura de directorios creada"
}

# Resolver conflictos de configuración
resolve_configuration_conflicts() {
    log_info "Resolviendo conflictos de configuración Docker..."
    
    # Backup de configuraciones existentes
    if [ -f "${PROJECT_DIR}/docker-compose.yml" ]; then
        cp "${PROJECT_DIR}/docker-compose.yml" "${BACKUP_DIR}/docker-compose.yml.backup.$(date +%Y%m%d_%H%M%S)"
        log_info "Backup de docker-compose.yml creado"
    fi
    
    if [ -f "${PROJECT_DIR}/infrastructure/docker-compose.yml" ]; then
        cp "${PROJECT_DIR}/infrastructure/docker-compose.yml" "${BACKUP_DIR}/infrastructure-docker-compose.yml.backup.$(date +%Y%m%d_%H%M%S)"
        log_info "Backup de infrastructure/docker-compose.yml creado"
    fi
    
    # Usar la configuración unificada
    if [ -f "${PROJECT_DIR}/docker-compose.unified.yml" ]; then
        cp "${PROJECT_DIR}/docker-compose.unified.yml" "${PROJECT_DIR}/docker-compose.yml"
        log_success "Configuración Docker unificada aplicada"
    else
        log_error "Archivo docker-compose.unified.yml no encontrado"
        exit 1
    fi
    
    # Limpiar configuraciones conflictivas
    if [ -f "${PROJECT_DIR}/infrastructure/docker-compose.yml" ]; then
        mv "${PROJECT_DIR}/infrastructure/docker-compose.yml" "${PROJECT_DIR}/infrastructure/docker-compose.yml.disabled"
        log_info "Configuración conflictiva en infrastructure/ deshabilitada"
    fi
}

# Configurar variables de entorno
setup_environment_variables() {
    log_info "Configurando variables de entorno..."
    
    # Crear archivo .env si no existe
    if [ ! -f "${PROJECT_DIR}/.env" ]; then
        cat > "${PROJECT_DIR}/.env" << EOF
# Protein Docking v2 - Environment Configuration
# Auto-generated by docker-setup.sh

# Database Configuration
POSTGRES_DB=protein_docking
POSTGRES_USER=protein_user
POSTGRES_PASSWORD=secure_protein_pass_2024
DATABASE_URL=postgresql://protein_user:secure_protein_pass_2024@postgres:5432/protein_docking

# Redis Configuration
REDIS_PASSWORD=protein_redis_secure_2024

# ClickHouse Configuration
CLICKHOUSE_DB=protein_analytics
CLICKHOUSE_USER=analytics_user
CLICKHOUSE_PASSWORD=secure_analytics_pass_2024

# Security
JWT_SECRET=ultra_secure_jwt_secret_key_2024_protein_docking

# Performance
RUST_LOG=info
TOKIO_WORKER_THREADS=16
RAYON_NUM_THREADS=16

# Frontend
NODE_ENV=production
VITE_API_URL=http://localhost:8080/api/v1
VITE_WS_URL=ws://localhost:8080/ws

# Monitoring
GRAFANA_ADMIN_PASSWORD=secure_grafana_pass_2024

# Network
DOCKER_SUBNET=172.30.0.0/16
DOCKER_GATEWAY=172.30.0.1
EOF
        log_success "Archivo .env creado con configuración por defecto"
    else
        log_info "Archivo .env ya existe, manteniendo configuración actual"
    fi
}

# Validar configuración Docker
validate_docker_configuration() {
    log_info "Validando configuración Docker..."
    
    # Verificar sintaxis de docker-compose
    if ! docker-compose -f "${PROJECT_DIR}/docker-compose.yml" config > /dev/null 2>&1; then
        log_error "Error en la configuración de docker-compose.yml"
        docker-compose -f "${PROJECT_DIR}/docker-compose.yml" config
        exit 1
    fi
    
    log_success "Configuración Docker validada correctamente"
}

# Optimizar configuración del sistema
optimize_system_configuration() {
    log_info "Aplicando optimizaciones del sistema..."
    
    # Configurar límites del sistema para Docker
    if [ -f /etc/security/limits.conf ]; then
        # Verificar si ya están configurados los límites
        if ! grep -q "# Docker optimization for protein docking" /etc/security/limits.conf; then
            log_info "Configurando límites del sistema..."
            sudo tee -a /etc/security/limits.conf > /dev/null << EOF

# Docker optimization for protein docking
* soft nofile 65536
* hard nofile 65536
* soft nproc 65536
* hard nproc 65536
EOF
            log_success "Límites del sistema configurados"
        fi
    fi
    
    # Configurar sysctl para red y memoria
    if [ -f /etc/sysctl.conf ]; then
        if ! grep -q "# Protein docking optimizations" /etc/sysctl.conf; then
            log_info "Configurando parámetros del kernel..."
            sudo tee -a /etc/sysctl.conf > /dev/null << EOF

# Protein docking optimizations
vm.max_map_count=262144
net.core.somaxconn=65535
net.core.netdev_max_backlog=5000
net.ipv4.tcp_congestion_control=bbr
net.ipv4.tcp_fastopen=3
fs.file-max=2097152
EOF
            sudo sysctl -p
            log_success "Parámetros del kernel optimizados"
        fi
    fi
}

# Limpiar recursos previos si existen
cleanup_previous_deployment() {
    log_info "Limpiando deployment previo..."
    
    # Parar contenedores existentes
    if docker-compose -f "${PROJECT_DIR}/docker-compose.yml" ps | grep -q Up; then
        log_info "Parando contenedores existentes..."
        docker-compose -f "${PROJECT_DIR}/docker-compose.yml" down --remove-orphans
    fi
    
    # Limpiar imágenes órfanas
    if [ "$(docker images -f "dangling=true" -q)" ]; then
        log_info "Limpiando imágenes órfanas..."
        docker rmi $(docker images -f "dangling=true" -q) 2>/dev/null || true
    fi
    
    # Limpiar volúmenes órfanos (solo si se solicita explícitamente)
    if [ "${CLEAN_VOLUMES:-false}" = "true" ]; then
        log_warning "Limpiando volúmenes órfanos (datos serán perdidos)..."
        docker volume prune -f
    fi
    
    log_success "Limpieza completada"
}

# Construir imágenes
build_images() {
    log_info "Construyendo imágenes Docker..."
    
    # Construir en paralelo para mejor rendimiento
    docker-compose -f "${PROJECT_DIR}/docker-compose.yml" build --parallel --no-cache
    
    log_success "Imágenes construidas exitosamente"
}

# Inicializar servicios en orden correcto
initialize_services() {
    log_info "Inicializando servicios..."
    
    # Iniciar servicios de base en orden de dependencias
    log_info "Iniciando servicios de base de datos..."
    docker-compose -f "${PROJECT_DIR}/docker-compose.yml" up -d postgres clickhouse
    
    # Esperar a que estén listos
    log_info "Esperando a que las bases de datos estén listas..."
    sleep 30
    
    # Verificar salud de PostgreSQL
    until docker-compose -f "${PROJECT_DIR}/docker-compose.yml" exec -T postgres pg_isready -U protein_user -d protein_docking; do
        log_info "Esperando PostgreSQL..."
        sleep 5
    done
    log_success "PostgreSQL listo"
    
    # Iniciar Redis cluster
    log_info "Iniciando cluster Redis..."
    docker-compose -f "${PROJECT_DIR}/docker-compose.yml" up -d redis-1 redis-2 redis-3
    sleep 15
    
    # Inicializar cluster Redis
    log_info "Inicializando cluster Redis..."
    docker-compose -f "${PROJECT_DIR}/docker-compose.yml" up redis-cluster-init
    
    # Iniciar servicios de aplicación
    log_info "Iniciando servicios de aplicación..."
    docker-compose -f "${PROJECT_DIR}/docker-compose.yml" up -d backend frontend
    
    # Esperar a que estén listos
    sleep 20
    
    # Iniciar servicios de infraestructura
    log_info "Iniciando servicios de infraestructura..."
    docker-compose -f "${PROJECT_DIR}/docker-compose.yml" up -d nginx prometheus grafana
    
    # Iniciar exporters de métricas
    log_info "Iniciando exporters de métricas..."
    docker-compose -f "${PROJECT_DIR}/docker-compose.yml" up -d node-exporter redis-exporter postgres-exporter
    
    log_success "Todos los servicios iniciados"
}

# Verificar health de servicios
verify_services_health() {
    log_info "Verificando salud de servicios..."
    
    local max_attempts=30
    local attempt=1
    
    # Servicios a verificar
    local services=("backend:8080/health" "frontend:3000" "grafana:3000/api/health")
    
    for service in "${services[@]}"; do
        local service_name=$(echo $service | cut -d: -f1)
        local endpoint=$(echo $service | cut -d: -f2-)
        
        log_info "Verificando $service_name..."
        
        attempt=1
        while [ $attempt -le $max_attempts ]; do
            if curl -f -s "http://localhost:$endpoint" > /dev/null 2>&1; then
                log_success "$service_name está saludable"
                break
            fi
            
            if [ $attempt -eq $max_attempts ]; then
                log_warning "$service_name no responde después de $max_attempts intentos"
            else
                sleep 5
                ((attempt++))
            fi
        done
    done
}

# Mostrar estado final
show_final_status() {
    echo ""
    echo -e "${GREEN}🎉 Setup de Docker completado exitosamente!${NC}"
    echo -e "${BLUE}============================================${NC}"
    echo ""
    echo -e "${BLUE}Servicios disponibles:${NC}"
    echo -e "  • Frontend:     ${GREEN}http://localhost:3000${NC}"
    echo -e "  • Backend API:  ${GREEN}http://localhost:8080${NC}"
    echo -e "  • Grafana:      ${GREEN}http://localhost:3001${NC} (admin/secure_grafana_pass_2024)"
    echo -e "  • Prometheus:   ${GREEN}http://localhost:9090${NC}"
    echo -e "  • ClickHouse:   ${GREEN}http://localhost:8123${NC}"
    echo ""
    echo -e "${BLUE}Bases de datos:${NC}"
    echo -e "  • PostgreSQL:   ${GREEN}localhost:5432${NC} (protein_user/secure_protein_pass_2024)"
    echo -e "  • Redis Cluster: ${GREEN}localhost:6379,6380,6381${NC}"
    echo ""
    echo -e "${BLUE}Comandos útiles:${NC}"
    echo -e "  • Ver logs:     ${YELLOW}docker-compose logs -f [servicio]${NC}"
    echo -e "  • Parar todo:   ${YELLOW}docker-compose down${NC}"
    echo -e "  • Reiniciar:    ${YELLOW}docker-compose restart [servicio]${NC}"
    echo -e "  • Ver estado:   ${YELLOW}docker-compose ps${NC}"
    echo ""
    echo -e "${BLUE}Directorios importantes:${NC}"
    echo -e "  • Datos:        ${YELLOW}${DATA_DIR}${NC}"
    echo -e "  • Logs:         ${YELLOW}${LOGS_DIR}${NC}"
    echo -e "  • Backups:      ${YELLOW}${BACKUP_DIR}${NC}"
    echo ""
}

# Script principal
main() {
    echo -e "${BLUE}Iniciando setup de Docker para Protein Docking v2...${NC}"
    echo ""
    
    # Verificar si se está ejecutando con permisos adecuados
    if [ "$EUID" -eq 0 ]; then
        log_warning "No ejecutes este script como root. Usa tu usuario normal."
        exit 1
    fi
    
    # Pasos del setup
    check_system_requirements
    create_directory_structure
    resolve_configuration_conflicts
    setup_environment_variables
    validate_docker_configuration
    optimize_system_configuration
    cleanup_previous_deployment
    build_images
    initialize_services
    verify_services_health
    show_final_status
    
    log_success "Setup completo! Protein Docking v2 está listo para usar."
}

# Manejar señales
trap 'log_error "Setup interrumpido"; exit 1' INT TERM

# Verificar argumentos
if [ "${1:-}" = "--clean-volumes" ]; then
    export CLEAN_VOLUMES=true
    log_warning "Modo de limpieza completa activado - se eliminarán todos los volúmenes"
fi

if [ "${1:-}" = "--help" ] || [ "${1:-}" = "-h" ]; then
    echo "Uso: $0 [opciones]"
    echo ""
    echo "Opciones:"
    echo "  --clean-volumes    Limpiar todos los volúmenes (¡perderás todos los datos!)"
    echo "  --help, -h         Mostrar esta ayuda"
    echo ""
    exit 0
fi

# Ejecutar setup principal
main "$@"