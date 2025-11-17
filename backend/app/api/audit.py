"""
Audit log query endpoints

Provides API for querying and searching audit logs.
Restricted to admin users only.
"""
from fastapi import APIRouter, Depends, Query, status
from fastapi.responses import JSONResponse
from sqlalchemy.orm import Session
from sqlalchemy import and_, or_, desc
from typing import List, Optional
from datetime import datetime, timedelta

from app.database import get_db
from app.models.user import User
from app.models.audit_log import AuditLog, AuditAction, AuditSeverity
from app.dependencies import get_current_user
from app.core.exceptions import ForbiddenException
from app.core.audit import get_user_audit_history, get_resource_audit_history, get_security_events
from app.core.logging import get_logger

logger = get_logger(__name__)
router = APIRouter()


def require_admin(current_user: User = Depends(get_current_user)) -> User:
    """Dependency to require admin access"""
    if not current_user.is_superuser:
        raise ForbiddenException(detail="Admin access required")
    return current_user


@router.get("/")
async def query_audit_logs(
    skip: int = Query(0, ge=0),
    limit: int = Query(100, ge=1, le=1000),
    user_id: Optional[int] = Query(None),
    action: Optional[str] = Query(None),
    resource_type: Optional[str] = Query(None),
    resource_id: Optional[str] = Query(None),
    severity: Optional[str] = Query(None),
    status: Optional[str] = Query(None),
    start_date: Optional[datetime] = Query(None),
    end_date: Optional[datetime] = Query(None),
    current_user: User = Depends(require_admin),
    db: Session = Depends(get_db)
):
    """
    Query audit logs with filtering.

    Requires admin/superuser access.

    Parameters:
    - skip: Number of records to skip
    - limit: Maximum number of records to return
    - user_id: Filter by user ID
    - action: Filter by action (e.g., 'auth.login.success')
    - resource_type: Filter by resource type (e.g., 'protein', 'job')
    - resource_id: Filter by resource ID
    - severity: Filter by severity (info, warning, error, critical)
    - status: Filter by status (success, failure, partial)
    - start_date: Filter logs after this date (ISO format)
    - end_date: Filter logs before this date (ISO format)

    Returns:
    - total: Total number of matching records
    - logs: List of audit log entries
    """
    query = db.query(AuditLog)

    # Apply filters
    filters = []

    if user_id is not None:
        filters.append(AuditLog.user_id == user_id)

    if action:
        filters.append(AuditLog.action == action)

    if resource_type:
        filters.append(AuditLog.resource_type == resource_type)

    if resource_id:
        filters.append(AuditLog.resource_id == resource_id)

    if severity:
        filters.append(AuditLog.severity == severity)

    if status:
        filters.append(AuditLog.status == status)

    if start_date:
        filters.append(AuditLog.timestamp >= start_date)

    if end_date:
        filters.append(AuditLog.timestamp <= end_date)

    if filters:
        query = query.filter(and_(*filters))

    # Get total count
    total = query.count()

    # Get paginated results
    logs = query.order_by(desc(AuditLog.timestamp)).offset(skip).limit(limit).all()

    return {
        "total": total,
        "skip": skip,
        "limit": limit,
        "logs": [log.to_dict() for log in logs]
    }


@router.get("/user/{user_id}")
async def get_user_logs(
    user_id: int,
    limit: int = Query(100, ge=1, le=1000),
    action: Optional[str] = Query(None),
    severity: Optional[str] = Query(None),
    current_user: User = Depends(require_admin),
    db: Session = Depends(get_db)
):
    """
    Get audit logs for a specific user.

    Requires admin access.
    """
    logs = get_user_audit_history(
        db=db,
        user_id=user_id,
        limit=limit,
        action=action,
        severity=severity
    )

    return {
        "user_id": user_id,
        "total": len(logs),
        "logs": [log.to_dict() for log in logs]
    }


@router.get("/resource/{resource_type}/{resource_id}")
async def get_resource_logs(
    resource_type: str,
    resource_id: str,
    limit: int = Query(100, ge=1, le=1000),
    current_user: User = Depends(require_admin),
    db: Session = Depends(get_db)
):
    """
    Get audit logs for a specific resource.

    Examples:
    - GET /audit/resource/protein/123
    - GET /audit/resource/job/456

    Requires admin access.
    """
    logs = get_resource_audit_history(
        db=db,
        resource_type=resource_type,
        resource_id=resource_id,
        limit=limit
    )

    return {
        "resource_type": resource_type,
        "resource_id": resource_id,
        "total": len(logs),
        "logs": [log.to_dict() for log in logs]
    }


