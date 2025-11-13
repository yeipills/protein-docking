"""
Pydantic schemas for Protein operations
"""
from pydantic import BaseModel, Field, ConfigDict
from typing import Optional, List, Dict, Any
from datetime import datetime


class ProteinBase(BaseModel):
    """Base protein schema"""
    name: str = Field(..., min_length=1, max_length=200)
    description: Optional[str] = None
    is_public: bool = False


class ProteinCreate(ProteinBase):
    """Schema for creating a new protein"""
    pass


class ProteinUpdate(BaseModel):
    """Schema for updating protein information"""
    name: Optional[str] = Field(None, min_length=1, max_length=200)
    description: Optional[str] = None
    is_public: Optional[bool] = None


class ProteinResponse(ProteinBase):
    """Schema for protein response"""
    id: int
    user_id: int
    stl_file: Optional[str] = None
    vertices_file: Optional[str] = None
    faces_file: Optional[str] = None
    cr_totals_file: Optional[str] = None
    context_rays_file: Optional[str] = None
    centroid_count: Optional[int] = None
    layer_files: Optional[Dict[str, Any]] = None
    file_size_bytes: Optional[int] = None
    processing_metadata: Optional[Dict[str, Any]] = None
    created_at: datetime
    updated_at: Optional[datetime] = None

    model_config = ConfigDict(from_attributes=True)


class ProteinListResponse(BaseModel):
    """Schema for list of proteins"""
    total: int
    proteins: List[ProteinResponse]
