"""
Job management endpoints
"""
from fastapi import APIRouter, Depends, status, Query
from sqlalchemy.orm import Session
from typing import List
from app.database import get_db
from app.models.user import User
from app.models.job import Job, JobStatus
from app.schemas.job import JobResponse, JobListResponse
from app.dependencies import get_current_user
from app.core.exceptions import NotFoundException, ForbiddenException
from app.core.logging import get_logger

logger = get_logger(__name__)
router = APIRouter()


@router.get("/", response_model=JobListResponse)
async def list_jobs(
    skip: int = Query(0, ge=0),
    limit: int = Query(50, ge=1, le=100),
    status: JobStatus = None,
    current_user: User = Depends(get_current_user),
    db: Session = Depends(get_db)
):
    """List user's jobs with pagination"""
    query = db.query(Job).filter(Job.user_id == current_user.id)

    if status:
        query = query.filter(Job.status == status)

    total = query.count()
    jobs = query.order_by(Job.created_at.desc()).offset(skip).limit(limit).all()

    return {"total": total, "jobs": jobs}


@router.get("/{job_id}", response_model=JobResponse)
async def get_job(
    job_id: int,
    current_user: User = Depends(get_current_user),
    db: Session = Depends(get_db)
):
    """Get job details"""
    job = db.query(Job).filter(Job.id == job_id).first()

    if not job:
        raise NotFoundException(detail="Job not found")

    if job.user_id != current_user.id and not current_user.is_superuser:
        raise ForbiddenException(detail="Not authorized to access this job")

    return job


@router.delete("/{job_id}", status_code=status.HTTP_204_NO_CONTENT)
async def cancel_job(
    job_id: int,
    current_user: User = Depends(get_current_user),
    db: Session = Depends(get_db)
):
    """Cancel a job"""
    job = db.query(Job).filter(Job.id == job_id).first()

    if not job:
        raise NotFoundException(detail="Job not found")

    if job.user_id != current_user.id:
        raise ForbiddenException(detail="Not authorized to cancel this job")

    if job.status in [JobStatus.COMPLETED, JobStatus.FAILED, JobStatus.CANCELLED]:
        raise ForbiddenException(detail="Cannot cancel completed/failed job")

    job.status = JobStatus.CANCELLED
    db.commit()

    logger.info(f"Job cancelled: {job_id} by user {current_user.username}")
    return None
