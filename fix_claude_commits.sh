#!/bin/bash

# Script para corregir commits de Claude a yeipills
# Ejecutar desde la raíz del repositorio

set -e

echo "=== Corrigiendo autoría de commits de Claude ==="

# Rama 1: claude/realiza-to-015Lkp2QPWre319xZtd9n5uV (8 commits)
echo ""
echo "[1/3] Procesando rama: claude/realiza-to-015Lkp2QPWre319xZtd9n5uV"
git fetch --all
git checkout claude/realiza-to-015Lkp2QPWre319xZtd9n5uV

FILTER_BRANCH_SQUELCH_WARNING=1 git filter-branch --force --env-filter '
if [ "$GIT_AUTHOR_EMAIL" = "noreply@anthropic.com" ]; then
    export GIT_AUTHOR_NAME="yeipills"
    export GIT_AUTHOR_EMAIL="juanpablorosasmartin@gmail.com"
    export GIT_COMMITTER_NAME="yeipills"
    export GIT_COMMITTER_EMAIL="juanpablorosasmartin@gmail.com"
fi
' HEAD~8..HEAD

echo "Haciendo push forzado..."
git push origin claude/realiza-to-015Lkp2QPWre319xZtd9n5uV --force
echo "✓ Rama 1 completada"

# Rama 2: claude/redis-caching-layer-01VWW3HMrZu1kirke5TaFNc4 (3 commits)
echo ""
echo "[2/3] Procesando rama: claude/redis-caching-layer-01VWW3HMrZu1kirke5TaFNc4"
git checkout claude/redis-caching-layer-01VWW3HMrZu1kirke5TaFNc4

FILTER_BRANCH_SQUELCH_WARNING=1 git filter-branch --force --env-filter '
if [ "$GIT_AUTHOR_EMAIL" = "noreply@anthropic.com" ]; then
    export GIT_AUTHOR_NAME="yeipills"
    export GIT_AUTHOR_EMAIL="juanpablorosasmartin@gmail.com"
    export GIT_COMMITTER_NAME="yeipills"
    export GIT_COMMITTER_EMAIL="juanpablorosasmartin@gmail.com"
fi
' HEAD~3..HEAD

echo "Haciendo push forzado..."
git push origin claude/redis-caching-layer-01VWW3HMrZu1kirke5TaFNc4 --force
echo "✓ Rama 2 completada"

# Rama 3: claude/scanning-vulnerabilities-feature-015mrsBs34WBhUdzoZD25K2X (1 commit)
echo ""
echo "[3/3] Procesando rama: claude/scanning-vulnerabilities-feature-015mrsBs34WBhUdzoZD25K2X"
git checkout claude/scanning-vulnerabilities-feature-015mrsBs34WBhUdzoZD25K2X

FILTER_BRANCH_SQUELCH_WARNING=1 git filter-branch --force --env-filter '
if [ "$GIT_AUTHOR_EMAIL" = "noreply@anthropic.com" ]; then
    export GIT_AUTHOR_NAME="yeipills"
    export GIT_AUTHOR_EMAIL="juanpablorosasmartin@gmail.com"
    export GIT_COMMITTER_NAME="yeipills"
    export GIT_COMMITTER_EMAIL="juanpablorosasmartin@gmail.com"
fi
' HEAD~1..HEAD

echo "Haciendo push forzado..."
git push origin claude/scanning-vulnerabilities-feature-015mrsBs34WBhUdzoZD25K2X --force
echo "✓ Rama 3 completada"

echo ""
echo "=== ✓ Todos los commits corregidos exitosamente ==="
echo "Total: 12 commits cambiados de 'Claude' a 'yeipills'"
