"""
Job service layer with caching support

This module demonstrates how to use Redis caching with business logic.
Functions here can be cached and invalidated as needed.
"""
from typing import Dict, List, Optional
from sqlalchemy.orm import Session

from app.models.job import Job, JobStatus
from app.core.cache import cache, invalidate_pattern, CacheTTL
from app.core.logging import get_logger

logger = get_logger(__name__)


@cache(ttl=CacheTTL.SHORT, prefix="job:stats")
def get_user_job_stats(user_id: int, db: Session) -> Dict:
    """
    Get aggregated statistics for a user's jobs.

    Cached for 5 minutes. This is an example of how to cache
    expensive aggregation queries.

    Args:
        user_id: User ID
        db: Database session

    Returns:
        dict: Job statistics including counts by status
    """
    total_jobs = db.query(Job).filter(Job.user_id == user_id).count()

    stats = {
        "total": total_jobs,
        "by_status": {}
    }

    for status in JobStatus:
        count = db.query(Job).filter(
            Job.user_id == user_id,
            Job.status == status
        ).count()
        stats["by_status"][status.value] = count

    logger.debug(f"Computed job stats for user {user_id}: {stats}")
    return stats


@cache(ttl=CacheTTL.VERY_SHORT, prefix="job:recent")
def get_recent_completed_jobs(user_id: int, limit: int, db: Session) -> List[Dict]:
    """
    Get recent completed jobs for a user.

    Cached for 1 minute since job status changes frequently.

    Args:
        user_id: User ID
        limit: Maximum number of jobs to return
        db: Database session

    Returns:
        list: List of job dictionaries
    """
    jobs = db.query(Job).filter(
        Job.user_id == user_id,
        Job.status == JobStatus.COMPLETED
    ).order_by(Job.completed_at.desc()).limit(limit).all()

    result = []
    for job in jobs:
        result.append({
            "id": job.id,
            "protein_id": job.protein_id,
            "job_type": job.job_type.value,
            "completed_at": job.completed_at.isoformat() if job.completed_at else None,
        })

    return result


def invalidate_user_job_cache(user_id: int):
    """
    Invalidate all job-related caches for a user.

    Call this when a user's jobs are created, updated, or deleted.

    Args:
        user_id: User ID to invalidate caches for
    """
    patterns = [
        f"cache:app.services.job_service.get_user_job_stats:*{user_id}*",
        f"cache:app.services.job_service.get_recent_completed_jobs:*{user_id}*",
    ]

    for pattern in patterns:
        count = invalidate_pattern(pattern)
        if count > 0:
            logger.info(f"Invalidated {count} cache entries for user {user_id}")


def invalidate_job_cache(job_id: int, user_id: int):
    """
    Invalidate cache for a specific job.

    Call this when a job is updated or deleted.

    Args:
        job_id: Job ID
        user_id: User ID that owns the job
    """
    # Invalidate user-level caches
    invalidate_user_job_cache(user_id)

    logger.info(f"Invalidated cache for job {job_id}")
