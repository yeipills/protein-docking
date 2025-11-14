# Database Backup and Restore Documentation

## Table of Contents
- [Overview](#overview)
- [Backup Scripts](#backup-scripts)
- [Automated Backups](#automated-backups)
- [Manual Backups](#manual-backups)
- [Restore Procedures](#restore-procedures)
- [Monitoring](#monitoring)
- [Troubleshooting](#troubleshooting)
- [Security Considerations](#security-considerations)

## Overview

The Protein Docking application includes a comprehensive backup and restore system for the PostgreSQL database. The system supports both manual and automated backups with configurable retention policies.

### Features
- **Automated daily backups** via systemd timers
- **Manual on-demand backups**
- **Safe restore with automatic safety backups**
- **7-day retention policy** (automatically deletes old backups)
- **Compressed backups** (gzip) to save storage space
- **Docker-based** execution (no direct database access needed)

### Backup Location
All backups are stored in: `./backups/`

### Naming Convention
Backups follow the pattern: `protein_docking_YYYYMMDD_HHMMSS.sql.gz`

Example: `protein_docking_20251114_143000.sql.gz`

---

## Backup Scripts

### 1. backup-db.sh
**Location:** `scripts/backup-db.sh`

**Purpose:** Creates a backup of the PostgreSQL database

**Features:**
- Reads configuration from `.env` file
- Creates timestamped compressed backup
- Automatically deletes backups older than 7 days
- Validates Docker container is running
- Color-coded output for easy monitoring

**Usage:**
```bash
./scripts/backup-db.sh
```

**Requirements:**
- Docker container `protein_docking_postgres` must be running
- `.env` file must exist with database credentials
- `backups/` directory will be created automatically

**Output Example:**
```
Database Backup Utility
Container: protein_docking_postgres
Database: protein_docking
Creating backup...
✓ Backup created: ./backups/protein_docking_20251114_143000.sql.gz
Size: 2.5M
Cleaning up old backups (older than 7 days)...
Deleted: protein_docking_20251107_020000.sql.gz
✓ Backup completed successfully!
```

---

### 2. setup-backup-cron.sh
**Location:** `scripts/setup-backup-cron.sh`

**Purpose:** One-time setup for automated daily backups

**Features:**
- Creates systemd service and timer
- Schedules daily backups at 2:00 AM
- Logs to system journal
- Persistent across reboots

**Usage:**
```bash
sudo ./scripts/setup-backup-cron.sh
```

**Requirements:**
- Must be run as root or with sudo
- Systemd-based Linux system
- Docker service must be available

**What it creates:**
1. `/etc/systemd/system/protein-docking-backup.service` - The backup service
2. `/etc/systemd/system/protein-docking-backup.timer` - The daily scheduler

**Post-Installation:**
The script automatically:
- Enables the timer
- Starts the timer
- Shows timer status and next scheduled run

---

### 3. restore-db.sh
**Location:** `scripts/restore-db.sh`

**Purpose:** Safely restore database from a backup file

**Features:**
- Creates automatic safety backup before restore
- Terminates active database connections
- Drops and recreates database
- Supports both compressed (.gz) and uncompressed (.sql) files
- Requires explicit confirmation before proceeding

**Usage:**
```bash
./scripts/restore-db.sh <backup_file>
```

**Example:**
```bash
./scripts/restore-db.sh ./backups/protein_docking_20251114_143000.sql.gz
```

**Safety Features:**
1. **Confirmation prompt** - Requires typing "yes" to proceed
2. **Safety backup** - Creates `pre_restore_YYYYMMDD_HHMMSS.sql.gz` before any changes
3. **Connection termination** - Cleanly closes all active database connections
4. **Full replacement** - Ensures clean restore without conflicting data

**Output Example:**
```
Database Restore Utility
Backup file: ./backups/protein_docking_20251114_143000.sql.gz
Container: protein_docking_postgres
Database: protein_docking

⚠️  WARNING: This will REPLACE the current database!
All current data will be LOST and replaced with the backup.
Are you sure you want to continue? (yes/no): yes

Creating safety backup of current database...
✓ Safety backup created: ./backups/pre_restore_20251114_150000.sql.gz

Restoring database from backup...
Terminating existing connections...
Recreating database...
Restoring data...

✓ Database restored successfully!
Safety backup available at: ./backups/pre_restore_20251114_150000.sql.gz

You may want to restart your application:
  docker-compose restart backend
```

---

## Automated Backups

### Initial Setup

1. **Run the setup script** (one-time only):
```bash
sudo ./scripts/setup-backup-cron.sh
```

2. **Verify the timer is active**:
```bash
systemctl status protein-docking-backup.timer
```

3. **Check next scheduled backup**:
```bash
systemctl list-timers protein-docking-backup.timer
```

### Schedule
Backups run automatically every day at **2:00 AM** local time.

### Monitoring Automated Backups

**Check timer status:**
```bash
systemctl status protein-docking-backup.timer
```

**View backup logs:**
```bash
journalctl -u protein-docking-backup.service
```

**View recent backup logs:**
```bash
journalctl -u protein-docking-backup.service --since today
```

**View last 50 log entries:**
```bash
journalctl -u protein-docking-backup.service -n 50
```

### Manual Control

**Run backup immediately:**
```bash
sudo systemctl start protein-docking-backup.service
```

**Stop automatic backups temporarily:**
```bash
sudo systemctl stop protein-docking-backup.timer
```

**Restart automatic backups:**
```bash
sudo systemctl start protein-docking-backup.timer
```

**Disable automatic backups permanently:**
```bash
sudo systemctl disable protein-docking-backup.timer
sudo systemctl stop protein-docking-backup.timer
```

**Re-enable automatic backups:**
```bash
sudo systemctl enable protein-docking-backup.timer
sudo systemctl start protein-docking-backup.timer
```

---

## Manual Backups

### Creating a Manual Backup

**Standard backup:**
```bash
./scripts/backup-db.sh
```

**Before major updates or migrations:**
```bash
# Create a backup with custom naming for important milestones
./scripts/backup-db.sh
# Then rename if needed:
mv ./backups/protein_docking_20251114_143000.sql.gz \
   ./backups/protein_docking_pre_migration_v2.0.sql.gz
```

### When to Create Manual Backups

1. **Before database migrations**
2. **Before major application updates**
3. **Before data imports**
4. **Before testing destructive operations**
5. **After important data changes** (create a checkpoint)

---

## Restore Procedures

### Standard Restore

1. **List available backups:**
```bash
ls -lh ./backups/*.gz
```

2. **Choose a backup and restore:**
```bash
./scripts/restore-db.sh ./backups/protein_docking_20251114_143000.sql.gz
```

3. **Confirm the operation** by typing `yes`

4. **Restart the backend** (recommended):
```bash
docker-compose restart backend
```

### Emergency Restore Scenarios

#### Scenario 1: Recent Data Corruption
**Situation:** You discovered data corruption that happened today

**Solution:**
```bash
# Find yesterday's backup
ls -lh ./backups/ | grep "protein_docking_202511"

# Restore from yesterday
./scripts/restore-db.sh ./backups/protein_docking_20251113_020000.sql.gz

# Restart services
docker-compose restart backend
```

#### Scenario 2: Failed Migration
**Situation:** Database migration failed and broke the schema

**Solution:**
```bash
# The safety backup was created automatically during restore attempt
# Look for pre_restore backups
ls -lh ./backups/pre_restore*.gz

# Or restore from last night's automated backup
./scripts/restore-db.sh ./backups/protein_docking_20251113_020000.sql.gz
```

#### Scenario 3: Complete Database Loss
**Situation:** Database container was deleted or database is completely corrupted

**Solution:**
```bash
# Ensure containers are running
docker-compose up -d postgres

# Wait for PostgreSQL to be ready
docker-compose logs postgres

# Restore from most recent backup
./scripts/restore-db.sh ./backups/protein_docking_20251113_020000.sql.gz

# Restart all services
docker-compose restart
```

#### Scenario 4: Restore to Specific Point in Time
**Situation:** Need to restore to exactly 3 days ago

**Solution:**
```bash
# List backups with dates
ls -lh ./backups/ | grep "protein_docking_202511"

# Calculate the date (3 days ago from Nov 14 = Nov 11)
# Restore from that date
./scripts/restore-db.sh ./backups/protein_docking_20251111_020000.sql.gz
```

### Post-Restore Verification

After any restore, verify the data:

```bash
# Connect to database
docker exec -it protein_docking_postgres psql -U your_user -d protein_docking

# Check record counts
SELECT COUNT(*) FROM users;
SELECT COUNT(*) FROM proteins;
SELECT COUNT(*) FROM jobs;

# Check recent entries
SELECT * FROM users ORDER BY created_at DESC LIMIT 5;
SELECT * FROM jobs ORDER BY created_at DESC LIMIT 5;

# Exit
\q
```

---

## Monitoring

### Backup Health Checks

**Check if backups are running:**
```bash
# List recent backups
ls -lth ./backups/ | head -n 5

# Check file sizes (should be reasonable, not 0 bytes)
du -sh ./backups/*.gz
```

**Set up monitoring alerts** (optional, manual setup required):

Create a script `scripts/check-backup-health.sh`:
```bash
#!/bin/bash
LATEST_BACKUP=$(ls -t ./backups/protein_docking_*.gz 2>/dev/null | head -n 1)

if [ -z "$LATEST_BACKUP" ]; then
    echo "ERROR: No backups found!"
    exit 1
fi

# Check if latest backup is less than 25 hours old
BACKUP_AGE=$(find "$LATEST_BACKUP" -mtime +1 2>/dev/null)
if [ ! -z "$BACKUP_AGE" ]; then
    echo "WARNING: Latest backup is older than 24 hours!"
    exit 1
fi

# Check backup size (should be > 1KB)
SIZE=$(stat -f%z "$LATEST_BACKUP" 2>/dev/null || stat -c%s "$LATEST_BACKUP" 2>/dev/null)
if [ "$SIZE" -lt 1024 ]; then
    echo "ERROR: Latest backup is too small (${SIZE} bytes)!"
    exit 1
fi

echo "OK: Backups are healthy"
exit 0
```

### Log Monitoring

**Watch backup logs in real-time:**
```bash
journalctl -u protein-docking-backup.service -f
```

**Email notifications** (requires mail setup):
Add to systemd service:
```ini
[Service]
OnFailure=failure-notification@%n.service
```

---

## Troubleshooting

### Issue: Backup script fails with "container not running"

**Symptoms:**
```
ERROR: PostgreSQL container is not running
```

**Solution:**
```bash
# Start the database container
docker-compose up -d postgres

# Wait a few seconds for PostgreSQL to initialize
sleep 5

# Retry backup
./scripts/backup-db.sh
```

---

### Issue: Restore fails with "database is being accessed"

**Symptoms:**
```
ERROR: database "protein_docking" is being accessed by other users
```

**Solution:**
```bash
# Stop the backend to close connections
docker-compose stop backend

# Retry restore
./scripts/restore-db.sh ./backups/protein_docking_20251114_143000.sql.gz

# Restart backend
docker-compose start backend
```

---

### Issue: Permission denied when running setup-backup-cron.sh

**Symptoms:**
```
Permission denied: /etc/systemd/system/
```

**Solution:**
```bash
# Must run with sudo
sudo ./scripts/setup-backup-cron.sh
```

---

### Issue: Backup file is very small or 0 bytes

**Symptoms:**
Backup completes but file size is < 1KB

**Possible Causes:**
1. Database is empty (new installation)
2. Database connection failed
3. Insufficient permissions

**Solution:**
```bash
# Check if database has data
docker exec -it protein_docking_postgres psql -U your_user -d protein_docking -c "SELECT COUNT(*) FROM users;"

# Check container logs
docker-compose logs postgres

# Verify environment variables
cat .env | grep POSTGRES
```

---

### Issue: Timer not running backups

**Symptoms:**
```bash
systemctl list-timers protein-docking-backup.timer
# Shows "n/a" or timer not listed
```

**Solution:**
```bash
# Reload systemd
sudo systemctl daemon-reload

# Enable timer
sudo systemctl enable protein-docking-backup.timer

# Start timer
sudo systemctl start protein-docking-backup.timer

# Verify
systemctl status protein-docking-backup.timer
```

---

### Issue: Cannot restore - .env file not found

**Symptoms:**
```
ERROR: .env file not found
```

**Solution:**
```bash
# Ensure you're in project root
cd /path/to/protein-docking

# Verify .env exists
ls -la .env

# If missing, copy from example
cp .env.example .env

# Edit with correct values
nano .env
```

---

## Security Considerations

### Access Control

**Backup files contain sensitive data:**
- User credentials (hashed passwords)
- Job data
- Protein structures
- Application configuration

**Best practices:**

1. **Restrict file permissions:**
```bash
chmod 600 ./backups/*.gz
```

2. **Restrict directory access:**
```bash
chmod 700 ./backups/
```

3. **Use a dedicated backup user:**
```bash
# Create backup user (if needed)
sudo useradd -r -s /bin/bash backup-user

# Change ownership
sudo chown -R backup-user:backup-user ./backups/
```

### Off-Site Backups

**For production systems, implement off-site backup strategy:**

1. **Sync to remote storage (example with rsync):**
```bash
# Add to scripts/backup-db.sh after backup creation
rsync -avz --delete ./backups/ user@remote-server:/backups/protein-docking/
```

2. **Cloud storage (example with rclone):**
```bash
# Install rclone and configure
rclone copy ./backups/ remote:protein-docking-backups/
```

3. **Encrypted backups:**
```bash
# Encrypt before uploading
gpg --symmetric --cipher-algo AES256 backup_file.sql.gz
```

### Retention Policy

**Current policy:** 7 days

**To modify retention:**

Edit `scripts/backup-db.sh` line 38:
```bash
# Change +7 to desired number of days
find ./backups/ -name "protein_docking_*.gz" -type f -mtime +7 -delete
```

**Recommended policies by environment:**
- **Development:** 3-7 days
- **Staging:** 14 days
- **Production:** 30+ days with off-site archival

---

## Quick Reference

### Common Commands

```bash
# Manual backup
./scripts/backup-db.sh

# Setup automated backups (one-time)
sudo ./scripts/setup-backup-cron.sh

# Restore database
./scripts/restore-db.sh ./backups/protein_docking_20251114_143000.sql.gz

# List backups
ls -lh ./backups/

# Check timer status
systemctl status protein-docking-backup.timer

# View backup logs
journalctl -u protein-docking-backup.service --since today

# Run backup immediately
sudo systemctl start protein-docking-backup.service
```

### Emergency Quick Start

**Complete database restore:**
```bash
# 1. List available backups
ls -lh ./backups/

# 2. Restore from chosen backup
./scripts/restore-db.sh ./backups/protein_docking_YYYYMMDD_HHMMSS.sql.gz

# 3. Type "yes" when prompted

# 4. Restart services
docker-compose restart
```

---

## Appendix: File Structure

```
protein-docking/
├── scripts/
│   ├── backup-db.sh              # Manual backup script
│   ├── setup-backup-cron.sh      # Automated backup installer
│   └── restore-db.sh             # Database restore script
├── backups/                       # Backup storage directory
│   ├── protein_docking_20251114_020000.sql.gz
│   ├── protein_docking_20251113_020000.sql.gz
│   └── pre_restore_20251114_150000.sql.gz
├── docs/
│   └── BACKUP_RESTORE.md         # This document
└── .env                          # Database credentials
```

---

## Support

For issues or questions:
1. Check the [Troubleshooting](#troubleshooting) section
2. Review system logs: `journalctl -u protein-docking-backup.service`
3. Verify Docker containers: `docker-compose ps`
4. Check environment configuration: `.env` file

---

**Last Updated:** 2025-11-14
**Version:** 1.0
**Maintained by:** yeipills
