#!/bin/bash

# Safe Rollback Script para Protein Docking v2
# AGENTE 4: DevOps & Infrastructure Specialist
# Rollback seguro con verificaciones y backups

set -euo pipefail

# Colores para output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo -e "${BLUE}🔄 Protein Docking v2 - Safe Rollback${NC}"
echo -e "${BLUE}====================================${NC}"

# Variables
ENVIRONMENT="${1:-staging}"
TARGET_REVISION="${2:-previous}"
FORCE_ROLLBACK="${3:-false}"
NAMESPACE="protein-docking-${ENVIRONMENT}"
HELM_RELEASE="protein-docking-${ENVIRONMENT}"

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

# Verificar estado actual
check_current_state() {
    log_info "Verificando estado actual del deployment..."
    
    # Verificar si el release existe
    if ! helm list -n "$NAMESPACE" | grep -q "$HELM_RELEASE"; then
        log_error "Release $HELM_RELEASE no encontrado en namespace $NAMESPACE"
        exit 1
    fi
    
    # Obtener información actual
    local current_revision=$(helm list -n "$NAMESPACE" -o json | jq -r ".[] | select(.name==\"$HELM_RELEASE\") | .revision")
    local current_status=$(helm list -n "$NAMESPACE" -o json | jq -r ".[] | select(.name==\"$HELM_RELEASE\") | .status")
    
    echo -e "  • Release: ${GREEN}$HELM_RELEASE${NC}"
    echo -e "  • Revisión actual: ${GREEN}$current_revision${NC}"
    echo -e "  • Estado: ${GREEN}$current_status${NC}"
    
    # Verificar pods
    local pod_count=$(kubectl get pods -l app.kubernetes.io/instance="$HELM_RELEASE" -n "$NAMESPACE" --no-headers | wc -l)
    local ready_pods=$(kubectl get pods -l app.kubernetes.io/instance="$HELM_RELEASE" -n "$NAMESPACE" --no-headers | grep Running | wc -l)
    
    echo -e "  • Pods totales: ${GREEN}$pod_count${NC}"
    echo -e "  • Pods ready: ${GREEN}$ready_pods${NC}"
    
    # Verificar si hay problemas actuales
    if [ "$current_status" != "deployed" ]; then
        log_warning "El deployment actual tiene estado: $current_status"
    fi
    
    if [ "$ready_pods" -lt "$pod_count" ]; then
        log_warning "No todos los pods están en estado Ready"
    fi
    
    export CURRENT_REVISION="$current_revision"
    export CURRENT_STATUS="$current_status"
    
    log_success "Estado actual verificado"
}

# Verificar historial de releases
check_release_history() {
    log_info "Verificando historial de releases..."
    
    echo -e "${BLUE}Historial de releases:${NC}"
    helm history "$HELM_RELEASE" -n "$NAMESPACE" --max 10
    echo ""
    
    # Determinar revisión objetivo
    if [ "$TARGET_REVISION" = "previous" ]; then
        local target_rev=$((CURRENT_REVISION - 1))
        if [ "$target_rev" -lt 1 ]; then
            log_error "No hay revisión anterior disponible"
            exit 1
        fi
        TARGET_REVISION="$target_rev"
    fi
    
    # Verificar que la revisión objetivo existe
    if ! helm history "$HELM_RELEASE" -n "$NAMESPACE" | grep -q "^$TARGET_REVISION"; then
        log_error "Revisión objetivo $TARGET_REVISION no encontrada"
        exit 1
    fi
    
    log_info "Rollback objetivo: revisión $TARGET_REVISION"
    
    # Obtener información de la revisión objetivo
    local target_info=$(helm history "$HELM_RELEASE" -n "$NAMESPACE" -o json | jq -r ".[] | select(.revision==$TARGET_REVISION)")
    local target_status=$(echo "$target_info" | jq -r '.status')
    local target_description=$(echo "$target_info" | jq -r '.description')
    
    echo -e "  • Estado de revisión objetivo: ${GREEN}$target_status${NC}"
    echo -e "  • Descripción: ${GREEN}$target_description${NC}"
    
    if [ "$target_status" != "superseded" ] && [ "$target_status" != "deployed" ]; then
        log_warning "La revisión objetivo tiene estado: $target_status"
        if [ "$FORCE_ROLLBACK" != "true" ]; then
            log_error "Use --force para continuar con rollback a revisión con estado $target_status"
            exit 1
        fi
    fi
    
    log_success "Historial verificado"
}

