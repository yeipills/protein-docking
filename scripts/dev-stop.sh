#!/bin/bash
# ================================
# Development Environment Shutdown
# ================================
# Stops all development services gracefully
# Usage: ./scripts/dev-stop.sh [--clean]
#   --clean: Remove volumes (data will be lost!)

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Parse arguments
CLEAN_VOLUMES=false
if [ "$1" == "--clean" ]; then
    CLEAN_VOLUMES=true
fi

echo -e "${BLUE}================================${NC}"
echo -e "${BLUE}Protein Docking - Shutdown${NC}"
echo -e "${BLUE}================================${NC}\n"

# Check if docker-compose.dev.yml exists
if [ ! -f docker-compose.dev.yml ]; then
    echo -e "${RED}ERROR: docker-compose.dev.yml not found${NC}"
    exit 1
fi

# Show current running containers
echo -e "${YELLOW}Current running services:${NC}"
docker compose -f docker-compose.dev.yml ps

# Stop services gracefully
echo -e "\n${YELLOW}Stopping services gracefully...${NC}"
docker compose -f docker-compose.dev.yml stop

# Remove containers
echo -e "\n${YELLOW}Removing containers...${NC}"
if [ "$CLEAN_VOLUMES" = true ]; then
    echo -e "${RED}WARNING: Removing volumes - all data will be lost!${NC}"
    read -p "Are you sure? (yes/no): " -r
    if [[ $REPLY =~ ^[Yy][Ee][Ss]$ ]]; then
        docker compose -f docker-compose.dev.yml down -v
        echo -e "${GREEN}✓ Containers and volumes removed${NC}"
    else
        echo -e "${YELLOW}Cancelled volume removal${NC}"
        docker compose -f docker-compose.dev.yml down
        echo -e "${GREEN}✓ Containers removed (volumes preserved)${NC}"
    fi
else
    docker compose -f docker-compose.dev.yml down
    echo -e "${GREEN}✓ Containers removed (volumes preserved)${NC}"
fi

# Show final status
echo -e "\n${YELLOW}Checking remaining containers...${NC}"
RUNNING=$(docker compose -f docker-compose.dev.yml ps -q | wc -l)

if [ "$RUNNING" -eq 0 ]; then
    echo -e "${GREEN}✓ All services stopped successfully${NC}"
else
    echo -e "${YELLOW}Warning: Some containers may still be running${NC}"
    docker compose -f docker-compose.dev.yml ps
fi

# Show resource cleanup info
echo -e "\n${BLUE}================================${NC}"
echo -e "${BLUE}Cleanup Information:${NC}"
echo -e "${BLUE}================================${NC}"
echo -e "${GREEN}Preserved volumes:${NC}"
echo -e "  - postgres_dev_data (database)"
echo -e "  - uploads_dev (uploaded files)"
echo -e "  - results_dev (docking results)"
echo -e "  - logs_dev (application logs)"

echo -e "\n${BLUE}================================${NC}"
echo -e "${BLUE}Useful Commands:${NC}"
echo -e "${BLUE}================================${NC}"
echo -e "${YELLOW}Restart services:${NC}     ./scripts/dev-start.sh"
echo -e "${YELLOW}Clean volumes:${NC}        ./scripts/dev-stop.sh --clean"
echo -e "${YELLOW}View volumes:${NC}         docker volume ls | grep protein"
echo -e "${YELLOW}Prune system:${NC}         docker system prune -f"

echo -e "\n${GREEN}Environment stopped successfully! 👋${NC}\n"
