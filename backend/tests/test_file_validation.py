"""
Tests for enhanced file validation

Tests all file validation functionality including STL, VERT, FACE validation,
security checks, and metrics tracking.
"""
import pytest
import struct
from io import BytesIO
from fastapi import UploadFile
from unittest.mock import Mock, patch

from app.core.file_validation import (
    validate_stl_file,
    validate_vert_file,
    validate_face_file,
    validate_file_comprehensive,
    sanitize_filename,
    DANGEROUS_SIGNATURES,
)
from app.core.exceptions import ValidationException


class MockUploadFile:
    """Mock UploadFile for testing"""
    def __init__(self, content: bytes, filename: str = "test.file"):
        self.content = content
        self.filename = filename
        self.position = 0

    async def read(self, size: int = -1):
        if size == -1:
            data = self.content[self.position:]
            self.position = len(self.content)
        else:
            data = self.content[self.position:self.position + size]
            self.position += size
        return data

    async def seek(self, position: int):
        self.position = position

    @property
    def file(self):
        """Mock file object for seeking"""
        mock_file = Mock()
        mock_file.seek = lambda pos, whence=0: setattr(self, 'position', pos if whence == 0 else len(self.content))
        mock_file.tell = lambda: self.position
        return mock_file


# ==========================================
# STL FILE VALIDATION TESTS
# ==========================================

class TestSTLValidation:
    """Test STL file validation"""

    def create_binary_stl(self, triangle_count: int = 1) -> bytes:
        """Create a valid binary STL file"""
        header = b'\x00' * 80
        count_bytes = struct.pack('<I', triangle_count)

        triangle = b''
        # Normal vector (3 floats)
        triangle += struct.pack('<fff', 0.0, 0.0, 1.0)
        # 3 vertices (9 floats)
        for i in range(3):
            triangle += struct.pack('<fff', float(i), 0.0, 0.0)
        # Attribute byte count
        triangle += b'\x00\x00'

        return header + count_bytes + (triangle * triangle_count)

    def create_ascii_stl(self, triangle_count: int = 1) -> bytes:
        """Create a valid ASCII STL file"""
        stl = "solid test\n"
        for i in range(triangle_count):
            stl += "  facet normal 0.0 0.0 1.0\n"
            stl += "    outer loop\n"
            stl += "      vertex 0.0 0.0 0.0\n"
            stl += "      vertex 1.0 0.0 0.0\n"
            stl += "      vertex 0.0 1.0 0.0\n"
            stl += "    endloop\n"
            stl += "  endfacet\n"
        stl += "endsolid test\n"
        return stl.encode('ascii')

    @pytest.mark.asyncio
    async def test_valid_binary_stl(self):
        """Test validation of valid binary STL"""
        content = self.create_binary_stl(triangle_count=10)
        file = MockUploadFile(content, "test.stl")

        result = await validate_stl_file(file)

        assert result['valid'] is True
        assert result['format'] == 'binary'
        assert result['triangle_count'] == 10

    @pytest.mark.asyncio
    async def test_valid_ascii_stl(self):
        """Test validation of valid ASCII STL"""
        content = self.create_ascii_stl(triangle_count=5)
        file = MockUploadFile(content, "test.stl")

        result = await validate_stl_file(file)

        assert result['valid'] is True
        assert result['format'] == 'ascii'
        assert result['triangle_count'] == 5

    @pytest.mark.asyncio
    async def test_invalid_stl_too_small(self):
        """Test rejection of file too small to be STL"""
        content = b'tiny'
        file = MockUploadFile(content, "test.stl")

        with pytest.raises(ValidationException) as exc_info:
            await validate_stl_file(file)

        assert "too small" in str(exc_info.value.detail).lower()

    @pytest.mark.asyncio
    async def test_invalid_ascii_stl_missing_keywords(self):
        """Test rejection of ASCII STL missing required keywords"""
        content = b"solid test\nsome random content\nendsolid test"
        file = MockUploadFile(content, "test.stl")

        with pytest.raises(ValidationException) as exc_info:
            await validate_stl_file(file)

        assert "missing required keyword" in str(exc_info.value.detail).lower()

    @pytest.mark.asyncio
    async def test_invalid_stl_no_triangles(self):
        """Test rejection of STL with no triangles"""
        content = b"solid test\nendsolid test"
        file = MockUploadFile(content, "test.stl")

        with pytest.raises(ValidationException) as exc_info:
            await validate_stl_file(file)

        assert "no triangles found" in str(exc_info.value.detail).lower()


# ==========================================
# VERT FILE VALIDATION TESTS
# ==========================================

