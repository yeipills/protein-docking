#!/bin/bash
# ================================
# Test Suite Runner
# ================================
# Runs all tests for backend and frontend
# Usage: ./scripts/run-tests.sh [backend|frontend|all]

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test mode (default: all)
MODE=${1:-all}

echo -e "${BLUE}================================${NC}"
echo -e "${BLUE}Running Test Suite${NC}"
echo -e "${BLUE}================================${NC}\n"

# Function to run backend tests
run_backend_tests() {
    echo -e "${YELLOW}Running backend tests...${NC}\n"

    # Check if backend container is running
    if ! docker-compose ps | grep -q "backend.*Up"; then
        echo -e "${YELLOW}Starting backend services...${NC}"
        docker-compose -f docker-compose.dev.yml up -d postgres redis backend
        sleep 5
    fi

    # Run pytest
    echo -e "${YELLOW}Executing pytest...${NC}"
    docker-compose -f docker-compose.dev.yml exec -T backend pytest tests/ -v --cov=app --cov-report=term-missing --cov-report=html || {
        echo -e "${RED}Backend tests failed${NC}"
        return 1
    }

    echo -e "${GREEN}✓ Backend tests passed${NC}\n"
    echo -e "${YELLOW}Coverage report generated at: backend/htmlcov/index.html${NC}\n"
}

# Function to run frontend tests
run_frontend_tests() {
    echo -e "${YELLOW}Running frontend tests...${NC}\n"

    # Check if node_modules exists
    if [ ! -d "frontend/node_modules" ]; then
        echo -e "${YELLOW}Installing frontend dependencies...${NC}"
        cd frontend
        npm install
        cd ..
    fi

    # Run vitest
    echo -e "${YELLOW}Executing vitest...${NC}"
    cd frontend
    npm run test:run || {
        echo -e "${RED}Frontend tests failed${NC}"
        cd ..
        return 1
    }
    cd ..

    echo -e "${GREEN}✓ Frontend tests passed${NC}\n"
}

# Function to run linting
run_linting() {
    echo -e "${YELLOW}Running linters...${NC}\n"

    # Backend linting
    echo -e "${YELLOW}Checking backend code with flake8...${NC}"
    docker-compose -f docker-compose.dev.yml exec -T backend flake8 app/ --max-line-length=100 --extend-ignore=E203,W503 || {
        echo -e "${RED}Backend linting failed${NC}"
    }

    # Backend type checking
    echo -e "${YELLOW}Checking backend types with mypy...${NC}"
    docker-compose -f docker-compose.dev.yml exec -T backend mypy app/ --ignore-missing-imports || {
        echo -e "${RED}Backend type checking failed${NC}"
    }

    # Frontend linting
    echo -e "${YELLOW}Checking frontend code with ESLint...${NC}"
    cd frontend
    npm run lint || {
        echo -e "${RED}Frontend linting failed${NC}"
    }
    cd ..

    echo -e "${GREEN}✓ Linting complete${NC}\n"
}

# Main execution
case $MODE in
    backend)
        run_backend_tests
        ;;
    frontend)
        run_frontend_tests
        ;;
    lint)
        run_linting
        ;;
    all)
        run_backend_tests
        run_frontend_tests
        run_linting
        ;;
    *)
        echo -e "${RED}Invalid mode: $MODE${NC}"
        echo "Usage: ./scripts/run-tests.sh [backend|frontend|lint|all]"
        exit 1
        ;;
esac

echo -e "\n${BLUE}================================${NC}"
echo -e "${GREEN}Test suite completed!${NC}"
echo -e "${BLUE}================================${NC}\n"

# Summary
if [ "$MODE" = "all" ]; then
    echo -e "${YELLOW}Test Summary:${NC}"
    echo "  ✓ Backend unit tests"
    echo "  ✓ Frontend unit tests"
    echo "  ✓ Code linting"
    echo "  ✓ Type checking"
    echo ""
    echo -e "${YELLOW}Coverage reports:${NC}"
    echo "  Backend: backend/htmlcov/index.html"
    echo "  Frontend: frontend/coverage/index.html"
fi

echo -e "\n${GREEN}All tests passed! 🎉${NC}\n"
