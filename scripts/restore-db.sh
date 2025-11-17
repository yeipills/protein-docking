#!/bin/bash
# ================================
# Database Restore Script
# ================================
# Restores PostgreSQL database from backup
# Usage: ./scripts/restore-db.sh <backup_file>

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Check if backup file is provided
if [ -z "$1" ]; then
    echo -e "${RED}ERROR: No backup file specified${NC}"
    echo "Usage: $0 <backup_file>"
    echo "Example: $0 ./backups/protein_docking_20251114_120000.sql.gz"
    echo -e "\n${YELLOW}Available backups:${NC}"
    ls -lh ./backups/*.gz 2>/dev/null || echo "No backups found"
    exit 1
fi

BACKUP_FILE="$1"

# Check if backup file exists
if [ ! -f "${BACKUP_FILE}" ]; then
    echo -e "${RED}ERROR: Backup file not found: ${BACKUP_FILE}${NC}"
    exit 1
fi

# Load environment variables
if [ -f .env ]; then
    export $(grep -v '^#' .env | xargs)
else
    echo -e "${RED}ERROR: .env file not found${NC}"
    exit 1
fi

CONTAINER_NAME="protein_docking_postgres"

echo -e "${YELLOW}Database Restore Utility${NC}"
echo "Backup file: ${BACKUP_FILE}"
echo "Container: ${CONTAINER_NAME}"
echo "Database: ${POSTGRES_DB}"

# Check if container is running
if ! docker ps | grep -q "${CONTAINER_NAME}"; then
    echo -e "${RED}ERROR: PostgreSQL container is not running${NC}"
    echo "Start the container with: docker-compose up -d postgres"
    exit 1
fi

# Warning
echo -e "\n${RED}⚠️  WARNING: This will REPLACE the current database!${NC}"
echo -e "${YELLOW}All current data will be LOST and replaced with the backup.${NC}"
read -p "Are you sure you want to continue? (yes/no): " CONFIRM

if [ "$CONFIRM" != "yes" ]; then
    echo -e "${YELLOW}Restore cancelled${NC}"
    exit 0
fi

# Create a safety backup of current database first
SAFETY_BACKUP="./backups/pre_restore_$(date +%Y%m%d_%H%M%S).sql.gz"
echo -e "\n${YELLOW}Creating safety backup of current database...${NC}"
docker exec -t "${CONTAINER_NAME}" pg_dump -U "${POSTGRES_USER}" "${POSTGRES_DB}" | gzip > "${SAFETY_BACKUP}"
echo -e "${GREEN}✓ Safety backup created: ${SAFETY_BACKUP}${NC}"

# Restore database
echo -e "\n${YELLOW}Restoring database from backup...${NC}"

# Drop existing connections
echo "Terminating existing connections..."
docker exec -i "${CONTAINER_NAME}" psql -U "${POSTGRES_USER}" -d postgres -c \
    "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '${POSTGRES_DB}' AND pid <> pg_backend_pid();" || true

# Drop and recreate database
echo "Recreating database..."
docker exec -i "${CONTAINER_NAME}" psql -U "${POSTGRES_USER}" -d postgres -c "DROP DATABASE IF EXISTS ${POSTGRES_DB};"
docker exec -i "${CONTAINER_NAME}" psql -U "${POSTGRES_USER}" -d postgres -c "CREATE DATABASE ${POSTGRES_DB};"

# Restore from backup
echo "Restoring data..."
if [[ "${BACKUP_FILE}" == *.gz ]]; then
    gunzip -c "${BACKUP_FILE}" | docker exec -i "${CONTAINER_NAME}" psql -U "${POSTGRES_USER}" "${POSTGRES_DB}"
else
    cat "${BACKUP_FILE}" | docker exec -i "${CONTAINER_NAME}" psql -U "${POSTGRES_USER}" "${POSTGRES_DB}"
fi

echo -e "\n${GREEN}✓ Database restored successfully!${NC}"
echo -e "${YELLOW}Safety backup available at: ${SAFETY_BACKUP}${NC}"
echo -e "\n${GREEN}You may want to restart your application:${NC}"
echo "  docker-compose restart backend"
