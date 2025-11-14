"""
Enhanced file validation with magic bytes checking
Provides secure file upload validation beyond extension checking
"""
from pathlib import Path
from typing import List, Optional, Dict, Tuple
import magic
import struct
from fastapi import UploadFile

from app.core.exceptions import ValidationException
from app.core.logging import get_logger
from app.config import get_settings
from app.core.metrics import (
    file_validation_total,
    file_validation_failures,
    file_validation_duration_seconds
)
import time

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


async def validate_file_comprehensive(
    file: UploadFile,
    expected_extension: str,
    max_size: Optional[int] = None,
    validate_content: bool = True,
) -> Dict[str, any]:
    """
    Comprehensive file validation with metrics tracking.

    Performs all validation checks:
    1. Extension validation
    2. Size validation
    3. Magic bytes / MIME type
    4. Filename security
    5. Content structure validation (STL/VERT/FACE)

    Args:
        file: Uploaded file
        expected_extension: Expected extension ('.stl', '.vert', '.face')
        max_size: Maximum size in bytes
        validate_content: Whether to validate file content structure

    Returns:
        dict: Validation results with file metadata

    Raises:
        ValidationException: If validation fails
    """
    start_time = time.time()
    file_type = expected_extension.lstrip('.')

    try:
        # Run basic validation
        await validate_file_upload(file, expected_extension, max_size, check_magic_bytes=True)

        result = {
            "valid": True,
            "extension": expected_extension,
            "filename": sanitize_filename(file.filename or "unknown")
        }

        # Content-specific validation
        if validate_content:
            if expected_extension == '.stl':
                stl_info = await validate_stl_file(file)
                result.update(stl_info)

            elif expected_extension in ['.vert', '.vertex']:
                vert_info = await validate_vert_file(file)
                result.update(vert_info)

            elif expected_extension in ['.face', '.faces']:
                face_info = await validate_face_file(file)
                result.update(face_info)

        # Track success metrics
        duration = time.time() - start_time
        file_validation_total.labels(file_type=file_type, status='success').inc()
        file_validation_duration_seconds.labels(file_type=file_type).observe(duration)

        logger.info(f"File validation passed: {file.filename} ({duration:.3f}s)")
        return result

    except ValidationException as e:
        # Track failure metrics
        duration = time.time() - start_time
        file_validation_total.labels(file_type=file_type, status='failed').inc()
        file_validation_duration_seconds.labels(file_type=file_type).observe(duration)

        # Determine failure reason
        error_msg = str(e.detail).lower()
        if 'size' in error_msg or 'large' in error_msg:
            reason = 'size'
        elif 'extension' in error_msg:
            reason = 'extension'
        elif 'mime' in error_msg or 'magic' in error_msg:
            reason = 'mime'
        elif 'format' in error_msg or 'structure' in error_msg or 'invalid' in error_msg:
            reason = 'content'
        else:
            reason = 'other'

        file_validation_failures.labels(file_type=file_type, reason=reason).inc()
        raise


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


async def validate_stl_file(file: UploadFile) -> Dict[str, any]:
    """
    Validate STL file structure (ASCII or Binary format).

    STL Binary Format:
    - 80 byte header
    - 4 byte uint32 triangle count
    - For each triangle:
      - 12 bytes (3 floats) for normal vector
      - 36 bytes (9 floats) for 3 vertices
      - 2 bytes for attribute byte count

    STL ASCII Format:
    - Starts with "solid"
    - Contains "facet normal", "vertex", "endloop", "endfacet"
    - Ends with "endsolid"

    Args:
        file: STL file to validate

    Returns:
        dict: Validation results with format type and triangle count

    Raises:
        ValidationException: If file is not a valid STL
    """
    content = await file.read()
    await file.seek(0)

    if len(content) < 84:  # Minimum binary STL size
        raise ValidationException(
            detail="STL file too small to be valid (< 84 bytes)"
        )

    # Check if ASCII STL
    try:
        text_content = content[:100].decode('ascii', errors='ignore')
        if text_content.strip().lower().startswith('solid'):
            # ASCII STL validation
            try:
                full_text = content.decode('ascii')
            except UnicodeDecodeError:
                raise ValidationException(
                    detail="STL file appears to be ASCII but contains invalid characters"
                )

            # Check for required keywords
            required_keywords = ['facet', 'normal', 'vertex', 'endloop', 'endfacet']
            for keyword in required_keywords:
                if keyword not in full_text.lower():
                    raise ValidationException(
                        detail=f"Invalid ASCII STL: missing required keyword '{keyword}'"
                    )

            # Count triangles (facets)
            triangle_count = full_text.lower().count('endfacet')

            if triangle_count == 0:
                raise ValidationException(
                    detail="Invalid ASCII STL: no triangles found"
                )

            logger.info(f"Valid ASCII STL with {triangle_count} triangles")
            return {
                "format": "ascii",
                "triangle_count": triangle_count,
                "valid": True
            }

    except UnicodeDecodeError:
        pass  # Not ASCII, try binary

    # Binary STL validation
    if len(content) < 84:
        raise ValidationException(
            detail="File too small to be binary STL"
        )

    try:
        # Read triangle count from bytes 80-84
        triangle_count = struct.unpack('<I', content[80:84])[0]

        # Calculate expected file size
        # Header (80) + count (4) + triangles (50 bytes each)
        expected_size = 80 + 4 + (triangle_count * 50)

        if len(content) != expected_size:
            logger.warning(
                f"Binary STL size mismatch: expected {expected_size}, got {len(content)}"
            )
            # Don't fail, some files have padding

        if triangle_count > 10_000_000:  # Sanity check
            raise ValidationException(
                detail=f"STL triangle count seems invalid: {triangle_count}"
            )

        logger.info(f"Valid binary STL with {triangle_count} triangles")
        return {
            "format": "binary",
            "triangle_count": triangle_count,
            "valid": True
        }

    except struct.error as e:
        raise ValidationException(
            detail=f"Invalid binary STL structure: {str(e)}"
        )


