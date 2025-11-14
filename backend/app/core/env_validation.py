"""
Environment configuration validation
Validates critical environment variables at application startup
"""
from app.config import get_settings
from app.core.logging import get_logger
import sys

logger = get_logger(__name__)


def validate_environment() -> None:
    """
    Validate critical environment variables at startup.

    Checks:
    - Default secrets are not used in production
    - Required security configurations are set
    - Production-specific validations

    Raises:
        SystemExit: If critical validation errors are found
    """
    settings = get_settings()
    errors = []
    warnings = []

    logger.info("Validating environment configuration...")

    # ==========================================
    # CRITICAL: Check for default secrets
    # ==========================================

    if "change_this" in settings.JWT_SECRET_KEY.lower():
        errors.append(
            "JWT_SECRET_KEY is using default value - CRITICAL SECURITY RISK! "
            "Set a strong random key (minimum 64 characters)"
        )

    if len(settings.JWT_SECRET_KEY) < 32:
        errors.append(
            f"JWT_SECRET_KEY is too short ({len(settings.JWT_SECRET_KEY)} chars). "
            "Minimum 32 characters required, 64+ recommended"
        )

    if "change_this" in settings.POSTGRES_PASSWORD.lower():
        errors.append(
            "POSTGRES_PASSWORD is using default value - CRITICAL SECURITY RISK! "
            "Set a strong password (minimum 16 characters)"
        )

    if "change_this" in settings.SECRET_KEY.lower():
        errors.append(
            "SECRET_KEY is using default value - CRITICAL SECURITY RISK! "
            "Set a strong random key (minimum 32 characters)"
        )

    if "change_this" in settings.SOCKET_SECRET_KEY.lower():
        errors.append(
            "SOCKET_SECRET_KEY is using default value - SECURITY RISK! "
            "Set a strong random key (minimum 32 characters)"
        )

    # ==========================================
    # PRODUCTION-SPECIFIC VALIDATIONS
    # ==========================================

    if settings.ENVIRONMENT == "production":
        logger.info("Running production-specific validations...")

        # Check CORS origins
        if "localhost" in settings.ALLOWED_ORIGINS:
            warnings.append(
                "ALLOWED_ORIGINS contains 'localhost' in production environment. "
                "This may block legitimate traffic from your production domain. "
                "Update ALLOWED_ORIGINS to include your production domain(s)."
            )

        # Check auto-reload
        if settings.BACKEND_RELOAD:
            warnings.append(
                "BACKEND_RELOAD is enabled in production. "
                "This causes performance overhead. Set BACKEND_RELOAD=false in production."
            )

        # Check debug logging
        if settings.LOG_LEVEL == "DEBUG":
            warnings.append(
                "LOG_LEVEL is set to DEBUG in production. "
                "This generates excessive logs. Set LOG_LEVEL=INFO or WARNING."
            )

        # Check JWT expiration
        if settings.JWT_ACCESS_TOKEN_EXPIRE_MINUTES > 120:
            warnings.append(
                f"JWT_ACCESS_TOKEN_EXPIRE_MINUTES is set to {settings.JWT_ACCESS_TOKEN_EXPIRE_MINUTES} minutes. "
                "Long token expiration reduces security. Consider 60 minutes or less."
            )

    # ==========================================
    # GENERAL VALIDATIONS
    # ==========================================

    # Check file upload limits
    if settings.MAX_FILE_SIZE_MB > 500:
        warnings.append(
            f"MAX_FILE_SIZE_MB is set to {settings.MAX_FILE_SIZE_MB}. "
            "Very large files may cause memory issues. Consider limiting to 100-200 MB."
        )

    # Check rate limiting
    if settings.RATE_LIMIT_PER_MINUTE > 1000:
        warnings.append(
            f"RATE_LIMIT_PER_MINUTE is very high ({settings.RATE_LIMIT_PER_MINUTE}). "
            "This may not effectively prevent abuse."
        )

    # Check processing timeout
    if settings.PROCESSING_TIMEOUT_SECONDS > 7200:  # 2 hours
        warnings.append(
            f"PROCESSING_TIMEOUT_SECONDS is very long ({settings.PROCESSING_TIMEOUT_SECONDS}s). "
            "Consider a lower timeout to prevent hung jobs."
        )

    # ==========================================
    # REPORT RESULTS
    # ==========================================

    # Print errors
    if errors:
        logger.error("=" * 80)
        logger.error("❌ ENVIRONMENT VALIDATION FAILED - CRITICAL ERRORS FOUND")
        logger.error("=" * 80)
        for i, error in enumerate(errors, 1):
            logger.error(f"{i}. {error}")
        logger.error("=" * 80)
        logger.error("Fix these critical errors before starting the application.")
        logger.error("See .env.example for secure configuration examples.")
        logger.error("=" * 80)
        sys.exit(1)

    # Print warnings
    if warnings:
        logger.warning("=" * 80)
        logger.warning("⚠️  ENVIRONMENT VALIDATION WARNINGS")
        logger.warning("=" * 80)
        for i, warning in enumerate(warnings, 1):
            logger.warning(f"{i}. {warning}")
        logger.warning("=" * 80)
        logger.warning("These warnings should be addressed for optimal security and performance.")
        logger.warning("=" * 80)

    # Success message
    if not errors and not warnings:
        logger.info("✅ Environment validation passed - all checks OK")
    elif not errors:
        logger.info(f"✅ Environment validation passed with {len(warnings)} warning(s)")


def validate_required_directories() -> None:
    """
    Validate and create required directories at startup.

    Creates:
    - Upload directory
    - Results directory
    - Logs directory
    """
    from pathlib import Path

    settings = get_settings()

    required_dirs = [
        settings.UPLOAD_DIR,
        settings.RESULTS_DIR,
        Path(settings.LOG_FILE).parent,
    ]

    for dir_path in required_dirs:
        path = Path(dir_path)
        if not path.exists():
            logger.info(f"Creating required directory: {path}")
            path.mkdir(parents=True, exist_ok=True)
            logger.info(f"✅ Created directory: {path}")


def startup_validation() -> None:
    """
    Run all startup validations.

    This function should be called during application startup
    (in FastAPI lifespan or similar).
    """
    logger.info("=" * 80)
    logger.info("Running startup validations...")
    logger.info("=" * 80)

    # Validate environment variables
    validate_environment()

    # Validate and create directories
    validate_required_directories()

    logger.info("=" * 80)
    logger.info("✅ All startup validations completed")
    logger.info("=" * 80)
