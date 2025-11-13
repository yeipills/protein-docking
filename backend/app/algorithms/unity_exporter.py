"""
Unity exporter (migrated from Script05_preparacion_capas_unity.py)
Reformats context shape layer data for Unity visualization

This script processes the layer files from Script04 and reshapes them
into a format suitable for Unity 3D visualization of protein docking.
"""
import numpy as np
from pathlib import Path
from typing import List, Tuple, Dict
from app.core.logging import get_logger
from app.config import get_settings

logger = get_logger(__name__)
settings = get_settings()


def export_for_unity(
    protein_name: str,
    context_rays_file: str,
    layer_files: Dict[str, str],
    output_dir: str,
    n_segments: int = 15,
    n_rays: int = 100
) -> Dict[str, str]:
    """
    Export context shape layers in Unity-compatible format

    Takes the raw layer files from Script04 and reshapes them for
    Unity visualization. Each ray is formatted with CS number, ray index,
    segment values array, and origin/endpoint coordinates.

    Args:
        protein_name: Name of the protein
        context_rays_file: Path to rayos_contexto.txt from Script03
        layer_files: Dict mapping layer names to file paths
                    Expected keys: 'ses', 'in1', 'in2', 'in3', 'in4',
                                   'out1', 'out2', 'out3', 'out4', 'vol'
        output_dir: Directory to save Unity-formatted files
        n_segments: Number of segments per ray (default 15)
        n_rays: Number of rays per centroid (default 100, from delta*delta in Script03)

    Returns:
        Dict mapping layer names to Unity output file paths

    Output format per line:
        cs_number ray_index seg1 seg2 ... seg15 origin_x origin_y origin_z end_x end_y end_z
    """
    logger.info(f"=" * 60)
    logger.info(f"Starting Unity export for: {protein_name}")
    logger.info(f"=" * 60)

    logger.info(f"Parameters:")
    logger.info(f"  Segments per ray: {n_segments}")
    logger.info(f"  Rays per centroid: {n_rays}")

    # Step 1: Parse context rays file to get CS metadata
    logger.info(f"Reading context rays file: {context_rays_file}")
    cs_metadata = parse_context_rays(context_rays_file)

    logger.info(f"Found {cs_metadata['total_cs']} context shapes")
    logger.info(f"Total rays: {len(cs_metadata['ray_origins'])}")

    # Step 2: Create Unity output directory
    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)

    # Step 3: Export CS summary file (CS number and centroid coordinates)
    summary_file = str(output_path / f"{protein_name}_resumen_cs_unity.txt")
    logger.info(f"Exporting CS summary: {summary_file}")
    export_cs_summary(cs_metadata['centroids'], summary_file)

    # Step 4: Process and export each layer
    output_files = {'summary': summary_file}

    layer_names = ['ses', 'in1', 'in2', 'in3', 'in4', 'out1', 'out2', 'out3', 'out4', 'vol']

    for layer_name in layer_names:
        if layer_name not in layer_files:
            logger.warning(f"Layer '{layer_name}' not found in layer_files, skipping")
            continue

        layer_file = layer_files[layer_name]
        output_file = str(output_path / f"{protein_name}_cs_{layer_name}_unity.txt")

        logger.info(f"Processing layer '{layer_name}'...")
        logger.info(f"  Input: {layer_file}")
        logger.info(f"  Output: {output_file}")

        # Reshape and export
        reshape_and_export_layer(
            layer_file=layer_file,
            output_file=output_file,
            cs_metadata=cs_metadata,
            n_segments=n_segments,
            n_rays=n_rays
        )

        output_files[layer_name] = output_file

    logger.info(f"Unity export complete!")
    logger.info(f"Exported {len(output_files)} files")
    logger.info(f"=" * 60)

    return output_files


def parse_context_rays(context_rays_file: str) -> Dict:
    """
    Parse context rays file to extract CS metadata

    Format of rayos_contexto.txt (from Script03):
        counter origin_x origin_y origin_z end_x end_y end_z

    Args:
        context_rays_file: Path to rayos_contexto.txt

    Returns:
        Dict with:
            - 'cs_indices': List of CS indices for each ray
            - 'ray_origins': List of [x, y, z] ray origins
            - 'ray_endpoints': List of [x, y, z] ray endpoints
            - 'inicio_fin': List of [[origin], [endpoint]] pairs
            - 'centroids': List of unique centroids with CS numbers
            - 'total_cs': Total number of context shapes
    """
    logger.info(f"Parsing context rays file...")

    cs_indices = []
    ray_origins = []
    ray_endpoints = []
    inicio_fin = []
    centro_raw = []

    with open(context_rays_file, 'r') as f:
        lines = f.readlines()

    for line in lines:
        parts = line.strip().split()

        # Format: counter origin_x origin_y origin_z end_x end_y end_z
        cs_index = parts[0]
        origin = [float(parts[1]), float(parts[2]), float(parts[3])]
        endpoint = [float(parts[4]), float(parts[5]), float(parts[6])]

        cs_indices.append(cs_index)
        ray_origins.append(origin)
        ray_endpoints.append(endpoint)
        inicio_fin.append([origin, endpoint])
        centro_raw.append(origin)

    # Find unique centroids (CS centers)
    centro_unique = np.unique(centro_raw, axis=0)
    total_cs = len(centro_unique)

    # Create numbered centroids (CS number, coordinates)
    centroids = []
    for i, centroid in enumerate(centro_unique):
        centroids.append([i, centroid])

    logger.info(f"Parsed {len(lines)} rays")
    logger.info(f"Found {total_cs} unique context shapes")

    return {
        'cs_indices': cs_indices,
        'ray_origins': ray_origins,
        'ray_endpoints': ray_endpoints,
        'inicio_fin': inicio_fin,
        'centroids': centroids,
        'total_cs': total_cs
    }


