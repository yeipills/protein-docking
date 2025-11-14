"""
Database connection and session management
"""
from sqlalchemy import create_engine
from sqlalchemy.ext.declarative import declarative_base
from sqlalchemy.orm import sessionmaker, Session
from sqlalchemy.exc import ProgrammingError
from typing import Generator
from app.config import get_settings
from app.core.logging import get_logger

logger = get_logger(__name__)

settings = get_settings()

# Create SQLAlchemy engine with optimized pool configuration
engine = create_engine(
    settings.DATABASE_URL,
    pool_pre_ping=True,  # Verify connections before using
    pool_size=10,  # Base pool size
    max_overflow=20,  # Additional connections when pool is full
    pool_recycle=3600,  # Recycle connections after 1 hour (prevents stale connections)
    pool_timeout=30,  # Timeout for getting connection from pool
    echo=settings.ENVIRONMENT == "development",  # SQL logging in development
    connect_args={
        "connect_timeout": 10,  # Connection timeout
        "options": "-c statement_timeout=30000"  # 30 second query timeout
    }
)

# Create SessionLocal class
SessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)

# Create Base class for models
Base = declarative_base()


def get_db() -> Generator[Session, None, None]:
    """
    Dependency to get database session
    Usage in FastAPI: db: Session = Depends(get_db)
    """
    db = SessionLocal()
    try:
        yield db
    finally:
        db.close()


def create_tables():
    """Create all tables in the database"""
    try:
        Base.metadata.create_all(bind=engine, checkfirst=True)
    except ProgrammingError as e:
        # Handle duplicate table/index errors during development hot-reload
        if "already exists" in str(e):
            logger.warning(f"Database objects already exist (likely due to hot-reload): {e}")
        else:
            raise


def drop_tables():
    """Drop all tables in the database - Use with caution!"""
    Base.metadata.drop_all(bind=engine)
