"""
Pydantic schemas for Job operations
"""
from pydantic import BaseModel, Field, ConfigDict
from typing import Optional, List, Dict, Any
from datetime import datetime
from app.models.job import JobStatus, JobType


class JobBase(BaseModel):
    """Base job schema"""
    job_type: JobType
    processing_params: Optional[Dict[str, Any]] = None


class JobCreate(JobBase):
    """Schema for creating a new job"""
    protein_id: Optional[int] = None


class JobUpdate(BaseModel):
    """Schema for updating job information"""
    status: Optional[JobStatus] = None
    progress: Optional[int] = Field(None, ge=0, le=100)
    error_message: Optional[str] = None
    output_files: Optional[List[str]] = None


class JobResponse(JobBase):
    """Schema for job response"""
    id: int
    user_id: int
    protein_id: Optional[int] = None
    status: JobStatus
    progress: int
    celery_task_id: Optional[str] = None
    input_files: Optional[List[str]] = None
    output_files: Optional[List[str]] = None
    error_message: Optional[str] = None
    processing_time_seconds: Optional[int] = None
    created_at: datetime
    started_at: Optional[datetime] = None
    completed_at: Optional[datetime] = None
    updated_at: Optional[datetime] = None

    model_config = ConfigDict(from_attributes=True)


class JobListResponse(BaseModel):
    """Schema for list of jobs"""
    total: int
    jobs: List[JobResponse]
