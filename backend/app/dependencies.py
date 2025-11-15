"""
FastAPI dependencies for dependency injection
Handles authentication, authorization, and common dependencies
"""
from typing import Optional
from fastapi import Depends, Header, Request
from fastapi.security import HTTPBearer, HTTPAuthorizationCredentials
from sqlalchemy.orm import Session
from app.database import get_db
from app.models.user import User
from app.core.security import decode_token
from app.core.exceptions import UnauthorizedException, NotFoundException
from app.core.logging import get_logger

logger = get_logger(__name__)
security = HTTPBearer(auto_error=False)  # Don't auto-error to allow cookie fallback


async def get_current_user(
    request: Request,
    credentials: Optional[HTTPAuthorizationCredentials] = Depends(security),
    db: Session = Depends(get_db)
) -> User:
    """
    Get current authenticated user from JWT token (from cookie or Authorization header)

    Args:
        request: Request object (to read cookies)
        credentials: HTTP Bearer token (optional, from Authorization header)
        db: Database session

    Returns:
        User: Current authenticated user

    Raises:
        UnauthorizedException: If token is invalid or user not found
    """
    # Try to get token from Authorization header first
    token = None
    if credentials:
        token = credentials.credentials

    # If no token in header, try to get from cookie
    if not token:
        token = request.cookies.get("access_token")

    # If still no token, raise unauthorized
    if not token:
        logger.warning("No authentication token provided")
        raise UnauthorizedException(detail="Not authenticated")

    # Decode token
    payload = decode_token(token)
    if payload is None:
        logger.warning("Invalid or expired token")
        raise UnauthorizedException(detail="Invalid or expired token")

    # Check token type
    if payload.get("type") != "access":
        logger.warning("Invalid token type")
        raise UnauthorizedException(detail="Invalid token type")

    # Get user ID from token
    user_id: Optional[int] = payload.get("user_id")
    if user_id is None:
        logger.warning("Token missing user_id")
        raise UnauthorizedException(detail="Invalid token payload")

    # Query user from database
    user = db.query(User).filter(User.id == user_id).first()
    if user is None:
        logger.warning(f"User not found: {user_id}")
        raise NotFoundException(detail="User not found")

    # Check if user is active
    if not user.is_active:
        logger.warning(f"Inactive user attempted access: {user_id}")
        raise UnauthorizedException(detail="User account is inactive")

    logger.debug(f"Authenticated user: {user.username} (ID: {user.id})")
    return user


async def get_current_active_user(
    current_user: User = Depends(get_current_user)
) -> User:
    """
    Get current active user (alias for get_current_user for clarity)

    Args:
        current_user: Current user from get_current_user

    Returns:
        User: Current active user
    """
    return current_user


async def get_current_superuser(
    current_user: User = Depends(get_current_user)
) -> User:
    """
    Get current superuser (admin)

    Args:
        current_user: Current user from get_current_user

    Returns:
        User: Current superuser

    Raises:
        UnauthorizedException: If user is not a superuser
    """
    if not current_user.is_superuser:
        logger.warning(
            f"Non-superuser attempted admin access: {current_user.username}"
        )
        raise UnauthorizedException(detail="Not enough permissions")

    return current_user


async def verify_socket_token(
    token: str = Header(..., alias="Authorization")
) -> dict:
    """
    Verify socket connection token

    Args:
        token: JWT token from Authorization header

    Returns:
        dict: Token payload

    Raises:
        UnauthorizedException: If token is invalid
    """
    # Remove 'Bearer ' prefix if present
    if token.startswith("Bearer "):
        token = token[7:]

    payload = decode_token(token)
    if payload is None:
        raise UnauthorizedException(detail="Invalid or expired token")

    return payload
