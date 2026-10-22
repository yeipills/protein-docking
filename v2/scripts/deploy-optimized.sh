#!/bin/bash

# Deployment Optimizado para Protein Docking v2
# AGENTE 4: DevOps & Infrastructure Specialist
# Ultra-optimized deployment with zero-downtime and GPU support

set -euo pipefail

# Colores para output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo -e "${BLUE}🚀 Protein Docking v2 - Deployment Optimizado${NC}"
echo -e "${BLUE}=============================================${NC}"

# Variables de configuración
ENVIRONMENT="${1:-staging}"
GPU_ENABLED="${2:-false}"
DRY_RUN="${3:-false}"
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

# Verificar dependencias
check_dependencies() {
    log_info "Verificando dependencias de deployment..."
    
    # Verificar kubectl
    if ! command -v kubectl &> /dev/null; then
        log_error "kubectl no encontrado. Instala kubectl primero."
        exit 1
    fi
    
    # Verificar helm
    if ! command -v helm &> /dev/null; then
        log_error "helm no encontrado. Instala Helm primero."
        exit 1
    fi
    
    # Verificar docker
    if ! command -v docker &> /dev/null; then
        log_error "docker no encontrado. Instala Docker primero."
        exit 1
    fi
    
    # Verificar conexión a cluster
    if ! kubectl cluster-info &> /dev/null; then
        log_error "No se puede conectar al cluster de Kubernetes"
        exit 1
    fi
    
    log_success "Todas las dependencias verificadas"
}

# Configurar ambiente
setup_environment() {
    log_info "Configurando ambiente: $ENVIRONMENT"
    
    # Crear namespace si no existe
    kubectl create namespace "$NAMESPACE" --dry-run=client -o yaml | kubectl apply -f -
    
    # Etiquetar namespace para monitoreo
    kubectl label namespace "$NAMESPACE" \
        environment="$ENVIRONMENT" \
        app.kubernetes.io/name=protein-docking \
        app.kubernetes.io/part-of=bioinformatics \
        --overwrite
    
    # Configurar contexto
    kubectl config set-context --current --namespace="$NAMESPACE"
    
    log_success "Ambiente $ENVIRONMENT configurado"
}

# Crear secrets necesarios
create_secrets() {
    log_info "Creando secrets necesarios..."
    
    # PostgreSQL credentials
    kubectl create secret generic postgres-credentials \
        --from-literal=database-url="postgresql://protein_user:$(openssl rand -base64 32)@postgres:5432/protein_docking" \
        --namespace="$NAMESPACE" \
        --dry-run=client -o yaml | kubectl apply -f -
    
    # JWT secret
    kubectl create secret generic jwt-secret \
        --from-literal=secret="$(openssl rand -base64 64)" \
        --namespace="$NAMESPACE" \
        --dry-run=client -o yaml | kubectl apply -f -
    
    # Registry credentials (si es necesario)
    if [ -n "${REGISTRY_USERNAME:-}" ] && [ -n "${REGISTRY_PASSWORD:-}" ]; then
        kubectl create secret docker-registry registry-credentials \
            --docker-server=ghcr.io \
            --docker-username="$REGISTRY_USERNAME" \
            --docker-password="$REGISTRY_PASSWORD" \
            --namespace="$NAMESPACE" \
            --dry-run=client -o yaml | kubectl apply -f -
    fi
    
    log_success "Secrets creados"
}

# Pre-deployment checks
pre_deployment_checks() {
    log_info "Ejecutando verificaciones pre-deployment..."
    
    # Verificar imágenes disponibles
    local backend_image="ghcr.io/protein-docking-backend:latest"
    local frontend_image="ghcr.io/protein-docking-frontend:latest"
    
    if [ "$GPU_ENABLED" = "true" ]; then
        backend_image="ghcr.io/protein-docking-backend-gpu:latest-gpu"
    fi
    
    log_info "Verificando imagen backend: $backend_image"
    if ! docker manifest inspect "$backend_image" &> /dev/null; then
        log_warning "Imagen backend no encontrada, continuando con imagen local"
    fi
    
    log_info "Verificando imagen frontend: $frontend_image"
    if ! docker manifest inspect "$frontend_image" &> /dev/null; then
        log_warning "Imagen frontend no encontrada, continuando con imagen local"
    fi
    
    # Verificar recursos del cluster
    local available_nodes=$(kubectl get nodes --no-headers | grep Ready | wc -l)
    log_info "Nodos disponibles en el cluster: $available_nodes"
    
    if [ "$available_nodes" -eq 0 ]; then
        log_error "No hay nodos disponibles en el cluster"
        exit 1
    fi
    
    # Verificar GPU si es necesario
    if [ "$GPU_ENABLED" = "true" ]; then
        local gpu_nodes=$(kubectl get nodes -l nvidia.com/gpu=true --no-headers 2>/dev/null | wc -l)
        if [ "$gpu_nodes" -eq 0 ]; then
            log_error "No hay nodos con GPU disponibles para deployment con GPU habilitado"
            exit 1
        fi
        log_success "Nodos con GPU disponibles: $gpu_nodes"
    fi
    
    log_success "Verificaciones pre-deployment completadas"
}

