"""
Centralized logging configuration
Supports both JSON and text formats with request tracing
"""
import logging
import sys
import uuid
from pathlib import Path
from contextvars import ContextVar
from typing import Optional, Dict, Any
from pythonjsonlogger import jsonlogger
from app.config import get_settings

settings = get_settings()

# Context variables for request tracing
request_id_ctx: ContextVar[Optional[str]] = ContextVar('request_id', default=None)
user_id_ctx: ContextVar[Optional[str]] = ContextVar('user_id', default=None)
request_path_ctx: ContextVar[Optional[str]] = ContextVar('request_path', default=None)
request_method_ctx: ContextVar[Optional[str]] = ContextVar('request_method', default=None)


def setup_logging():
    """
    Configure logging for the application
    Supports both JSON and text formats based on configuration
    """
    # Create logs directory if it doesn't exist
    log_file_path = Path(settings.LOG_FILE)
    log_file_path.parent.mkdir(parents=True, exist_ok=True)

    # Set log level
    log_level = getattr(logging, settings.LOG_LEVEL.upper(), logging.INFO)

    # Create logger
    logger = logging.getLogger()
    logger.setLevel(log_level)

    # Remove existing handlers
    logger.handlers = []

    # Console handler
    console_handler = logging.StreamHandler(sys.stdout)
    console_handler.setLevel(log_level)

    # File handler
    file_handler = logging.FileHandler(settings.LOG_FILE)
    file_handler.setLevel(log_level)

    # Add contextual filter to both handlers
    contextual_filter = ContextualFilter()
    console_handler.addFilter(contextual_filter)
    file_handler.addFilter(contextual_filter)

    # Choose format based on configuration
    if settings.LOG_FORMAT == "json":
        # JSON format for structured logging with context
        json_formatter = ContextualJsonFormatter(
            "%(asctime)s %(name)s %(levelname)s %(message)s %(pathname)s %(lineno)d"
        )
        console_handler.setFormatter(json_formatter)
        file_handler.setFormatter(json_formatter)
    else:
        # Text format with context variables
        text_formatter = logging.Formatter(
            "%(asctime)s - %(request_id)s - %(user_id)s - "
            "%(request_method)s %(request_path)s - "
            "%(name)s - %(levelname)s - %(message)s",
            datefmt="%Y-%m-%d %H:%M:%S"
        )
        console_handler.setFormatter(text_formatter)
        file_handler.setFormatter(text_formatter)

    # Add handlers
    logger.addHandler(console_handler)
    logger.addHandler(file_handler)

    # Silence noisy loggers
    logging.getLogger("uvicorn.access").setLevel(logging.WARNING)
    logging.getLogger("sqlalchemy.engine").setLevel(logging.WARNING)

    return logger


def get_logger(name: str):
    """
    Get a logger instance for a specific module

    Args:
        name: Module name (usually __name__)

    Returns:
        logging.Logger: Logger instance
    """
    return logging.getLogger(name)


class ContextualFilter(logging.Filter):
    """
    Logging filter that adds context variables to log records
    Includes request ID, user ID, path, and method for request tracing
    """

    def filter(self, record: logging.LogRecord) -> bool:
        # Add context variables to log record
        record.request_id = request_id_ctx.get() or '-'
        record.user_id = user_id_ctx.get() or '-'
        record.request_path = request_path_ctx.get() or '-'
        record.request_method = request_method_ctx.get() or '-'
        return True


class ContextualJsonFormatter(jsonlogger.JsonFormatter):
    """
    JSON formatter with context variables
    Includes request tracing information in JSON logs
    """

    def add_fields(self, log_record: Dict[str, Any], record: logging.LogRecord, message_dict: Dict[str, Any]) -> None:
        super().add_fields(log_record, record, message_dict)

        # Add request context
        if hasattr(record, 'request_id'):
            log_record['request_id'] = record.request_id
        if hasattr(record, 'user_id'):
            log_record['user_id'] = record.user_id
        if hasattr(record, 'request_path'):
            log_record['request_path'] = record.request_path
        if hasattr(record, 'request_method'):
            log_record['request_method'] = record.request_method

        # Add environment
        log_record['environment'] = settings.ENVIRONMENT


# Request context management functions
def set_request_context(
    request_id: Optional[str] = None,
    user_id: Optional[str] = None,
    request_path: Optional[str] = None,
    request_method: Optional[str] = None
) -> str:
    """
    Set request context for logging

    Args:
        request_id: Unique request identifier (auto-generated if None)
        user_id: User ID making the request
        request_path: Request path/endpoint
        request_method: HTTP method (GET, POST, etc.)

    Returns:
        str: The request ID (generated or provided)
    """
    if request_id is None:
        request_id = str(uuid.uuid4())

    request_id_ctx.set(request_id)
    user_id_ctx.set(user_id)
    request_path_ctx.set(request_path)
    request_method_ctx.set(request_method)

    return request_id


def get_request_id() -> Optional[str]:
    """Get current request ID from context"""
    return request_id_ctx.get()


def get_user_id() -> Optional[str]:
    """Get current user ID from context"""
    return user_id_ctx.get()


def clear_request_context():
    """Clear all request context variables"""
    request_id_ctx.set(None)
    user_id_ctx.set(None)
    request_path_ctx.set(None)
    request_method_ctx.set(None)


def get_request_context() -> Dict[str, Optional[str]]:
    """
    Get all current request context variables

    Returns:
        Dict containing request_id, user_id, path, and method
    """
    return {
        'request_id': request_id_ctx.get(),
        'user_id': user_id_ctx.get(),
        'request_path': request_path_ctx.get(),
        'request_method': request_method_ctx.get(),
    }
