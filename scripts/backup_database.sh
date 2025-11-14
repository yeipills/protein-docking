#!/bin/bash

################################################################################
# Database Backup Script
# Automated PostgreSQL backup with rotation and compression
################################################################################

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BACKUP_DIR="${BACKUP_DIR:-$PROJECT_ROOT/backups/database}"
RETENTION_DAYS="${RETENTION_DAYS:-7}"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")

# Load environment variables
if [ -f "$PROJECT_ROOT/.env" ]; then
    set -a
    source "$PROJECT_ROOT/.env"
    set +a
fi

# Database configuration
DB_HOST="${POSTGRES_HOST:-localhost}"
DB_PORT="${POSTGRES_PORT:-5432}"
DB_NAME="${POSTGRES_DB:-protein_docking}"
DB_USER="${POSTGRES_USER:-postgres}"
DB_PASSWORD="${POSTGRES_PASSWORD:-}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Create backup directory
create_backup_dir() {
    if [ ! -d "$BACKUP_DIR" ]; then
        log_info "Creating backup directory: $BACKUP_DIR"
        mkdir -p "$BACKUP_DIR"
    fi
}

# Perform database backup
backup_database() {
    local backup_file="$BACKUP_DIR/${DB_NAME}_${TIMESTAMP}.sql"
    local compressed_file="${backup_file}.gz"

    log_info "Starting database backup..."
    log_info "Database: $DB_NAME"
    log_info "Host: $DB_HOST:$DB_PORT"
    log_info "Backup file: $compressed_file"

    # Export password for pg_dump
    export PGPASSWORD="$DB_PASSWORD"

    # Perform backup
    if pg_dump -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" \
        --format=plain \
        --no-owner \
        --no-privileges \
        --verbose \
        > "$backup_file" 2>&1; then

        log_info "Database dumped successfully"

        # Compress backup
        log_info "Compressing backup..."
        gzip -9 "$backup_file"

        # Get file size
        local file_size=$(du -h "$compressed_file" | cut -f1)
        log_info "Backup completed successfully"
        log_info "File size: $file_size"
        log_info "Location: $compressed_file"

        return 0
    else
        log_error "Database backup failed"
        rm -f "$backup_file"
        return 1
    fi

    # Clear password
    unset PGPASSWORD
}

# Rotate old backups
rotate_backups() {
    log_info "Rotating backups older than $RETENTION_DAYS days..."

    local deleted_count=0
    while IFS= read -r -d '' file; do
        log_info "Deleting old backup: $(basename "$file")"
        rm -f "$file"
        ((deleted_count++))
    done < <(find "$BACKUP_DIR" -name "*.sql.gz" -type f -mtime +$RETENTION_DAYS -print0)

    if [ $deleted_count -eq 0 ]; then
        log_info "No old backups to delete"
    else
        log_info "Deleted $deleted_count old backup(s)"
    fi
}

# Verify backup
verify_backup() {
    local compressed_file="$BACKUP_DIR/${DB_NAME}_${TIMESTAMP}.sql.gz"

    log_info "Verifying backup integrity..."

    if [ ! -f "$compressed_file" ]; then
        log_error "Backup file not found: $compressed_file"
        return 1
    fi

    if gzip -t "$compressed_file" 2>/dev/null; then
        log_info "Backup file integrity verified"
        return 0
    else
        log_error "Backup file is corrupted"
        return 1
    fi
}

# List existing backups
list_backups() {
    log_info "Existing backups:"
    echo ""

    if [ -d "$BACKUP_DIR" ] && [ "$(ls -A "$BACKUP_DIR"/*.sql.gz 2>/dev/null)" ]; then
        ls -lh "$BACKUP_DIR"/*.sql.gz | awk '{print $9, "(" $5 ")", $6, $7, $8}'
    else
        log_warn "No backups found"
    fi
}

# Main execution
main() {
    log_info "=== Database Backup Script ==="
    log_info "Started at: $(date)"
    echo ""

    # Check if pg_dump is available
    if ! command -v pg_dump &> /dev/null; then
        log_error "pg_dump not found. Please install PostgreSQL client tools."
        exit 1
    fi

    # Create backup directory
    create_backup_dir

    # Perform backup
    if backup_database; then
        # Verify backup
        if verify_backup; then
            # Rotate old backups
            rotate_backups

            echo ""
            list_backups

            log_info "Backup process completed successfully"
            exit 0
        else
            log_error "Backup verification failed"
            exit 1
        fi
    else
        log_error "Backup process failed"
        exit 1
    fi
}

# Run main function
main "$@"