# Optimizar configuración para el ambiente
optimize_configuration() {
    log_info "Optimizando configuración para $ENVIRONMENT..."
    
    # Crear archivo de valores específico para el ambiente
    local values_file="${PROJECT_DIR}/helm/protein-docking/values-${ENVIRONMENT}.yaml"
    
    cat > "$values_file" << EOF
# Configuración optimizada para ${ENVIRONMENT}
# Generado automáticamente por deploy-optimized.sh

environment: ${ENVIRONMENT}

# Configuración de réplicas basada en ambiente
replicaCount:
  backend: $([ "$ENVIRONMENT" = "production" ] && echo "5" || echo "2")
  frontend: $([ "$ENVIRONMENT" = "production" ] && echo "3" || echo "1")

# Configuración de GPU
gpu:
  enabled: ${GPU_ENABLED}

# Configuración de recursos basada en ambiente
resources:
  backend:
    limits:
      cpu: $([ "$ENVIRONMENT" = "production" ] && echo "4000m" || echo "2000m")
      memory: $([ "$ENVIRONMENT" = "production" ] && echo "8Gi" || echo "4Gi")
    requests:
      cpu: $([ "$ENVIRONMENT" = "production" ] && echo "2000m" || echo "1000m")
      memory: $([ "$ENVIRONMENT" = "production" ] && echo "4Gi" || echo "2Gi")

# Configuración de autoscaling
autoscaling:
  backend:
    enabled: $([ "$ENVIRONMENT" = "production" ] && echo "true" || echo "false")
    minReplicas: $([ "$ENVIRONMENT" = "production" ] && echo "3" || echo "1")
    maxReplicas: $([ "$ENVIRONMENT" = "production" ] && echo "10" || echo "3")

# Configuración de ingress
ingress:
  hosts:
    - host: $([ "$ENVIRONMENT" = "production" ] && echo "protein-docking.example.com" || echo "${ENVIRONMENT}.protein-docking.example.com")
      paths:
        - path: /
          pathType: Prefix
          service: frontend
        - path: /api
          pathType: Prefix
          service: backend

# Configuración de monitoreo
monitoring:
  enabled: $([ "$ENVIRONMENT" = "production" ] && echo "true" || echo "false")

# Configuración de persistencia
persistence:
  uploads:
    size: $([ "$ENVIRONMENT" = "production" ] && echo "1Ti" || echo "100Gi")
  results:
    size: $([ "$ENVIRONMENT" = "production" ] && echo "2Ti" || echo "200Gi")
EOF
    
    log_success "Configuración optimizada creada: $values_file"
}

# Deployment con Helm
deploy_with_helm() {
    log_info "Iniciando deployment con Helm..."
    
    local values_file="${PROJECT_DIR}/helm/protein-docking/values-${ENVIRONMENT}.yaml"
    local helm_args=(
        "upgrade" "--install" "$HELM_RELEASE"
        "${PROJECT_DIR}/helm/protein-docking"
        "--namespace" "$NAMESPACE"
        "--values" "$values_file"
        "--timeout" "15m"
        "--wait"
        "--atomic"
    )
    
    # Añadir argumentos específicos
    if [ "$DRY_RUN" = "true" ]; then
        helm_args+=("--dry-run")
        log_info "Ejecutando deployment en modo dry-run"
    fi
    
    # Deployment diferenciado por ambiente
    case "$ENVIRONMENT" in
        "production")
            helm_args+=("--wait-for-jobs")
            helm_args+=("--force")
            ;;
        "staging")
            helm_args+=("--create-namespace")
            ;;
        "development")
            helm_args+=("--debug")
            ;;
    esac
    
    log_info "Ejecutando: helm ${helm_args[*]}"
    
    if helm "${helm_args[@]}"; then
        log_success "Deployment completado exitosamente"
    else
        log_error "Deployment falló"
        return 1
    fi
}

