"""
Tests for centroid_calculator.py - Triangle centroid calculation
"""
import pytest
import tempfile
import os
from app.algorithms.centroid_calculator import calculate_centroids, export_centroids


@pytest.fixture
def mock_vertices():
    """Mock vertex array (11 columns)"""
    return [
        ['', '0.0', '0.0', '0.0', '0', '0', '0', '0', '0', '0', '0'],  # index 0
        ['', '3.0', '0.0', '0.0', '0', '0', '0', '0', '0', '0', '0'],  # index 1
        ['', '0.0', '3.0', '0.0', '0', '0', '0', '0', '0', '0', '0'],  # index 2
        ['', '0.0', '0.0', '3.0', '0', '0', '0', '0', '0', '0', '0'],  # index 3
    ]


@pytest.fixture
def mock_faces():
    """Mock face array (6 columns, 1-indexed)"""
    return [
        ['', '1', '2', '3', '2', '0'],  # type 2 (should process)
        ['', '2', '3', '4', '1', '0'],  # type 1 (should skip)
        ['', '1', '2', '4', '3', '0'],  # type 3 (should process)
    ]


@pytest.mark.unit
@pytest.mark.algorithms
class TestCentroidCalculation:
    """Tests for triangle centroid calculation"""

    def test_calculate_centroids_basic(self, mock_vertices, mock_faces):
        """Test basic centroid calculation"""
        centros, centroids = calculate_centroids(mock_vertices, mock_faces)

        # Should process 2 faces (skip type 1)
        assert len(centros) == 2
        assert len(centroids) == 2

    def test_centroid_coordinates(self, mock_vertices, mock_faces):
        """Test centroid coordinates are correct"""
        centros, _ = calculate_centroids(mock_vertices, mock_faces)

        # First face: vertices (0,0,0), (3,0,0), (0,3,0)
        # Centroid: (1, 1, 0)
        assert abs(centros[0][0] - 1.0) < 0.001
        assert abs(centros[0][1] - 1.0) < 0.001
        assert abs(centros[0][2] - 0.0) < 0.001

    def test_skip_type_1_faces(self, mock_vertices, mock_faces):
        """Test that type 1 faces are skipped"""
        # mock_faces has 3 faces, but 1 is type 1
        centros, _ = calculate_centroids(mock_vertices, mock_faces)

        assert len(centros) == 2  # Only 2 processed

    def test_centroid_string_format(self, mock_vertices, mock_faces):
        """Test centroid strings are formatted correctly"""
        _, centroids = calculate_centroids(mock_vertices, mock_faces)

        # Each should be "x y z" format
        for centroid_str in centroids:
            parts = centroid_str.split()
            assert len(parts) == 3
            # Verify they're valid floats
            float(parts[0])
            float(parts[1])
            float(parts[2])

    def test_centroid_average_calculation(self):
        """Test centroid is average of three vertices"""
        vertices = [
            ['', '1.0', '2.0', '3.0', '0', '0', '0', '0', '0', '0', '0'],
            ['', '4.0', '5.0', '6.0', '0', '0', '0', '0', '0', '0', '0'],
            ['', '7.0', '8.0', '9.0', '0', '0', '0', '0', '0', '0', '0'],
        ]
        faces = [
            ['', '1', '2', '3', '2', '0'],  # type 2
        ]

        centros, _ = calculate_centroids(vertices, faces)

        # Centroid should be average: (4, 5, 6)
        assert abs(centros[0][0] - 4.0) < 0.001
        assert abs(centros[0][1] - 5.0) < 0.001
        assert abs(centros[0][2] - 6.0) < 0.001

    def test_empty_faces(self, mock_vertices):
        """Test with no faces"""
        centros, centroids = calculate_centroids(mock_vertices, [])

        assert len(centros) == 0
        assert len(centroids) == 0

    def test_all_type_1_faces(self, mock_vertices):
        """Test when all faces are type 1 (all skipped)"""
        faces = [
            ['', '1', '2', '3', '1', '0'],
            ['', '2', '3', '4', '1', '0'],
        ]

        centros, centroids = calculate_centroids(mock_vertices, faces)

        assert len(centros) == 0
        assert len(centroids) == 0


@pytest.mark.unit
@pytest.mark.algorithms
class TestCentroidExport:
    """Tests for centroid export functionality"""

    def test_export_centroids(self):
        """Test exporting centroids to file"""
        centroids = [
            "1.0 2.0 3.0",
            "4.0 5.0 6.0",
            "7.0 8.0 9.0",
        ]

        with tempfile.NamedTemporaryFile(mode='w', suffix='.txt', delete=False) as f:
            output_file = f.name

        try:
            export_centroids(centroids, output_file)

            # Verify file contents
            with open(output_file, 'r') as f:
                lines = f.readlines()

            assert len(lines) == 3
            assert lines[0].strip() == "1.0 2.0 3.0"
            assert lines[1].strip() == "4.0 5.0 6.0"
            assert lines[2].strip() == "7.0 8.0 9.0"
        finally:
            os.unlink(output_file)

    def test_export_empty_centroids(self):
        """Test exporting empty centroid list"""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.txt', delete=False) as f:
            output_file = f.name

        try:
            export_centroids([], output_file)

            # File should exist but be empty
            with open(output_file, 'r') as f:
                content = f.read()

            assert content == ""
        finally:
            os.unlink(output_file)

    def test_export_invalid_path(self):
        """Test export with invalid file path"""
        centroids = ["1.0 2.0 3.0"]

        with pytest.raises(Exception):
            export_centroids(centroids, "/invalid/path/file.txt")
