"""
Pytest configuration and fixtures
Shared fixtures for all tests
"""
import pytest
from fastapi.testclient import TestClient
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker
from sqlalchemy.pool import StaticPool

from app.main import app
from app.database import Base, get_db
from app.config import get_settings
from app.core.security import create_access_token

# Test database URL (in-memory SQLite)
SQLALCHEMY_DATABASE_URL = "sqlite:///:memory:"

# Create test engine
engine = create_engine(
    SQLALCHEMY_DATABASE_URL,
    connect_args={"check_same_thread": False},
    poolclass=StaticPool,
)

# Create test session
TestingSessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)


@pytest.fixture(scope="function")
def db_session():
    """
    Create a fresh database session for each test.
    Rolls back all changes after the test.
    """
    # Create tables
    Base.metadata.create_all(bind=engine)

    # Create session
    session = TestingSessionLocal()

    try:
        yield session
    finally:
        session.rollback()
        session.close()

    # Drop tables
    Base.metadata.drop_all(bind=engine)


@pytest.fixture(scope="function")
def client(db_session):
    """
    Create a test client with overridden database dependency.
    """

    def override_get_db():
        try:
            yield db_session
        finally:
            pass

    app.dependency_overrides[get_db] = override_get_db

    with TestClient(app) as test_client:
        yield test_client

    app.dependency_overrides.clear()


@pytest.fixture
def test_user(db_session):
    """
    Create a test user in the database.
    """
    from app.models.user import User
    from app.core.security import get_password_hash

    user = User(
        email="test@example.com",
        username="testuser",
        full_name="Test User",
        hashed_password=get_password_hash("testpass123"),
        is_active=True,
        is_superuser=False,
    )
    db_session.add(user)
    db_session.commit()
    db_session.refresh(user)

    return user


@pytest.fixture
def test_superuser(db_session):
    """
    Create a test superuser in the database.
    """
    from app.models.user import User
    from app.core.security import get_password_hash

    user = User(
        email="admin@example.com",
        username="admin",
        full_name="Admin User",
        hashed_password=get_password_hash("adminpass123"),
        is_active=True,
        is_superuser=True,
    )
    db_session.add(user)
    db_session.commit()
    db_session.refresh(user)

    return user


@pytest.fixture
def user_token(test_user):
    """
    Create an access token for the test user.
    """
    return create_access_token(subject=test_user.id)


@pytest.fixture
def admin_token(test_superuser):
    """
    Create an access token for the test superuser.
    """
    return create_access_token(subject=test_superuser.id)


@pytest.fixture
def auth_headers(user_token):
    """
    Create authorization headers for authenticated requests.
    """
    return {"Authorization": f"Bearer {user_token}"}


@pytest.fixture
def admin_headers(admin_token):
    """
    Create authorization headers for superuser requests.
    """
    return {"Authorization": f"Bearer {admin_token}"}


@pytest.fixture
def test_protein(db_session, test_user):
    """
    Create a test protein in the database.
    """
    from app.models.protein import Protein

    protein = Protein(
        user_id=test_user.id,
        name="Test Protein",
        description="A test protein for testing",
    )
    db_session.add(protein)
    db_session.commit()
    db_session.refresh(protein)

    return protein


@pytest.fixture
def test_job(db_session, test_user, test_protein):
    """
    Create a test job in the database.
    """
    from app.models.job import Job, JobType, JobStatus

    job = Job(
        user_id=test_user.id,
        protein_id=test_protein.id,
        job_type=JobType.PART_ONE,
        status=JobStatus.PENDING,
        progress=0,
    )
    db_session.add(job)
    db_session.commit()
    db_session.refresh(job)

    return job


@pytest.fixture(autouse=True)
def reset_settings():
    """
    Reset settings cache after each test.
    """
    yield
    get_settings.cache_clear()


@pytest.fixture
def mock_celery(monkeypatch):
    """
    Mock Celery task execution for testing.
    """
    tasks_called = []

    class MockTask:
        def __init__(self, name):
            self.name = name

        def delay(self, *args, **kwargs):
            tasks_called.append({"name": self.name, "args": args, "kwargs": kwargs})
            return MockAsyncResult()

        def apply_async(self, *args, **kwargs):
            tasks_called.append({"name": self.name, "args": args, "kwargs": kwargs})
            return MockAsyncResult()

    class MockAsyncResult:
        id = "test-task-id-12345"
        state = "PENDING"

        def get(self, timeout=None):
            return {"status": "success"}

    def mock_task_decorator(*args, **kwargs):
        def decorator(func):
            task = MockTask(func.__name__)
            return task

        return decorator

    # Mock the Celery app task decorator
    monkeypatch.setattr("app.tasks.celery_app.celery_app.task", mock_task_decorator)

    yield {"tasks_called": tasks_called}


# Test data factories
@pytest.fixture
def user_data():
    """Test user registration data"""
    return {
        "email": "newuser@example.com",
        "username": "newuser",
        "full_name": "New User",
        "password": "securepass123",
    }


@pytest.fixture
def login_data():
    """Test login data"""
    return {"username": "testuser", "password": "testpass123"}
