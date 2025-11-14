#!/bin/bash
# ================================
# Production Deployment Script
# ================================
# Deploys the application to production with safety checks
# Usage: ./scripts/deploy-production.sh

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}================================${NC}"
echo -e "${BLUE}Production Deployment${NC}"
echo -e "${BLUE}================================${NC}\n"

# Function to ask for confirmation
confirm() {
    read -p "$(echo -e ${YELLOW}$1 [y/N]: ${NC})" -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo -e "${RED}Deployment cancelled${NC}"
        exit 1
    fi
}

# Pre-deployment checks
echo -e "${YELLOW}Running pre-deployment checks...${NC}\n"

# Check if .env exists
if [ ! -f .env ]; then
    echo -e "${RED}ERROR: .env file not found${NC}"
    exit 1
fi

# Check for production-ready settings
echo -e "${YELLOW}Checking environment configuration...${NC}"

# Check if ENVIRONMENT is set to production
if ! grep -q "ENVIRONMENT=production" .env; then
    echo -e "${RED}WARNING: ENVIRONMENT is not set to 'production' in .env${NC}"
    confirm "Continue anyway?"
fi

# Check for default passwords
if grep -q "change_this_password" .env; then
    echo -e "${RED}ERROR: Default passwords detected in .env${NC}"
    echo -e "${RED}Please update all passwords before deploying to production${NC}"
    exit 1
fi

if grep -q "change_this_secret" .env; then
    echo -e "${RED}ERROR: Default secret keys detected in .env${NC}"
    echo -e "${RED}Please update all secret keys before deploying to production${NC}"
    exit 1
fi

# Check ALLOWED_ORIGINS
if grep -q "localhost" .env | grep "ALLOWED_ORIGINS"; then
    echo -e "${YELLOW}WARNING: ALLOWED_ORIGINS contains 'localhost'${NC}"
    confirm "This may block production traffic. Continue?"
fi

echo -e "${GREEN}✓ Configuration checks passed${NC}\n"

# Backup current database
echo -e "${YELLOW}Creating database backup...${NC}"
if docker ps | grep -q "protein_docking_postgres"; then
    ./scripts/backup-db.sh
    echo -e "${GREEN}✓ Backup created${NC}\n"
else
    echo -e "${YELLOW}Database not running, skipping backup${NC}\n"
fi

# Confirm deployment
confirm "Ready to deploy to production?"

# Pull latest code
echo -e "\n${YELLOW}Pulling latest code from repository...${NC}"
git pull origin main || {
    echo -e "${RED}Failed to pull latest code${NC}"
    exit 1
}

# Build new images
echo -e "\n${YELLOW}Building production images...${NC}"
docker-compose build --no-cache

# Stop current services gracefully
echo -e "\n${YELLOW}Stopping current services...${NC}"
docker-compose down

# Start services
echo -e "\n${YELLOW}Starting production services...${NC}"
docker-compose up -d

# Wait for services to be ready
echo -e "\n${YELLOW}Waiting for services to be ready...${NC}"
sleep 10

# Run database migrations
echo -e "\n${YELLOW}Running database migrations...${NC}"
docker-compose exec -T backend alembic upgrade head || {
    echo -e "${YELLOW}No migrations to run${NC}"
}

# Health check
echo -e "\n${YELLOW}Running health checks...${NC}"
sleep 5

# Check if Nginx is responding
if curl -f http://localhost/health &>/dev/null; then
    echo -e "${GREEN}✓ Nginx health check passed${NC}"
else
    echo -e "${RED}✗ Nginx health check failed${NC}"
    echo -e "${YELLOW}Check logs: docker-compose logs nginx${NC}"
fi

# Check if backend is responding
if docker-compose exec -T backend wget --quiet --tries=1 --spider http://localhost:5000/health; then
    echo -e "${GREEN}✓ Backend health check passed${NC}"
else
    echo -e "${RED}✗ Backend health check failed${NC}"
    echo -e "${YELLOW}Check logs: docker-compose logs backend${NC}"
fi

# Show service status
echo -e "\n${YELLOW}Service status:${NC}"
docker-compose ps

# Deployment summary
echo -e "\n${BLUE}================================${NC}"
echo -e "${GREEN}Deployment completed!${NC}"
echo -e "${BLUE}================================${NC}"

echo -e "\n${YELLOW}Next steps:${NC}"
echo "1. Monitor logs: docker-compose logs -f"
echo "2. Check service health: docker-compose ps"
echo "3. Test critical endpoints"
echo "4. Monitor system resources: docker stats"

echo -e "\n${YELLOW}Rollback if needed:${NC}"
echo "1. Stop services: docker-compose down"
echo "2. Restore database: gunzip -c backups/[backup-file].sql.gz | docker exec -i protein_docking_postgres psql -U [user] [db]"
echo "3. Checkout previous commit: git checkout [commit-hash]"
echo "4. Rebuild and restart: docker-compose up -d --build"

echo -e "\n${GREEN}Deployment script finished${NC}\n"
