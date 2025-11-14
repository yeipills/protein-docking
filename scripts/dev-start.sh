#!/bin/bash
# ================================
# Development Environment Startup
# ================================
# Starts all services in development mode with hot reload
# Usage: ./scripts/dev-start.sh

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}================================${NC}"
echo -e "${BLUE}Protein Docking - Dev Environment${NC}"
echo -e "${BLUE}================================${NC}\n"

# Check if .env exists
if [ ! -f .env ]; then
    echo -e "${YELLOW}WARNING: .env file not found${NC}"
    echo -e "${YELLOW}Creating .env from .env.example...${NC}"
    cp .env.example .env
    echo -e "${GREEN}✓ .env created${NC}"
    echo -e "${RED}IMPORTANT: Update .env with your configuration before continuing${NC}"
    exit 1
fi

# Check if docker-compose.dev.yml exists
if [ ! -f docker-compose.dev.yml ]; then
    echo -e "${RED}ERROR: docker-compose.dev.yml not found${NC}"
    exit 1
fi

# Stop any running containers
echo -e "${YELLOW}Stopping any running containers...${NC}"
docker compose -f docker-compose.dev.yml down 2>/dev/null || true

# Pull latest images
echo -e "\n${YELLOW}Pulling latest base images...${NC}"
docker  compose -f docker-compose.dev.yml pull

# Build services
echo -e "\n${YELLOW}Building services...${NC}"
docker compose -f docker-compose.dev.yml build

# Start services
echo -e "\n${YELLOW}Starting services...${NC}"
docker compose -f docker-compose.dev.yml up -d

# Wait for database to be ready
echo -e "\n${YELLOW}Waiting for PostgreSQL to be ready...${NC}"
sleep 5

# Run database migrations
echo -e "\n${YELLOW}Running database migrations...${NC}"
docker compose -f docker-compose.dev.yml exec -T backend alembic upgrade head || {
    echo -e "${YELLOW}No migrations to run or Alembic not initialized${NC}"
}

# Show service status
echo -e "\n${GREEN}Services started successfully!${NC}\n"
docker compose -f docker-compose.dev.yml ps

# Show useful information
echo -e "\n${BLUE}================================${NC}"
echo -e "${BLUE}Development URLs:${NC}"
echo -e "${BLUE}================================${NC}"
echo -e "${GREEN}Frontend:${NC}        http://localhost:3000"
echo -e "${GREEN}Backend API:${NC}     http://localhost:5000"
echo -e "${GREEN}API Docs:${NC}        http://localhost:5000/docs"
echo -e "${GREEN}Socket Server:${NC}   http://localhost:8080"
echo -e "${GREEN}PostgreSQL:${NC}      localhost:5432"
echo -e "${GREEN}Redis:${NC}           localhost:6379"

echo -e "\n${BLUE}================================${NC}"
echo -e "${BLUE}Useful Commands:${NC}"
echo -e "${BLUE}================================${NC}"
echo -e "${YELLOW}View logs:${NC}           docker-compose -f docker-compose.dev.yml logs -f"
echo -e "${YELLOW}Stop services:${NC}       docker-compose -f docker-compose.dev.yml down"
echo -e "${YELLOW}Restart service:${NC}     docker-compose -f docker-compose.dev.yml restart [service]"
echo -e "${YELLOW}Run tests:${NC}           ./scripts/run-tests.sh"
echo -e "${YELLOW}Backup database:${NC}     ./scripts/backup-db.sh"

echo -e "\n${GREEN}Happy coding! 🚀${NC}\n"
