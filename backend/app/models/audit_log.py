"""
Audit Log model for tracking critical operations

Stores comprehensive audit trail for compliance and security monitoring.
"""
from sqlalchemy import Column, Integer, String, Text, DateTime, JSON, Index
from sqlalchemy.sql import func
from datetime import datetime
from enum import Enum

from app.database import Base


class AuditAction(str, Enum):
    """Enumeration of auditable actions"""

    # Authentication
    LOGIN_SUCCESS = "auth.login.success"
    LOGIN_FAILURE = "auth.login.failure"
    LOGOUT = "auth.logout"
    TOKEN_REFRESH = "auth.token.refresh"

    # User Management
    USER_CREATE = "user.create"
    USER_UPDATE = "user.update"
    USER_DELETE = "user.delete"
    PASSWORD_CHANGE = "user.password.change"

    # File Operations
    FILE_UPLOAD = "file.upload"
    FILE_DELETE = "file.delete"
    FILE_DOWNLOAD = "file.download"

    # Protein Operations
    PROTEIN_CREATE = "protein.create"
    PROTEIN_UPDATE = "protein.update"
    PROTEIN_DELETE = "protein.delete"
    PROTEIN_VIEW = "protein.view"

    # Job Operations
    JOB_CREATE = "job.create"
    JOB_CANCEL = "job.cancel"
    JOB_VIEW = "job.view"
    JOB_COMPLETE = "job.complete"
    JOB_FAIL = "job.fail"

    # Data Access
    DATA_EXPORT = "data.export"
    SENSITIVE_ACCESS = "data.sensitive.access"

    # System
    CONFIG_CHANGE = "system.config.change"
    PERMISSION_CHANGE = "system.permission.change"


class AuditSeverity(str, Enum):
    """Severity levels for audit events"""
    INFO = "info"          # Normal operations
    WARNING = "warning"    # Suspicious activity
    ERROR = "error"        # Failed operations
    CRITICAL = "critical"  # Security incidents


class AuditLog(Base):
    """
    Audit log entry for tracking all critical system operations.

    Stores who did what, when, from where, and with what result.
    Immutable once created - never updated or deleted (except by retention policy).
    """
    __tablename__ = "audit_logs"

    id = Column(Integer, primary_key=True, index=True)

    # When
    timestamp = Column(
        DateTime(timezone=True),
        server_default=func.now(),
        nullable=False,
        index=True
    )

    # Who
    user_id = Column(Integer, nullable=True, index=True)  # None for anonymous/system
    username = Column(String(100), nullable=True, index=True)

    # What
    action = Column(String(100), nullable=False, index=True)  # AuditAction enum value
    resource_type = Column(String(50), nullable=True, index=True)  # 'protein', 'job', 'user', etc.
    resource_id = Column(String(100), nullable=True, index=True)  # ID of affected resource

    # How/Result
    severity = Column(String(20), nullable=False, default=AuditSeverity.INFO, index=True)
    status = Column(String(20), nullable=False)  # 'success', 'failure', 'partial'

    # Where/Context
    ip_address = Column(String(45), nullable=True)  # IPv4 or IPv6
    user_agent = Column(String(500), nullable=True)
    request_id = Column(String(100), nullable=True, index=True)  # For correlation

    # Details
    description = Column(Text, nullable=True)  # Human-readable description
    extra_metadata = Column(JSON, nullable=True)  # Additional structured data

    # Changes (for update operations)
    changes = Column(JSON, nullable=True)  # Before/after values

    # Error tracking
    error_message = Column(Text, nullable=True)
    error_code = Column(String(50), nullable=True)

    # Indexes for common queries
    __table_args__ = (
        Index('idx_audit_user_action', 'user_id', 'action'),
        Index('idx_audit_timestamp_action', 'timestamp', 'action'),
        Index('idx_audit_resource', 'resource_type', 'resource_id'),
        Index('idx_audit_severity_timestamp', 'severity', 'timestamp'),
    )

    def __repr__(self):
        return (
            f"<AuditLog(id={self.id}, action={self.action}, "
            f"user={self.username}, timestamp={self.timestamp})>"
        )

    def to_dict(self):
        """Convert to dictionary for API responses"""
        return {
            "id": self.id,
            "timestamp": self.timestamp.isoformat() if self.timestamp else None,
            "user_id": self.user_id,
            "username": self.username,
            "action": self.action,
            "resource_type": self.resource_type,
            "resource_id": self.resource_id,
            "severity": self.severity,
            "status": self.status,
            "ip_address": self.ip_address,
            "user_agent": self.user_agent,
            "request_id": self.request_id,
            "description": self.description,
            "extra_metadata": self.extra_metadata,
            "changes": self.changes,
            "error_message": self.error_message,
            "error_code": self.error_code,
        }
