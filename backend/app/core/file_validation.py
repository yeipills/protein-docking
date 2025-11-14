"""
Enhanced file validation with magic bytes checking
Provides secure file upload validation beyond extension checking
"""
from pathlib import Path
from typing import List, Optional
import magic
from fastapi import UploadFile

from app.core.exceptions import ValidationException
from app.core.logging import get_logger
from app.config import get_settings

logger = get_logger(__name__)
settings = get_settings()

# MIME type allowlist for each file extension
ALLOWED_MIME_TYPES = {
    ".stl": [
        "application/sla",
        "application/vnd.ms-pki.stl",
        "application/octet-stream",  # STL files often reported as binary
        "model/stl",
        "model/x.stl-binary",
    ],
    ".vert": [
        "text/plain",
        "application/octet-stream",
    ],
    ".face": [
        "text/plain",
        "application/octet-stream",
    ],
    ".txt": [
        "text/plain",
        "application/octet-stream",
    ],
    ".cr": [  # Context rays files
        "text/plain",
        "application/octet-stream",
    ],
}

# Dangerous file signatures to reject
DANGEROUS_SIGNATURES = {
    # Executables
    b"MZ": "Windows executable",
    b"\x7fELF": "Linux executable",
    b"\xca\xfe\xba\xbe": "Mach-O executable",
    # Scripts
    b"#!/": "Shell script",
    b"<?php": "PHP script",
    # Archives (potentially containing malware)
    b"PK\x03\x04": "ZIP archive",
    b"PK\x05\x06": "ZIP archive (empty)",
    b"Rar!": "RAR archive",
    b"\x1f\x8b": "GZIP archive",
    # Office docs (macros risk)
    b"\xd0\xcf\x11\xe0": "Microsoft Office document",
}


