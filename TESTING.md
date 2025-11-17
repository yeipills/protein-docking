# Testing Guide - Protein Docking Platform

> **Version:** 2.4.0 (In Development)
> **Last Updated:** 2025-11-15
> **Status:** Testing Infrastructure Setup

## Overview

This document provides a comprehensive guide for testing the Protein Docking Platform. Our testing strategy includes:

- **E2E Tests** - End-to-end user flow testing with Playwright
- **Backend Unit Tests** - API and algorithm testing with pytest
- **Frontend Unit Tests** - Component and hook testing with Vitest
- **Integration Tests** - Service integration testing
- **Load Testing** - Performance and stress testing

**Testing Goals:**
- ✅ Minimum 80% code coverage
- ✅ All critical paths covered by E2E tests
- ✅ Automated CI/CD testing
- ✅ Fast test execution (<5 minutes)

---

## Table of Contents

1. [Quick Start](#quick-start)
2. [Backend Testing (pytest)](#backend-testing-pytest)
3. [Frontend Testing (Vitest)](#frontend-testing-vitest)
4. [E2E Testing (Playwright)](#e2e-testing-playwright)
5. [Integration Testing](#integration-testing)
6. [Load Testing](#load-testing)
7. [CI/CD Integration](#cicd-integration)
8. [Coverage Reports](#coverage-reports)
9. [Best Practices](#best-practices)

---

## Quick Start

### Run All Tests

```bash
# Run comprehensive test suite
./scripts/run-tests.sh

# Run specific test suites
./scripts/run-tests.sh backend   # Backend only
./scripts/run-tests.sh frontend  # Frontend only
./scripts/run-tests.sh e2e       # E2E only
./scripts/run-tests.sh lint      # Linting only
```

### Development Mode

```bash
# Backend tests in watch mode
cd backend
pytest --watch

# Frontend tests in watch mode
cd frontend
npm run test:watch

# E2E tests in UI mode
cd frontend
npm run test:e2e:ui
```

---

## Backend Testing (pytest)

### Setup

**Install Dependencies:**
```bash
cd backend
pip install -r requirements-test.txt
```

**Requirements (requirements-test.txt):**
```
pytest==8.3.3
pytest-asyncio==0.24.0
pytest-cov==6.0.0
pytest-mock==3.14.0
pytest-env==1.1.5
httpx==0.28.0
faker==30.8.2
```

### Running Tests

```bash
# Run all backend tests
pytest

# Run with coverage
pytest --cov=app --cov-report=html --cov-report=term

# Run specific test file
pytest tests/test_auth.py

# Run specific test function
pytest tests/test_auth.py::test_user_registration

# Run tests in parallel (faster)
pytest -n auto

# Run with verbose output
pytest -v

# Run only failed tests from last run
pytest --lf
```

### Test Structure

```
backend/tests/
├── conftest.py                 # Shared fixtures
├── unit/
│   ├── test_auth.py           # Authentication tests
│   ├── test_users.py          # User management tests
│   ├── test_jobs.py           # Job management tests
│   ├── test_proteins.py       # Protein operations tests
│   ├── test_file_validation.py # File validation tests
│   ├── test_password_policy.py # Password validation tests
│   ├── test_account_lockout.py # Account lockout tests
│   └── algorithms/
│       ├── test_surface_reader.py
│       ├── test_centroid_calculator.py
│       ├── test_context_rays.py
│       └── test_layer_evaluator.py
├── integration/
│   ├── test_database.py       # Database integration
│   ├── test_redis.py          # Redis integration
│   └── test_celery.py         # Celery task tests
└── fixtures/
    ├── users.py               # User fixtures
    ├── proteins.py            # Protein test data
    └── files.py               # Test file fixtures
```

### Example Tests

#### Authentication Tests

```python
# tests/unit/test_auth.py
import pytest
from fastapi.testclient import TestClient
from app.main import app

client = TestClient(app)

def test_user_registration_success(db_session):
    """Test successful user registration"""
    response = client.post("/api/v1/auth/register", json={
        "email": "test@example.com",
        "username": "testuser",
        "full_name": "Test User",
        "password": "SecurePassword123!"
    })

    assert response.status_code == 201
    data = response.json()
    assert data["email"] == "test@example.com"
    assert data["username"] == "testuser"
    assert "id" in data

def test_user_registration_weak_password(db_session):
    """Test registration fails with weak password"""
    response = client.post("/api/v1/auth/register", json={
        "email": "test@example.com",
        "username": "testuser",
        "full_name": "Test User",
        "password": "weak"  # Too short, no special chars
    })

    assert response.status_code == 422
    assert "password" in response.json()["detail"][0]["loc"]

def test_login_success(db_session, test_user):
    """Test successful login"""
    response = client.post("/api/v1/auth/login", json={
        "username": "testuser",
        "password": "SecurePassword123!"
    })

    assert response.status_code == 200
    assert "access_token" in response.cookies
    assert "refresh_token" in response.cookies

def test_account_lockout_after_failed_attempts(db_session, test_user):
    """Test account locks after 5 failed login attempts"""
    for i in range(5):
        response = client.post("/api/v1/auth/login", json={
            "username": "testuser",
            "password": "WrongPassword123!"
        })
        assert response.status_code == 401

    # 6th attempt should return locked status
    response = client.post("/api/v1/auth/login", json={
        "username": "testuser",
        "password": "SecurePassword123!"  # Even correct password
    })

    assert response.status_code == 401
    assert "locked" in response.json()["detail"].lower()
```

#### File Validation Tests

```python
# tests/unit/test_file_validation.py
import pytest
from app.core.file_validation import validate_file_upload, sanitize_filename

@pytest.mark.asyncio
async def test_validate_stl_file_success(stl_file_mock):
    """Test valid STL file passes validation"""
    await validate_file_upload(stl_file_mock, '.stl')
    # Should not raise exception

@pytest.mark.asyncio
async def test_validate_file_wrong_extension():
    """Test validation fails for wrong extension"""
    with pytest.raises(ValueError, match="Invalid file extension"):
        await validate_file_upload(executable_file_mock, '.stl')

@pytest.mark.asyncio
async def test_validate_file_too_large():
    """Test validation fails for oversized file"""
    with pytest.raises(ValueError, match="File too large"):
        await validate_file_upload(large_file_mock, '.stl')

def test_sanitize_filename_removes_path_traversal():
    """Test filename sanitization prevents path traversal"""
    assert sanitize_filename("../../etc/passwd") == "etc_passwd"
    assert sanitize_filename("../../../malicious.txt") == "malicious.txt"
    assert sanitize_filename("normal_file.stl") == "normal_file.stl"
```

#### Algorithm Tests

```python
# tests/unit/algorithms/test_centroid_calculator.py
import pytest
import numpy as np
from app.algorithms.centroid_calculator import calculate_centroids

def test_calculate_centroids_simple_triangle():
    """Test centroid calculation for single triangle"""
    vertices = [
        [1, 0.0, 0.0, 0.0],  # v1: (0, 0, 0)
        [2, 3.0, 0.0, 0.0],  # v2: (3, 0, 0)
        [3, 0.0, 3.0, 0.0],  # v3: (0, 3, 0)
    ]

    faces = [
        [1, 1, 2, 3, 2]  # Triangle using v1, v2, v3, type 2
    ]

    centroids, centroid_strs = calculate_centroids(vertices, faces)

    # Centroid should be at (1, 1, 0)
    assert len(centroids) == 1
    assert np.allclose(centroids[0], [1.0, 1.0, 0.0])
    assert centroid_strs[0] == "1.0 1.0 0.0"

def test_calculate_centroids_filters_type1_faces():
    """Test that type 1 faces are filtered out"""
    vertices = [[1, 0.0, 0.0, 0.0], [2, 1.0, 0.0, 0.0], [3, 0.0, 1.0, 0.0]]
    faces = [
        [1, 1, 2, 3, 1],  # Type 1 - should be filtered
        [2, 1, 2, 3, 2],  # Type 2 - should be included
    ]

    centroids, _ = calculate_centroids(vertices, faces)
    assert len(centroids) == 1
```

### Fixtures (conftest.py)

```python
# tests/conftest.py
import pytest
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker
from app.database import Base
from app.models import User
from app.core.security import hash_password

@pytest.fixture(scope="session")
def engine():
    """Create test database engine"""
    engine = create_engine("postgresql://test:test@localhost/test_protein_docking")
    Base.metadata.create_all(engine)
    yield engine
    Base.metadata.drop_all(engine)

@pytest.fixture(scope="function")
def db_session(engine):
    """Create a new database session for each test"""
    Session = sessionmaker(bind=engine)
    session = Session()
    yield session
    session.rollback()
    session.close()

@pytest.fixture
def test_user(db_session):
    """Create a test user"""
    user = User(
        email="test@example.com",
        username="testuser",
        full_name="Test User",
        hashed_password=hash_password("SecurePassword123!"),
        is_active=True,
    )
    db_session.add(user)
    db_session.commit()
    db_session.refresh(user)
    return user

@pytest.fixture
def authenticated_client(test_user):
    """Create an authenticated test client"""
    response = client.post("/api/v1/auth/login", json={
        "username": "testuser",
        "password": "SecurePassword123!"
    })
    return client
```

---

## Frontend Testing (Vitest)

### Setup

**Install Dependencies:**
```bash
cd frontend
npm install --save-dev vitest @testing-library/react @testing-library/user-event @testing-library/jest-dom jsdom
```

**Configuration (vite.config.ts):**
```typescript
/// <reference types="vitest" />
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: './src/test/setup.ts',
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      exclude: ['node_modules/', 'src/test/'],
    },
  },
})
```

### Running Tests

```bash
# Run all frontend tests
npm run test

# Run with coverage
npm run test:coverage

# Watch mode
npm run test:watch

# UI mode (interactive)
npm run test:ui
```

### Test Structure

```
frontend/src/
├── components/
│   ├── Button.tsx
│   ├── Button.test.tsx
│   ├── JobCard.tsx
│   ├── JobCard.test.tsx
│   └── ...
├── hooks/
│   ├── useAuth.ts
│   ├── useAuth.test.ts
│   └── ...
├── services/
│   ├── api.ts
│   ├── api.test.ts
│   └── ...
└── test/
    ├── setup.ts
    ├── mocks/
    │   ├── handlers.ts     # MSW request handlers
    │   └── server.ts       # MSW server setup
    └── utils.tsx           # Test utilities
```

### Example Tests

#### Component Tests

```typescript
// src/components/JobCard.test.tsx
import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { JobCard } from './JobCard'
import { JobStatus } from '../types'

describe('JobCard', () => {
  const mockJob = {
    id: 1,
    protein_name: 'Test Protein',
    job_type: 'part_one',
    status: JobStatus.PROCESSING,
    progress: 50,
    created_at: '2025-11-15T12:00:00Z',
  }

  it('renders job information correctly', () => {
    render(<JobCard job={mockJob} />)

    expect(screen.getByText('Test Protein')).toBeInTheDocument()
    expect(screen.getByText('50%')).toBeInTheDocument()
    expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '50')
  })

  it('shows cancel button for processing jobs', () => {
    render(<JobCard job={mockJob} />)

    const cancelButton = screen.getByRole('button', { name: /cancel/i })
    expect(cancelButton).toBeInTheDocument()
    expect(cancelButton).not.toBeDisabled()
  })

  it('does not show cancel button for completed jobs', () => {
    const completedJob = { ...mockJob, status: JobStatus.COMPLETED }
    render(<JobCard job={completedJob} />)

    expect(screen.queryByRole('button', { name: /cancel/i })).not.toBeInTheDocument()
  })

  it('calls cancel mutation when cancel button clicked', async () => {
    const user = userEvent.setup()
    const mockCancel = vi.fn()

    render(<JobCard job={mockJob} onCancel={mockCancel} />)

    await user.click(screen.getByRole('button', { name: /cancel/i }))

    expect(mockCancel).toHaveBeenCalledWith(mockJob.id)
  })
})
```

#### Hook Tests

```typescript
// src/hooks/useAuth.test.ts
import { describe, it, expect } from 'vitest'
import { renderHook, waitFor } from '@testing-library/react'
import { useAuth } from './useAuth'
import { wrapper } from '../test/utils'

describe('useAuth', () => {
  it('returns null user when not authenticated', () => {
    const { result } = renderHook(() => useAuth(), { wrapper })

    expect(result.current.user).toBeNull()
    expect(result.current.isAuthenticated).toBe(false)
  })

  it('logs in user successfully', async () => {
    const { result } = renderHook(() => useAuth(), { wrapper })

    await result.current.login('testuser', 'SecurePassword123!')

    await waitFor(() => {
      expect(result.current.isAuthenticated).toBe(true)
      expect(result.current.user).toMatchObject({
        username: 'testuser',
      })
    })
  })

  it('handles login failure', async () => {
    const { result } = renderHook(() => useAuth(), { wrapper })

    await expect(
      result.current.login('testuser', 'wrong_password')
    ).rejects.toThrow('Invalid credentials')

    expect(result.current.isAuthenticated).toBe(false)
  })
})
```

#### API Service Tests

```typescript
// src/services/api.test.ts
import { describe, it, expect, beforeAll, afterAll, afterEach } from 'vitest'
import { rest } from 'msw'
import { setupServer } from 'msw/node'
import { apiClient } from './api'

const server = setupServer(
  rest.post('/api/v1/auth/login', (req, res, ctx) => {
    return res(ctx.json({ access_token: 'test-token' }))
  }),
  rest.get('/api/v1/jobs', (req, res, ctx) => {
    return res(ctx.json([
      { id: 1, protein_name: 'Test', status: 'completed' }
    ]))
  })
)

beforeAll(() => server.listen())
afterEach(() => server.resetHandlers())
afterAll(() => server.close())

describe('API Client', () => {
  it('sends login request correctly', async () => {
    const response = await apiClient.login('user', 'pass')
    expect(response.access_token).toBe('test-token')
  })

  it('fetches jobs list', async () => {
    const jobs = await apiClient.getJobs()
    expect(jobs).toHaveLength(1)
    expect(jobs[0].protein_name).toBe('Test')
  })
})
```

---

## E2E Testing (Playwright)

### Setup

**Install Playwright:**
```bash
cd frontend
npm install --save-dev @playwright/test
npx playwright install
```

**Configuration (playwright.config.ts):**
```typescript
import { defineConfig, devices } from '@playwright/test'

export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: 'html',
  use: {
    baseURL: 'http://localhost:3000',
    trace: 'on-first-retry',
  },
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
    { name: 'firefox', use: { ...devices['Desktop Firefox'] } },
    { name: 'webkit', use: { ...devices['Desktop Safari'] } },
  ],
  webServer: {
    command: 'npm run dev',
    url: 'http://localhost:3000',
    reuseExistingServer: !process.env.CI,
  },
})
```

### Running E2E Tests

```bash
# Run all E2E tests
npm run test:e2e

# Run in UI mode (interactive)
npm run test:e2e:ui

# Run in headed mode (see browser)
npm run test:e2e:headed

# Run specific test file
npx playwright test e2e/auth.spec.ts

# Debug mode
npx playwright test --debug

# Generate test report
npx playwright show-report
```

### Test Structure

```
frontend/e2e/
├── auth.spec.ts              # Authentication flows
├── protein-upload.spec.ts    # Protein upload flows
├── job-tracking.spec.ts      # Job tracking and progress
├── websocket.spec.ts         # Real-time updates
└── fixtures/
    └── test-files/           # Sample STL, vert, face files
```

### Example E2E Tests

```typescript
// e2e/auth.spec.ts
import { test, expect } from '@playwright/test'

test.describe('Authentication', () => {
  test('user can register new account', async ({ page }) => {
    await page.goto('/')
    await page.click('text=Sign Up')

    await page.fill('[name="email"]', 'newuser@example.com')
    await page.fill('[name="username"]', 'newuser')
    await page.fill('[name="full_name"]', 'New User')
    await page.fill('[name="password"]', 'SecurePassword123!')

    await page.click('button[type="submit"]')

    // Should redirect to dashboard
    await expect(page).toHaveURL('/dashboard')
    await expect(page.locator('text=Welcome, newuser')).toBeVisible()
  })

  test('user can login and logout', async ({ page }) => {
    await page.goto('/login')

    await page.fill('[name="username"]', 'testuser')
    await page.fill('[name="password"]', 'SecurePassword123!')
    await page.click('button[type="submit"]')

    // Should be logged in
    await expect(page).toHaveURL('/dashboard')

    // Logout
    await page.click('button[aria-label="User menu"]')
    await page.click('text=Logout')

    // Should redirect to login
    await expect(page).toHaveURL('/login')
  })

  test('shows error for invalid credentials', async ({ page }) => {
    await page.goto('/login')

    await page.fill('[name="username"]', 'testuser')
    await page.fill('[name="password"]', 'WrongPassword')
    await page.click('button[type="submit"]')

    await expect(page.locator('text=Invalid credentials')).toBeVisible()
  })
})

// e2e/protein-upload.spec.ts
test.describe('Protein Upload', () => {
  test.beforeEach(async ({ page }) => {
    // Login before each test
    await page.goto('/login')
    await page.fill('[name="username"]', 'testuser')
    await page.fill('[name="password"]', 'SecurePassword123!')
    await page.click('button[type="submit"]')
    await expect(page).toHaveURL('/dashboard')
  })

  test('user can upload protein files for Part One', async ({ page }) => {
    await page.goto('/upload')

    // Fill protein name
    await page.fill('[name="protein_name"]', 'Test Protein')

    // Upload files
    const [stlFileChooser] = await Promise.all([
      page.waitForEvent('filechooser'),
      page.click('text=Choose STL file'),
    ])
    await stlFileChooser.setFiles('./e2e/fixtures/test.stl')

    const [vertFileChooser] = await Promise.all([
      page.waitForEvent('filechooser'),
      page.click('text=Choose Vertices file'),
    ])
    await vertFileChooser.setFiles('./e2e/fixtures/test.vert')

    const [faceFileChooser] = await Promise.all([
      page.waitForEvent('filechooser'),
      page.click('text=Choose Faces file'),
    ])
    await faceFileChooser.setFiles('./e2e/fixtures/test.face')

    // Submit
    await page.click('button[type="submit"]')

    // Should show success and redirect
    await expect(page.locator('text=Upload successful')).toBeVisible()
    await expect(page).toHaveURL('/dashboard')

    // Job should appear in list
    await expect(page.locator('text=Test Protein')).toBeVisible()
  })
})
```

---

## Integration Testing

### Database Integration

```python
# tests/integration/test_database.py
import pytest
from app.models import User, Job, Protein
from app.database import SessionLocal

@pytest.mark.integration
def test_user_job_relationship():
    """Test user-job database relationship"""
    db = SessionLocal()

    # Create user
    user = User(email="test@example.com", username="test")
    db.add(user)
    db.commit()

    # Create job for user
    job = Job(user_id=user.id, job_type="part_one", status="pending")
    db.add(job)
    db.commit()

    # Verify relationship
    assert len(user.jobs) == 1
    assert user.jobs[0].id == job.id

    db.close()
```

### Redis Integration

```python
# tests/integration/test_redis.py
import pytest
from app.core.cache import cache_set, cache_get

@pytest.mark.integration
async def test_redis_cache():
    """Test Redis caching works correctly"""
    key = "test:key"
    value = {"data": "test"}

    # Set cache
    await cache_set(key, value, ttl=60)

    # Get cache
    cached = await cache_get(key)
    assert cached == value
```

---

## Load Testing

### Locust Configuration

**Install:**
```bash
pip install locust
```

**Load Test Script (locustfile.py):**
```python
from locust import HttpUser, task, between

class ProteinDockingUser(HttpUser):
    wait_time = between(1, 3)

    def on_start(self):
        """Login before starting tasks"""
        self.client.post("/api/v1/auth/login", json={
            "username": "testuser",
            "password": "SecurePassword123!"
        })

    @task(3)
    def get_jobs(self):
        """Fetch jobs list (most common operation)"""
        self.client.get("/api/v1/jobs")

    @task(1)
    def get_job_detail(self):
        """Fetch single job details"""
        self.client.get("/api/v1/jobs/1")

    @task(2)
    def get_proteins(self):
        """Fetch proteins list"""
        self.client.get("/api/v1/proteins")
```

**Run Load Test:**
```bash
# Start locust
locust -f locustfile.py --host=http://localhost:5000

# Or run headless
locust -f locustfile.py --host=http://localhost:5000 --users 100 --spawn-rate 10 --run-time 1m
```

---

## CI/CD Integration

### GitHub Actions

**`.github/workflows/test.yml`:**
```yaml
name: Tests

on: [push, pull_request]

jobs:
  backend-tests:
    runs-on: ubuntu-latest

    services:
      postgres:
        image: postgres:16
        env:
          POSTGRES_PASSWORD: test
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5

      redis:
        image: redis:7
        options: >-
          --health-cmd "redis-cli ping"
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5

    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: '3.11'

      - name: Install dependencies
        run: |
          cd backend
          pip install -r requirements.txt
          pip install -r requirements-test.txt

      - name: Run tests
        run: |
          cd backend
          pytest --cov=app --cov-report=xml

      - name: Upload coverage
        uses: codecov/codecov-action@v4
        with:
          file: ./backend/coverage.xml

  frontend-tests:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '20'

      - name: Install dependencies
        run: |
          cd frontend
          npm ci

      - name: Run tests
        run: |
          cd frontend
          npm run test:coverage

      - name: Upload coverage
        uses: codecov/codecov-action@v4
        with:
          file: ./frontend/coverage/coverage-final.json

  e2e-tests:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '20'

      - name: Install dependencies
        run: |
          cd frontend
          npm ci
          npx playwright install --with-deps

      - name: Run E2E tests
        run: |
          cd frontend
          npm run test:e2e

      - name: Upload test results
        uses: actions/upload-artifact@v4
        if: failure()
        with:
          name: playwright-report
          path: frontend/playwright-report
```

---

## Coverage Reports

### Backend Coverage

```bash
# Generate HTML coverage report
cd backend
pytest --cov=app --cov-report=html

# Open report
open htmlcov/index.html
```

### Frontend Coverage

```bash
# Generate coverage report
cd frontend
npm run test:coverage

# Open report
open coverage/index.html
```

### Coverage Goals

**Minimum Coverage Targets:**
- Overall: 80%
- Critical paths (auth, file validation, algorithms): 90%+
- Utils and helpers: 70%+

---

## Best Practices

### General

1. **Write tests first** - TDD when possible
2. **Keep tests isolated** - No dependencies between tests
3. **Use descriptive names** - Test names should describe what they test
4. **Test edge cases** - Not just happy paths
5. **Mock external dependencies** - Database, APIs, file system
6. **Fast execution** - Tests should run in <5 minutes

### Backend

1. **Use fixtures** - Share common setup code
2. **Test database rollback** - Each test gets clean database
3. **Test authentication** - Verify JWT, permissions, lockout
4. **Test validation** - File validation, password strength, sanitization
5. **Test algorithms** - Verify correctness, performance, edge cases

### Frontend

1. **Test user interactions** - Click, type, submit
2. **Test accessibility** - Screen readers, keyboard navigation
3. **Test loading states** - Spinners, skeletons
4. **Test error states** - Network errors, validation errors
5. **Mock API calls** - Use MSW for consistent mocking

### E2E

1. **Test critical paths** - Registration, login, upload, download
2. **Test real-time updates** - WebSocket notifications
3. **Test different browsers** - Chrome, Firefox, Safari
4. **Take screenshots on failure** - Debug test failures
5. **Test mobile responsive** - Use device emulation

---

## Troubleshooting

### Common Issues

**Tests fail in CI but pass locally:**
- Check environment variables
- Verify database/Redis availability
- Check for race conditions
- Ensure consistent timezone

**Slow test execution:**
- Run tests in parallel (`pytest -n auto`)
- Mock expensive operations (file I/O, API calls)
- Use in-memory database for unit tests
- Skip integration tests in development

**Flaky E2E tests:**
- Add explicit waits (`waitFor`, `waitForSelector`)
- Increase timeout for slow operations
- Use test isolation (clean database between tests)
- Avoid timing-dependent assertions

---

## Resources

### Documentation

- **pytest**: https://docs.pytest.org/
- **Vitest**: https://vitest.dev/
- **Playwright**: https://playwright.dev/
- **Testing Library**: https://testing-library.com/
- **Locust**: https://locust.io/

### Examples

See `tests/` directories in backend and frontend for complete examples.

---

**Next Steps:**

1. ✅ Read this guide
2. ⬜ Install testing dependencies
3. ⬜ Write first unit tests
4. ⬜ Set up E2E testing
5. ⬜ Configure CI/CD
6. ⬜ Achieve 80% coverage
7. ⬜ Add load testing

---

**Generated:** 2025-11-15
**Version:** 2.4.0 (In Development)
**Status:** Documentation Complete - Implementation Pending
**Next:** Setup pytest, Vitest, and Playwright
