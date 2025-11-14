"""
Main FastAPI application
Protein Docking Platform - Backend API
"""
from fastapi import FastAPI, Request, status
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse
from slowapi import Limiter, _rate_limit_exceeded_handler
from slowapi.util import get_remote_address
from slowapi.errors import RateLimitExceeded
from contextlib import asynccontextmanager
import time

from app.config import get_settings
from app.database import create_tables
from app.core.logging import setup_logging, get_logger
from app.core.env_validation import startup_validation
from app.core.metrics import track_request_metrics, http_requests_in_progress
from app.api import api_router
from prometheus_client import make_asgi_app

# Load settings
settings = get_settings()

# Setup logging
setup_logging()
logger = get_logger(__name__)

# Initialize rate limiter
limiter = Limiter(key_func=get_remote_address)


@asynccontextmanager
async def lifespan(app: FastAPI):
    """Lifespan events for the application"""
    # Startup
    logger.info("Starting Protein Docking Platform API")
    logger.info(f"Environment: {settings.ENVIRONMENT}")

    # Validate environment configuration
    startup_validation()

    # Create database tables
    logger.info("Creating database tables...")
    create_tables()
    logger.info("Database tables created successfully")

    yield

    # Shutdown
    logger.info("Shutting down Protein Docking Platform API")


# Create FastAPI application
app = FastAPI(
    title="Protein Docking Platform API",
    description="RESTful API for protein docking analysis and processing",
    version="2.0.0",
    docs_url="/docs" if settings.ENVIRONMENT != "production" else None,
    redoc_url="/redoc" if settings.ENVIRONMENT != "production" else None,
    lifespan=lifespan
)

# Add rate limiter
app.state.limiter = limiter
app.add_exception_handler(RateLimitExceeded, _rate_limit_exceeded_handler)

# Configure CORS
app.add_middleware(
    CORSMiddleware,
    allow_origins=settings.ALLOWED_ORIGINS_LIST,
    allow_credentials=settings.CORS_ALLOW_CREDENTIALS,
    allow_methods=["*"],
    allow_headers=["*"],
)


# Request logging and metrics middleware
@app.middleware("http")
async def log_and_track_requests(request: Request, call_next):
    """Log all requests and track metrics"""
    start_time = time.time()
    method = request.method
    path = request.url.path

    # Track in-progress requests
    http_requests_in_progress.labels(method=method, endpoint=path).inc()

    try:
        # Log request
        logger.info(f"Request: {method} {path}")

        # Process request
        response = await call_next(request)

        # Calculate duration
        duration = time.time() - start_time

        # Log response
        logger.info(
            f"Response: {response.status_code} - "
            f"Time: {duration:.3f}s - "
            f"Path: {path}"
        )

        # Track metrics (exclude /metrics endpoint from tracking itself)
        if path != "/metrics":
            track_request_metrics(method, path, response.status_code, duration)

        return response

    finally:
        # Decrement in-progress counter
        http_requests_in_progress.labels(method=method, endpoint=path).dec()


# Health check endpoint
@app.get("/health", tags=["Health"])
async def health_check():
    """Health check endpoint"""
    return {
        "status": "healthy",
        "environment": settings.ENVIRONMENT,
        "version": "2.0.0"
    }


# Root endpoint
@app.get("/", tags=["Root"])
async def root():
    """Root endpoint"""
    return {
        "message": "Protein Docking Platform API",
        "version": "2.0.0",
        "docs": "/docs",
        "health": "/health"
    }


# Include API routes
app.include_router(api_router, prefix="/api/v1")

# Prometheus metrics endpoint
metrics_app = make_asgi_app()
app.mount("/metrics", metrics_app)


# Global exception handler
@app.exception_handler(Exception)
async def global_exception_handler(request: Request, exc: Exception):
    """Handle uncaught exceptions"""
    logger.error(f"Unhandled exception: {str(exc)}", exc_info=True)

    return JSONResponse(
        status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
        content={
            "detail": "Internal server error",
            "message": str(exc) if settings.ENVIRONMENT == "development" else "An error occurred"
        }
    )


if __name__ == "__main__":
    import uvicorn

    uvicorn.run(
        "app.main:app",
        host=settings.BACKEND_HOST,
        port=settings.BACKEND_PORT,
        reload=settings.BACKEND_RELOAD,
        log_level=settings.LOG_LEVEL.lower()
    )
