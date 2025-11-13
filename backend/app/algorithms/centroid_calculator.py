"""
Centroid calculator (migrated from Script02_calculo_centroides.py)
Calculates triangle centroids from vertices and faces
"""
import numpy as np
from typing import List
from app.core.logging import get_logger

logger = get_logger(__name__)


def calculate_centroids(vertices: np.ndarray, faces: np.ndarray) -> np.ndarray:
    """
    Calculate centroids for each triangular face

    Args:
        vertices: Array of vertex coordinates (Nx3)
        faces: Array of face indices (Mx3)

    Returns:
        Array of centroid coordinates (Mx3)

    TODO: Migrate complete logic from Backend/C-lculos-Previos-main/.../Script02_calculo_centroides.py
    """
    logger.info(f"Calculating centroids for {len(faces)} faces")

    centroids = []
    for face in faces:
        v1, v2, v3 = vertices[face[0]], vertices[face[1]], vertices[face[2]]
        # Centroid is the average of the three vertices
        centroid = (v1 + v2 + v3) / 3.0
        centroids.append(centroid)

    centroids_array = np.array(centroids)
    logger.info(f"Calculated {len(centroids_array)} centroids")

    return centroids_array
