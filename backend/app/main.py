"""
Main FastAPI application
Protein Docking Platform - Backend API
"""
from fastapi import FastAPI, Request, status
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse
from slowapi import _rate_limit_exceeded_handler
from slowapi.errors import RateLimitExceeded
from contextlib import asynccontextmanager
import time

from app.config import get_settings
from app.database import create_tables
from app.core.logging import (
    setup_logging,
    get_logger,
    set_request_context,
    clear_request_context,
    get_request_id
)
from app.core.rate_limit import limiter, user_limiter, RateLimitTier
from app.core.env_validation import startup_validation
from app.core.metrics import track_request_metrics, http_requests_in_progress
from app.api import api_router
from prometheus_client import make_asgi_app

# Load settings
settings = get_settings()

# Setup logging
setup_logging()
logger = get_logger(__name__)


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


# Create FastAPI application with enhanced documentation
app = FastAPI(
    title="Protein Docking Platform API",
    description="""
    ## 🧬 Protein Docking Platform - RESTful API

    ### Overview
    Advanced bioinformatics platform for protein-protein docking analysis and molecular modeling.

    ### Features
    * 🔐 **Secure Authentication** - JWT-based authentication with role-based access control
    * 📊 **Docking Analysis** - Submit and analyze protein docking simulations
    * 📁 **File Management** - Upload and manage PDB protein structure files
    * ⚡ **Real-time Processing** - Asynchronous job processing with Celery
    * 📈 **Metrics & Monitoring** - Prometheus metrics and health checks
    * 🔒 **Rate Limiting** - Granular rate limiting per endpoint type
    * 📝 **Request Tracing** - Full request tracing with unique request IDs

    ### Authentication
    Most endpoints require authentication. To authenticate:
    1. Register a new account via `/api/v1/auth/register`
    2. Login via `/api/v1/auth/login` to receive JWT tokens
    3. Use the `Bearer <access_token>` in the Authorization header
    4. Click the 🔓 Authorize button and enter your token

    ### Rate Limits
    - **Public endpoints**: 100 requests/hour
    - **Authentication**: 5 login attempts/minute
    - **File uploads**: 10 uploads/minute
    - **Docking jobs**: 20 jobs/hour
    - **Standard authenticated**: 1000 requests/hour

    ### Request Tracing
    All requests are assigned a unique `X-Request-ID` header for tracing.
    Include this ID when reporting issues.

    ### Health Checks
    - `/api/v1/health/liveness` - Pod liveness check
    - `/api/v1/health/readiness` - Readiness with dependency checks
    - `/api/v1/health/startup` - Startup validation

    ### Support
    For issues and questions, please refer to the documentation or contact support.
    """,
    version="2.1.0",
    docs_url="/docs" if settings.ENVIRONMENT != "production" else None,
    redoc_url="/redoc" if settings.ENVIRONMENT != "production" else None,
    openapi_tags=[
        {
            "name": "Root",
            "description": "Root endpoints and API information"
        },
        {
            "name": "Health",
            "description": "Health check endpoints for monitoring and Kubernetes"
        },
        {
            "name": "Auth",
            "description": "Authentication and authorization endpoints"
        },
        {
            "name": "Users",
            "description": "User management and profile operations"
        },
        {
            "name": "Docking",
            "description": "Protein docking job submission and analysis"
        },
        {
            "name": "Files",
            "description": "File upload and management operations"
        },
        {
            "name": "Metrics",
            "description": "Prometheus metrics endpoint"
        }
    ],
    contact={
        "name": "Protein Docking Platform Team",
        "url": "https://github.com/yeipills/protein-docking",
        "email": "support@protein-docking.example.com"
    },
    license_info={
        "name": "MIT License",
        "url": "https://opensource.org/licenses/MIT"
    },
    terms_of_service="https://protein-docking.example.com/terms",
    openapi_url="/api/v1/openapi.json" if settings.ENVIRONMENT != "production" else None,
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


# Request logging, metrics and tracing middleware
@app.middleware("http")
async def log_and_track_requests(request: Request, call_next):
    """Log all requests, track metrics and add request tracing"""
    start_time = time.time()
    method = request.method
    path = request.url.path

    # Get or generate request ID
    request_id = request.headers.get('X-Request-ID', None)

    # Get user ID if authenticated
    user_id = None
    if hasattr(request.state, 'user') and request.state.user:
        user_id = str(getattr(request.state.user, 'id', None))

    # Set request context for logging
    request_id = set_request_context(
        request_id=request_id,
        user_id=user_id,
        request_path=path,
        request_method=method
    )

    # Track in-progress requests
    http_requests_in_progress.labels(method=method, endpoint=path).inc()

    try:
        # Log request start
        logger.info("Request started")

        # Process request
        response = await call_next(request)

        # Add request ID to response headers
        response.headers["X-Request-ID"] = request_id

        # Calculate duration
        duration = time.time() - start_time

        # Log response
        logger.info(
            f"Request completed - "
            f"Status: {response.status_code} - "
            f"Duration: {duration:.3f}s"
        )

        # Track metrics (exclude /metrics endpoint from tracking itself)
        if path != "/metrics":
            track_request_metrics(method, path, response.status_code, duration)

        return response

    except Exception as e:
        duration = time.time() - start_time
        logger.error(
            f"Request failed - "
            f"Error: {str(e)} - "
            f"Duration: {duration:.3f}s",
            exc_info=True
        )
        raise

    finally:
        # Decrement in-progress counter
        http_requests_in_progress.labels(method=method, endpoint=path).dec()
        # Clear request context
        clear_request_context()


# Health check endpoint
@app.get(
    "/health",
    tags=["Health"],
    summary="Basic health check",
    description="Simple health check endpoint to verify the API is running",
    response_description="Health status information",
    responses={
        200: {
            "description": "Service is healthy and running",
            "content": {
                "application/json": {
                    "example": {
                        "status": "healthy",
                        "environment": "development",
                        "version": "2.1.0"
                    }
                }
            }
        }
    }
)
@limiter.limit(RateLimitTier.HEALTH)
async def health_check(request: Request):
    """
    Basic health check endpoint

    Returns the current status of the API service, including:
    - Service health status
    - Environment (development/staging/production)
    - API version

    This endpoint is rate-limited to 60 requests per minute.
    """
    return {
        "status": "healthy",
        "environment": settings.ENVIRONMENT,
        "version": "2.1.0"
    }


# Root endpoint
@app.get(
    "/",
    tags=["Root"],
    summary="API root information",
    description="Returns basic API information and available endpoints",
    response_description="API welcome message and documentation links",
    responses={
        200: {
            "description": "API information",
            "content": {
                "application/json": {
                    "example": {
                        "message": "Protein Docking Platform API",
                        "version": "2.1.0",
                        "docs": "/docs",
                        "redoc": "/redoc",
                        "health": "/health",
                        "api_base": "/api/v1"
                    }
                }
            }
        }
    }
)
@limiter.limit(RateLimitTier.PUBLIC)
async def root(request: Request):
    """
    API root endpoint

    Provides:
    - Welcome message
    - API version
    - Links to documentation (Swagger UI and ReDoc)
    - Health check endpoint
    - API base path

    This endpoint is publicly accessible with a rate limit of 100 requests per hour.
    """
    return {
        "message": "Protein Docking Platform API",
        "version": "2.1.0",
        "docs": "/docs",
        "redoc": "/redoc",
        "health": "/health",
        "api_base": "/api/v1"
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
