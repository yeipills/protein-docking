"""
Celery tasks for asynchronous processing
"""
from app.tasks.celery_app import celery_app
from app.tasks.protein_tasks import process_part_one, process_part_two

__all__ = ["celery_app", "process_part_one", "process_part_two"]