async def validate_vert_file(file: UploadFile) -> Dict[str, any]:
    """
    Validate VERT file (vertex coordinates file).

    Expected format:
    - Text file with one vertex per line
    - Each line: X Y Z (three float values)
    - Example: "1.234 5.678 9.012"

    Args:
        file: VERT file to validate

    Returns:
        dict: Validation results with vertex count

    Raises:
        ValidationException: If file format is invalid
    """
    content = await file.read()
    await file.seek(0)

    try:
        text = content.decode('utf-8')
    except UnicodeDecodeError:
        try:
            text = content.decode('ascii')
        except UnicodeDecodeError:
            raise ValidationException(
                detail="VERT file must be text (UTF-8 or ASCII)"
            )

    lines = [line.strip() for line in text.strip().split('\n') if line.strip()]

    if len(lines) == 0:
        raise ValidationException(
            detail="VERT file is empty"
        )

    # Validate first few lines to check format
    sample_size = min(10, len(lines))
    for i, line in enumerate(lines[:sample_size]):
        parts = line.split()
        if len(parts) != 3:
            raise ValidationException(
                detail=f"Invalid VERT format at line {i+1}: expected 3 values (X Y Z), got {len(parts)}"
            )

        # Try to parse as floats
        try:
            x, y, z = float(parts[0]), float(parts[1]), float(parts[2])
        except ValueError as e:
            raise ValidationException(
                detail=f"Invalid VERT format at line {i+1}: values must be numeric ({str(e)})"
            )

    vertex_count = len(lines)

    if vertex_count > 100_000_000:  # Sanity check
        raise ValidationException(
            detail=f"VERT file too large: {vertex_count} vertices"
        )

    logger.info(f"Valid VERT file with {vertex_count} vertices")
    return {
        "vertex_count": vertex_count,
        "valid": True
    }


async def validate_face_file(file: UploadFile) -> Dict[str, any]:
    """
    Validate FACE file (triangle faces file).

    Expected format:
    - Text file with one face per line
    - Each line: V1 V2 V3 (three integer indices)
    - Example: "0 1 2" (triangle using vertices 0, 1, 2)

    Args:
        file: FACE file to validate

    Returns:
        dict: Validation results with face count

    Raises:
        ValidationException: If file format is invalid
    """
    content = await file.read()
    await file.seek(0)

    try:
        text = content.decode('utf-8')
    except UnicodeDecodeError:
        try:
            text = content.decode('ascii')
        except UnicodeDecodeError:
            raise ValidationException(
                detail="FACE file must be text (UTF-8 or ASCII)"
            )

    lines = [line.strip() for line in text.strip().split('\n') if line.strip()]

    if len(lines) == 0:
        raise ValidationException(
            detail="FACE file is empty"
        )

    # Validate first few lines
    sample_size = min(10, len(lines))
    max_vertex_index = 0

    for i, line in enumerate(lines[:sample_size]):
        parts = line.split()
        if len(parts) != 3:
            raise ValidationException(
                detail=f"Invalid FACE format at line {i+1}: expected 3 indices, got {len(parts)}"
            )

        # Try to parse as integers
        try:
            v1, v2, v3 = int(parts[0]), int(parts[1]), int(parts[2])

            # Check non-negative
            if v1 < 0 or v2 < 0 or v3 < 0:
                raise ValidationException(
                    detail=f"Invalid FACE format at line {i+1}: vertex indices must be non-negative"
                )

            max_vertex_index = max(max_vertex_index, v1, v2, v3)

        except ValueError as e:
            raise ValidationException(
                detail=f"Invalid FACE format at line {i+1}: indices must be integers ({str(e)})"
            )

    face_count = len(lines)

    if face_count > 100_000_000:  # Sanity check
        raise ValidationException(
            detail=f"FACE file too large: {face_count} faces"
        )

    logger.info(f"Valid FACE file with {face_count} faces")
    return {
        "face_count": face_count,
        "max_vertex_index": max_vertex_index,
        "valid": True
    }
