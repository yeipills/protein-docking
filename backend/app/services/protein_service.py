"""
Protein service layer with caching support

Demonstrates caching patterns for protein-related queries.
"""
from typing import Dict, List, Optional
from sqlalchemy.orm import Session

from app.models.protein import Protein
from app.core.cache import cache, invalidate_pattern, CacheTTL
from app.core.logging import get_logger

logger = get_logger(__name__)


@cache(ttl=CacheTTL.MEDIUM, prefix="protein:count")
def get_user_protein_count(user_id: int, db: Session) -> int:
    """
    Get total count of proteins for a user.

    Cached for 30 minutes since protein count doesn't change often.

    Args:
        user_id: User ID
        db: Database session

    Returns:
        int: Total protein count
    """
    count = db.query(Protein).filter(
        Protein.user_id == user_id,
        Protein.is_deleted == False
    ).count()

    return count


@cache(ttl=CacheTTL.LONG, prefix="protein:details")
def get_protein_details(protein_id: int, db: Session) -> Optional[Dict]:
    """
    Get detailed information about a protein.

    Cached for 1 hour since protein details are mostly static.

    Args:
        protein_id: Protein ID
        db: Database session

    Returns:
        dict: Protein details or None if not found
    """
    protein = db.query(Protein).filter(Protein.id == protein_id).first()

    if not protein or protein.is_deleted:
        return None

    return {
        "id": protein.id,
        "user_id": protein.user_id,
        "name": protein.name,
        "stl_file": protein.stl_file,
        "vertices_file": protein.vertices_file,
        "faces_file": protein.faces_file,
        "cr_totals_file": protein.cr_totals_file,
        "context_rays_file": protein.context_rays_file,
        "is_public": protein.is_public,
        "created_at": protein.created_at.isoformat() if protein.created_at else None,
        "updated_at": protein.updated_at.isoformat() if protein.updated_at else None,
    }


@cache(ttl=CacheTTL.MEDIUM, prefix="protein:list")
def get_user_proteins_summary(user_id: int, limit: int, db: Session) -> List[Dict]:
    """
    Get a summary list of user's proteins.

    Cached for 30 minutes.

    Args:
        user_id: User ID
        limit: Maximum number to return
        db: Database session

    Returns:
        list: List of protein summaries
    """
    proteins = db.query(Protein).filter(
        Protein.user_id == user_id,
        Protein.is_deleted == False
    ).order_by(Protein.created_at.desc()).limit(limit).all()

    result = []
    for protein in proteins:
        result.append({
            "id": protein.id,
            "name": protein.name,
            "is_public": protein.is_public,
            "created_at": protein.created_at.isoformat() if protein.created_at else None,
        })

    return result


def invalidate_user_protein_cache(user_id: int):
    """
    Invalidate all protein-related caches for a user.

    Call this when proteins are created, updated, or deleted.

    Args:
        user_id: User ID
    """
    patterns = [
        f"cache:app.services.protein_service.get_user_protein_count:*{user_id}*",
        f"cache:app.services.protein_service.get_user_proteins_summary:*{user_id}*",
    ]

    for pattern in patterns:
        count = invalidate_pattern(pattern)
        if count > 0:
            logger.info(f"Invalidated {count} protein cache entries for user {user_id}")


def invalidate_protein_cache(protein_id: int, user_id: int):
    """
    Invalidate cache for a specific protein.

    Call this when a protein is updated or files are uploaded.

    Args:
        protein_id: Protein ID
        user_id: User ID that owns the protein
    """
    # Invalidate specific protein cache
    pattern = f"cache:app.services.protein_service.get_protein_details:*{protein_id}*"
    invalidate_pattern(pattern)

    # Invalidate user-level caches
    invalidate_user_protein_cache(user_id)

    logger.info(f"Invalidated cache for protein {protein_id}")
