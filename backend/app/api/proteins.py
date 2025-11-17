"""
Protein management and file upload endpoints
"""
from fastapi import APIRouter, Depends, UploadFile, File, Form, Query, status
from sqlalchemy.orm import Session
from typing import List, Optional
from pathlib import Path
import aiofiles
import os
from app.database import get_db
from app.models.user import User
from app.models.protein import Protein
from app.models.job import Job, JobType, JobStatus
from app.schemas.protein import ProteinResponse, ProteinListResponse, ProteinCreate
from app.schemas.job import JobResponse
from app.dependencies import get_current_user
from app.core.exceptions import (
    NotFoundException,
    ForbiddenException,
    ValidationException
)
from app.core.logging import get_logger
from app.config import get_settings
from app.tasks.protein_tasks import process_part_one, process_part_two
from app.core.file_validation import validate_file_upload, sanitize_filename

logger = get_logger(__name__)
router = APIRouter()
settings = get_settings()


@router.post("/upload/part-one", response_model=JobResponse, status_code=status.HTTP_201_CREATED)
async def upload_part_one(
    protein_name: str = Form(...),
    stl_file: UploadFile = File(...),
    vertices_file: UploadFile = File(...),
    faces_file: UploadFile = File(...),
    current_user: User = Depends(get_current_user),
    db: Session = Depends(get_db)
):
    """
    Upload protein files (Part One) and start processing
    Generates centroids and context rays
    """
    # Comprehensive file validation
    await validate_file_upload(stl_file, '.stl')
    await validate_file_upload(vertices_file, '.vert')
    await validate_file_upload(faces_file, '.face')

    # Sanitize protein name to prevent path traversal
    protein_name = sanitize_filename(protein_name)

    try:
        # Create protein record
        protein = Protein(
            user_id=current_user.id,
            name=protein_name
        )
        db.add(protein)
        db.commit()
        db.refresh(protein)

        # Create upload directory
        upload_dir = Path(settings.UPLOAD_DIR) / str(current_user.id) / str(protein.id)
        upload_dir.mkdir(parents=True, exist_ok=True)

        # Save files
        file_paths = {}
        for file, file_type in [(stl_file, 'stl'), (vertices_file, 'vert'), (faces_file, 'face')]:
            file_path = upload_dir / f"{protein_name}.{file_type}"
            async with aiofiles.open(file_path, 'wb') as f:
                content = await file.read()
                await f.write(content)
            file_paths[file_type] = str(file_path)

        # Update protein with file paths
        protein.stl_file = file_paths['stl']
        protein.vertices_file = file_paths['vert']
        protein.faces_file = file_paths['face']

        # Create job
        job = Job(
            user_id=current_user.id,
            protein_id=protein.id,
            job_type=JobType.PART_ONE,
            status=JobStatus.PENDING,
            input_files=list(file_paths.values())
        )
        db.add(job)
        db.commit()
        db.refresh(job)

        # Start Celery task
        task = process_part_one.delay(job.id)
        job.celery_task_id = task.id
        db.commit()

        logger.info(f"Part One job created: {job.id} for protein {protein_name}")
        return job

    except Exception as e:
        # Rollback database changes
        db.rollback()

        # Clean up uploaded files if they exist
        try:
            if 'upload_dir' in locals() and upload_dir.exists():
                import shutil
                shutil.rmtree(upload_dir)
        except Exception as cleanup_error:
            logger.error(f"Error during cleanup: {cleanup_error}")

        logger.error(f"Error during Part One upload: {e}")
        raise


@router.post("/upload/part-two", response_model=JobResponse, status_code=status.HTTP_201_CREATED)
async def upload_part_two(
    protein_id: int = Form(...),
    cr_totals_file: UploadFile = File(...),
    context_rays_file: UploadFile = File(...),
    current_user: User = Depends(get_current_user),
    db: Session = Depends(get_db)
):
    """
    Upload CR files (Part Two) and start layer processing.

    Performs validation on context rays files including:
    - File extension and size checks
    - Text file encoding validation
    - Security checks

    Generates layer files for Unity visualization.
    """
    # Comprehensive file validation
    await validate_file_upload(cr_totals_file, '.txt')
    await validate_file_upload(context_rays_file, '.txt')

    # Get protein
    protein = db.query(Protein).filter(Protein.id == protein_id).first()
    if not protein:
        raise NotFoundException("Protein not found")

    if protein.user_id != current_user.id:
        raise ForbiddenException("Not authorized to access this protein")

    try:
        # Create directory
        upload_dir = Path(settings.UPLOAD_DIR) / str(current_user.id) / str(protein.id)
        upload_dir.mkdir(parents=True, exist_ok=True)

        # Save files
        file_paths = {}
        for file, file_type in [(cr_totals_file, 'cr_totals'), (context_rays_file, 'context_rays')]:
            # Sanitize filename
            safe_protein_name = sanitize_filename(protein.name)
            file_path = upload_dir / f"{safe_protein_name}_{file_type}.txt"
            async with aiofiles.open(file_path, 'wb') as f:
                content = await file.read()
                await f.write(content)
            file_paths[file_type] = str(file_path)

        # Update protein
        protein.cr_totals_file = file_paths['cr_totals']
        protein.context_rays_file = file_paths['context_rays']

        # Create job
        job = Job(
            user_id=current_user.id,
            protein_id=protein.id,
            job_type=JobType.PART_TWO,
            status=JobStatus.PENDING,
            input_files=list(file_paths.values())
        )
        db.add(job)
        db.commit()
        db.refresh(job)

        # Start Celery task
        task = process_part_two.delay(job.id)
        job.celery_task_id = task.id
        db.commit()

        logger.info(f"Part Two job created: {job.id} for protein {protein.name}")
        return job

    except Exception as e:
        # Rollback database changes
        db.rollback()

        # Clean up uploaded files if they exist
        try:
            if 'file_paths' in locals():
                import shutil
                for file_path in file_paths.values():
                    if os.path.exists(file_path):
                        os.remove(file_path)
        except Exception as cleanup_error:
            logger.error(f"Error during cleanup: {cleanup_error}")

        logger.error(f"Error during Part Two upload: {e}")
        raise


@router.get("/", response_model=ProteinListResponse)
async def list_proteins(
    skip: int = Query(0, ge=0),
    limit: int = Query(50, ge=1, le=100),
    current_user: User = Depends(get_current_user),
    db: Session = Depends(get_db)
):
    """List user's proteins"""
    query = db.query(Protein).filter(
        Protein.user_id == current_user.id,
        Protein.is_deleted == False
    )

    total = query.count()
    proteins = query.order_by(Protein.created_at.desc()).offset(skip).limit(limit).all()

    return {"total": total, "proteins": proteins}


@router.get("/{protein_id}", response_model=ProteinResponse)
async def get_protein(
    protein_id: int,
    current_user: User = Depends(get_current_user),
    db: Session = Depends(get_db)
):
    """Get protein details"""
    protein = db.query(Protein).filter(Protein.id == protein_id).first()

    if not protein or protein.is_deleted:
        raise NotFoundException("Protein not found")

    if protein.user_id != current_user.id and not protein.is_public:
        raise ForbiddenException("Not authorized to access this protein")

    return protein
