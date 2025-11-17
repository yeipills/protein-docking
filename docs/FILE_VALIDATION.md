# Enhanced File Validation

## Overview

The Protein Docking Platform includes comprehensive file validation to ensure data integrity, security, and compatibility. This document details the validation system for all uploaded files.

## Table of Contents

- [Supported File Types](#supported-file-types)
- [Validation Layers](#validation-layers)
- [File Format Specifications](#file-format-specifications)
- [Security Features](#security-features)
- [Usage Examples](#usage-examples)
- [Error Handling](#error-handling)
- [Metrics & Monitoring](#metrics--monitoring)
- [Troubleshooting](#troubleshooting)

## Supported File Types

### Part One Upload
- **STL** (`.stl`) - 3D model surface mesh (ASCII or Binary)
- **VERT** (`.vert`) - Vertex coordinates
- **FACE** (`.face`) - Triangle face definitions

### Part Two Upload
- **CR Totals** (`.txt`) - Context rays totals
- **Context Rays** (`.txt`) - Context rays data

## Validation Layers

The validation system performs multiple layers of checks:

```
┌─────────────────────────────────────┐
│   1. Extension Validation           │  ← Basic file type check
├─────────────────────────────────────┤
│   2. Size Validation                │  ← Prevent oversized files
├─────────────────────────────────────┤
│   3. Security Checks                │  ← Path traversal, null bytes
├─────────────────────────────────────┤
│   4. MIME Type Detection            │  ← Magic bytes verification
├─────────────────────────────────────┤
│   5. Content Structure Validation   │  ← Format-specific checks
├─────────────────────────────────────┤
│   6. Cross-file Validation          │  ← Consistency checks
└─────────────────────────────────────┘
```

### 1. Extension Validation

Verifies the file extension matches the expected type:
- Only whitelisted extensions are allowed
- Case-insensitive matching
- Prevents extension spoofing

### 2. Size Validation

Prevents resource exhaustion from large files:
- Configurable maximum file size (default: 100MB)
- Per-file-type size limits
- Empty file detection

### 3. Security Checks

Protects against common attack vectors:
- **Path Traversal**: Rejects `../` sequences
- **Null Byte Injection**: Removes `\x00` characters
- **Dangerous Signatures**: Blocks executables, archives, scripts
- **Filename Sanitization**: Removes special characters

### 4. MIME Type Detection

Uses magic bytes to verify actual file type:
- Python `magic` library integration
- Whitelist of allowed MIME types per extension
- Fallback handling for ambiguous types

### 5. Content Structure Validation

Format-specific validation for data integrity:

#### STL Files
- Binary or ASCII format detection
- Triangle count verification
- Structure integrity checks
- Size consistency validation

#### VERT Files
- One vertex per line (X Y Z coordinates)
- Numeric value validation
- Coordinate format checking

#### FACE Files
- One face per line (V1 V2 V3 indices)
- Integer index validation
- Non-negative index requirement
- Maximum index tracking

### 6. Cross-file Validation

Ensures consistency across related files:
- FACE vertex indices must exist in VERT file
- Logged for debugging but not blocking

## File Format Specifications

### STL Format

**Binary STL:**
```
Bytes 0-79:    80-byte header
Bytes 80-83:   uint32 triangle count
Bytes 84+:     50 bytes per triangle
               - 12 bytes: normal vector (3 floats)
               - 36 bytes: vertices (9 floats)
               - 2 bytes:  attribute byte count
```

**ASCII STL:**
```
solid [name]
  facet normal nx ny nz
    outer loop
      vertex x1 y1 z1
      vertex x2 y2 z2
      vertex x3 y3 z3
    endloop
  endfacet
  ...
endsolid [name]
```

**Example:**
```stl
solid cube
  facet normal 0.0 0.0 1.0
    outer loop
      vertex 0.0 0.0 1.0
      vertex 1.0 0.0 1.0
      vertex 1.0 1.0 1.0
    endloop
  endfacet
  ...
endsolid cube
```

### VERT Format

**Specification:**
- Plain text file (UTF-8 or ASCII)
- One vertex per line
- Three space-separated float values: `X Y Z`
- Negative values allowed
- Scientific notation supported

**Example:**
```
0.0 0.0 0.0
1.234 5.678 9.012
-2.5 3.14159 -1.0
1.23e-4 5.67e2 9.01
```

**Validation:**
- Checks first 10 lines for format
- Counts total vertices
- Sanity check: max 100M vertices

### FACE Format

**Specification:**
- Plain text file (UTF-8 or ASCII)
- One triangle per line
- Three space-separated integer indices: `V1 V2 V3`
- Zero-indexed vertex references
- Non-negative values only

**Example:**
```
0 1 2
1 3 2
0 2 3
4 5 6
```

**Validation:**
- Checks first 10 lines for format
- Tracks maximum vertex index
- Sanity check: max 100M faces
- Cross-validates with VERT file

## Security Features

### Dangerous File Signatures

The system blocks files with these signatures:

| Signature | Type | Description |
|-----------|------|-------------|
| `MZ` | Executable | Windows .exe/.dll |
| `\x7fELF` | Executable | Linux ELF binary |
| `\xca\xfe\xba\xbe` | Executable | macOS Mach-O |
| `#!/` | Script | Shell/Python/etc script |
| `<?php` | Script | PHP script |
| `PK\x03\x04` | Archive | ZIP archive |
| `Rar!` | Archive | RAR archive |
| `\x1f\x8b` | Archive | GZIP archive |
| `\xd0\xcf\x11\xe0` | Office | MS Office (macro risk) |

### Filename Sanitization

Filenames are automatically sanitized:

**Removed Characters:**
- Path separators: `/` `\`
- Special characters: `<` `>` `:` `"` `|` `?` `*`
- Control characters: `\x00` (null byte)

**Other Sanitization:**
- Length limited to 255 characters
- Path components stripped
- Unicode normalization

**Examples:**
```python
"../../etc/passwd"        → "etc_passwd"
"file<test>.txt"          → "file_test_.txt"
"very_long_name..."       → "very_long_name...{truncated}.txt"
"\x00evil.txt"            → "_evil.txt"
```

## Usage Examples

### Basic Validation

```python
from fastapi import UploadFile
from app.core.file_validation import validate_file_comprehensive

@app.post("/upload")
async def upload_file(file: UploadFile):
    # Validate STL file with content checking
    result = await validate_file_comprehensive(
        file,
        expected_extension='.stl',
        validate_content=True
    )

    print(f"Format: {result['format']}")          # 'binary' or 'ascii'
    print(f"Triangles: {result['triangle_count']}")  # e.g., 1250
    print(f"Valid: {result['valid']}")            # True
```

### Validating Multiple Files

```python
# Validate all three files for Part One
stl_result = await validate_file_comprehensive(
    stl_file, '.stl', validate_content=True
)
vert_result = await validate_file_comprehensive(
    vert_file, '.vert', validate_content=True
)
face_result = await validate_file_comprehensive(
    face_file, '.face', validate_content=True
)

# Cross-validate FACE against VERT
if face_result['max_vertex_index'] >= vert_result['vertex_count']:
    raise ValidationException(
        f"FACE references vertex {face_result['max_vertex_index']}, "
        f"but VERT only has {vert_result['vertex_count']} vertices"
    )
```

### Filename Sanitization

```python
from app.core.file_validation import sanitize_filename

# Sanitize user-provided names
safe_name = sanitize_filename(protein_name)
file_path = upload_dir / f"{safe_name}.stl"
```

### Validation Without Content Check

For simple text files where structure validation isn't needed:

```python
# Basic validation only (faster)
result = await validate_file_comprehensive(
    txt_file,
    expected_extension='.txt',
    validate_content=False  # Skip format checking
)
```

### Custom Size Limits

```python
# Smaller limit for specific file
result = await validate_file_comprehensive(
    file,
    expected_extension='.stl',
    max_size=10 * 1024 * 1024,  # 10 MB
    validate_content=True
)
```

## Error Handling

### Validation Exceptions

All validation failures raise `ValidationException` with detailed messages:

```python
from app.core.exceptions import ValidationException

try:
    result = await validate_file_comprehensive(file, '.stl')
except ValidationException as e:
    # e.detail contains user-friendly error message
    print(f"Validation failed: {e.detail}")
    # Example: "Invalid STL: missing required keyword 'facet'"
```

### Common Error Messages

| Error | Cause | Solution |
|-------|-------|----------|
| "File too small to be valid" | STL < 84 bytes | Check file isn't corrupted |
| "Invalid file extension" | Wrong extension | Rename file with correct extension |
| "File too large: X MB. Maximum: Y MB" | Exceeds size limit | Compress or simplify model |
| "Invalid VERT format at line N" | Bad coordinate format | Check line has 3 numeric values |
| "FACE file references vertex index X" | Index out of bounds | Ensure FACE indices match VERT |
| "Forbidden file type detected" | Dangerous signature | File type not allowed for security |
| "Invalid filename: path traversal" | Contains `../` | Use simple filename |

### Validation Result Structure

Successful validation returns a dictionary:

```python
{
    "valid": True,
    "extension": ".stl",
    "filename": "protein_model.stl",

    # STL-specific
    "format": "binary",              # or "ascii"
    "triangle_count": 15234,

    # VERT-specific
    "vertex_count": 5000,

    # FACE-specific
    "face_count": 10000,
    "max_vertex_index": 4999
}
```

## Metrics & Monitoring

### Prometheus Metrics

The validation system exports metrics for monitoring:

**`file_validation_total{file_type, status}`**
- Counter of total validations
- Labels: `file_type` (stl/vert/face/txt), `status` (success/failed)

**`file_validation_failures_total{file_type, reason}`**
- Counter of validation failures
- Labels: `file_type`, `reason` (size/extension/content/mime/other)

**`file_validation_duration_seconds{file_type}`**
- Histogram of validation duration
- Labels: `file_type`

### Monitoring Validation Performance

**Check validation success rate:**
```promql
rate(file_validation_total{status="success"}[5m]) /
rate(file_validation_total[5m])
```

**Monitor failure reasons:**
```promql
rate(file_validation_failures_total[5m])
```

**Track validation latency:**
```promql
histogram_quantile(0.95, rate(file_validation_duration_seconds_bucket[5m]))
```

### Grafana Dashboards

Create panels for:
1. Validation success/failure rate
2. Validation duration (P50, P95, P99)
3. Failure reasons breakdown
4. Files validated per type

## Troubleshooting

### Issue: STL File Rejected as Invalid

**Symptoms:** "Invalid binary STL structure" error

**Solutions:**
1. Verify file is actually STL format
   ```bash
   file protein.stl
   # Should show: "protein.stl: data" or mention STL
   ```

2. Check first bytes
   ```bash
   hexdump -C protein.stl | head
   # Binary STL: various bytes for header
   # ASCII STL: should see "solid" in ASCII
   ```

3. Validate with external tool
   ```bash
   # Use meshlab, blender, or online STL validator
   ```

### Issue: VERT/FACE Format Errors

**Symptoms:** "Invalid VERT format at line N"

**Solutions:**
1. Check file encoding
   ```bash
   file -i vertices.vert
   # Should be: text/plain; charset=utf-8 or ascii
   ```

2. Inspect problematic line
   ```bash
   sed -n 'Np' vertices.vert  # Replace N with line number
   ```

3. Validate format manually
   ```python
   # Each VERT line should have exactly 3 numbers
   with open('vertices.vert') as f:
       for i, line in enumerate(f, 1):
           parts = line.strip().split()
           if len(parts) != 3:
               print(f"Line {i}: {len(parts)} values (expected 3)")
               print(f"Content: {line.strip()}")
   ```

### Issue: Cross-Validation Failure

**Symptoms:** "FACE references vertex X, but VERT only has Y vertices"

**Solutions:**
1. Check FACE indices are zero-based
   ```python
   # VERT with 3 vertices (indices 0, 1, 2)
   # FACE can use: 0 1 2, but NOT: 1 2 3
   ```

2. Count vertices in VERT
   ```bash
   wc -l vertices.vert
   ```

3. Find maximum index in FACE
   ```bash
   awk '{for(i=1;i<=NF;i++) if($i>max) max=$i} END{print max}' faces.face
   ```

### Issue: File Upload Slow

**Symptoms:** Validation takes > 5 seconds

**Solutions:**
1. Check file size
   ```bash
   ls -lh protein.stl
   ```

2. For very large files, consider:
   - Simplifying the mesh (reduce triangle count)
   - Using binary STL instead of ASCII
   - Compressing before upload (then decompress server-side)

3. Monitor validation metrics
   ```bash
   curl http://localhost:5000/metrics | grep file_validation_duration
   ```

### Issue: Filename Sanitization Unexpected

**Symptoms:** Saved filename different from uploaded

**Explanation:** This is intentional for security

**What Happens:**
```python
# Input:  "../../my_protein<test>.stl"
# Output: "my_protein_test_.stl"
```

**To Control:**
- Use simple filenames without special characters
- Avoid path separators (`/`, `\`)
- Keep names under 255 characters

## Best Practices

### For Users

1. **Use Standard Formats**
   - Export STL from CAD software using standard settings
   - Use ASCII text format for VERT/FACE files
   - Avoid proprietary or compressed formats

2. **Validate Locally First**
   - Use CAD software to verify STL integrity
   - Check VERT/FACE files open in text editor
   - Confirm file sizes are reasonable

3. **Name Files Safely**
   - Use alphanumeric characters, underscores, hyphens
   - Avoid spaces (use underscores instead)
   - Keep names under 100 characters

4. **Check Indices Match**
   - FACE indices should be 0-based
   - Maximum FACE index should be `vertex_count - 1`
   - Ensure files are for the same model

### For Developers

1. **Always Validate Before Processing**
   ```python
   # ✅ Good
   await validate_file_comprehensive(file, '.stl', validate_content=True)
   process_stl_file(file)

   # ❌ Bad
   process_stl_file(file)  # No validation!
   ```

2. **Handle Validation Errors Gracefully**
   ```python
   try:
       result = await validate_file_comprehensive(file, '.stl')
   except ValidationException as e:
       logger.error(f"Validation failed: {e.detail}")
       return {"error": e.detail}, 400
   ```

3. **Use Sanitized Filenames**
   ```python
   # ✅ Good
   safe_name = sanitize_filename(user_input)
   path = base_dir / safe_name

   # ❌ Bad
   path = base_dir / user_input  # Security risk!
   ```

4. **Log Validation Results**
   ```python
   logger.info(f"Validated {file.filename}: {result}")
   # Helps debugging and monitoring
   ```

## Configuration

### Environment Variables

Validation behavior can be configured via `.env`:

```bash
# Maximum file size (bytes)
MAX_FILE_SIZE_BYTES=104857600  # 100 MB

# Allowed file extensions
ALLOWED_FILE_EXTENSIONS=".stl,.vert,.face,.txt"
```

### Custom Validation

To add validation for new file types:

```python
# app/core/file_validation.py

async def validate_custom_file(file: UploadFile) -> Dict:
    """Validate custom file format"""
    content = await file.read()
    await file.seek(0)

    # Your validation logic here
    if not is_valid_format(content):
        raise ValidationException(detail="Invalid custom format")

    return {
        "valid": True,
        "custom_field": extract_metadata(content)
    }
```

## See Also

- [API Documentation](./API.md) - Upload endpoints
- [Observability Guide](./OBSERVABILITY.md) - Monitoring validation metrics
- [Testing Guide](./TESTING.md) - Testing validation logic

---

**Last Updated:** 2024-01-14
**Version:** 2.1.0
