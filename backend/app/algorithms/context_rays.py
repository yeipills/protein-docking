"""
Context rays calculator (migrated from Script03_rayos_contexto.py)
Calculates context rays using spherical sampling and ray-mesh intersection

This is the most computationally intensive algorithm in the pipeline.
Performance Note: This Python implementation works but is slower than Cython version.
For production with large proteins, consider Cython optimization.
"""
import numpy as np
import trimesh
import math
from pathlib import Path
from typing import List, Tuple
from scipy.spatial import cKDTree
from app.core.logging import get_logger
from app.config import get_settings

logger = get_logger(__name__)
settings = get_settings()


def calculate_context_rays(
    protein_name: str,
    stl_file: str,
    arr_vert: List[List[str]],
    arr_face: List[List[str]],
    centroids: List[List[float]],
    output_dir: str,
    radius: int = None,
    delta: int = None,
    n_segments: int = 17,
    max_distance_filter: float = 10.0
) -> Tuple[str, str]:
    """
    Calculate context rays (CR) for protein surface using spherical sampling

    This function implements the core context-based protein docking algorithm:
    1. Loads protein 3D mesh from STL
    2. Filters centroids to reduce computation (removes nearby points)
    3. For each centroid, generates a sphere of ray samples
    4. Divides each ray into segments and checks mesh intersection
    5. Exports results for layer evaluation

    Args:
        protein_name: Name of the protein (for output files)
        stl_file: Path to STL mesh file
        arr_vert: Vertices array from read_surface_files
        arr_face: Faces array from read_surface_files
        centroids: List of centroid coordinates [x,y,z]
        output_dir: Directory to save output files
        radius: Sphere radius for CR sampling (default from settings)
        delta: Grid size for spherical sampling (delta x delta rays)
        n_segments: Number of segments to divide each ray
        max_distance_filter: Max distance for centroid filtering (Angstroms)

    Returns:
        Tuple of (cr_totals_file_path, context_rays_file_path)

    Performance:
        - ~10-30 minutes for typical protein (pure Python)
        - Can be optimized 6-7x with Cython (see Script03.pyx)
        - Bottleneck: ray-mesh intersections
    """
    logger.info(f"=" * 60)
    logger.info(f"Starting Context Rays calculation for: {protein_name}")
    logger.info(f"=" * 60)

    # Use settings defaults if not provided
    if radius is None:
        radius = settings.CONTEXT_RAYS_RADIUS
    if delta is None:
        delta = settings.CONTEXT_RAYS_DELTA

    logger.info(f"Parameters:")
    logger.info(f"  Radius: {radius} Angstroms")
    logger.info(f"  Delta (grid size): {delta} x {delta} = {delta*delta} rays/centroid")
    logger.info(f"  Segments per ray: {n_segments}")
    logger.info(f"  Max filter distance: {max_distance_filter} Angstroms")

    # Step 1: Load STL mesh
    logger.info(f"Loading STL mesh from: {stl_file}")
    try:
        mesh_SES = trimesh.load(stl_file)
        logger.info(f"Mesh loaded successfully: {len(mesh_SES.vertices)} vertices, {len(mesh_SES.faces)} faces")
    except Exception as e:
        logger.error(f"Error loading STL file: {str(e)}")
        raise

    # Step 2: Filter centroids to reduce computational load
    logger.info(f"Filtering centroids (maxdist={max_distance_filter}Å)")
    logger.info(f"Initial centroids: {len(centroids)}")

    filtered_centroids = filter_centroids(centroids, max_distance_filter)

    logger.info(f"Filtered centroids: {len(filtered_centroids)}")
    logger.info(f"Reduction: {len(centroids) - len(filtered_centroids)} centroids removed")

    # Step 3: Compute context rays for filtered centroids
    logger.info(f"Computing context rays...")
    logger.info(f"Total rays to generate: {len(filtered_centroids) * delta * delta}")

    cr_data, rayos_contexto = compute_CR(radius, delta, filtered_centroids)

    logger.info(f"Context rays generated: {len(rayos_contexto)} rays")

    # Step 4: Evaluate ray-mesh intersections
    logger.info(f"Evaluating ray-mesh intersections (this may take several minutes)...")

    cr_segments = evaluate_ray_intersections(
        cr_data,
        mesh_SES,
        n_segments
    )

    logger.info(f"Ray intersections evaluated: {len(cr_segments)} segments")

    # Step 5: Export results
    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)

    cr_totals_file = str(output_path / f"{protein_name}_CRtotales.txt")
    context_rays_file = str(output_path / f"{protein_name}_rayos_contexto.txt")

    logger.info(f"Exporting results...")
    logger.info(f"  CR totals: {cr_totals_file}")
    logger.info(f"  Context rays: {context_rays_file}")

    # Export CR segments (with intersection data)
    with open(cr_totals_file, 'w') as f:
        for item in cr_segments:
            f.write(f"{item}\n")

    # Export raw ray data
    with open(context_rays_file, 'w') as f:
        for item in rayos_contexto:
            f.write(f"{item}\n")

    logger.info(f"Context rays calculation complete!")
    logger.info(f"=" * 60)

    return cr_totals_file, context_rays_file