def export_cs_summary(centroids: List, output_file: str) -> None:
    """
    Export CS summary file for Unity

    Format: cs_number x y z

    Args:
        centroids: List of [cs_number, [x, y, z]]
        output_file: Output file path
    """
    logger.info(f"Writing CS summary to: {output_file}")

    with open(output_file, 'w') as f:
        for cs_number, coords in centroids:
            line = f"{cs_number} {coords[0]} {coords[1]} {coords[2]}\n"
            f.write(line)

    logger.info(f"CS summary exported: {len(centroids)} context shapes")


def reshape_and_export_layer(
    layer_file: str,
    output_file: str,
    cs_metadata: Dict,
    n_segments: int,
    n_rays: int
) -> None:
    """
    Reshape layer data and export in Unity format

    Input format (from Script04):
        cs cr value

    Output format:
        cs_number ray_index seg1 seg2 ... segN origin_x origin_y origin_z end_x end_y end_z

    Args:
        layer_file: Input layer file from Script04
        output_file: Output Unity-formatted file
        cs_metadata: Metadata from parse_context_rays
        n_segments: Number of segments per ray
        n_rays: Number of rays per CS
    """
    logger.debug(f"Reading layer file: {layer_file}")

    # Step 1: Read layer data
    segment_values = []

    with open(layer_file, 'r') as f:
        lines = f.readlines()

    for line in lines:
        parts = line.strip().split()
        # Format: cs cr value
        cs = int(parts[0])
        cr = int(parts[1])
        value = int(parts[2])
        segment_values.append(value)

    logger.debug(f"Read {len(segment_values)} segment values")

    # Step 2: Reshape into (n_rays x n_segments) array
    total_rays = len(segment_values) // n_segments
    array_segments = np.reshape(segment_values, (total_rays, n_segments))

    logger.debug(f"Reshaped to {array_segments.shape} array")

    # Step 3: Create CS number array
    # Each CS has n_rays rays, so repeat CS numbers
    total_cs = cs_metadata['total_cs']
    cs_numbers = []
    for cs_num in range(total_cs):
        cs_numbers.extend([cs_num] * n_rays)

    # Step 4: Create ray indices
    ray_indices = np.arange(len(array_segments))

    # Step 5: Get origin/endpoint pairs
    inicio_fin = cs_metadata['inicio_fin']

    # Step 6: Write Unity-formatted output
    logger.debug(f"Writing Unity format to: {output_file}")

    with open(output_file, 'w') as f:
        for i in range(len(array_segments)):
            cs_number = cs_numbers[i]
            ray_index = ray_indices[i]
            segments = array_segments[i]
            origin, endpoint = inicio_fin[i]

            # Format: cs_number ray_index seg1 seg2 ... segN origin_x origin_y origin_z end_x end_y end_z
            line_parts = [
                str(cs_number),
                str(ray_index),
                " ".join([str(seg) for seg in segments]),
                f"{origin[0]} {origin[1]} {origin[2]}",
                f"{endpoint[0]} {endpoint[1]} {endpoint[2]}"
            ]

            line = " ".join(line_parts) + "\n"
            f.write(line)

    logger.debug(f"Unity layer exported: {len(array_segments)} rays")


def export_all_layers_for_unity(
    protein_name: str,
    cr_output_dir: str,
    cs_output_dir: str,
    unity_output_dir: str
) -> Dict[str, str]:
    """
    Convenience function to export all layers for Unity at once

    Automatically locates all required files and exports them

    Args:
        protein_name: Name of the protein
        cr_output_dir: Directory containing context rays files from Script03
        cs_output_dir: Directory containing layer files from Script04
        unity_output_dir: Directory to save Unity-formatted files

    Returns:
        Dict mapping layer names to Unity output file paths
    """
    logger.info(f"Exporting all layers for Unity: {protein_name}")

    # Locate input files
    cr_dir = Path(cr_output_dir)
    cs_dir = Path(cs_output_dir)

    context_rays_file = str(cr_dir / f"{protein_name}_rayos_contexto.txt")

    layer_files = {
        'ses': str(cs_dir / f"{protein_name}_ses.txt"),
        'in1': str(cs_dir / f"{protein_name}_in1.txt"),
        'in2': str(cs_dir / f"{protein_name}_in2.txt"),
        'in3': str(cs_dir / f"{protein_name}_in3.txt"),
        'in4': str(cs_dir / f"{protein_name}_in4.txt"),
        'out1': str(cs_dir / f"{protein_name}_out1.txt"),
        'out2': str(cs_dir / f"{protein_name}_out2.txt"),
        'out3': str(cs_dir / f"{protein_name}_out3.txt"),
        'out4': str(cs_dir / f"{protein_name}_out4.txt"),
        'vol': str(cs_dir / f"{protein_name}_vol.txt"),
    }

    # Verify files exist
    if not Path(context_rays_file).exists():
        raise FileNotFoundError(f"Context rays file not found: {context_rays_file}")

    for layer_name, layer_file in layer_files.items():
        if not Path(layer_file).exists():
            logger.warning(f"Layer file not found: {layer_file}")

    # Export
    return export_for_unity(
        protein_name=protein_name,
        context_rays_file=context_rays_file,
        layer_files=layer_files,
        output_dir=unity_output_dir
    )
