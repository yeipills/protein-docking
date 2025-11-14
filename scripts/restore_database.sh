#!/bin/bash

################################################################################
# Database Restore Script
# Restore PostgreSQL database from backup
################################################################################

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BACKUP_DIR="${BACKUP_DIR:-$PROJECT_ROOT/backups/database}"

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
BLUE='\033[0;34m'
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

log_prompt() {
    echo -e "${BLUE}[PROMPT]${NC} $1"
}

# List available backups
list_backups() {
    log_info "Available backups:"
    echo ""

    if [ ! -d "$BACKUP_DIR" ] || [ ! "$(ls -A "$BACKUP_DIR"/*.sql.gz 2>/dev/null)" ]; then
        log_error "No backups found in $BACKUP_DIR"
        exit 1
    fi

    local count=1
    declare -g -A backup_files

    while IFS= read -r file; do
        local filename=$(basename "$file")
        local filesize=$(du -h "$file" | cut -f1)
        local filedate=$(stat -c %y "$file" | cut -d'.' -f1)

        backup_files[$count]=$file
        echo "  [$count] $filename ($filesize) - $filedate"
        ((count++))
    done < <(find "$BACKUP_DIR" -name "*.sql.gz" -type f | sort -r)

    echo ""
}

# Select backup file
select_backup() {
    local backup_file=""

    if [ $# -eq 1 ]; then
        # Backup file provided as argument
        if [ -f "$1" ]; then
            backup_file="$1"
        elif [ -f "$BACKUP_DIR/$1" ]; then
            backup_file="$BACKUP_DIR/$1"
        else
            log_error "Backup file not found: $1"
            exit 1
        fi
    else
        # Interactive selection
        list_backups

        log_prompt "Select backup number to restore (or 'q' to quit): "
        read -r selection

        if [ "$selection" = "q" ]; then
            log_info "Restore cancelled"
            exit 0
        fi

        if [ -z "${backup_files[$selection]:-}" ]; then
            log_error "Invalid selection"
            exit 1
        fi

        backup_file="${backup_files[$selection]}"
    fi

    echo "$backup_file"
}

# Verify backup file
verify_backup() {
    local backup_file="$1"

    log_info "Verifying backup file: $(basename "$backup_file")"

    if ! gzip -t "$backup_file" 2>/dev/null; then
        log_error "Backup file is corrupted"
        exit 1
    fi

    log_info "Backup file integrity verified"
}

# Confirm restore
confirm_restore() {
    local backup_file="$1"

    echo ""
    log_warn "WARNING: This will DROP and recreate the database '$DB_NAME'"
    log_warn "All existing data will be LOST!"
    echo ""
    log_prompt "Are you sure you want to restore from: $(basename "$backup_file")? (yes/no): "
    read -r confirmation

    if [ "$confirmation" != "yes" ]; then
        log_info "Restore cancelled"
        exit 0
    fi
}

# Create database backup before restore
backup_current_db() {
    log_info "Creating safety backup of current database..."

    local safety_backup="$BACKUP_DIR/${DB_NAME}_pre_restore_$(date +%Y%m%d_%H%M%S).sql.gz"

    export PGPASSWORD="$DB_PASSWORD"

    if pg_dump -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" \
        --format=plain --no-owner --no-privileges 2>/dev/null | gzip -9 > "$safety_backup"; then
        log_info "Safety backup created: $(basename "$safety_backup")"
    else
        log_warn "Could not create safety backup (database might not exist)"
    fi

    unset PGPASSWORD
}

# Restore database
restore_database() {
    local backup_file="$1"

    log_info "Starting database restore..."
    log_info "Database: $DB_NAME"
    log_info "Host: $DB_HOST:$DB_PORT"
    log_info "Backup: $(basename "$backup_file")"

    export PGPASSWORD="$DB_PASSWORD"

    # Drop existing database
    log_info "Dropping existing database..."
    dropdb -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" --if-exists "$DB_NAME" 2>/dev/null || true

    # Create new database
    log_info "Creating new database..."
    createdb -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" "$DB_NAME"

    # Restore from backup
    log_info "Restoring data..."
    if gunzip -c "$backup_file" | psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" \
        --quiet 2>&1 | grep -v "^$"; then

        log_info "Database restored successfully"
        unset PGPASSWORD
        return 0
    else
        log_error "Database restore failed"
        unset PGPASSWORD
        return 1
    fi
}

# Verify restore
verify_restore() {
    log_info "Verifying database restore..."

    export PGPASSWORD="$DB_PASSWORD"

    # Check if database exists and has tables
    local table_count=$(psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" \
        -t -c "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = 'public';" 2>/dev/null | tr -d ' ')

    unset PGPASSWORD

    if [ "$table_count" -gt 0 ]; then
        log_info "Restore verified: $table_count tables found"
        return 0
    else
        log_error "Restore verification failed: no tables found"
        return 1
    fi
}

# Main execution
main() {
    log_info "=== Database Restore Script ==="
    log_info "Started at: $(date)"
    echo ""

    # Check if psql is available
    if ! command -v psql &> /dev/null; then
        log_error "psql not found. Please install PostgreSQL client tools."
        exit 1
    fi

    # Select backup file
    local backup_file=$(select_backup "$@")

    # Verify backup
    verify_backup "$backup_file"

    # Confirm restore
    confirm_restore "$backup_file"

    # Create safety backup
    backup_current_db

    # Perform restore
    echo ""
    if restore_database "$backup_file"; then
        # Verify restore
        if verify_restore; then
            log_info "Restore process completed successfully"
            exit 0
        else
            log_error "Restore verification failed"
            exit 1
        fi
    else
        log_error "Restore process failed"
        exit 1
    fi
}

# Run main function
main "$@"
