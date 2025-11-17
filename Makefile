# Makefile for Protein Docking Platform
# Convenient shortcuts for common development tasks

.PHONY: help dev build test lint format clean install setup

# Default target
help:
	@echo "🧬 Protein Docking Platform - Development Commands"
	@echo ""
	@echo "Setup & Installation:"
	@echo "  make install          Install all dependencies (backend + frontend)"
	@echo "  make setup            Complete setup (install + migrate + create dirs)"
	@echo ""
	@echo "Development:"
	@echo "  make dev              Start development environment (all services)"
	@echo "  make dev-stop         Stop development environment"
	@echo "  make dev-backend      Start only backend services"
	@echo "  make dev-frontend     Start only frontend dev server"
	@echo ""
	@echo "Building:"
	@echo "  make build            Build Docker images for production"
	@echo "  make build-dev        Build Docker images for development"
	@echo ""
	@echo "Database:"
	@echo "  make migrate          Run database migrations"
	@echo "  make migration        Create new migration (MSG='description')"
	@echo "  make db-shell         Open PostgreSQL shell"
	@echo "  make backup           Create database backup"
	@echo ""
	@echo "Testing & Quality:"
	@echo "  make test             Run all tests"
	@echo "  make test-backend     Run backend tests only"
	@echo "  make test-frontend    Run frontend tests only"
	@echo "  make lint             Run linters (backend + frontend)"
	@echo "  make lint-fix         Run linters with auto-fix"
	@echo "  make format           Format code (black + prettier)"
	@echo "  make type-check       Run type checking (mypy + tsc)"
	@echo ""
	@echo "Docker Operations:"
	@echo "  make up               Start all services (production mode)"
	@echo "  make down             Stop all services"
	@echo "  make restart          Restart all services"
	@echo "  make logs             Tail all logs"
	@echo "  make ps               Show running containers"
	@echo ""
	@echo "Cleaning:"
	@echo "  make clean            Remove build artifacts and cache"
	@echo "  make clean-all        Deep clean (including Docker volumes)"
	@echo ""
	@echo "Deployment:"
	@echo "  make deploy           Deploy to production (with checks)"
	@echo "  make health-check     Check service health"

# ================================
# Setup & Installation
# ================================

install:
	@echo "📦 Installing backend dependencies..."
	cd backend && pip install -r requirements.txt
	@echo "📦 Installing frontend dependencies..."
	cd frontend && npm install
	@echo "✅ All dependencies installed"

setup: install
	@echo "🔧 Creating directories..."
	mkdir -p backend/uploads backend/results backend/logs
	mkdir -p backups
	@echo "🗄️  Running database migrations..."
	make migrate
	@echo "✅ Setup complete!"

# ================================
# Development
# ================================

dev:
	@echo "🚀 Starting development environment..."
	./scripts/dev-start.sh

dev-stop:
	@echo "⏹️  Stopping development environment..."
	./scripts/dev-stop.sh

dev-backend:
	@echo "🐍 Starting backend services..."
	docker-compose -f docker-compose.dev.yml up postgres redis backend celery_worker socket

dev-frontend:
	@echo "⚛️  Starting frontend dev server..."
	cd frontend && npm run dev

# ================================
# Building
# ================================

build:
	@echo "🏗️  Building production images..."
	docker-compose build --no-cache

build-dev:
	@echo "🏗️  Building development images..."
	docker-compose -f docker-compose.dev.yml build

# ================================
# Database
# ================================

migrate:
	@echo "🗄️  Running database migrations..."
	docker-compose exec -T backend alembic upgrade head || \
	docker-compose -f docker-compose.dev.yml exec -T backend alembic upgrade head

migration:
	@echo "📝 Creating new migration: $(MSG)"
	docker-compose exec backend alembic revision --autogenerate -m "$(MSG)"

db-shell:
	@echo "🗄️  Opening PostgreSQL shell..."
	docker-compose exec postgres psql -U protein_user -d protein_docking

backup:
	@echo "💾 Creating database backup..."
	./scripts/backup-db.sh

# ================================
# Testing & Quality
# ================================

test:
	@echo "🧪 Running all tests..."
	./scripts/run-tests.sh

test-backend:
	@echo "🐍 Running backend tests..."
	./scripts/run-tests.sh backend

test-frontend:
	@echo "⚛️  Running frontend tests..."
	./scripts/run-tests.sh frontend

lint:
	@echo "🔍 Running linters..."
	@echo "Backend..."
	docker-compose exec backend flake8 app/ --max-line-length=100
	@echo "Frontend..."
	cd frontend && npm run lint

lint-fix:
	@echo "🔧 Running linters with auto-fix..."
	@echo "Backend..."
	docker-compose exec backend black app/
	docker-compose exec backend isort app/
	@echo "Frontend..."
	cd frontend && npm run lint:fix

format:
	@echo "✨ Formatting code..."
	@echo "Backend (black + isort)..."
	cd backend && black app/ && isort app/
	@echo "Frontend (prettier)..."
	cd frontend && npm run format
	@echo "✅ Code formatted"

type-check:
	@echo "📝 Type checking..."
	@echo "Backend (mypy)..."
	cd backend && mypy app/ --ignore-missing-imports
	@echo "Frontend (tsc)..."
	cd frontend && npm run type-check
	@echo "✅ Type check complete"

# ================================
# Docker Operations
# ================================

up:
	@echo "⬆️  Starting all services..."
	docker-compose up -d

down:
	@echo "⬇️  Stopping all services..."
	docker-compose down

restart:
	@echo "🔄 Restarting all services..."
	docker-compose restart

logs:
	@echo "📋 Tailing logs..."
	docker-compose logs -f

ps:
	@echo "📊 Container status:"
	docker-compose ps

# ================================
# Cleaning
# ================================

clean:
	@echo "🧹 Cleaning build artifacts..."
	find . -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true
	find . -type d -name ".pytest_cache" -exec rm -rf {} + 2>/dev/null || true
	find . -type d -name ".mypy_cache" -exec rm -rf {} + 2>/dev/null || true
	find . -type d -name "*.egg-info" -exec rm -rf {} + 2>/dev/null || true
	rm -rf frontend/dist frontend/node_modules/.vite
	@echo "✅ Cleaned"

clean-all: clean down
	@echo "🧹 Deep cleaning (including Docker volumes)..."
	docker-compose down -v
	docker system prune -f
	@echo "⚠️  All data removed!"

# ================================
# Deployment
# ================================

deploy:
	@echo "🚀 Deploying to production..."
	./scripts/deploy-production.sh

health-check:
	@echo "🏥 Checking service health..."
	@echo "Backend API:"
	@curl -f http://localhost/api/v1/health || echo "❌ Backend unhealthy"
	@echo "\nNginx:"
	@curl -f http://localhost/health || echo "❌ Nginx unhealthy"
	@echo "\nPostgreSQL:"
	@docker-compose exec postgres pg_isready -U protein_user || echo "❌ Database unhealthy"
	@echo "\nRedis:"
	@docker-compose exec redis redis-cli ping || echo "❌ Redis unhealthy"