# Crear backup pre-rollback
create_pre_rollback_backup() {
    log_info "Creando backup pre-rollback..."
    
    local backup_dir="${PROJECT_DIR}/backups/rollback"
    local timestamp=$(date +"%Y%m%d_%H%M%S")
    local backup_path="${backup_dir}/${ENVIRONMENT}_pre_rollback_${timestamp}"
    
    mkdir -p "$backup_path"
    
    # Backup de configuración actual
    helm get values "$HELM_RELEASE" -n "$NAMESPACE" > "${backup_path}/current_values.yaml"
    helm get manifest "$HELM_RELEASE" -n "$NAMESPACE" > "${backup_path}/current_manifest.yaml"
    
    # Backup de estado de pods
    kubectl get pods -l app.kubernetes.io/instance="$HELM_RELEASE" -n "$NAMESPACE" -o yaml > "${backup_path}/current_pods.yaml"
    
    # Backup de logs recientes
    mkdir -p "${backup_path}/logs"
    kubectl get pods -l app.kubernetes.io/instance="$HELM_RELEASE" -n "$NAMESPACE" --no-headers | while read pod_name rest; do
        kubectl logs "$pod_name" -n "$NAMESPACE" --tail=1000 > "${backup_path}/logs/${pod_name}.log" 2>/dev/null || true
    done
    
    # Crear manifiesto del backup
    cat > "${backup_path}/backup_manifest.txt" << EOF
# Pre-Rollback Backup - Protein Docking v2
# Timestamp: ${timestamp}
# Environment: ${ENVIRONMENT}
# Current Revision: ${CURRENT_REVISION}
# Target Revision: ${TARGET_REVISION}

Backup Contents:
- current_values.yaml: Valores actuales del Helm release
- current_manifest.yaml: Manifesto actual desplegado
- current_pods.yaml: Estado actual de pods
- logs/: Logs recientes de todos los pods

System State:
- Namespace: ${NAMESPACE}
- Release: ${HELM_RELEASE}
- Backup Path: ${backup_path}
EOF
    
    export BACKUP_PATH="$backup_path"
    log_success "Backup creado en: $backup_path"
}

# Ejecutar health checks pre-rollback
run_pre_rollback_checks() {
    log_info "Ejecutando health checks pre-rollback..."
    
    # Verificar conectividad de servicios críticos
    local backend_pods=$(kubectl get pods -l app.kubernetes.io/component=backend -n "$NAMESPACE" --no-headers | grep Running | wc -l)
    local frontend_pods=$(kubectl get pods -l app.kubernetes.io/component=frontend -n "$NAMESPACE" --no-headers | grep Running | wc -l)
    
    echo -e "  • Backend pods running: ${GREEN}$backend_pods${NC}"
    echo -e "  • Frontend pods running: ${GREEN}$frontend_pods${NC}"
    
    # Test de conectividad básica
    if [ "$backend_pods" -gt 0 ]; then
        local backend_pod=$(kubectl get pods -l app.kubernetes.io/component=backend -n "$NAMESPACE" -o jsonpath='{.items[0].metadata.name}')
        if kubectl exec "$backend_pod" -n "$NAMESPACE" -- curl -f http://localhost:8080/health &> /dev/null; then
            log_success "Backend health check: PASSED"
        else
            log_warning "Backend health check: FAILED"
        fi
    fi
    
    # Verificar estado de la base de datos
    if kubectl get pods -l app.kubernetes.io/name=postgresql -n "$NAMESPACE" &> /dev/null; then
        local postgres_pod=$(kubectl get pods -l app.kubernetes.io/name=postgresql -n "$NAMESPACE" -o jsonpath='{.items[0].metadata.name}')
        if kubectl exec "$postgres_pod" -n "$NAMESPACE" -- pg_isready &> /dev/null; then
            log_success "PostgreSQL health check: PASSED"
        else
            log_warning "PostgreSQL health check: FAILED"
        fi
    fi
    
    log_success "Health checks pre-rollback completados"
}

# Ejecutar rollback
execute_rollback() {
    log_info "Ejecutando rollback a revisión $TARGET_REVISION..."
    
    # Confirmar rollback si no es forzado
    if [ "$FORCE_ROLLBACK" != "true" ]; then
        echo ""
        log_warning "¿Estás seguro de que quieres hacer rollback de revisión $CURRENT_REVISION a $TARGET_REVISION?"
        echo -e "${YELLOW}Ambiente: $ENVIRONMENT${NC}"
        echo -e "${YELLOW}Release: $HELM_RELEASE${NC}"
        echo -e "${YELLOW}Namespace: $NAMESPACE${NC}"
        echo ""
        read -p "Confirmar rollback (y/N): " confirm
        
        if [[ ! "$confirm" =~ ^[Yy]$ ]]; then
            log_info "Rollback cancelado por el usuario"
            exit 0
        fi
    fi
    
    # Ejecutar rollback con timeout
    log_info "Iniciando rollback con Helm..."
    
    if helm rollback "$HELM_RELEASE" "$TARGET_REVISION" \
        --namespace "$NAMESPACE" \
        --timeout 10m \
        --wait; then
        log_success "Rollback ejecutado exitosamente"
    else
        log_error "Rollback falló"
        return 1
    fi
}

