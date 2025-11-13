"""
Protein model for storing protein information and files
"""
from sqlalchemy import Column, Integer, String, DateTime, ForeignKey, Text, JSON, Boolean
from sqlalchemy.orm import relationship
from sqlalchemy.sql import func
from app.database import Base


class Protein(Base):
    __tablename__ = "proteins"

    id = Column(Integer, primary_key=True, index=True)
    user_id = Column(Integer, ForeignKey("users.id"), nullable=False, index=True)

    # Protein identification
    name = Column(String, nullable=False, index=True)
    description = Column(Text, nullable=True)

    # File paths
    stl_file = Column(String, nullable=True)
    vertices_file = Column(String, nullable=True)
    faces_file = Column(String, nullable=True)
    cr_totals_file = Column(String, nullable=True)
    context_rays_file = Column(String, nullable=True)

    # Processing results
    centroid_count = Column(Integer, nullable=True)
    layer_files = Column(JSON, nullable=True)  # Dictionary of layer files

    # Metadata
    file_size_bytes = Column(Integer, nullable=True)
    processing_metadata = Column(JSON, nullable=True)

    # Status
    is_public = Column(Boolean, default=False)
    is_deleted = Column(Boolean, default=False)

    # Timestamps
    created_at = Column(DateTime(timezone=True), server_default=func.now())
    updated_at = Column(DateTime(timezone=True), onupdate=func.now())

    # Relationships
    user = relationship("User", back_populates="proteins")
    jobs = relationship("Job", back_populates="protein", cascade="all, delete-orphan")

    def __repr__(self):
        return f"<Protein(id={self.id}, name={self.name}, user_id={self.user_id})>"
