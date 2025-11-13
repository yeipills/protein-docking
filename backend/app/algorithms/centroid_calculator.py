"""
Centroid calculator (migrated from Script02_calculo_centroides.py)
Calculates triangle centroids from vertices and faces
"""
from typing import List, Tuple
from app.core.logging import get_logger

logger = get_logger(__name__)


def calculate_centroids(
    arr_vert: List[List[str]],
    arr_face: List[List[str]]
) -> Tuple[List[List[float]], List[str]]:
    """
    Calculate centroids for triangular faces from MSMS data

    Only processes faces where face_type != 1 (excludes certain face types)
    Centroid is calculated as the average of the three vertex coordinates

    Args:
        arr_vert: Array of vertices from read_surface_files (11 columns)
                  Format: [x, y, z, nx, ny, nz, area, unused, face_type, ...]
        arr_face: Array of faces from read_surface_files (6 columns)
                  Format: [v1_index, v2_index, v3_index, face_type, unused, ...]

    Returns:
        Tuple of (centroids_list, centroids_strings)
        - centroids_list: List of [x, y, z] coordinates
        - centroids_strings: List of "x y z" formatted strings

    Note: face_type == 1 faces are skipped (only reentrant or contact patches)
    """
    logger.info("Starting centroid calculation")
    logger.info(f"Processing {len(arr_vert)} vertices and {len(arr_face)} faces")

    # Extract vertex coordinates (columns 1, 2, 3 = x, y, z)
    vertices = []
    for i in range(len(arr_vert)):
        fila = arr_vert[i]
        # Extract x, y, z coordinates
        x = float(fila[1])
        y = float(fila[2])
        z = float(fila[3])
        vertices.append([x, y, z])

    logger.info(f"Extracted {len(vertices)} vertex coordinates")

    # Calculate centroids for each face
    centroids = []  # String format for export
    centros = []    # Float format for computation

    skipped_count = 0
    for i in range(len(arr_face)):
        fila = arr_face[i]

        # Get face type (column 4)
        type_face = float(fila[4])

        # Only process faces where type_face != 1
        if type_face != 1:
            # Get vertex indices (MSMS uses 1-based indexing)
            indvert1 = int(fila[1]) - 1
            indvert2 = int(fila[2]) - 1
            indvert3 = int(fila[3]) - 1

            # Get the three vertices
            vertice1 = vertices[indvert1]
            vertice2 = vertices[indvert2]
            vertice3 = vertices[indvert3]

            # Calculate centroid as average
            centroX = (float(vertice1[0]) + float(vertice2[0]) + float(vertice3[0])) / 3
            centroY = (float(vertice1[1]) + float(vertice2[1]) + float(vertice3[1])) / 3
            centroZ = (float(vertice1[2]) + float(vertice2[2]) + float(vertice3[2])) / 3

            centroids.append(f"{centroX} {centroY} {centroZ}")
            centros.append([centroX, centroY, centroZ])
        else:
            skipped_count += 1

    logger.info(f"Calculated {len(centros)} centroids")
    logger.info(f"Skipped {skipped_count} faces with type_face == 1")
    logger.info("Centroid calculation complete")

    return centros, centroids


def export_centroids(centroids: List[str], output_file: str) -> None:
    """
    Export centroids to text file for Unity visualization

    Args:
        centroids: List of centroid strings in "x y z" format
        output_file: Path to output file
    """
    logger.info(f"Exporting centroids to: {output_file}")

    try:
        with open(output_file, 'w') as f:
            for item in centroids:
                f.write(f"{item}\n")

        logger.info(f"Centroids exported successfully: {len(centroids)} entries")
    except Exception as e:
        logger.error(f"Error exporting centroids: {str(e)}")
        raise
