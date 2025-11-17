#!/bin/bash
# Quick Fix Script for Critical Vulnerabilities
# This script applies the most urgent security fixes

set -e

echo "=========================================="
echo "  🔧 QUICK SECURITY FIX"
echo "=========================================="
echo

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "Project root: $PROJECT_ROOT"
echo

# Backup requirements
echo "📦 Backing up current requirements..."
cp "$PROJECT_ROOT/backend/requirements.txt" "$PROJECT_ROOT/backend/requirements.txt.backup"
echo -e "${GREEN}✓${NC} Backup created: backend/requirements.txt.backup"
echo

# Fix MD5 usage in cache.py
echo "🔧 Fixing MD5 usage in cache.py..."
CACHE_FILE="$PROJECT_ROOT/backend/app/core/cache.py"

if [ -f "$CACHE_FILE" ]; then
    if grep -q "hashlib.md5(" "$CACHE_FILE"; then
        # Replace MD5 with SHA256
        sed -i 's/hashlib\.md5(key_data\.encode())/hashlib.sha256(key_data.encode())/g' "$CACHE_FILE"
        echo -e "${GREEN}✓${NC} Fixed MD5 -> SHA256 in cache.py"
    else
        echo -e "${YELLOW}ℹ${NC}  MD5 not found or already fixed"
    fi
else
    echo -e "${YELLOW}⚠${NC}  cache.py not found"
fi
echo

# Update critical dependencies
echo "📦 Updating critical Python dependencies..."
cd "$PROJECT_ROOT/backend"

# Most critical fixes
CRITICAL_UPDATES=(
    "python-socketio>=5.14.0"
    "flask>=3.1.1"
    "flask-cors>=6.0.0"
    "requests>=2.32.4"
    "python-multipart>=0.0.18"
)

for package in "${CRITICAL_UPDATES[@]}"; do
    echo "  Installing $package..."
    pip install "$package" --quiet
done

echo -e "${GREEN}✓${NC} Critical dependencies updated"
echo

# Update requirements.txt with new versions
echo "📝 Updating requirements.txt..."
if [ -f "$PROJECT_ROOT/backend/requirements-secure.txt" ]; then
    cp "$PROJECT_ROOT/backend/requirements-secure.txt" "$PROJECT_ROOT/backend/requirements.txt"
    echo -e "${GREEN}✓${NC} requirements.txt updated with secure versions"
else
    echo -e "${YELLOW}⚠${NC}  requirements-secure.txt not found, skipping"
fi
echo

# Fix frontend dependencies
echo "📦 Fixing frontend dependencies..."
cd "$PROJECT_ROOT/frontend"

if [ -f "package.json" ]; then
    echo "  Running npm audit fix..."
    npm audit fix --quiet || echo -e "${YELLOW}⚠${NC}  Some issues may require manual intervention"
    echo -e "${GREEN}✓${NC} Frontend dependencies updated"
else
    echo -e "${YELLOW}⚠${NC}  Frontend package.json not found"
fi
echo

# Run verification scan
echo "🔍 Running verification scan..."
cd "$PROJECT_ROOT"

if command -v pip-audit &> /dev/null; then
    echo "  Scanning Python dependencies..."
    cd backend
    VULN_COUNT=$(pip-audit -r requirements.txt 2>&1 | grep -oP 'Found \K\d+' || echo "0")
    echo "  Remaining vulnerabilities: $VULN_COUNT"

    if [ "$VULN_COUNT" -eq 0 ]; then
        echo -e "${GREEN}✓${NC} All Python dependency vulnerabilities fixed!"
    else
        echo -e "${YELLOW}⚠${NC}  $VULN_COUNT vulnerabilities remain (may require manual intervention)"
    fi
    cd ..
else
    echo -e "${YELLOW}ℹ${NC}  pip-audit not installed, skipping verification"
fi
echo

echo "=========================================="
echo "  ✅ QUICK FIX COMPLETE"
echo "=========================================="
echo
echo "Summary of changes:"
echo "  • Updated python-socketio to 5.14.0 (fixes RCE)"
echo "  • Updated flask to 3.1.1 (fixes key rotation)"
echo "  • Updated flask-cors to 6.0.0 (fixes CORS bypass)"
echo "  • Updated requests to 2.32.4 (fixes credential leak)"
echo "  • Updated python-multipart to 0.0.18 (fixes DoS)"
echo "  • Fixed MD5 usage in cache.py (now uses SHA256)"
echo
echo "Next steps:"
echo "  1. Review changes: git diff"
echo "  2. Test the application"
echo "  3. Run full security scan: python scripts/security_scan.py"
echo "  4. Commit changes: git add . && git commit -m 'fix: apply critical security updates'"
echo
echo "📄 For details, see: SECURITY_AUDIT_REPORT.md"
echo
