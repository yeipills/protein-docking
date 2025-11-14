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
from app.core.file_validation import validate_file_comprehensive, sanitize_filename
from app.core.audit import create_audit_log
from app.models.audit_log import AuditAction, AuditSeverity

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
    Upload protein files (Part One) and start processing.

    Performs comprehensive validation including:
    - File extension and size checks
    - MIME type validation
    - STL structure validation (ASCII/Binary)
    - VERT/FACE format validation
    - Security checks (path traversal, null bytes, etc.)

    Generates centroids and context rays.
    """
    # Comprehensive file validation with content checking
    logger.info(f"Validating uploaded files for protein: {protein_name}")

    try:
        # Validate STL file
        stl_validation = await validate_file_comprehensive(
            stl_file,
            expected_extension='.stl',
            validate_content=True
        )
        logger.info(f"STL validation: {stl_validation}")

        # Validate VERT file
        vert_validation = await validate_file_comprehensive(
            vertices_file,
            expected_extension='.vert',
            validate_content=True
        )
        logger.info(f"VERT validation: {vert_validation}")

        # Validate FACE file
        face_validation = await validate_file_comprehensive(
            faces_file,
            expected_extension='.face',
            validate_content=True
        )
        logger.info(f"FACE validation: {face_validation}")

        # Cross-validation: ensure FACE indices don't exceed VERT count
        if 'max_vertex_index' in face_validation and 'vertex_count' in vert_validation:
            if face_validation['max_vertex_index'] >= vert_validation['vertex_count']:
                raise ValidationException(
                    detail=f"FACE file references vertex index {face_validation['max_vertex_index']}, "
                    f"but VERT file only has {vert_validation['vertex_count']} vertices"
                )

    except ValidationException as e:
        logger.error(f"File validation failed for protein {protein_name}: {e.detail}")
        raise

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

    # Sanitize protein name for safe filesystem use
    safe_protein_name = sanitize_filename(protein_name)

    # Save files with sanitized names
    file_paths = {}
    for file, file_type in [(stl_file, 'stl'), (vertices_file, 'vert'), (faces_file, 'face')]:
        safe_filename = sanitize_filename(file.filename or f"{safe_protein_name}.{file_type}")
        file_path = upload_dir / f"{safe_protein_name}.{file_type}"

        async with aiofiles.open(file_path, 'wb') as f:
            content = await file.read()
            await f.write(content)
        file_paths[file_type] = str(file_path)

        logger.info(f"Saved {file_type} file: {file_path} ({len(content)} bytes)")

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

    # Audit log file upload and job creation
    create_audit_log(
        db=db,
        action=AuditAction.FILE_UPLOAD,
        user_id=current_user.id,
        username=current_user.username,
        resource_type="protein",
        resource_id=str(protein.id),
        description=f"Uploaded Part One files for protein {protein_name}",
        metadata={
            "protein_id": protein.id,
            "job_id": job.id,
            "files": ["stl", "vert", "face"],
            "stl_format": stl_validation.get("format"),
            "triangle_count": stl_validation.get("triangle_count"),
            "vertex_count": vert_validation.get("vertex_count"),
            "face_count": face_validation.get("face_count"),
        }
    )

    create_audit_log(
        db=db,
        action=AuditAction.JOB_CREATE,
        user_id=current_user.id,
        username=current_user.username,
        resource_type="job",
        resource_id=str(job.id),
        description=f"Created Part One job for protein {protein_name}",
        metadata={"job_id": job.id, "job_type": job.job_type.value}
    )

    logger.info(f"Part One job created: {job.id} for protein {protein_name}")
    return job


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
    # Get protein
    protein = db.query(Protein).filter(Protein.id == protein_id).first()
    if not protein:
        raise NotFoundException("Protein not found")

    if protein.user_id != current_user.id:
        raise ForbiddenException("Not authorized to access this protein")

    # Validate uploaded files
    logger.info(f"Validating CR files for protein ID: {protein_id}")

    try:
        # Validate CR totals file
        cr_totals_validation = await validate_file_comprehensive(
            cr_totals_file,
            expected_extension='.txt',
            validate_content=False  # Basic validation only for txt files
        )
        logger.info(f"CR totals validation: {cr_totals_validation}")

        # Validate context rays file
        context_rays_validation = await validate_file_comprehensive(
            context_rays_file,
            expected_extension='.txt',
            validate_content=False
        )
        logger.info(f"Context rays validation: {context_rays_validation}")

    except ValidationException as e:
        logger.error(f"File validation failed for Part Two: {e.detail}")
        raise

    # Create directory
    upload_dir = Path(settings.UPLOAD_DIR) / str(current_user.id) / str(protein.id)
    upload_dir.mkdir(parents=True, exist_ok=True)

    # Sanitize filenames
    safe_protein_name = sanitize_filename(protein.name)

    # Save files
    file_paths = {}
    for file, file_type in [(cr_totals_file, 'cr_totals'), (context_rays_file, 'context_rays')]:
        file_path = upload_dir / f"{safe_protein_name}_{file_type}.txt"
        async with aiofiles.open(file_path, 'wb') as f:
            content = await file.read()
            await f.write(content)
        file_paths[file_type] = str(file_path)

        logger.info(f"Saved {file_type} file: {file_path} ({len(content)} bytes)")

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
