"""
API routes
"""
from fastapi import APIRouter
from app.api import auth, users, jobs, proteins, health, audit

# Create main API router
api_router = APIRouter()

# Include sub-routers
api_router.include_router(auth.router, prefix="/auth", tags=["Authentication"])
api_router.include_router(users.router, prefix="/users", tags=["Users"])
api_router.include_router(jobs.router, prefix="/jobs", tags=["Jobs"])
api_router.include_router(proteins.router, prefix="/proteins", tags=["Proteins"])
api_router.include_router(health.router, prefix="/health", tags=["Health"])
api_router.include_router(audit.router, prefix="/audit", tags=["Audit Logs"])

__all__ = ["api_router"]
