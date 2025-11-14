#!/bin/bash
# ================================
# Database Backup Script
# ================================
# Creates timestamped PostgreSQL backups
# Usage: ./scripts/backup-db.sh

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Load environment variables
if [ -f .env ]; then
    export $(grep -v '^#' .env | xargs)
else
    echo -e "${RED}ERROR: .env file not found${NC}"
    exit 1
fi

# Configuration
BACKUP_DIR="./backups"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
BACKUP_FILE="${BACKUP_DIR}/protein_docking_${TIMESTAMP}.sql"
CONTAINER_NAME="protein_docking_postgres"

# Create backup directory if it doesn't exist
mkdir -p "${BACKUP_DIR}"

echo -e "${YELLOW}Starting database backup...${NC}"
echo "Backup file: ${BACKUP_FILE}"

# Check if container is running
if ! docker ps | grep -q "${CONTAINER_NAME}"; then
    echo -e "${RED}ERROR: PostgreSQL container is not running${NC}"
    echo "Start the container with: docker-compose up -d postgres"
    exit 1
fi

# Create backup using pg_dump
docker exec -t "${CONTAINER_NAME}" pg_dump -U "${POSTGRES_USER}" "${POSTGRES_DB}" > "${BACKUP_FILE}"

# Compress the backup
echo -e "${YELLOW}Compressing backup...${NC}"
gzip "${BACKUP_FILE}"

COMPRESSED_FILE="${BACKUP_FILE}.gz"
BACKUP_SIZE=$(du -h "${COMPRESSED_FILE}" | cut -f1)

echo -e "${GREEN}✓ Backup completed successfully${NC}"
echo "File: ${COMPRESSED_FILE}"
echo "Size: ${BACKUP_SIZE}"

# Clean up old backups (keep last 7 days)
echo -e "${YELLOW}Cleaning up old backups (keeping last 7 days)...${NC}"
find "${BACKUP_DIR}" -name "protein_docking_*.sql.gz" -mtime +7 -delete

# List recent backups
echo -e "\n${GREEN}Recent backups:${NC}"
ls -lh "${BACKUP_DIR}" | tail -n 5

echo -e "\n${GREEN}To restore this backup, use:${NC}"
echo "gunzip -c ${COMPRESSED_FILE} | docker exec -i ${CONTAINER_NAME} psql -U ${POSTGRES_USER} ${POSTGRES_DB}"