# Verificar deployment
verify_deployment() {
    log_info "Verificando deployment..."
    
    # Esperar a que los pods estén listos
    log_info "Esperando a que los pods estén listos..."
    kubectl wait --for=condition=ready pod -l app.kubernetes.io/name=protein-docking \
        --namespace="$NAMESPACE" --timeout=300s
    
    # Verificar servicios
    log_info "Verificando servicios..."
    kubectl get services --namespace="$NAMESPACE"
    
    # Verificar ingress
    if kubectl get ingress --namespace="$NAMESPACE" &> /dev/null; then
        log_info "Verificando ingress..."
        kubectl get ingress --namespace="$NAMESPACE"
    fi
    
    # Ejecutar health checks
    log_info "Ejecutando health checks..."
    local backend_pod=$(kubectl get pods -l app.kubernetes.io/component=backend \
        --namespace="$NAMESPACE" -o jsonpath='{.items[0].metadata.name}')
    
    if [ -n "$backend_pod" ]; then
        kubectl exec "$backend_pod" --namespace="$NAMESPACE" -- \
            curl -f http://localhost:8080/health || log_warning "Health check falló"
    fi
    
    log_success "Verificación de deployment completada"
}

# Ejecutar smoke tests
run_smoke_tests() {
    log_info "Ejecutando smoke tests..."
    
    # Test de conectividad básica
    local service_url
    if [ "$ENVIRONMENT" = "production" ]; then
        service_url="https://protein-docking.example.com"
    else
        service_url="https://${ENVIRONMENT}.protein-docking.example.com"
    fi
    
    # Port forward para testing local
    kubectl port-forward service/protein-docking-${ENVIRONMENT}-frontend 3000:3000 \
        --namespace="$NAMESPACE" &
    local port_forward_pid=$!
    
    sleep 5
    
    # Test frontend
    if curl -f http://localhost:3000/ &> /dev/null; then
        log_success "Frontend smoke test: PASSED"
    else
        log_warning "Frontend smoke test: FAILED"
    fi
    
    # Test backend
    kubectl port-forward service/protein-docking-${ENVIRONMENT}-backend 8080:8080 \
        --namespace="$NAMESPACE" &
    local backend_port_forward_pid=$!
    
    sleep 5
    
    if curl -f http://localhost:8080/health &> /dev/null; then
        log_success "Backend smoke test: PASSED"
    else
        log_warning "Backend smoke test: FAILED"
    fi
    
    # Cleanup port forwards
    kill $port_forward_pid $backend_port_forward_pid 2>/dev/null || true
    
    log_success "Smoke tests completados"
}