def filter_centroids(centroids: List[List[float]], max_distance: float) -> List[List[float]]:
    """
    Filter centroids to remove those that are too close together
    Reduces computational load while maintaining surface coverage

    Uses cKDTree for efficient nearest neighbor search
    Marks centroids within max_distance of each other and keeps only one

    Args:
        centroids: List of [x, y, z] coordinates
        max_distance: Maximum distance threshold (Angstroms)

    Returns:
        Filtered list of centroids
    """
    cp = []
    centroids_copy = centroids.copy()

    # Placeholder value for removed centroids
    REMOVED_MARKER = [1000, 1000, 1000]

    for item in centroids_copy:
        # Skip if already marked as removed
        if int(item[0]) == 1000:
            continue

        # Find nearby points using KD-tree
        kd = cKDTree(centroids_copy)
        distances, indices = kd.query(item, k=100)

        # Find positions within max distance
        nearby_indices = [i for i, d in enumerate(distances) if d < max_distance]
        nearby_positions = [indices[i] for i in nearby_indices]

        # Keep the first point, mark others as removed
        cp.append(centroids_copy[nearby_positions[0]])

        for idx in nearby_positions:
            centroids_copy[idx] = REMOVED_MARKER

    return cp


def compute_CR(
    radius: float,
    delta: int,
    centers: List[List[float]]
) -> Tuple[List, List[str]]:
    """
    Compute Context Rays for each centroid using spherical sampling

    Generates rays from each centroid in all directions using
    spherical coordinates with uniform sampling

    Args:
        radius: Sphere radius for ray generation
        delta: Grid size (delta x delta samples)
        centers: List of centroid coordinates

    Returns:
        Tuple of (cr_data, rayos_contexto)
        - cr_data: List of [counter, center, ray_endpoints]
        - rayos_contexto: List of formatted ray strings
    """
    # Angular sampling for sphere
    phi = np.linspace(0, 2 * math.pi, delta)
    theta = np.linspace(0, 2 * math.pi, delta)

    rayos_contexto = []
    cr_data = []
    counter = 0

    for center_idx, center in enumerate(centers):
        if (center_idx + 1) % 10 == 0:
            logger.debug(f"Processing centroid {center_idx + 1}/{len(centers)}")

        ray_directions_item = []

        # Generate rays using spherical coordinates
        for i in phi:
            for j in theta:
                # Spherical to Cartesian conversion
                x = radius * math.sin(i) * math.cos(j) + center[0]
                y = radius * math.sin(i) * math.sin(j) + center[1]
                z = radius * math.cos(i) + center[2]

                counter += 1

                ray_directions_item.append([x, y, z])

                # Format for export
                final = f"{x} {y} {z}"
                origen = f"{center[0]} {center[1]} {center[2]}"
                rayos_contexto.append(f"{counter} {origen} {final}")

        # Store CR data for this center
        cr_data.append([counter, center, ray_directions_item])
        counter += 1

    logger.info(f"Context rays computed: {len(rayos_contexto)} rays from {len(centers)} centers")

    return cr_data, rayos_contexto


def evaluate_ray_intersections(
    cr_data: List,
    mesh: trimesh.Trimesh,
    n_segments: int
) -> List[str]:
    """
    Evaluate ray-mesh intersections by dividing rays into segments

    This is the computational bottleneck of the algorithm.
    For each ray:
    1. Divide into n_segments
    2. Check if each segment intersects the mesh
    3. Record True/False for each segment

    Args:
        cr_data: CR data from compute_CR
        mesh: Trimesh object of protein surface
        n_segments: Number of segments per ray

    Returns:
        List of formatted strings with intersection results
    """
    cr_segments = []
    total_rays = sum(len(item[2]) for item in cr_data)

    logger.info(f"Evaluating {total_rays} rays with {n_segments} segments each...")
    logger.info(f"Total intersection checks: {total_rays * n_segments}")
    logger.info(f"This may take 10-30 minutes depending on protein size...")

    ray_count = 0
    for item_idx, item in enumerate(cr_data):
        if (item_idx + 1) % 10 == 0:
            logger.info(f"Progress: {item_idx + 1}/{len(cr_data)} centroids")

        # item[0] = index, item[1] = center, item[2] = ray_endpoints
        center = item[1]
        ray_endpoints = item[2]

        for ray in ray_endpoints:
            ray_count += 1

            # Divide ray into segments
            vector = np.linspace(center, ray, n_segments)

            # Check intersections for each segment
            # Uses ray.intersects_any for each segment pair
            hits = mesh.ray.intersects_any(
                ray_origins=vector[0:len(vector)-1],
                ray_directions=vector[1:len(vector)]
            )

            # Format output
            string1 = str(item[0])
            string2 = " ".join([str(elem) for elem in center])
            string3 = " ".join([f"{elem[0]} {elem[1]} {elem[2]}" for elem in vector])
            string4 = " ".join([str(hit) for hit in hits])

            cr_segments.append(f"{string1} {string2} {string3} {string4}")

    logger.info(f"Ray intersection evaluation complete!")

    return cr_segments
