"""
Enhanced health check endpoints
Provides detailed health status of application and dependencies
"""
from fastapi import APIRouter, status
from fastapi.responses import JSONResponse
from sqlalchemy import text
import redis

from app.database import SessionLocal
from app.config import get_settings

router = APIRouter()
settings = get_settings()


@router.get("/liveness")
async def liveness():
    """
    Liveness probe - checks if the application is running.

    This endpoint should return 200 if the application process is alive.
    Kubernetes/Docker uses this to know if the container should be restarted.

    Returns:
        dict: Simple status message
    """
    return {"status": "alive", "service": "protein-docking-api"}


@router.get("/readiness")
async def readiness():
    """
    Readiness probe - checks if the application is ready to serve traffic.

    This endpoint checks all critical dependencies:
    - Database connection
    - Redis connection
    - Celery workers availability

    Returns:
        JSONResponse: Detailed health status with 200 (healthy) or 503 (unhealthy)
    """
    checks = {}
    all_healthy = True

    # ==========================================
    # DATABASE CHECK
    # ==========================================
    try:
        db = SessionLocal()
        db.execute(text("SELECT 1"))
        db.close()
        checks["database"] = {"status": "healthy", "details": "PostgreSQL connection OK"}
    except Exception as e:
        checks["database"] = {
            "status": "unhealthy",
            "error": str(e),
            "details": "Cannot connect to PostgreSQL",
        }
        all_healthy = False

    # ==========================================
    # REDIS CHECK
    # ==========================================
    try:
        r = redis.from_url(settings.REDIS_URL, socket_connect_timeout=2)
        r.ping()
        checks["redis"] = {"status": "healthy", "details": "Redis connection OK"}
    except Exception as e:
        checks["redis"] = {
            "status": "unhealthy",
            "error": str(e),
            "details": "Cannot connect to Redis",
        }
        all_healthy = False

    # ==========================================
    # CELERY WORKERS CHECK
    # ==========================================
    try:
        from app.tasks.celery_app import celery_app

        inspect = celery_app.control.inspect(timeout=2.0)
        active_workers = inspect.active()

        if active_workers and len(active_workers) > 0:
            worker_count = len(active_workers)
            checks["celery"] = {
                "status": "healthy",
                "details": f"{worker_count} worker(s) active",
                "workers": list(active_workers.keys()),
            }
        else:
            checks["celery"] = {
                "status": "degraded",
                "details": "No active workers found",
            }
            # Don't mark as unhealthy, just degraded
    except Exception as e:
        checks["celery"] = {
            "status": "degraded",
            "error": str(e),
            "details": "Cannot check Celery workers",
        }
        # Don't mark as unhealthy, just degraded

    # ==========================================
    # FILE SYSTEM CHECKS
    # ==========================================
    try:
        from pathlib import Path

        upload_dir = Path(settings.UPLOAD_DIR)
        results_dir = Path(settings.RESULTS_DIR)

        upload_writable = upload_dir.exists() and upload_dir.is_dir()
        results_writable = results_dir.exists() and results_dir.is_dir()

        if upload_writable and results_writable:
            checks["filesystem"] = {
                "status": "healthy",
                "details": "Upload and results directories accessible",
            }
        else:
            checks["filesystem"] = {
                "status": "degraded",
                "details": f"Upload: {upload_writable}, Results: {results_writable}",
            }
    except Exception as e:
        checks["filesystem"] = {
            "status": "degraded",
            "error": str(e),
            "details": "Cannot check filesystem",
        }

    # ==========================================
    # RESPONSE
    # ==========================================
    status_code = status.HTTP_200_OK if all_healthy else status.HTTP_503_SERVICE_UNAVAILABLE

    response_data = {
        "status": "healthy" if all_healthy else "unhealthy",
        "service": "protein-docking-api",
        "version": "2.1.0",
        "environment": settings.ENVIRONMENT,
        "checks": checks,
    }

    return JSONResponse(status_code=status_code, content=response_data)


@router.get("/startup")
async def startup():
    """
    Startup probe - checks if the application has finished starting up.

    This is useful for slow-starting applications.
    Kubernetes uses this to know when to start liveness/readiness checks.

    Returns:
        dict: Startup status
    """
    # You can add initialization checks here
    # For example, check if database migrations are complete

    checks = {}

    # Check database is accessible
    try:
        db = SessionLocal()
        db.execute(text("SELECT 1"))
        db.close()
        checks["database_initialized"] = True
    except Exception:
        checks["database_initialized"] = False
        return JSONResponse(
            status_code=status.HTTP_503_SERVICE_UNAVAILABLE,
            content={"status": "starting", "checks": checks},
        )

    return {
        "status": "started",
        "service": "protein-docking-api",
        "checks": checks,
    }


@router.get("/cache")
async def cache_stats():
    """
    Cache statistics endpoint - provides detailed Redis cache metrics.

    Returns cache usage, hit rate, and other relevant metrics.
    Useful for monitoring and debugging caching behavior.

    Returns:
        dict: Cache statistics
    """
    from app.core.cache import get_cache_stats

    stats = get_cache_stats()

    return {
        "service": "protein-docking-api",
        "cache": stats
    }
