"""
Surface file reader (migrated from Script01_lectura_caras_vertices.py)
Reads MSMS vertex and face files
"""
import numpy as np
from typing import Tuple
from app.core.logging import get_logger

logger = get_logger(__name__)


def read_surface_files(vertices_file: str, faces_file: str) -> Tuple[np.ndarray, np.ndarray]:
    """
    Read MSMS vertex and face files

    Args:
        vertices_file: Path to .vert file
        faces_file: Path to .face file

    Returns:
        Tuple of (vertices, faces) as numpy arrays

    TODO: Migrate complete logic from Backend/C-lculos-Previos-main/.../Script01_lectura_caras_vertices.py
    """
    logger.info(f"Reading vertices from: {vertices_file}")
    logger.info(f"Reading faces from: {faces_file}")

    # Read vertices (.vert file)
    # Format: x, y, z, nx, ny, nz, area, unused, face_type
    vertices_data = []
    with open(vertices_file, 'r') as f:
        for line in f:
            if line.strip() and not line.startswith('#'):
                parts = line.split()
                if len(parts) >= 3:
                    x, y, z = float(parts[0]), float(parts[1]), float(parts[2])
                    vertices_data.append([x, y, z])

    vertices = np.array(vertices_data)

    # Read faces (.face file)
    # Format: v1, v2, v3, face_type, unused
    faces_data = []
    with open(faces_file, 'r') as f:
        for line in f:
            if line.strip() and not line.startswith('#'):
                parts = line.split()
                if len(parts) >= 3:
                    # Convert to 0-indexed
                    v1 = int(parts[0]) - 1
                    v2 = int(parts[1]) - 1
                    v3 = int(parts[2]) - 1
                    faces_data.append([v1, v2, v3])

    faces = np.array(faces_data)

    logger.info(f"Loaded {len(vertices)} vertices and {len(faces)} faces")

    return vertices, faces
