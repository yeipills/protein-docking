"""
Audit logging service for tracking critical operations

Provides functions and decorators for comprehensive audit logging.
"""
from typing import Optional, Dict, Any
from fastapi import Request
from sqlalchemy.orm import Session
from functools import wraps
import traceback
from datetime import datetime, timedelta

from app.models.audit_log import AuditLog, AuditAction, AuditSeverity
from app.core.logging import get_logger
from app.core.metrics import audit_events_total, audit_events_by_severity

logger = get_logger(__name__)


def create_audit_log(
    db: Session,
    action: str,
    user_id: Optional[int] = None,
    username: Optional[str] = None,
    resource_type: Optional[str] = None,
    resource_id: Optional[str] = None,
    severity: str = AuditSeverity.INFO,
    status: str = "success",
    description: Optional[str] = None,
    metadata: Optional[Dict[str, Any]] = None,
    changes: Optional[Dict[str, Any]] = None,
    error_message: Optional[str] = None,
    error_code: Optional[str] = None,
    ip_address: Optional[str] = None,
    user_agent: Optional[str] = None,
    request_id: Optional[str] = None,
) -> AuditLog:
    """
    Create an audit log entry.

    Args:
        db: Database session
        action: Action performed (use AuditAction enum)
        user_id: ID of user performing action
        username: Username of user
        resource_type: Type of resource affected ('protein', 'job', etc.)
        resource_id: ID of affected resource
        severity: Severity level (use AuditSeverity enum)
        status: Operation status ('success', 'failure', 'partial')
        description: Human-readable description
        metadata: Additional structured data
        changes: Before/after values for updates
        error_message: Error message if failed
        error_code: Error code if failed
        ip_address: Client IP address
        user_agent: Client user agent
        request_id: Request ID for correlation

    Returns:
        AuditLog: Created audit log entry

    Example:
        create_audit_log(
            db=db,
            action=AuditAction.FILE_UPLOAD,
            user_id=user.id,
            username=user.username,
            resource_type="protein",
            resource_id=str(protein.id),
            description=f"Uploaded STL file for protein {protein.name}",
            metadata={"file_size": 12345, "file_type": "stl"}
        )
    """
    try:
        audit_log = AuditLog(
            user_id=user_id,
            username=username,
            action=action,
            resource_type=resource_type,
            resource_id=resource_id,
            severity=severity,
            status=status,
            description=description,
            metadata=metadata,
            changes=changes,
            error_message=error_message,
            error_code=error_code,
            ip_address=ip_address,
            user_agent=user_agent,
            request_id=request_id,
        )

        db.add(audit_log)
        db.commit()
        db.refresh(audit_log)

        # Update Prometheus metrics
        audit_events_total.labels(action=action, status=status).inc()
        audit_events_by_severity.labels(severity=severity).inc()

        logger.debug(
            f"Audit log created: {action} by user {username} ({user_id}) - {status}"
        )

        return audit_log

    except Exception as e:
        logger.error(f"Failed to create audit log: {e}")
        # Don't fail the main operation if audit logging fails
        db.rollback()
        return None


def log_from_request(
    db: Session,
    request: Request,
    action: str,
    user_id: Optional[int] = None,
    username: Optional[str] = None,
    **kwargs
) -> AuditLog:
    """
    Create audit log from FastAPI Request object.

    Automatically extracts IP, user agent, and request ID from request.

    Args:
        db: Database session
        request: FastAPI Request object
        action: Action performed
        user_id: User ID
        username: Username
        **kwargs: Additional arguments for create_audit_log

    Returns:
        AuditLog: Created audit log entry
    """
    # Extract client IP
    ip_address = request.client.host if request.client else None

    # X-Forwarded-For header for proxied requests
    if "x-forwarded-for" in request.headers:
        ip_address = request.headers["x-forwarded-for"].split(",")[0].strip()

    # Extract user agent
    user_agent = request.headers.get("user-agent")

    # Extract or generate request ID
    request_id = request.headers.get("x-request-id") or request.state.__dict__.get("request_id")

    return create_audit_log(
        db=db,
        action=action,
        user_id=user_id,
        username=username,
        ip_address=ip_address,
        user_agent=user_agent,
        request_id=request_id,
        **kwargs
    )


