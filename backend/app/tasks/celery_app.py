"""
Celery application configuration
"""
from celery import Celery
from app.config import get_settings

settings = get_settings()

# Create Celery application
celery_app = Celery(
    "protein_docking",
    broker=settings.CELERY_BROKER_URL,
    backend=settings.CELERY_RESULT_BACKEND,
    include=["app.tasks.protein_tasks"]
)

# Configure Celery
celery_app.conf.update(
    task_serializer="json",
    accept_content=["json"],
    result_serializer="json",
    timezone="UTC",
    enable_utc=True,
    task_track_started=True,
    task_time_limit=settings.PROCESSING_TIMEOUT_SECONDS,
    task_soft_time_limit=settings.PROCESSING_TIMEOUT_SECONDS - 60,
    worker_prefetch_multiplier=1,
    worker_max_tasks_per_child=50,
)
