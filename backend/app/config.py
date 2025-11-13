"""
Configuration management for Protein Docking Platform
Centralizes all environment variables and settings
"""
from pydantic_settings import BaseSettings
from typing import List
from functools import lru_cache


class Settings(BaseSettings):
    """Application settings loaded from environment variables"""

    # Environment
    ENVIRONMENT: str = "development"

    # Database
    POSTGRES_USER: str
    POSTGRES_PASSWORD: str
    POSTGRES_DB: str
    POSTGRES_HOST: str = "postgres"
    POSTGRES_PORT: int = 5432

    @property
    def DATABASE_URL(self) -> str:
        return (
            f"postgresql://{self.POSTGRES_USER}:{self.POSTGRES_PASSWORD}"
            f"@{self.POSTGRES_HOST}:{self.POSTGRES_PORT}/{self.POSTGRES_DB}"
        )

    # Redis & Celery
    REDIS_HOST: str = "redis"
    REDIS_PORT: int = 6379
    REDIS_DB: int = 0

    @property
    def REDIS_URL(self) -> str:
        return f"redis://{self.REDIS_HOST}:{self.REDIS_PORT}/{self.REDIS_DB}"

    @property
    def CELERY_BROKER_URL(self) -> str:
        return self.REDIS_URL

    @property
    def CELERY_RESULT_BACKEND(self) -> str:
        return self.REDIS_URL

    # Backend API
    BACKEND_HOST: str = "0.0.0.0"
    BACKEND_PORT: int = 5000
    BACKEND_WORKERS: int = 4
    BACKEND_RELOAD: bool = True

    # Socket Server
    SOCKET_HOST: str = "0.0.0.0"
    SOCKET_PORT: int = 8080
    SOCKET_SECRET_KEY: str

    # JWT Authentication
    JWT_SECRET_KEY: str
    JWT_ALGORITHM: str = "HS256"
    JWT_ACCESS_TOKEN_EXPIRE_MINUTES: int = 60
    JWT_REFRESH_TOKEN_EXPIRE_DAYS: int = 7

    # Security
    ALLOWED_ORIGINS: str = "http://localhost:3000,http://localhost:80"
    CORS_ALLOW_CREDENTIALS: bool = True
    SECRET_KEY: str

    @property
    def ALLOWED_ORIGINS_LIST(self) -> List[str]:
        return [origin.strip() for origin in self.ALLOWED_ORIGINS.split(",")]

    # Rate Limiting
    RATE_LIMIT_PER_MINUTE: int = 60
    RATE_LIMIT_PER_HOUR: int = 1000
    MAX_CONCURRENT_JOBS_PER_USER: int = 3

    # File Upload
    MAX_FILE_SIZE_MB: int = 100
    MAX_FILES_PER_UPLOAD: int = 10
    ALLOWED_FILE_EXTENSIONS: str = ".stl,.vert,.face,.txt"

    @property
    def MAX_FILE_SIZE_BYTES(self) -> int:
        return self.MAX_FILE_SIZE_MB * 1024 * 1024

    @property
    def ALLOWED_FILE_EXTENSIONS_LIST(self) -> List[str]:
        return [ext.strip() for ext in self.ALLOWED_FILE_EXTENSIONS.split(",")]

    # Protein Processing
    PROCESSING_TIMEOUT_SECONDS: int = 3600
    CLEANUP_OLD_FILES_DAYS: int = 7
    CONTEXT_RAYS_RADIUS: int = 3
    CONTEXT_RAYS_DELTA: int = 10

    # Logging
    LOG_LEVEL: str = "INFO"
    LOG_FORMAT: str = "json"
    LOG_FILE: str = "/app/logs/app.log"

    # Paths
    UPLOAD_DIR: str = "/app/uploads"
    RESULTS_DIR: str = "/app/results"

    class Config:
        env_file = ".env"
        case_sensitive = True


@lru_cache()
def get_settings() -> Settings:
    """
    Get cached settings instance
    Using lru_cache ensures we only create one instance
    """
    return Settings()
