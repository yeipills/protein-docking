#!/bin/bash

# Docker Health Check Script
# AGENTE 4: DevOps & Infrastructure Specialist
# Verifica el estado de todos los servicios y proporciona diagnósticos

set -euo pipefail

# Colores para output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo -e "${BLUE}🔍 Protein Docking v2 - Health Check${NC}"
echo -e "${BLUE}====================================${NC}"

# Verificar estado de contenedores
check_container_status() {
    echo -e "\n${BLUE}📦 Estado de Contenedores:${NC}"
    
    local containers=(
        "protein-docking-backend"
        "protein-docking-frontend" 
        "protein-docking-postgres"
        "protein-docking-redis-1"
        "protein-docking-redis-2"
        "protein-docking-redis-3"
        "protein-docking-clickhouse"
        "protein-docking-nginx"
        "protein-docking-prometheus"
        "protein-docking-grafana"
    )
    
    for container in "${containers[@]}"; do
        if docker ps --format "table {{.Names}}\t{{.Status}}" | grep -q "$container"; then
            local status=$(docker ps --format "table {{.Names}}\t{{.Status}}" | grep "$container" | awk '{print $2}')
            if [[ "$status" == "Up" ]]; then
                echo -e "  ✅ $container: ${GREEN}Running${NC}"
            else
                echo -e "  ⚠️  $container: ${YELLOW}$status${NC}"
            fi
        else
            echo -e "  ❌ $container: ${RED}Not running${NC}"
        fi
    done
}

# Verificar health checks
check_health_endpoints() {
    echo -e "\n${BLUE}🏥 Health Endpoints:${NC}"
    
    local endpoints=(
        "Backend:http://localhost:8080/health"
        "Frontend:http://localhost:3000"
        "Nginx:http://localhost:8090/health"
        "Prometheus:http://localhost:9090/-/healthy"
        "Grafana:http://localhost:3001/api/health"
        "ClickHouse:http://localhost:8123/ping"
    )
    
    for endpoint in "${endpoints[@]}"; do
        local name=$(echo $endpoint | cut -d: -f1)
        local url=$(echo $endpoint | cut -d: -f2-)
        
        if curl -f -s --max-time 10 "$url" > /dev/null 2>&1; then
            echo -e "  ✅ $name: ${GREEN}Healthy${NC}"
        else
            echo -e "  ❌ $name: ${RED}Unhealthy${NC}"
        fi
    done
}

# Verificar conectividad de bases de datos
check_database_connectivity() {
    echo -e "\n${BLUE}🗄️  Conectividad de Bases de Datos:${NC}"
    
    # PostgreSQL
    if docker exec protein-docking-postgres pg_isready -U protein_user -d protein_docking > /dev/null 2>&1; then
        echo -e "  ✅ PostgreSQL: ${GREEN}Connected${NC}"
        
        # Verificar número de conexiones
        local connections=$(docker exec protein-docking-postgres psql -U protein_user -d protein_docking -t -c "SELECT count(*) FROM pg_stat_activity;" 2>/dev/null | xargs)
        echo -e "     Active connections: $connections"
    else
        echo -e "  ❌ PostgreSQL: ${RED}Connection failed${NC}"
    fi
    
    # Redis Cluster
    local redis_nodes=("6379" "6380" "6381")
    local healthy_nodes=0
    
    for port in "${redis_nodes[@]}"; do
        if docker exec protein-docking-redis-1 redis-cli -p $port ping 2>/dev/null | grep -q PONG; then
            ((healthy_nodes++))
        fi
    done
    
    if [ $healthy_nodes -eq 3 ]; then
        echo -e "  ✅ Redis Cluster: ${GREEN}All nodes healthy ($healthy_nodes/3)${NC}"
    elif [ $healthy_nodes -gt 0 ]; then
        echo -e "  ⚠️  Redis Cluster: ${YELLOW}Partial ($healthy_nodes/3 nodes)${NC}"
    else
        echo -e "  ❌ Redis Cluster: ${RED}No nodes responding${NC}"
    fi
}

# Verificar uso de recursos
check_resource_usage() {
    echo -e "\n${BLUE}📊 Uso de Recursos:${NC}"
    
    # Memoria total usada por contenedores
    local total_memory=$(docker stats --no-stream --format "table {{.MemUsage}}" | tail -n +2 | \
        awk -F'/' '{sum += $1} END {printf "%.1f", sum}' 2>/dev/null || echo "0")
    
    # CPU total usado
    local total_cpu=$(docker stats --no-stream --format "table {{.CPUPerc}}" | tail -n +2 | \
        awk '{gsub(/%/, ""); sum += $1} END {printf "%.1f", sum}' 2>/dev/null || echo "0")
    
    echo -e "  💾 Memoria total: ${total_memory}MB"
    echo -e "  🖥️  CPU total: ${total_cpu}%"
    
    # Top 3 contenedores por memoria
    echo -e "\n  Top 3 por memoria:"
    docker stats --no-stream --format "table {{.Name}}\t{{.MemUsage}}" | \
        tail -n +2 | sort -k2 -hr | head -3 | \
        while read line; do echo -e "    $line"; done
}