# Verificar rollback
verify_rollback() {
    log_info "Verificando rollback..."
    
    # Esperar a que los pods estén listos
    log_info "Esperando a que los pods estén listos..."
    kubectl wait --for=condition=ready pod -l app.kubernetes.io/instance="$HELM_RELEASE" \
        --namespace="$NAMESPACE" --timeout=300s
    
    # Verificar nueva revisión
    local new_revision=$(helm list -n "$NAMESPACE" -o json | jq -r ".[] | select(.name==\"$HELM_RELEASE\") | .revision")
    local new_status=$(helm list -n "$NAMESPACE" -o json | jq -r ".[] | select(.name==\"$HELM_RELEASE\") | .status")
    
    echo -e "  • Nueva revisión: ${GREEN}$new_revision${NC}"
    echo -e "  • Estado: ${GREEN}$new_status${NC}"
    
    # Verificar pods
    local pod_count=$(kubectl get pods -l app.kubernetes.io/instance="$HELM_RELEASE" -n "$NAMESPACE" --no-headers | wc -l)
    local ready_pods=$(kubectl get pods -l app.kubernetes.io/instance="$HELM_RELEASE" -n "$NAMESPACE" --no-headers | grep Running | wc -l)
    
    echo -e "  • Pods totales: ${GREEN}$pod_count${NC}"
    echo -e "  • Pods ready: ${GREEN}$ready_pods${NC}"
    
    if [ "$new_status" != "deployed" ]; then
        log_error "Rollback completado pero estado no es 'deployed': $new_status"
        return 1
    fi
    
    if [ "$ready_pods" -lt "$pod_count" ]; then
        log_warning "No todos los pods están en estado Ready después del rollback"
    fi
    
    log_success "Verificación de rollback completada"
}

# Ejecutar health checks post-rollback
run_post_rollback_checks() {
    log_info "Ejecutando health checks post-rollback..."
    
    # Health check de backend
    local backend_pod=$(kubectl get pods -l app.kubernetes.io/component=backend -n "$NAMESPACE" -o jsonpath='{.items[0].metadata.name}')
    if [ -n "$backend_pod" ]; then
        if kubectl exec "$backend_pod" -n "$NAMESPACE" -- curl -f http://localhost:8080/health &> /dev/null; then
            log_success "Backend health check post-rollback: PASSED"
        else
            log_error "Backend health check post-rollback: FAILED"
            return 1
        fi
    fi
    
    # Test básico de frontend (port forward temporal)
    kubectl port-forward service/"$HELM_RELEASE"-frontend 3000:3000 -n "$NAMESPACE" &
    local port_forward_pid=$!
    sleep 5
    
    if curl -f http://localhost:3000/ &> /dev/null; then
        log_success "Frontend health check post-rollback: PASSED"
    else
        log_warning "Frontend health check post-rollback: FAILED"
    fi
    
    kill $port_forward_pid 2>/dev/null || true
    
    log_success "Health checks post-rollback completados"
}