def audit(
    action: str,
    resource_type: Optional[str] = None,
    severity: str = AuditSeverity.INFO,
    include_args: bool = False,
):
    """
    Decorator for automatic audit logging of function calls.

    Args:
        action: Audit action to log
        resource_type: Type of resource
        severity: Severity level
        include_args: Whether to include function args in metadata

    Usage:
        @audit(action=AuditAction.PROTEIN_CREATE, resource_type="protein")
        async def create_protein(protein_data, user, db):
            # ... create protein ...
            return protein
    """
    def decorator(func):
        @wraps(func)
        async def async_wrapper(*args, **kwargs):
            db = kwargs.get('db')
            user = kwargs.get('current_user')
            request = kwargs.get('request')

            user_id = user.id if user else None
            username = user.username if user else None

            metadata = {}
            if include_args:
                # Sanitize args - don't log passwords, tokens, etc.
                safe_kwargs = {
                    k: v for k, v in kwargs.items()
                    if k not in ['password', 'token', 'secret', 'db', 'request', 'current_user']
                }
                metadata["function_args"] = safe_kwargs

            try:
                result = await func(*args, **kwargs)

                # Log success
                resource_id = None
                if hasattr(result, 'id'):
                    resource_id = str(result.id)

                if db:
                    if request:
                        log_from_request(
                            db=db,
                            request=request,
                            action=action,
                            user_id=user_id,
                            username=username,
                            resource_type=resource_type,
                            resource_id=resource_id,
                            severity=severity,
                            status="success",
                            description=f"{func.__name__} completed successfully",
                            metadata=metadata,
                        )
                    else:
                        create_audit_log(
                            db=db,
                            action=action,
                            user_id=user_id,
                            username=username,
                            resource_type=resource_type,
                            resource_id=resource_id,
                            severity=severity,
                            status="success",
                            description=f"{func.__name__} completed successfully",
                            metadata=metadata,
                        )

                return result

            except Exception as e:
                # Log failure
                if db:
                    error_msg = str(e)
                    error_trace = traceback.format_exc()

                    if request:
                        log_from_request(
                            db=db,
                            request=request,
                            action=action,
                            user_id=user_id,
                            username=username,
                            resource_type=resource_type,
                            severity=AuditSeverity.ERROR,
                            status="failure",
                            description=f"{func.__name__} failed",
                            error_message=error_msg,
                            metadata={**metadata, "traceback": error_trace},
                        )
                    else:
                        create_audit_log(
                            db=db,
                            action=action,
                            user_id=user_id,
                            username=username,
                            resource_type=resource_type,
                            severity=AuditSeverity.ERROR,
                            status="failure",
                            description=f"{func.__name__} failed",
                            error_message=error_msg,
                            metadata={**metadata, "traceback": error_trace},
                        )

                raise

        @wraps(func)
        def sync_wrapper(*args, **kwargs):
            # Similar implementation for sync functions
            # (Simplified - in production would mirror async logic)
            return func(*args, **kwargs)

        # Return appropriate wrapper
        import inspect
        if inspect.iscoroutinefunction(func):
            return async_wrapper
        return sync_wrapper

    return decorator


def cleanup_old_audit_logs(db: Session, days: int = 90) -> int:
    """
    Delete audit logs older than specified days.

    Args:
        db: Database session
        days: Number of days to retain (default: 90)

    Returns:
        int: Number of logs deleted

    Example:
        # Delete logs older than 90 days
        deleted = cleanup_old_audit_logs(db, days=90)
    """
    try:
        cutoff_date = datetime.utcnow() - timedelta(days=days)

        deleted = db.query(AuditLog).filter(
            AuditLog.timestamp < cutoff_date
        ).delete()

        db.commit()

        logger.info(f"Cleaned up {deleted} audit logs older than {days} days")
        return deleted

    except Exception as e:
        logger.error(f"Failed to cleanup audit logs: {e}")
        db.rollback()
        return 0


def get_user_audit_history(
    db: Session,
    user_id: int,
    limit: int = 100,
    action: Optional[str] = None,
    severity: Optional[str] = None
) -> list[AuditLog]:
    """
    Get audit history for a specific user.

    Args:
        db: Database session
        user_id: User ID to query
        limit: Maximum number of logs to return
        action: Filter by specific action
        severity: Filter by severity level

    Returns:
        list: List of AuditLog entries
    """
    query = db.query(AuditLog).filter(AuditLog.user_id == user_id)

    if action:
        query = query.filter(AuditLog.action == action)

    if severity:
        query = query.filter(AuditLog.severity == severity)

    return query.order_by(AuditLog.timestamp.desc()).limit(limit).all()


def get_resource_audit_history(
    db: Session,
    resource_type: str,
    resource_id: str,
    limit: int = 100
) -> list[AuditLog]:
    """
    Get audit history for a specific resource.

    Args:
        db: Database session
        resource_type: Type of resource ('protein', 'job', etc.)
        resource_id: ID of resource
        limit: Maximum number of logs to return

    Returns:
        list: List of AuditLog entries
    """
    return db.query(AuditLog).filter(
        AuditLog.resource_type == resource_type,
        AuditLog.resource_id == resource_id
    ).order_by(AuditLog.timestamp.desc()).limit(limit).all()


def get_security_events(
    db: Session,
    hours: int = 24,
    severity: str = AuditSeverity.WARNING
) -> list[AuditLog]:
    """
    Get recent security events (warnings and above).

    Args:
        db: Database session
        hours: Number of hours to look back
        severity: Minimum severity level

    Returns:
        list: List of security-related audit logs
    """
    cutoff_time = datetime.utcnow() - timedelta(hours=hours)

    severity_order = {
        AuditSeverity.INFO: 0,
        AuditSeverity.WARNING: 1,
        AuditSeverity.ERROR: 2,
        AuditSeverity.CRITICAL: 3,
    }

    min_severity_level = severity_order.get(severity, 1)

    return db.query(AuditLog).filter(
        AuditLog.timestamp >= cutoff_time,
        AuditLog.severity.in_([
            s for s, level in severity_order.items()
            if level >= min_severity_level
        ])
    ).order_by(AuditLog.timestamp.desc()).all()
