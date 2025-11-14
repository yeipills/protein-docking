#!/bin/bash
# ================================
# Backup Automation Setup Script
# ================================
# Sets up automated daily backups using cron
# Usage: sudo ./scripts/setup-backup-cron.sh

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Check if running as root
if [ "$EUID" -ne 0 ]; then 
    echo -e "${RED}ERROR: This script must be run as root or with sudo${NC}"
    echo "Usage: sudo ./scripts/setup-backup-cron.sh"
    exit 1
fi

# Get the actual user (not root if using sudo)
ACTUAL_USER=${SUDO_USER:-$USER}
ACTUAL_HOME=$(eval echo ~$ACTUAL_USER)

# Get project directory
PROJECT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )/.." && pwd )"

echo -e "${YELLOW}Setting up automated database backups...${NC}"
echo "Project directory: ${PROJECT_DIR}"
echo "Running as user: ${ACTUAL_USER}"

# Create systemd service
echo -e "\n${YELLOW}Creating systemd service...${NC}"
cat > /etc/systemd/system/protein-docking-backup.service << EOL
[Unit]
Description=Protein Docking Database Backup
After=docker.service
Requires=docker.service

[Service]
Type=oneshot
User=${ACTUAL_USER}
WorkingDirectory=${PROJECT_DIR}
ExecStart=/bin/bash ${PROJECT_DIR}/scripts/backup-db.sh
StandardOutput=journal
StandardError=journal
SyslogIdentifier=protein-docking-backup

[Install]
WantedBy=multi-user.target
EOL

# Create systemd timer
echo -e "${YELLOW}Creating systemd timer (daily at 2:00 AM)...${NC}"
cat > /etc/systemd/system/protein-docking-backup.timer << EOL
[Unit]
Description=Protein Docking Database Backup Timer
Requires=protein-docking-backup.service

[Timer]
# Run daily at 2:00 AM
OnCalendar=daily
OnCalendar=*-*-* 02:00:00
Persistent=true
AccuracySec=1m

[Install]
WantedBy=timers.target
EOL

# Reload systemd
echo -e "\n${YELLOW}Reloading systemd daemon...${NC}"
systemctl daemon-reload

# Enable and start timer
echo -e "${YELLOW}Enabling backup timer...${NC}"
systemctl enable protein-docking-backup.timer
systemctl start protein-docking-backup.timer

# Show status
echo -e "\n${GREEN}✓ Backup automation setup complete!${NC}"
echo -e "\n${GREEN}Timer status:${NC}"
systemctl status protein-docking-backup.timer --no-pager

echo -e "\n${GREEN}Next scheduled backups:${NC}"
systemctl list-timers protein-docking-backup.timer --no-pager

echo -e "\n${YELLOW}Useful commands:${NC}"
echo "  Check timer status:    systemctl status protein-docking-backup.timer"
echo "  Check service logs:    journalctl -u protein-docking-backup.service"
echo "  Run backup manually:   systemctl start protein-docking-backup.service"
echo "  Stop automatic backups: systemctl stop protein-docking-backup.timer"
echo "  Disable auto backups:  systemctl disable protein-docking-backup.timer"

echo -e "\n${GREEN}Backups will run daily at 2:00 AM${NC}"
echo "Backups are stored in: ${PROJECT_DIR}/backups/"
echo "Old backups (>7 days) are automatically deleted"
