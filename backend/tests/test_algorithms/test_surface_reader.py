"""
Tests for surface_reader.py - MSMS file parser
"""
import pytest
import tempfile
import os
from app.algorithms.surface_reader import read_surface_files


@pytest.fixture
def mock_vert_file():
    """Create a mock MSMS vertices file"""
    content = """# Header line 1
# Header line 2
# Header line 3
1.0 2.0 3.0 0.1 0.2 0.3 0.5 0 1 0 0
4.0 5.0 6.0 0.4 0.5 0.6 0.8 0 1 0 0
7.0 8.0 9.0 0.7 0.8 0.9 0.3 0 2 0 0
"""
    with tempfile.NamedTemporaryFile(mode='w', suffix='.vert', delete=False) as f:
        f.write(content)
        f.flush()
        yield f.name
    os.unlink(f.name)


@pytest.fixture
def mock_face_file():
    """Create a mock MSMS faces file"""
    content = """# Header line 1
# Header line 2
# Header line 3
1 2 3 1 0 0
2 3 4 1 0 0
3 4 5 2 0 0
"""
    with tempfile.NamedTemporaryFile(mode='w', suffix='.face', delete=False) as f:
        f.write(content)
        f.flush()
        yield f.name
    os.unlink(f.name)


@pytest.mark.unit
@pytest.mark.algorithms
class TestSurfaceReader:
    """Tests for MSMS surface file reader"""

    def test_read_valid_files(self, mock_vert_file, mock_face_file):
        """Test reading valid MSMS files"""
        vertices, faces = read_surface_files(mock_vert_file, mock_face_file)

        # Verify vertices
        assert len(vertices) == 3
        assert len(vertices[0]) == 11  # 11 columns per vertex
        assert vertices[0][0] == '1.0'  # x coordinate
        assert vertices[0][1] == '2.0'  # y coordinate
        assert vertices[0][2] == '3.0'  # z coordinate

        # Verify faces
        assert len(faces) == 3
        assert len(faces[0]) == 6  # 6 columns per face
        assert faces[0][0] == '1'  # vertex index 1
        assert faces[0][1] == '2'  # vertex index 2
        assert faces[0][2] == '3'  # vertex index 3

    def test_vertices_have_all_fields(self, mock_vert_file, mock_face_file):
        """Test that vertices contain all 11 fields"""
        vertices, _ = read_surface_files(mock_vert_file, mock_face_file)

        for vertex in vertices:
            assert len(vertex) == 11
            # Check coordinate fields are parseable as floats
            assert float(vertex[0])  # x
            assert float(vertex[1])  # y
            assert float(vertex[2])  # z

    def test_faces_reference_vertices(self, mock_vert_file, mock_face_file):
        """Test that face vertex indices are valid integers"""
        _, faces = read_surface_files(mock_vert_file, mock_face_file)

        for face in faces:
            # First 3 columns are vertex indices
            assert int(face[0])
            assert int(face[1])
            assert int(face[2])

    def test_nonexistent_vertices_file(self, mock_face_file):
        """Test error handling for missing vertices file"""
        with pytest.raises(FileNotFoundError):
            read_surface_files("nonexistent.vert", mock_face_file)

    def test_nonexistent_faces_file(self, mock_vert_file):
        """Test error handling for missing faces file"""
        with pytest.raises(FileNotFoundError):
            read_surface_files(mock_vert_file, "nonexistent.face")

    def test_empty_vert_file(self, mock_face_file):
        """Test handling of empty vertices file"""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.vert', delete=False) as f:
            f.write("# Header 1\n# Header 2\n# Header 3\n")
            f.flush()
            empty_vert = f.name

        try:
            vertices, _ = read_surface_files(empty_vert, mock_face_file)
            assert len(vertices) == 0
        finally:
            os.unlink(empty_vert)

    def test_malformed_line_padding(self, mock_face_file):
        """Test that short lines are padded correctly"""
        content = """# Header 1
# Header 2
# Header 3
1.0 2.0 3.0
"""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.vert', delete=False) as f:
            f.write(content)
            f.flush()
            malformed_vert = f.name

        try:
            vertices, _ = read_surface_files(malformed_vert, mock_face_file)
            # Should pad to 11 columns
            assert len(vertices[0]) == 11
        finally:
            os.unlink(malformed_vert)

    def test_multiple_spaces_separator(self, mock_face_file):
        """Test parsing with multiple spaces as separator"""
        content = """# Header 1
# Header 2
# Header 3
1.0    2.0     3.0   0.1  0.2  0.3  0.5  0  1  0  0
"""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.vert', delete=False) as f:
            f.write(content)
            f.flush()
            spaced_vert = f.name

        try:
            vertices, _ = read_surface_files(spaced_vert, mock_face_file)
            assert len(vertices) == 1
            assert vertices[0][0] == '1.0'
            assert vertices[0][1] == '2.0'
        finally:
            os.unlink(spaced_vert)
