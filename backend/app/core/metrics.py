"""
Prometheus metrics for monitoring application performance
"""
from prometheus_client import Counter, Histogram, Gauge, Info
import time
from functools import wraps
from typing import Callable

# ==========================================
# HTTP METRICS
# ==========================================

# Request counter
http_requests_total = Counter(
    'http_requests_total',
    'Total HTTP requests',
    ['method', 'endpoint', 'status_code']
)

# Request duration histogram
http_request_duration_seconds = Histogram(
    'http_request_duration_seconds',
    'HTTP request duration in seconds',
    ['method', 'endpoint'],
    buckets=(0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0)
)

# Active requests gauge
http_requests_in_progress = Gauge(
    'http_requests_in_progress',
    'Number of HTTP requests currently being processed',
    ['method', 'endpoint']
)

# ==========================================
# APPLICATION METRICS
# ==========================================

# Job metrics
jobs_total = Counter(
    'jobs_total',
    'Total number of jobs created',
    ['job_type', 'user_id']
)

jobs_completed = Counter(
    'jobs_completed_total',
    'Total number of completed jobs',
    ['job_type', 'status']
)

jobs_active = Gauge(
    'jobs_active',
    'Number of currently active jobs',
    ['job_type', 'status']
)

job_processing_duration_seconds = Histogram(
    'job_processing_duration_seconds',
    'Job processing duration in seconds',
    ['job_type'],
    buckets=(1, 5, 10, 30, 60, 120, 300, 600, 1800, 3600)
)

# Celery task metrics
celery_tasks_total = Counter(
    'celery_tasks_total',
    'Total number of Celery tasks',
    ['task_name', 'status']
)

celery_task_duration_seconds = Histogram(
    'celery_task_duration_seconds',
    'Celery task duration in seconds',
    ['task_name'],
    buckets=(1, 5, 10, 30, 60, 120, 300, 600, 1800, 3600)
)

# ==========================================
# DATABASE METRICS
# ==========================================

db_connections_active = Gauge(
    'db_connections_active',
    'Number of active database connections'
)

db_query_duration_seconds = Histogram(
    'db_query_duration_seconds',
    'Database query duration in seconds',
    ['operation'],
    buckets=(0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5)
)

# ==========================================
# FILE UPLOAD METRICS
# ==========================================

file_uploads_total = Counter(
    'file_uploads_total',
    'Total number of file uploads',
    ['file_type', 'user_id']
)

file_upload_size_bytes = Histogram(
    'file_upload_size_bytes',
    'File upload size in bytes',
    ['file_type'],
    buckets=(1024, 10240, 102400, 1048576, 10485760, 104857600)  # 1KB to 100MB
)

# ==========================================
# CACHE METRICS
# ==========================================

cache_operations = Counter(
    'cache_operations_total',
    'Total cache operations',
    ['operation']  # hit, miss, set, error, invalidate
)

cache_keys_total = Gauge(
    'cache_keys_total',
    'Total number of keys in cache'
)

cache_memory_bytes = Gauge(
    'cache_memory_bytes',
    'Memory used by cache in bytes'
)

# ==========================================
# FILE VALIDATION METRICS
# ==========================================

file_validation_total = Counter(
    'file_validation_total',
    'Total file validations performed',
    ['file_type', 'status']  # status: success, failed
)

file_validation_failures = Counter(
    'file_validation_failures_total',
    'Total file validation failures',
    ['file_type', 'reason']  # reason: size, extension, content, mime, etc.
)

file_validation_duration_seconds = Histogram(
    'file_validation_duration_seconds',
    'Time spent validating files',
    ['file_type'],
    buckets=(0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 2.0, 5.0)
)

# ==========================================
# AUDIT LOGGING METRICS
# ==========================================

audit_events_total = Counter(
    'audit_events_total',
    'Total audit events logged',
    ['action', 'status']  # action: login, upload, etc. status: success, failure
)

audit_events_by_severity = Counter(
    'audit_events_by_severity_total',
    'Audit events by severity level',
    ['severity']  # info, warning, error, critical
)

audit_log_size = Gauge(
    'audit_log_total_entries',
    'Total number of audit log entries in database'
)

# ==========================================
# APPLICATION INFO
# ==========================================

app_info = Info('app_info', 'Application information')
app_info.info({
    'version': '2.1.0',
    'name': 'Protein Docking Platform',
    'environment': 'production'
})

# ==========================================
# HELPER FUNCTIONS & DECORATORS
# ==========================================

def track_request_metrics(method: str, endpoint: str, status_code: int, duration: float):
    """
    Track HTTP request metrics.

    Args:
        method: HTTP method (GET, POST, etc.)
        endpoint: Request endpoint path
        status_code: HTTP status code
        duration: Request duration in seconds
    """
    http_requests_total.labels(
        method=method,
        endpoint=endpoint,
        status_code=status_code
    ).inc()

    http_request_duration_seconds.labels(
        method=method,
        endpoint=endpoint
    ).observe(duration)


def track_job_created(job_type: str, user_id: int):
    """Track job creation."""
    jobs_total.labels(job_type=job_type, user_id=str(user_id)).inc()


def track_job_completed(job_type: str, status: str, duration_seconds: float):
    """Track job completion."""
    jobs_completed.labels(job_type=job_type, status=status).inc()
    job_processing_duration_seconds.labels(job_type=job_type).observe(duration_seconds)


def track_file_upload(file_type: str, user_id: int, size_bytes: int):
    """Track file upload."""
    file_uploads_total.labels(file_type=file_type, user_id=str(user_id)).inc()
    file_upload_size_bytes.labels(file_type=file_type).observe(size_bytes)


def track_celery_task(task_name: str, status: str, duration_seconds: float = None):
    """Track Celery task execution."""
    celery_tasks_total.labels(task_name=task_name, status=status).inc()
    if duration_seconds is not None:
        celery_task_duration_seconds.labels(task_name=task_name).observe(duration_seconds)


def track_time(metric: Histogram, labels: dict = None):
    """
    Decorator to track execution time of a function.

    Usage:
        @track_time(db_query_duration_seconds, {'operation': 'select'})
        def my_function():
            pass
    """
    def decorator(func: Callable) -> Callable:
        @wraps(func)
        async def async_wrapper(*args, **kwargs):
            start = time.time()
            try:
                result = await func(*args, **kwargs)
                return result
            finally:
                duration = time.time() - start
                if labels:
                    metric.labels(**labels).observe(duration)
                else:
                    metric.observe(duration)

        @wraps(func)
        def sync_wrapper(*args, **kwargs):
            start = time.time()
            try:
                result = func(*args, **kwargs)
                return result
            finally:
                duration = time.time() - start
                if labels:
                    metric.labels(**labels).observe(duration)
                else:
                    metric.observe(duration)

        # Return appropriate wrapper based on function type
        import inspect
        if inspect.iscoroutinefunction(func):
            return async_wrapper
        return sync_wrapper

    return decorator