# Verificar logs de errores recientes
check_recent_errors() {
    echo -e "\n${BLUE}📋 Errores Recientes (última hora):${NC}"
    
    local containers=("protein-docking-backend" "protein-docking-frontend" "protein-docking-nginx")
    local found_errors=false
    
    for container in "${containers[@]}"; do
        if docker ps --format "{{.Names}}" | grep -q "$container"; then
            local errors=$(docker logs --since=1h "$container" 2>&1 | grep -i "error\|fatal\|exception" | wc -l)
            if [ "$errors" -gt 0 ]; then
                echo -e "  ⚠️  $container: ${YELLOW}$errors errores${NC}"
                found_errors=true
                
                # Mostrar últimos 3 errores
                echo -e "     Últimos errores:"
                docker logs --since=1h "$container" 2>&1 | grep -i "error\|fatal\|exception" | tail -3 | \
                    while read line; do echo -e "       ${RED}$line${NC}"; done
            fi
        fi
    done
    
    if [ "$found_errors" = false ]; then
        echo -e "  ✅ ${GREEN}No hay errores recientes${NC}"
    fi
}

# Verificar espacio en disco
check_disk_space() {
    echo -e "\n${BLUE}💽 Espacio en Disco:${NC}"
    
    # Espacio disponible en el sistema
    local available_space=$(df -h . | awk 'NR==2 {print $4}')
    echo -e "  📁 Espacio disponible: $available_space"
    
    # Espacio usado por Docker
    local docker_space=$(docker system df --format "table {{.Type}}\t{{.Size}}" 2>/dev/null | tail -n +2 | \
        awk '{print $2}' | grep -o '[0-9.]*' | awk '{sum += $1} END {printf "%.1f GB", sum}' || echo "N/A")
    echo -e "  🐳 Espacio usado por Docker: $docker_space"
    
    # Volúmenes de datos
    echo -e "\n  Volúmenes de datos:"
    local data_dirs=("postgres" "redis" "clickhouse" "uploads" "results")
    for dir in "${data_dirs[@]}"; do
        if [ -d "${PROJECT_DIR}/data/$dir" ]; then
            local dir_size=$(du -sh "${PROJECT_DIR}/data/$dir" 2>/dev/null | cut -f1)
            echo -e "    $dir: $dir_size"
        fi
    done
}

# Verificar conectividad de red entre servicios
check_network_connectivity() {
    echo -e "\n${BLUE}🌐 Conectividad de Red:${NC}"
    
    # Test desde frontend a backend
    if docker exec protein-docking-frontend nc -z backend 8080 2>/dev/null; then
        echo -e "  ✅ Frontend → Backend: ${GREEN}OK${NC}"
    else
        echo -e "  ❌ Frontend → Backend: ${RED}Failed${NC}"
    fi
    
    # Test desde backend a postgres
    if docker exec protein-docking-backend nc -z postgres 5432 2>/dev/null; then
        echo -e "  ✅ Backend → PostgreSQL: ${GREEN}OK${NC}"
    else
        echo -e "  ❌ Backend → PostgreSQL: ${RED}Failed${NC}"
    fi
    
    # Test desde backend a redis
    if docker exec protein-docking-backend nc -z redis-1 6379 2>/dev/null; then
        echo -e "  ✅ Backend → Redis: ${GREEN}OK${NC}"
    else
        echo -e "  ❌ Backend → Redis: ${RED}Failed${NC}"
    fi
}

# Generar reporte de salud general
generate_health_report() {
    echo -e "\n${BLUE}📄 Reporte de Salud General:${NC}"
    
    local running_containers=$(docker ps --format "{{.Names}}" | grep "protein-docking" | wc -l)
    local total_containers=10
    
    local health_score=$((running_containers * 100 / total_containers))
    
    if [ $health_score -eq 100 ]; then
        echo -e "  🎉 Estado general: ${GREEN}Excelente ($health_score%)${NC}"
    elif [ $health_score -ge 80 ]; then
        echo -e "  ✅ Estado general: ${GREEN}Bueno ($health_score%)${NC}"
    elif [ $health_score -ge 60 ]; then
        echo -e "  ⚠️  Estado general: ${YELLOW}Regular ($health_score%)${NC}"
    else
        echo -e "  ❌ Estado general: ${RED}Crítico ($health_score%)${NC}"
    fi
    
    echo -e "     Contenedores activos: $running_containers/$total_containers"
    
    # Uptime
    local uptime=$(docker ps --format "table {{.Names}}\t{{.Status}}" | grep "protein-docking-backend" | \
        awk '{for(i=2;i<=NF;i++) printf "%s ", $i; print ""}' | grep -o "Up.*" | sed 's/Up //' || echo "Unknown")
    echo -e "     Uptime estimado: $uptime"
}

# Script principal
main() {
    check_container_status
    check_health_endpoints
    check_database_connectivity
    check_resource_usage
    check_recent_errors
    check_disk_space
    check_network_connectivity
    generate_health_report
    
    echo -e "\n${BLUE}Health check completado.${NC}"
    echo -e "${YELLOW}Para más detalles, revisa los logs individuales con: docker-compose logs [servicio]${NC}"
}

# Verificar si Docker está disponible
if ! command -v docker &> /dev/null; then
    echo -e "${RED}Error: Docker no está instalado o no está disponible${NC}"
    exit 1
fi

# Verificar si docker-compose está disponible
if ! command -v docker-compose &> /dev/null; then
    echo -e "${RED}Error: Docker Compose no está instalado o no está disponible${NC}"
    exit 1
fi

# Ejecutar health check
main "$@"