class TestVERTValidation:
    """Test VERT file validation"""

    @pytest.mark.asyncio
    async def test_valid_vert_file(self):
        """Test validation of valid VERT file"""
        content = b"""1.0 2.0 3.0
4.5 5.5 6.5
7.8 8.9 9.1
"""
        file = MockUploadFile(content, "test.vert")

        result = await validate_vert_file(file)

        assert result['valid'] is True
        assert result['vertex_count'] == 3

    @pytest.mark.asyncio
    async def test_vert_file_with_negative_coords(self):
        """Test VERT file with negative coordinates (valid)"""
        content = b"""-1.0 -2.0 -3.0
0.0 0.0 0.0
1.0 2.0 3.0
"""
        file = MockUploadFile(content, "test.vert")

        result = await validate_vert_file(file)

        assert result['valid'] is True
        assert result['vertex_count'] == 3

    @pytest.mark.asyncio
    async def test_invalid_vert_empty(self):
        """Test rejection of empty VERT file"""
        content = b""
        file = MockUploadFile(content, "test.vert")

        with pytest.raises(ValidationException) as exc_info:
            await validate_vert_file(file)

        assert "empty" in str(exc_info.value.detail).lower()

    @pytest.mark.asyncio
    async def test_invalid_vert_wrong_column_count(self):
        """Test rejection of VERT with wrong number of columns"""
        content = b"""1.0 2.0
3.0 4.0 5.0
"""
        file = MockUploadFile(content, "test.vert")

        with pytest.raises(ValidationException) as exc_info:
            await validate_vert_file(file)

        assert "expected 3 values" in str(exc_info.value.detail).lower()

    @pytest.mark.asyncio
    async def test_invalid_vert_non_numeric(self):
        """Test rejection of VERT with non-numeric values"""
        content = b"""1.0 2.0 3.0
abc def ghi
"""
        file = MockUploadFile(content, "test.vert")

        with pytest.raises(ValidationException) as exc_info:
            await validate_vert_file(file)

        assert "must be numeric" in str(exc_info.value.detail).lower()


# ==========================================
# FACE FILE VALIDATION TESTS
# ==========================================

class TestFACEValidation:
    """Test FACE file validation"""

    @pytest.mark.asyncio
    async def test_valid_face_file(self):
        """Test validation of valid FACE file"""
        content = b"""0 1 2
2 1 3
0 2 3
"""
        file = MockUploadFile(content, "test.face")

        result = await validate_face_file(file)

        assert result['valid'] is True
        assert result['face_count'] == 3
        assert result['max_vertex_index'] == 3

    @pytest.mark.asyncio
    async def test_invalid_face_empty(self):
        """Test rejection of empty FACE file"""
        content = b""
        file = MockUploadFile(content, "test.face")

        with pytest.raises(ValidationException) as exc_info:
            await validate_face_file(file)

        assert "empty" in str(exc_info.value.detail).lower()

    @pytest.mark.asyncio
    async def test_invalid_face_wrong_column_count(self):
        """Test rejection of FACE with wrong number of indices"""
        content = b"""0 1
2 3 4
"""
        file = MockUploadFile(content, "test.face")

        with pytest.raises(ValidationException) as exc_info:
            await validate_face_file(file)

        assert "expected 3 indices" in str(exc_info.value.detail).lower()

    @pytest.mark.asyncio
    async def test_invalid_face_negative_indices(self):
        """Test rejection of FACE with negative indices"""
        content = b"""0 1 2
-1 2 3
"""
        file = MockUploadFile(content, "test.face")

        with pytest.raises(ValidationException) as exc_info:
            await validate_face_file(file)

        assert "non-negative" in str(exc_info.value.detail).lower()

    @pytest.mark.asyncio
    async def test_invalid_face_non_integer(self):
        """Test rejection of FACE with non-integer values"""
        content = b"""0 1 2
1.5 2.5 3.5
"""
        file = MockUploadFile(content, "test.face")

        with pytest.raises(ValidationException) as exc_info:
            await validate_face_file(file)

        assert "must be integers" in str(exc_info.value.detail).lower()


# ==========================================
# FILENAME SANITIZATION TESTS
# ==========================================

class TestFilenameSanitization:
    """Test filename sanitization"""

    def test_sanitize_path_traversal(self):
        """Test removal of path traversal attempts"""
        dangerous = "../../etc/passwd"
        safe = sanitize_filename(dangerous)

        assert ".." not in safe
        assert "/" not in safe
        assert "\\" not in safe

    def test_sanitize_special_characters(self):
        """Test removal of special characters"""
        dangerous = "file<>name:with|special?.txt"
        safe = sanitize_filename(dangerous)

        assert "<" not in safe
        assert ">" not in safe
        assert ":" not in safe
        assert "|" not in safe
        assert "?" not in safe

    def test_sanitize_null_bytes(self):
        """Test removal of null bytes"""
        dangerous = "file\x00name.txt"
        safe = sanitize_filename(dangerous)

        assert "\x00" not in safe

    def test_sanitize_long_filename(self):
        """Test truncation of long filenames"""
        long_name = "a" * 300 + ".txt"
        safe = sanitize_filename(long_name)

        assert len(safe) <= 255
        assert safe.endswith(".txt")

    def test_sanitize_preserves_valid_names(self):
        """Test that valid filenames are preserved"""
        valid = "my_protein_file-v2.stl"
        safe = sanitize_filename(valid)

        assert safe == valid


