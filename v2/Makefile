# Makefile para Protein Docking v2
# AGENTE 4: DevOps & Infrastructure Specialist
# Ultra-optimized development and deployment automation

.PHONY: help build test deploy clean docker lint security backup health

# Variables
PROJECT_NAME := protein-docking
VERSION := $(shell git describe --tags --always --dirty)
ENVIRONMENT := staging
GPU_ENABLED := false
REGISTRY := ghcr.io
IMAGE_TAG := $(VERSION)

# Colores para output
BLUE := \033[0;34m
GREEN := \033[0;32m
YELLOW := \033[1;33m
RED := \033[0;31m
NC := \033[0m

# Default target
help: ## 📚 Mostrar ayuda disponible
	@echo "$(BLUE)🧬 Protein Docking v2 - DevOps Automation$(NC)"
	@echo "$(BLUE)=========================================$(NC)"
	@echo ""
	@echo "$(GREEN)Comandos disponibles:$(NC)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(YELLOW)%-20s$(NC) %s\n", $$1, $$2}'
	@echo ""
	@echo "$(GREEN)Variables de entorno:$(NC)"
	@echo "  ENVIRONMENT=$(ENVIRONMENT)"
	@echo "  GPU_ENABLED=$(GPU_ENABLED)"
	@echo "  VERSION=$(VERSION)"
	@echo ""

# ============================================================================
# DESARROLLO LOCAL
# ============================================================================

setup: ## 🛠️  Setup completo del entorno de desarrollo
	@echo "$(BLUE)🛠️ Configurando entorno de desarrollo...$(NC)"
	@./scripts/docker-setup.sh
	@echo "$(GREEN)✅ Setup completado$(NC)"

install: ## 📦 Instalar dependencias
	@echo "$(BLUE)📦 Instalando dependencias...$(NC)"
	@cd backend && cargo build
	@cd frontend && npm ci
	@echo "$(GREEN)✅ Dependencias instaladas$(NC)"

dev: ## 🚀 Iniciar desarrollo local
	@echo "$(BLUE)🚀 Iniciando desarrollo local...$(NC)"
	@docker compose up -d
	@echo "$(GREEN)✅ Desarrollo local iniciado$(NC)"
	@echo "Frontend: http://localhost:3000"
	@echo "Backend: http://localhost:8080"

stop: ## ⏹️  Detener servicios locales
	@echo "$(BLUE)⏹️ Deteniendo servicios...$(NC)"
	@docker compose down
	@echo "$(GREEN)✅ Servicios detenidos$(NC)"

# ============================================================================
# TESTING Y CALIDAD
# ============================================================================

test: ## 🧪 Ejecutar todos los tests
	@echo "$(BLUE)🧪 Ejecutando tests...$(NC)"
	@$(MAKE) test-backend
	@$(MAKE) test-frontend
	@echo "$(GREEN)✅ Todos los tests completados$(NC)"

test-backend: ## 🦀 Tests del backend Rust
	@echo "$(BLUE)🦀 Ejecutando tests backend...$(NC)"
	@cd backend && cargo test --all-features --verbose
	@echo "$(GREEN)✅ Tests backend completados$(NC)"

test-frontend: ## 🎨 Tests del frontend Svelte
	@echo "$(BLUE)🎨 Ejecutando tests frontend...$(NC)"
	@cd frontend && npm run test:unit
	@echo "$(GREEN)✅ Tests frontend completados$(NC)"

test-e2e: ## 🎭 Tests end-to-end
	@echo "$(BLUE)🎭 Ejecutando tests E2E...$(NC)"
	@cd frontend && npm run test:e2e
	@echo "$(GREEN)✅ Tests E2E completados$(NC)"

test-performance: ## ⚡ Tests de performance
	@echo "$(BLUE)⚡ Ejecutando tests de performance...$(NC)"
	@docker compose up -d
	@sleep 30
	@echo "$(YELLOW)⚠️  Tests de performance configurados manualmente$(NC)"
	@echo "$(GREEN)✅ Tests de performance preparados$(NC)"

lint: ## 🎨 Linting y formateo de código
	@echo "$(BLUE)🎨 Ejecutando linting...$(NC)"
	@cd backend && cargo fmt --all -- --check
	@cd backend && cargo clippy --all-targets --all-features -- -D warnings
	@cd frontend && npm run lint
	@cd frontend && npm run check
	@echo "$(GREEN)✅ Linting completado$(NC)"

lint-fix: ## 🔧 Corregir issues de linting automáticamente
	@echo "$(BLUE)🔧 Corrigiendo linting...$(NC)"
	@cd backend && cargo fmt --all
	@cd backend && cargo clippy --all-targets --all-features --fix
	@cd frontend && npm run lint:fix
	@echo "$(GREEN)✅ Linting corregido$(NC)"

security: ## 🔒 Escaneo de seguridad
	@echo "$(BLUE)🔒 Ejecutando escaneo de seguridad...$(NC)"
	@cd backend && cargo audit
	@cd frontend && npm audit
	@trivy fs . --security-checks vuln,config,secret
	@echo "$(GREEN)✅ Escaneo de seguridad completado$(NC)"

# ============================================================================
# BUILD Y PACKAGING
# ============================================================================

build: ## 🏗️ Build de todos los componentes
	@echo "$(BLUE)🏗️ Building todos los componentes...$(NC)"
	@$(MAKE) build-backend
	@$(MAKE) build-frontend
	@echo "$(GREEN)✅ Build completado$(NC)"

build-backend: ## 🦀 Build del backend
	@echo "$(BLUE)🦀 Building backend...$(NC)"
	@cd backend && cargo build --release --all-features
	@echo "$(GREEN)✅ Backend build completado$(NC)"

build-frontend: ## 🎨 Build del frontend
	@echo "$(BLUE)🎨 Building frontend...$(NC)"
	@cd frontend && npm run build
	@echo "$(GREEN)✅ Frontend build completado$(NC)"

docker: ## 🐳 Build de imágenes Docker
	@echo "$(BLUE)🐳 Building imágenes Docker...$(NC)"
	@docker build -t $(REGISTRY)/$(PROJECT_NAME)-backend:$(IMAGE_TAG) ./backend
	@docker build -t $(REGISTRY)/$(PROJECT_NAME)-frontend:$(IMAGE_TAG) ./frontend
	@echo "$(GREEN)✅ Imágenes Docker completadas$(NC)"


docker-push: ## 📤 Push de imágenes Docker
	@echo "$(BLUE)📤 Pushing imágenes Docker...$(NC)"
	@docker push $(REGISTRY)/$(PROJECT_NAME)-backend:$(IMAGE_TAG)
	@docker push $(REGISTRY)/$(PROJECT_NAME)-frontend:$(IMAGE_TAG)
	@echo "$(GREEN)✅ Imágenes Docker pushed$(NC)"

# ============================================================================
# DEPLOYMENT
# ============================================================================

deploy: ## 🚀 Deploy a ambiente especificado
	@echo "$(BLUE)🚀 Deploying a $(ENVIRONMENT)...$(NC)"
	@./scripts/deploy-optimized.sh $(ENVIRONMENT)
	@echo "$(GREEN)✅ Deployment a $(ENVIRONMENT) completado$(NC)"

deploy-staging: ## 🧪 Deploy a staging
	@$(MAKE) deploy ENVIRONMENT=staging

deploy-production: ## 🌟 Deploy a production
	@$(MAKE) deploy ENVIRONMENT=production

deploy-dry-run: ## 🔍 Deploy dry-run (sin aplicar cambios)
	@echo "$(BLUE)🔍 Ejecutando deploy dry-run...$(NC)"
	@./scripts/deploy.sh --dry-run
	@echo "$(GREEN)✅ Deploy dry-run completado$(NC)"

rollback: ## 🔄 Rollback seguro
	@echo "$(BLUE)🔄 Ejecutando rollback a $(ENVIRONMENT)...$(NC)"
	@./scripts/rollback-safe.sh $(ENVIRONMENT)
	@echo "$(GREEN)✅ Rollback completado$(NC)"

# ============================================================================
# MONITOREO Y MANTENIMIENTO
# ============================================================================

health: ## 🏥 Health check completo del sistema
	@echo "$(BLUE)🏥 Ejecutando health check...$(NC)"
	@./scripts/docker-health-check.sh
	@echo "$(GREEN)✅ Health check completado$(NC)"

logs: ## 📋 Ver logs del ambiente actual
	@echo "$(BLUE)📋 Mostrando logs de $(ENVIRONMENT)...$(NC)"
	@kubectl logs -f deployment/protein-docking-$(ENVIRONMENT)-backend -n protein-docking-$(ENVIRONMENT)

logs-frontend: ## 🎨 Ver logs del frontend
	@kubectl logs -f deployment/protein-docking-$(ENVIRONMENT)-frontend -n protein-docking-$(ENVIRONMENT)

status: ## 📊 Status del deployment actual
	@echo "$(BLUE)📊 Status de $(ENVIRONMENT):$(NC)"
	@kubectl get pods,services,ingress -n protein-docking-$(ENVIRONMENT)
	@helm list -n protein-docking-$(ENVIRONMENT)

metrics: ## 📈 Ver métricas del sistema
	@echo "$(BLUE)📈 Abriendo métricas...$(NC)"
	@echo "Prometheus: http://localhost:9090"
	@echo "Grafana: http://localhost:3001"
	@kubectl port-forward svc/prometheus 9090:9090 -n protein-docking-$(ENVIRONMENT) &
	@kubectl port-forward svc/grafana 3001:3000 -n protein-docking-$(ENVIRONMENT) &

# ============================================================================
# BACKUP Y RESTORE
# ============================================================================

backup: ## 💾 Backup completo del sistema
	@echo "$(BLUE)💾 Ejecutando backup completo...$(NC)"
	@./scripts/docker-backup.sh
	@echo "$(GREEN)✅ Backup completado$(NC)"

backup-db: ## 🗄️ Backup de base de datos
	@echo "$(BLUE)🗄️ Ejecutando backup de DB...$(NC)"
	@kubectl exec deployment/postgres -n protein-docking-$(ENVIRONMENT) -- \
		pg_dump -U protein_user protein_docking > backup-db-$(shell date +%Y%m%d_%H%M%S).sql
	@echo "$(GREEN)✅ Backup DB completado$(NC)"

restore: ## 🔄 Restore desde backup
	@echo "$(BLUE)🔄 Ejecutando restore...$(NC)"
	@./scripts/restore.sh $(BACKUP_FILE)
	@echo "$(GREEN)✅ Restore completado$(NC)"

# ============================================================================
# LIMPIEZA Y UTILIDADES
# ============================================================================

clean: ## 🧹 Limpiar archivos temporales y caches
	@echo "$(BLUE)🧹 Limpiando archivos temporales...$(NC)"
	@cd backend && cargo clean
	@cd frontend && rm -rf node_modules/.cache dist
	@docker system prune -f
	@docker volume prune -f
	@echo "$(GREEN)✅ Limpieza completada$(NC)"

clean-docker: ## 🐳 Limpiar imágenes Docker no utilizadas
	@echo "$(BLUE)🐳 Limpiando Docker...$(NC)"
	@docker system prune -af
	@docker volume prune -f
	@echo "$(GREEN)✅ Docker limpiado$(NC)"

reset: ## 🔄 Reset completo del entorno local
	@echo "$(YELLOW)🔄 Reseteando entorno local...$(NC)"
	@$(MAKE) stop
	@$(MAKE) clean
	@docker compose down -v --remove-orphans
	@docker system prune -af --volumes
	@echo "$(GREEN)✅ Reset completado$(NC)"

update: ## 🔄 Actualizar dependencias
	@echo "$(BLUE)🔄 Actualizando dependencias...$(NC)"
	@cd backend && cargo update
	@cd frontend && npm update
	@echo "$(GREEN)✅ Dependencias actualizadas$(NC)"

# ============================================================================
# CI/CD Y AUTOMATIZACIÓN
# ============================================================================

ci: ## 🔄 Pipeline de CI completo
	@echo "$(BLUE)🔄 Ejecutando pipeline de CI...$(NC)"
	@$(MAKE) lint
	@$(MAKE) test
	@$(MAKE) security
	@$(MAKE) build
	@echo "$(GREEN)✅ Pipeline de CI completado$(NC)"

cd: ## 🚀 Pipeline de CD completo
	@echo "$(BLUE)🚀 Ejecutando pipeline de CD...$(NC)"
	@$(MAKE) docker
	@$(MAKE) docker-push
	@$(MAKE) deploy
	@$(MAKE) health
	@echo "$(GREEN)✅ Pipeline de CD completado$(NC)"

pipeline: ## 🔄 Pipeline completo CI/CD
	@echo "$(BLUE)🔄 Ejecutando pipeline completo...$(NC)"
	@$(MAKE) ci
	@$(MAKE) cd
	@echo "$(GREEN)✅ Pipeline completo ejecutado$(NC)"

# ============================================================================
# CONFIGURACIÓN DE AMBIENTES
# ============================================================================

init-staging: ## 🧪 Inicializar ambiente staging
	@echo "$(BLUE)🧪 Inicializando staging...$(NC)"
	@kubectl create namespace protein-docking-staging --dry-run=client -o yaml | kubectl apply -f -
	@helm repo add bitnami https://charts.bitnami.com/bitnami
	@helm repo update
	@echo "$(GREEN)✅ Staging inicializado$(NC)"

init-production: ## 🌟 Inicializar ambiente production
	@echo "$(BLUE)🌟 Inicializando production...$(NC)"
	@kubectl create namespace protein-docking-production --dry-run=client -o yaml | kubectl apply -f -
	@helm repo add bitnami https://charts.bitnami.com/bitnami
	@helm repo update
	@echo "$(GREEN)✅ Production inicializado$(NC)"

# ============================================================================
# INFORMACIÓN Y DEBUG
# ============================================================================

info: ## ℹ️  Información del proyecto
	@echo "$(BLUE)ℹ️ Información del Proyecto$(NC)"
	@echo "$(BLUE)========================$(NC)"
	@echo "Proyecto: $(PROJECT_NAME)"
	@echo "Versión: $(VERSION)"
	@echo "Environment: $(ENVIRONMENT)"
	@echo "GPU Enabled: $(GPU_ENABLED)"
	@echo "Registry: $(REGISTRY)"
	@echo "Image Tag: $(IMAGE_TAG)"
	@echo ""
	@echo "$(GREEN)Git Status:$(NC)"
	@git status --short
	@echo ""
	@echo "$(GREEN)Docker Images:$(NC)"
	@docker images | grep $(PROJECT_NAME) || echo "No images found"

debug: ## 🐛 Información de debug
	@echo "$(BLUE)🐛 Debug Information$(NC)"
	@echo "$(BLUE)==================$(NC)"
	@echo "Kubernetes Context: $$(kubectl config current-context)"
	@echo "Kubernetes Cluster: $$(kubectl cluster-info | head -1)"
	@echo "Docker Version: $$(docker --version)"
	@echo "Helm Version: $$(helm version --short)"
	@echo "Kubectl Version: $$(kubectl version --client --short)"
	@echo ""
	@echo "$(GREEN)Namespaces:$(NC)"
	@kubectl get namespaces | grep protein-docking || echo "No protein-docking namespaces found"

# ============================================================================
# VALIDACIONES
# ============================================================================

validate: ## ✅ Validar configuraciones
	@echo "$(BLUE)✅ Validando configuraciones...$(NC)"
	@docker compose config --quiet
	@echo "$(GREEN)✅ Validación completada$(NC)"

check-deps: ## 🔍 Verificar dependencias del sistema
	@echo "$(BLUE)🔍 Verificando dependencias...$(NC)"
	@command -v docker >/dev/null 2>&1 || { echo "$(RED)❌ Docker no encontrado$(NC)"; exit 1; }
	@command -v kubectl >/dev/null 2>&1 || { echo "$(RED)❌ kubectl no encontrado$(NC)"; exit 1; }
	@command -v helm >/dev/null 2>&1 || { echo "$(RED)❌ Helm no encontrado$(NC)"; exit 1; }
	@command -v cargo >/dev/null 2>&1 || { echo "$(RED)❌ Rust/Cargo no encontrado$(NC)"; exit 1; }
	@command -v npm >/dev/null 2>&1 || { echo "$(RED)❌ Node/NPM no encontrado$(NC)"; exit 1; }
	@echo "$(GREEN)✅ Todas las dependencias están disponibles$(NC)"