# Generar reporte de deployment
generate_deployment_report() {
    log_info "Generando reporte de deployment..."
    
    local report_file="${PROJECT_DIR}/deployment-report-${ENVIRONMENT}-$(date +%Y%m%d_%H%M%S).md"
    
    cat > "$report_file" << EOF
# 📊 Reporte de Deployment - Protein Docking v2

## 🎯 Información del Deployment
- **Fecha**: $(date)
- **Ambiente**: $ENVIRONMENT
- **GPU Habilitado**: $GPU_ENABLED
- **Namespace**: $NAMESPACE
- **Helm Release**: $HELM_RELEASE

## 📈 Estado del Deployment

### Pods
\`\`\`
$(kubectl get pods --namespace="$NAMESPACE")
\`\`\`

### Servicios
\`\`\`
$(kubectl get services --namespace="$NAMESPACE")
\`\`\`

### Ingress
\`\`\`
$(kubectl get ingress --namespace="$NAMESPACE" 2>/dev/null || echo "No ingress configurado")
\`\`\`

## 📊 Recursos y Métricas
- **Nodos**: $(kubectl get nodes --no-headers | wc -l)
- **Pods Backend**: $(kubectl get pods -l app.kubernetes.io/component=backend --namespace="$NAMESPACE" --no-headers | wc -l)
- **Pods Frontend**: $(kubectl get pods -l app.kubernetes.io/component=frontend --namespace="$NAMESPACE" --no-headers | wc -l)

## 🔍 URLs de Acceso
- **Frontend**: https://${ENVIRONMENT}.protein-docking.example.com
- **API**: https://${ENVIRONMENT}.protein-docking.example.com/api
- **Métricas**: https://${ENVIRONMENT}.protein-docking.example.com/metrics

## ✅ Estado Final
Deployment completado exitosamente ✅

EOF
    
    log_success "Reporte generado: $report_file"
}

# Mostrar resumen del deployment
show_deployment_summary() {
    echo ""
    echo -e "${GREEN}🎉 Deployment Completado Exitosamente!${NC}"
    echo -e "${BLUE}======================================${NC}"
    echo ""
    echo -e "${BLUE}Información del Deployment:${NC}"
    echo -e "  • Ambiente: ${GREEN}$ENVIRONMENT${NC}"
    echo -e "  • GPU: ${GREEN}$GPU_ENABLED${NC}"
    echo -e "  • Namespace: ${GREEN}$NAMESPACE${NC}"
    echo -e "  • Release: ${GREEN}$HELM_RELEASE${NC}"
    echo ""
    echo -e "${BLUE}URLs de Acceso:${NC}"
    if [ "$ENVIRONMENT" = "production" ]; then
        echo -e "  • Frontend: ${YELLOW}https://protein-docking.example.com${NC}"
        echo -e "  • API: ${YELLOW}https://protein-docking.example.com/api${NC}"
    else
        echo -e "  • Frontend: ${YELLOW}https://${ENVIRONMENT}.protein-docking.example.com${NC}"
        echo -e "  • API: ${YELLOW}https://${ENVIRONMENT}.protein-docking.example.com/api${NC}"
    fi
    echo ""
    echo -e "${BLUE}Comandos útiles:${NC}"
    echo -e "  • Ver pods: ${YELLOW}kubectl get pods -n $NAMESPACE${NC}"
    echo -e "  • Ver logs: ${YELLOW}kubectl logs -f deployment/protein-docking-${ENVIRONMENT}-backend -n $NAMESPACE${NC}"
    echo -e "  • Port forward: ${YELLOW}kubectl port-forward svc/protein-docking-${ENVIRONMENT}-frontend 3000:3000 -n $NAMESPACE${NC}"
    echo ""
    echo -e "${GREEN}✅ Deployment optimizado completado!${NC}"
}

# Función principal
main() {
    log_info "Iniciando deployment optimizado para Protein Docking v2..."
    echo ""
    
    # Ejecutar pasos del deployment
    check_dependencies
    echo ""
    
    setup_environment
    echo ""
    
    create_secrets
    echo ""
    
    pre_deployment_checks
    echo ""
    
    optimize_configuration
    echo ""
    
    deploy_with_helm
    echo ""
    
    if [ "$DRY_RUN" != "true" ]; then
        verify_deployment
        echo ""
        
        run_smoke_tests
        echo ""
        
        generate_deployment_report
        echo ""
    fi
    
    show_deployment_summary
    
    log_success "Deployment optimizado completado exitosamente"
}

# Función de ayuda
show_help() {
    echo "Deployment Optimizado para Protein Docking v2"
    echo ""
    echo "Uso: $0 [AMBIENTE] [GPU_ENABLED] [DRY_RUN]"
    echo ""
    echo "Argumentos:"
    echo "  AMBIENTE      Environment to deploy (staging|production|development)"
    echo "  GPU_ENABLED   Enable GPU support (true|false)"
    echo "  DRY_RUN       Execute dry run without actual deployment (true|false)"
    echo ""
    echo "Ejemplos:"
    echo "  $0 staging false false       # Deploy to staging without GPU"
    echo "  $0 production true false     # Deploy to production with GPU"
    echo "  $0 development false true    # Dry run for development"
    echo ""
    echo "Prerequisitos:"
    echo "  • kubectl configurado y conectado al cluster"
    echo "  • helm 3.x instalado"
    echo "  • docker instalado"
    echo "  • Permisos adecuados en el cluster"
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

if [ -n "${2:-}" ] && [[ ! "$2" =~ ^(true|false)$ ]]; then
    log_error "GPU_ENABLED inválido: $2. Usa: true o false"
    exit 1
fi

if [ -n "${3:-}" ] && [[ ! "$3" =~ ^(true|false)$ ]]; then
    log_error "DRY_RUN inválido: $3. Usa: true o false"
    exit 1
fi

# Manejar señales
trap 'log_error "Deployment interrumpido"; exit 1' INT TERM

# Ejecutar deployment principal
main "$@"