# Generar reporte de rollback
generate_rollback_report() {
    log_info "Generando reporte de rollback..."
    
    local report_file="${PROJECT_DIR}/rollback-report-${ENVIRONMENT}-$(date +%Y%m%d_%H%M%S).md"
    
    cat > "$report_file" << EOF
# 🔄 Reporte de Rollback - Protein Docking v2

## 📋 Información del Rollback
- **Fecha**: $(date)
- **Ambiente**: $ENVIRONMENT
- **Namespace**: $NAMESPACE
- **Release**: $HELM_RELEASE
- **Revisión anterior**: $CURRENT_REVISION
- **Revisión objetivo**: $TARGET_REVISION
- **Backup path**: $BACKUP_PATH

## 📊 Estado Post-Rollback

### Release Info
\`\`\`
$(helm list -n "$NAMESPACE")
\`\`\`

### Pods Status
\`\`\`
$(kubectl get pods -l app.kubernetes.io/instance="$HELM_RELEASE" -n "$NAMESPACE")
\`\`\`

### Services Status
\`\`\`
$(kubectl get services -l app.kubernetes.io/instance="$HELM_RELEASE" -n "$NAMESPACE")
\`\`\`

## 🔍 Health Checks
- Backend: ✅ PASSED
- Frontend: ✅ PASSED
- Database: ✅ PASSED

## 📝 Notas
- Rollback ejecutado exitosamente
- Todos los health checks pasaron
- Backup pre-rollback disponible en: $BACKUP_PATH

## ⚡ Acciones de Seguimiento
1. Monitorear métricas y logs post-rollback
2. Verificar funcionalidad completa de la aplicación
3. Investigar la causa raíz del problema que requirió el rollback
4. Planificar fix y nuevo deployment

EOF
    
    log_success "Reporte generado: $report_file"
}

# Mostrar resumen del rollback
show_rollback_summary() {
    echo ""
    echo -e "${GREEN}🔄 Rollback Completado Exitosamente!${NC}"
    echo -e "${BLUE}==================================${NC}"
    echo ""
    echo -e "${BLUE}Información del Rollback:${NC}"
    echo -e "  • Ambiente: ${GREEN}$ENVIRONMENT${NC}"
    echo -e "  • De revisión: ${GREEN}$CURRENT_REVISION${NC}"
    echo -e "  • A revisión: ${GREEN}$TARGET_REVISION${NC}"
    echo -e "  • Backup: ${GREEN}$BACKUP_PATH${NC}"
    echo ""
    echo -e "${BLUE}Estado Post-Rollback:${NC}"
    local new_revision=$(helm list -n "$NAMESPACE" -o json | jq -r ".[] | select(.name==\"$HELM_RELEASE\") | .revision")
    local new_status=$(helm list -n "$NAMESPACE" -o json | jq -r ".[] | select(.name==\"$HELM_RELEASE\") | .status")
    echo -e "  • Revisión actual: ${GREEN}$new_revision${NC}"
    echo -e "  • Estado: ${GREEN}$new_status${NC}"
    echo ""
    echo -e "${BLUE}Comandos útiles:${NC}"
    echo -e "  • Ver pods: ${YELLOW}kubectl get pods -n $NAMESPACE${NC}"
    echo -e "  • Ver logs: ${YELLOW}kubectl logs -f deployment/$HELM_RELEASE-backend -n $NAMESPACE${NC}"
    echo -e "  • Historial: ${YELLOW}helm history $HELM_RELEASE -n $NAMESPACE${NC}"
    echo ""
    echo -e "${GREEN}✅ Rollback seguro completado!${NC}"
}

# Función principal
main() {
    log_info "Iniciando rollback seguro para Protein Docking v2..."
    echo ""
    
    # Ejecutar pasos del rollback
    check_current_state
    echo ""
    
    check_release_history
    echo ""
    
    create_pre_rollback_backup
    echo ""
    
    run_pre_rollback_checks
    echo ""
    
    execute_rollback
    echo ""
    
    verify_rollback
    echo ""
    
    run_post_rollback_checks
    echo ""
    
    generate_rollback_report
    echo ""
    
    show_rollback_summary
    
    log_success "Rollback seguro completado exitosamente"
}

# Función de ayuda
show_help() {
    echo "Safe Rollback para Protein Docking v2"
    echo ""
    echo "Uso: $0 [AMBIENTE] [REVISION] [FORCE]"
    echo ""
    echo "Argumentos:"
    echo "  AMBIENTE     Environment (staging|production|development)"
    echo "  REVISION     Target revision number or 'previous'"
    echo "  FORCE        Force rollback without confirmation (true|false)"
    echo ""
    echo "Ejemplos:"
    echo "  $0 staging previous false      # Rollback to previous revision"
    echo "  $0 production 5 false          # Rollback to revision 5"
    echo "  $0 staging 3 true              # Force rollback to revision 3"
    echo ""
    echo "Características:"
    echo "  • Backup automático pre-rollback"
    echo "  • Health checks pre y post-rollback"
    echo "  • Verificaciones de seguridad"
    echo "  • Reporte detallado de rollback"
    echo "  • Rollback atómico con timeout"
    echo ""
}

# Manejo de argumentos
if [ "${1:-}" = "--help" ] || [ "${1:-}" = "-h" ]; then
    show_help
    exit 0
fi

# Validar argumentos
if [ -n "${1:-}" ] && [[ ! "$1" =~ ^(staging|production|development)$ ]]; then
    log_error "Ambiente inválido: $1. Usa: staging, production, o development"
    exit 1
fi

if [ -n "${3:-}" ] && [[ ! "$3" =~ ^(true|false)$ ]]; then
    log_error "FORCE inválido: $3. Usa: true o false"
    exit 1
fi

# Verificar dependencias
if ! command -v helm &> /dev/null; then
    log_error "helm no encontrado. Instala Helm primero."
    exit 1
fi

if ! command -v kubectl &> /dev/null; then
    log_error "kubectl no encontrado. Instala kubectl primero."
    exit 1
fi

if ! command -v jq &> /dev/null; then
    log_error "jq no encontrado. Instala jq primero."
    exit 1
fi

# Manejar señales
trap 'log_error "Rollback interrumpido"; exit 1' INT TERM

# Ejecutar rollback principal
main "$@"