@router.get("/security")
async def get_security_alerts(
    hours: int = Query(24, ge=1, le=168),
    severity: str = Query(AuditSeverity.WARNING),
    current_user: User = Depends(require_admin),
    db: Session = Depends(get_db)
):
    """
    Get recent security events (warnings and above).

    Parameters:
    - hours: Number of hours to look back (default: 24, max: 168)
    - severity: Minimum severity (warning, error, critical)

    Returns security-related audit events including:
    - Failed login attempts
    - Permission violations
    - Suspicious activity
    - Errors

    Requires admin access.
    """
    logs = get_security_events(
        db=db,
        hours=hours,
        severity=severity
    )

    return {
        "period_hours": hours,
        "minimum_severity": severity,
        "total": len(logs),
        "logs": [log.to_dict() for log in logs]
    }


@router.get("/actions")
async def list_actions(
    current_user: User = Depends(require_admin),
):
    """
    List all available audit actions.

    Returns all action types that can be filtered on.

    Requires admin access.
    """
    actions = [
        {"value": action.value, "description": action.value.replace(".", " ").title()}
        for action in AuditAction
    ]

    return {
        "total": len(actions),
        "actions": actions
    }


@router.get("/stats")
async def get_audit_stats(
    days: int = Query(7, ge=1, le=90),
    current_user: User = Depends(require_admin),
    db: Session = Depends(get_db)
):
    """
    Get audit log statistics.

    Parameters:
    - days: Number of days to include in stats (default: 7, max: 90)

    Returns aggregated statistics:
    - Total events
    - Events by action
    - Events by severity
    - Events by status
    - Top users by activity
    - Recent failed operations

    Requires admin access.
    """
    cutoff_date = datetime.utcnow() - timedelta(days=days)

    # Get all logs in period
    logs = db.query(AuditLog).filter(AuditLog.timestamp >= cutoff_date).all()

    # Calculate stats
    total_events = len(logs)

    # Events by action
    action_counts = {}
    for log in logs:
        action_counts[log.action] = action_counts.get(log.action, 0) + 1

    # Events by severity
    severity_counts = {}
    for log in logs:
        severity_counts[log.severity] = severity_counts.get(log.severity, 0) + 1

    # Events by status
    status_counts = {}
    for log in logs:
        status_counts[log.status] = status_counts.get(log.status, 0) + 1

    # Top users
    user_counts = {}
    for log in logs:
        if log.user_id:
            key = f"{log.username} (ID: {log.user_id})"
            user_counts[key] = user_counts.get(key, 0) + 1

    top_users = sorted(user_counts.items(), key=lambda x: x[1], reverse=True)[:10]

    # Recent failures
    recent_failures = db.query(AuditLog).filter(
        and_(
            AuditLog.timestamp >= cutoff_date,
            AuditLog.status == "failure"
        )
    ).order_by(desc(AuditLog.timestamp)).limit(10).all()

    return {
        "period_days": days,
        "start_date": cutoff_date.isoformat(),
        "end_date": datetime.utcnow().isoformat(),
        "total_events": total_events,
        "events_by_action": action_counts,
        "events_by_severity": severity_counts,
        "events_by_status": status_counts,
        "top_users": [{"user": user, "count": count} for user, count in top_users],
        "recent_failures": [log.to_dict() for log in recent_failures],
    }


@router.get("/my-history")
async def get_my_history(
    limit: int = Query(50, ge=1, le=500),
    current_user: User = Depends(get_current_user),
    db: Session = Depends(get_db)
):
    """
    Get audit history for the current user.

    Any authenticated user can view their own audit history.
    """
    logs = get_user_audit_history(
        db=db,
        user_id=current_user.id,
        limit=limit
    )

    return {
        "user_id": current_user.id,
        "username": current_user.username,
        "total": len(logs),
        "logs": [log.to_dict() for log in logs]
    }


@router.delete("/cleanup")
async def cleanup_old_logs(
    days: int = Query(90, ge=30, le=365),
    dry_run: bool = Query(True),
    current_user: User = Depends(require_admin),
    db: Session = Depends(get_db)
):
    """
    Delete audit logs older than specified days.

    Parameters:
    - days: Delete logs older than this many days (default: 90, min: 30, max: 365)
    - dry_run: If true, only count logs without deleting (default: true)

    Requires admin access.

    Safety feature: dry_run is true by default to prevent accidental deletion.
    """
    cutoff_date = datetime.utcnow() - timedelta(days=days)

    # Count logs to be deleted
    count = db.query(AuditLog).filter(AuditLog.timestamp < cutoff_date).count()

    if dry_run:
        return {
            "dry_run": True,
            "would_delete": count,
            "cutoff_date": cutoff_date.isoformat(),
            "message": f"Would delete {count} logs older than {days} days. Set dry_run=false to actually delete."
        }

    # Actually delete
    deleted = db.query(AuditLog).filter(AuditLog.timestamp < cutoff_date).delete()
    db.commit()

    logger.info(f"Admin {current_user.username} deleted {deleted} audit logs older than {days} days")

    return {
        "dry_run": False,
        "deleted": deleted,
        "cutoff_date": cutoff_date.isoformat(),
        "message": f"Successfully deleted {deleted} audit logs."
    }
