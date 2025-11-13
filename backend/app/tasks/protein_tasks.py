"""
Celery tasks for protein processing
"""
from celery import Task
from datetime import datetime
from pathlib import Path
import time
from app.tasks.celery_app import celery_app
from app.database import SessionLocal
from app.models.job import Job, JobStatus
from app.models.protein import Protein
from app.core.logging import get_logger

logger = get_logger(__name__)


class DatabaseTask(Task):
    """Base task that provides database session"""
    _db = None

    @property
    def db(self):
        if self._db is None:
            self._db = SessionLocal()
        return self._db

    def after_return(self, *args, **kwargs):
        if self._db is not None:
            self._db.close()
            self._db = None


@celery_app.task(base=DatabaseTask, bind=True)
def process_part_one(self, job_id: int):
    """
    Process Part One: Generate centroids and context rays

    Args:
        job_id: Job ID to process

    Returns:
        dict: Processing results
    """
    logger.info(f"Starting Part One processing for job {job_id}")
    db = self.db

    # Get job
    job = db.query(Job).filter(Job.id == job_id).first()
    if not job:
        logger.error(f"Job not found: {job_id}")
        return {"error": "Job not found"}

    # Update job status
    job.status = JobStatus.PROCESSING
    job.started_at = datetime.utcnow()
    job.progress = 10
    db.commit()

    try:
        # Get protein
        protein = db.query(Protein).filter(Protein.id == job.protein_id).first()
        if not protein:
            raise Exception("Protein not found")

        start_time = time.time()

        # Import processing modules
        from app.algorithms.surface_reader import read_surface_files
        from app.algorithms.centroid_calculator import calculate_centroids
        from app.algorithms.context_rays import calculate_context_rays

        # Step 1: Read surface files (20% progress)
        logger.info(f"Reading surface files for protein {protein.name}")
        vertices, faces = read_surface_files(
            protein.vertices_file,
            protein.faces_file
        )
        job.progress = 30
        db.commit()

        # Step 2: Calculate centroids (50% progress)
        logger.info(f"Calculating centroids for protein {protein.name}")
        centroids = calculate_centroids(vertices, faces)
        protein.centroid_count = len(centroids)
        job.progress = 50
        db.commit()

        # Step 3: Calculate context rays (90% progress)
        logger.info(f"Calculating context rays for protein {protein.name}")
        output_dir = Path(protein.stl_file).parent
        cr_totals_file, context_rays_file = calculate_context_rays(
            protein.name,
            vertices,
            faces,
            centroids,
            str(output_dir)
        )
        job.progress = 90
        db.commit()

        # Update protein with output files
        protein.cr_totals_file = cr_totals_file
        protein.context_rays_file = context_rays_file

        # Calculate processing time
        processing_time = int(time.time() - start_time)
        job.processing_time_seconds = processing_time

        # Update job as completed
        job.status = JobStatus.COMPLETED
        job.completed_at = datetime.utcnow()
        job.progress = 100
        job.output_files = [cr_totals_file, context_rays_file]

        db.commit()

        logger.info(f"Part One completed for job {job_id} in {processing_time}s")

        return {
            "job_id": job_id,
            "status": "completed",
            "output_files": [cr_totals_file, context_rays_file],
            "processing_time": processing_time
        }

    except Exception as e:
        logger.error(f"Error processing Part One for job {job_id}: {str(e)}", exc_info=True)

        job.status = JobStatus.FAILED
        job.error_message = str(e)
        job.completed_at = datetime.utcnow()
        db.commit()

        return {"error": str(e)}


@celery_app.task(base=DatabaseTask, bind=True)
def process_part_two(self, job_id: int):
    """
    Process Part Two: Generate layer files

    Args:
        job_id: Job ID to process

    Returns:
        dict: Processing results
    """
    logger.info(f"Starting Part Two processing for job {job_id}")
    db = self.db

    # Get job
    job = db.query(Job).filter(Job.id == job_id).first()
    if not job:
        logger.error(f"Job not found: {job_id}")
        return {"error": "Job not found"}

    # Update job status
    job.status = JobStatus.PROCESSING
    job.started_at = datetime.utcnow()
    job.progress = 10
    db.commit()

    try:
        # Get protein
        protein = db.query(Protein).filter(Protein.id == job.protein_id).first()
        if not protein:
            raise Exception("Protein not found")

        start_time = time.time()

        # Import processing modules
        from app.algorithms.layer_evaluator import evaluate_layers
        from app.algorithms.unity_exporter import export_for_unity

        # Step 1: Read CR files (20% progress)
        logger.info(f"Processing CR files for protein {protein.name}")
        job.progress = 20
        db.commit()

        # Step 2: Evaluate layers (70% progress)
        logger.info(f"Evaluating layers for protein {protein.name}")
        output_dir = Path(protein.cr_totals_file).parent
        layer_data = evaluate_layers(
            protein.cr_totals_file,
            protein.context_rays_file,
            str(output_dir)
        )
        job.progress = 70
        db.commit()

        # Step 3: Export for Unity (95% progress)
        logger.info(f"Exporting Unity files for protein {protein.name}")
        layer_files = export_for_unity(
            protein.name,
            layer_data,
            str(output_dir)
        )
        job.progress = 95
        db.commit()

        # Update protein with layer files
        protein.layer_files = layer_files

        # Calculate processing time
        processing_time = int(time.time() - start_time)
        job.processing_time_seconds = processing_time

        # Update job as completed
        job.status = JobStatus.COMPLETED
        job.completed_at = datetime.utcnow()
        job.progress = 100
        job.output_files = list(layer_files.values())

        db.commit()

        logger.info(f"Part Two completed for job {job_id} in {processing_time}s")

        return {
            "job_id": job_id,
            "status": "completed",
            "output_files": list(layer_files.values()),
            "processing_time": processing_time
        }

    except Exception as e:
        logger.error(f"Error processing Part Two for job {job_id}: {str(e)}", exc_info=True)

        job.status = JobStatus.FAILED
        job.error_message = str(e)
        job.completed_at = datetime.utcnow()
        db.commit()

        return {"error": str(e)}