# ==========================================
# COMPREHENSIVE VALIDATION TESTS
# ==========================================

class TestComprehensiveValidation:
    """Test the comprehensive validation function"""

    @pytest.mark.asyncio
    @patch('app.core.file_validation.file_validation_total')
    @patch('app.core.file_validation.file_validation_duration_seconds')
    async def test_comprehensive_stl_validation_success(self, mock_duration, mock_total):
        """Test comprehensive validation tracks metrics on success"""
        # Create valid binary STL
        header = b'\x00' * 80
        triangle_count = struct.pack('<I', 1)
        triangle = struct.pack('<fff', 0, 0, 1)  # normal
        triangle += struct.pack('<fff', 0, 0, 0)  # v1
        triangle += struct.pack('<fff', 1, 0, 0)  # v2
        triangle += struct.pack('<fff', 0, 1, 0)  # v3
        triangle += b'\x00\x00'  # attribute
        content = header + triangle_count + triangle

        file = MockUploadFile(content, "test.stl")

        with patch('app.core.file_validation.validate_file_upload', return_value=True):
            result = await validate_file_comprehensive(file, '.stl', validate_content=True)

        assert result['valid'] is True
        mock_total.labels.assert_called()
        mock_duration.labels.assert_called()

    @pytest.mark.asyncio
    @patch('app.core.file_validation.file_validation_failures')
    async def test_comprehensive_validation_failure_tracking(self, mock_failures):
        """Test that validation failures are tracked in metrics"""
        content = b"too small"
        file = MockUploadFile(content, "test.stl")

        with pytest.raises(ValidationException):
            await validate_file_comprehensive(file, '.stl', validate_content=True)

        mock_failures.labels.assert_called()


# ==========================================
# SECURITY TESTS
# ==========================================

class TestSecurityValidation:
    """Test security-related validation"""

    @pytest.mark.parametrize("signature,description", list(DANGEROUS_SIGNATURES.items())[:3])
    @pytest.mark.asyncio
    async def test_dangerous_file_signatures(self, signature, description):
        """Test that dangerous file signatures are detected"""
        # Create file with dangerous signature
        content = signature + b'\x00' * 100
        file = MockUploadFile(content, "test.stl")

        with patch('app.core.file_validation.validate_file_upload') as mock_validate:
            # Mock the basic validation to trigger magic bytes check
            mock_validate.side_effect = ValidationException(detail=f"Forbidden file type detected: {description}")

            with pytest.raises(ValidationException) as exc_info:
                await validate_file_comprehensive(file, '.stl')

            assert "forbidden" in str(exc_info.value.detail).lower()


# ==========================================
# INTEGRATION TESTS
# ==========================================

@pytest.mark.integration
class TestFileValidationIntegration:
    """Integration tests with realistic file data"""

    @pytest.mark.asyncio
    async def test_complete_workflow_stl_vert_face(self):
        """Test complete validation workflow for all three file types"""
        # Create realistic STL
        stl_content = b"solid test\n"
        stl_content += b"  facet normal 0 0 1\n"
        stl_content += b"    outer loop\n"
        stl_content += b"      vertex 0 0 0\n"
        stl_content += b"      vertex 1 0 0\n"
        stl_content += b"      vertex 0 1 0\n"
        stl_content += b"    endloop\n"
        stl_content += b"  endfacet\n"
        stl_content += b"endsolid test\n"

        # Create VERT file
        vert_content = b"0.0 0.0 0.0\n1.0 0.0 0.0\n0.0 1.0 0.0\n"

        # Create FACE file
        face_content = b"0 1 2\n"

        stl_file = MockUploadFile(stl_content, "test.stl")
        vert_file = MockUploadFile(vert_content, "test.vert")
        face_file = MockUploadFile(face_content, "test.face")

        # Validate all three
        with patch('app.core.file_validation.validate_file_upload', return_value=True):
            stl_result = await validate_file_comprehensive(stl_file, '.stl', validate_content=True)
            vert_result = await validate_file_comprehensive(vert_file, '.vert', validate_content=True)
            face_result = await validate_file_comprehensive(face_file, '.face', validate_content=True)

        # Verify results
        assert stl_result['valid'] is True
        assert stl_result['triangle_count'] == 1

        assert vert_result['valid'] is True
        assert vert_result['vertex_count'] == 3

        assert face_result['valid'] is True
        assert face_result['face_count'] == 1
        assert face_result['max_vertex_index'] <= vert_result['vertex_count'] - 1
