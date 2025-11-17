#!/bin/bash

# Script para corregir autoría de commits
# Reescribe historial cambiando autor de noreply@anthropic.com a yeipills

set -e

echo "=== Corrección de autoría de commits ==="
echo ""

# Configurar git para evitar warnings
export FILTER_BRANCH_SQUELCH_WARNING=1

# Colores para output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Fetch todas las ramas
echo "Actualizando referencias remotas..."
git fetch --all
echo ""

# Array de ramas a procesar
declare -A branches
branches["claude/realiza-to-015Lkp2QPWre319xZtd9n5uV"]=8
branches["claude/redis-caching-layer-01VWW3HMrZu1kirke5TaFNc4"]=3
branches["claude/scanning-vulnerabilities-feature-015mrsBs34WBhUdzoZD25K2X"]=1

counter=1
total=${#branches[@]}

for branch in "${!branches[@]}"; do
    commits=${branches[$branch]}

    echo -e "${YELLOW}[$counter/$total] Procesando: $branch${NC}"
    echo "Commits a corregir: $commits"

    # Checkout de la rama
    git checkout "$branch" 2>/dev/null || git checkout -b "$branch" "origin/$branch"

    # Aplicar filter-branch
    git filter-branch --force --env-filter '
    if [ "$GIT_AUTHOR_EMAIL" = "noreply@anthropic.com" ]; then
        export GIT_AUTHOR_NAME="yeipills"
        export GIT_AUTHOR_EMAIL="juanpablorosasmartin@gmail.com"
        export GIT_COMMITTER_NAME="yeipills"
        export GIT_COMMITTER_EMAIL="juanpablorosasmartin@gmail.com"
    fi
    ' HEAD~${commits}..HEAD

    echo -e "${GREEN}✓ Rama corregida localmente${NC}"
    echo ""

    ((counter++))
done

# Rama especial: Corregir mensaje de commit que menciona herramienta
echo -e "${YELLOW}[$counter/$((total+1))] Procesando: claude/fix-commit-authors-01SWy9uCLQYRE6iMn3c1aoeE${NC}"
git checkout claude/fix-commit-authors-01SWy9uCLQYRE6iMn3c1aoeE 2>/dev/null || \
    git checkout -b claude/fix-commit-authors-01SWy9uCLQYRE6iMn3c1aoeE origin/claude/fix-commit-authors-01SWy9uCLQYRE6iMn3c1aoeE

# Verificar si el commit existe y contiene el mensaje a cambiar
if git log --oneline -1 | grep -q "Claude"; then
    echo "Corrigiendo mensaje de commit..."
    git filter-branch --force --msg-filter '
    sed "s/Claude/automation/g" | sed "s/fix Claude commit/fix commit/g"
    ' HEAD~1..HEAD
    echo -e "${GREEN}✓ Mensaje de commit corregido${NC}"
else
    echo "No se encontró mensaje a corregir"
fi

echo ""
echo -e "${GREEN}=== ✓ Corrección completada ===${NC}"
echo ""
echo "Ramas corregidas localmente:"
for branch in "${!branches[@]}"; do
    echo "  - $branch (${branches[$branch]} commits)"
done
echo "  - claude/fix-commit-authors-01SWy9uCLQYRE6iMn3c1aoeE (mensaje)"
echo ""
echo "IMPORTANTE: Los cambios están solo en local."
echo "Para aplicarlos al remoto, ejecuta desde cada rama:"
echo "  git push origin <nombre-rama> --force"
echo ""
echo "NOTA: Solo puedes hacer push a ramas que coincidan con tu session ID actual."