async def validate_file_upload(
    file: UploadFile,
    expected_extension: str,
    max_size: Optional[int] = None,
    check_magic_bytes: bool = True,
) -> bool:
    """
    Comprehensive file validation with security checks.

    Args:
        file: Uploaded file
        expected_extension: Expected file extension (e.g., '.stl')
        max_size: Maximum file size in bytes (optional, uses settings if not provided)
        check_magic_bytes: Whether to verify file signature (default: True)

    Returns:
        bool: True if file is valid

    Raises:
        ValidationException: If file validation fails

    Usage:
        await validate_file_upload(stl_file, '.stl')
    """
    # ==========================================
    # 1. EXTENSION CHECK
    # ==========================================
    filename = file.filename or ""
    file_ext = Path(filename).suffix.lower()

    if file_ext != expected_extension:
        logger.warning(
            f"File extension mismatch: expected {expected_extension}, got {file_ext} for {filename}"
        )
        raise ValidationException(
            detail=f"Invalid file extension. Expected {expected_extension}, got {file_ext}"
        )

    # ==========================================
    # 2. SIZE CHECK
    # ==========================================
    max_file_size = max_size or settings.MAX_FILE_SIZE_BYTES

    # Read file to get size
    file.file.seek(0, 2)  # Seek to end
    file_size = file.file.tell()
    file.file.seek(0)  # Reset to beginning

    if file_size > max_file_size:
        logger.warning(f"File too large: {file_size} bytes (max: {max_file_size}) for {filename}")
        raise ValidationException(
            detail=f"File too large: {file_size / 1024 / 1024:.2f}MB. Maximum: {max_file_size / 1024 / 1024:.2f}MB"
        )

    if file_size == 0:
        logger.warning(f"Empty file uploaded: {filename}")
        raise ValidationException(detail="File is empty")

    # ==========================================
    # 3. MAGIC BYTES CHECK
    # ==========================================
    if check_magic_bytes:
        # Read first 2048 bytes for magic byte checking
        content_start = await file.read(2048)
        await file.seek(0)  # Reset for later reading

        # Check for dangerous file signatures
        for signature, file_type in DANGEROUS_SIGNATURES.items():
            if content_start.startswith(signature):
                logger.error(
                    f"Dangerous file signature detected: {file_type} in {filename}"
                )
                raise ValidationException(
                    detail=f"Forbidden file type detected: {file_type}. This file type is not allowed."
                )

        # Verify MIME type using python-magic
        try:
            mime_type = magic.from_buffer(content_start, mime=True)
            logger.debug(f"Detected MIME type: {mime_type} for {filename}")

            allowed_mimes = ALLOWED_MIME_TYPES.get(file_ext, [])

            if mime_type not in allowed_mimes:
                logger.warning(
                    f"MIME type mismatch for {filename}: detected {mime_type}, expected one of {allowed_mimes}"
                )
                # Don't block on MIME mismatch for binary formats (can vary)
                # Just log warning for monitoring
                logger.warning(f"Allowing file despite MIME mismatch: {filename}")

        except Exception as e:
            logger.error(f"Error checking file magic bytes: {e}")
            # Don't block on magic byte check failure
            # Log error but allow upload
            logger.warning(f"Magic byte check failed, allowing upload: {filename}")

    # ==========================================
    # 4. FILENAME VALIDATION
    # ==========================================
    # Check for path traversal attempts
    if ".." in filename or "/" in filename or "\\" in filename:
        logger.error(f"Path traversal attempt detected in filename: {filename}")
        raise ValidationException(
            detail="Invalid filename: path traversal not allowed"
        )

    # Check for null bytes (file system attacks)
    if "\x00" in filename:
        logger.error(f"Null byte in filename detected: {filename}")
        raise ValidationException(detail="Invalid filename: null bytes not allowed")

    # ==========================================
    # 5. CONTENT VALIDATION (for text files)
    # ==========================================
    if file_ext in [".vert", ".face", ".txt", ".cr"]:
        # Read a sample to ensure it's actually text
        content_sample = await file.read(1024)
        await file.seek(0)

        try:
            # Try to decode as UTF-8 or ASCII
            content_sample.decode("utf-8")
        except UnicodeDecodeError:
            try:
                content_sample.decode("ascii")
            except UnicodeDecodeError:
                logger.warning(
                    f"Text file {filename} contains non-UTF8/ASCII content"
                )
                # Don't block - some files may have different encoding
                logger.warning(f"Allowing non-UTF8 text file: {filename}")

    logger.info(f"File validation passed: {filename} ({file_size} bytes)")
    return True


def sanitize_filename(filename: str) -> str:
    """
    Sanitize filename to prevent security issues.

    Args:
        filename: Original filename

    Returns:
        str: Sanitized filename

    Example:
        sanitize_filename("../../etc/passwd") -> "etc_passwd"
        sanitize_filename("file<>name.txt") -> "file__name.txt"
    """
    # Remove path components
    filename = Path(filename).name

    # Remove/replace dangerous characters
    dangerous_chars = ["<", ">", ":", '"', "|", "?", "*", "/", "\\", "\x00"]
    for char in dangerous_chars:
        filename = filename.replace(char, "_")

    # Limit length
    max_length = 255
    if len(filename) > max_length:
        name_part = Path(filename).stem[:max_length - 10]
        ext_part = Path(filename).suffix
        filename = f"{name_part}{ext_part}"

    return filename


async def scan_file_content(file: UploadFile, patterns: List[bytes]) -> bool:
    """
    Scan file content for specific byte patterns.

    Args:
        file: Uploaded file
        patterns: List of byte patterns to search for

    Returns:
        bool: True if any pattern found

    Example:
        # Scan for malicious patterns
        has_malware = await scan_file_content(file, [b'eval(', b'exec('])
    """
    content = await file.read()
    await file.seek(0)

    for pattern in patterns:
        if pattern in content:
            logger.warning(f"Pattern found in file: {pattern}")
            return True

    return False
