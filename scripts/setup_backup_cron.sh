#!/bin/bash

################################################################################
# Setup Automated Database Backups
# Configures cron job for daily database backups
################################################################################

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKUP_SCRIPT="$SCRIPT_DIR/backup_database.sh"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

# Check if backup script exists
if [ ! -f "$BACKUP_SCRIPT" ]; then
    echo "Error: Backup script not found: $BACKUP_SCRIPT"
    exit 1
fi

# Make backup script executable
chmod +x "$BACKUP_SCRIPT"

# Cron job configuration
CRON_TIME="${CRON_TIME:-0 2 * * *}"  # Default: 2 AM daily
CRON_JOB="$CRON_TIME $BACKUP_SCRIPT >> /var/log/protein-docking-backup.log 2>&1"

log_info "=== Backup Automation Setup ==="
echo ""
log_info "This script will configure automated daily backups"
log_info "Schedule: $CRON_TIME (Daily at 2 AM)"
echo ""

# Check if cron job already exists
if crontab -l 2>/dev/null | grep -q "$BACKUP_SCRIPT"; then
    log_warn "Cron job already exists!"
    echo ""
    echo "Current backup cron jobs:"
    crontab -l | grep "$BACKUP_SCRIPT"
    echo ""
    read -p "Replace existing cron job? (yes/no): " replace

    if [ "$replace" != "yes" ]; then
        log_info "Setup cancelled"
        exit 0
    fi

    # Remove existing cron job
    crontab -l | grep -v "$BACKUP_SCRIPT" | crontab -
    log_info "Removed existing cron job"
fi

# Add new cron job
(crontab -l 2>/dev/null; echo "$CRON_JOB") | crontab -

log_info "Cron job added successfully!"
echo ""
log_info "Backup schedule: $CRON_TIME"
log_info "Log file: /var/log/protein-docking-backup.log"
echo ""
log_info "Current crontab:"
crontab -l | grep "$BACKUP_SCRIPT"

echo ""
log_info "Setup complete!"
log_info ""
log_info "Manual backup command:"
log_info "  $BACKUP_SCRIPT"
echo ""
log_info "To modify the schedule, edit the cron job:"
log_info "  crontab -e"
echo ""
log_info "To view backup logs:"
log_info "  tail -f /var/log/protein-docking-backup.log"
