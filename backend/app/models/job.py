"""
Job model for tracking protein processing tasks
"""
from sqlalchemy import Column, Integer, String, DateTime, ForeignKey, Enum, Text, JSON
from sqlalchemy.orm import relationship
from sqlalchemy.sql import func
from app.database import Base
import enum


class JobStatus(str, enum.Enum):
    """Job status enumeration"""
    PENDING = "pending"
    PROCESSING = "processing"
    COMPLETED = "completed"
    FAILED = "failed"
    CANCELLED = "cancelled"


class JobType(str, enum.Enum):
    """Job type enumeration"""
    PART_ONE = "part_one"  # Upload STL, vertices, faces - generate CR
    PART_TWO = "part_two"  # Upload CR files - generate layers


class Job(Base):
    __tablename__ = "jobs"

    id = Column(Integer, primary_key=True, index=True)
    user_id = Column(Integer, ForeignKey("users.id"), nullable=False, index=True)
    protein_id = Column(Integer, ForeignKey("proteins.id"), nullable=True, index=True)

    # Job details
    job_type = Column(Enum(JobType), nullable=False)
    status = Column(Enum(JobStatus), default=JobStatus.PENDING, index=True)
    progress = Column(Integer, default=0)  # 0-100

    # Celery task tracking
    celery_task_id = Column(String, unique=True, nullable=True, index=True)

    # File information
    input_files = Column(JSON, nullable=True)  # List of input file paths
    output_files = Column(JSON, nullable=True)  # List of output file paths

    # Error handling
    error_message = Column(Text, nullable=True)

    # Processing metadata
    processing_params = Column(JSON, nullable=True)  # radius, delta, etc.
    processing_time_seconds = Column(Integer, nullable=True)

    # Timestamps
    created_at = Column(DateTime(timezone=True), server_default=func.now())
    started_at = Column(DateTime(timezone=True), nullable=True)
    completed_at = Column(DateTime(timezone=True), nullable=True)
    updated_at = Column(DateTime(timezone=True), onupdate=func.now())

    # Relationships
    user = relationship("User", back_populates="jobs")
    protein = relationship("Protein", back_populates="jobs")

    def __repr__(self):
        return f"<Job(id={self.id}, type={self.job_type}, status={self.status}, user_id={self.user_id})>"
