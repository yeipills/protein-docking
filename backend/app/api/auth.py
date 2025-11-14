"""
Authentication endpoints
Handles user registration, login, and token refresh
"""
from fastapi import APIRouter, Depends, status, Request
from sqlalchemy.orm import Session
from app.database import get_db
from app.models.user import User
from app.schemas.user import UserCreate, UserLogin, UserResponse, Token
from app.core.security import (
    verify_password,
    get_password_hash,
    create_access_token,
    create_refresh_token,
    decode_token
)
from app.core.exceptions import (
    UnauthorizedException,
    ConflictException,
    ValidationException
)
from app.core.logging import get_logger
from app.core.rate_limit import (
    limiter,
    RateLimitTier
)

logger = get_logger(__name__)
router = APIRouter()


@router.post("/register", response_model=UserResponse, status_code=status.HTTP_201_CREATED)
@limiter.limit(RateLimitTier.AUTH_REGISTER)
async def register(request: Request, user_data: UserCreate, db: Session = Depends(get_db)):
    """
    Register a new user

    Args:
        user_data: User registration data
        db: Database session

    Returns:
        UserResponse: Created user data

    Raises:
        ConflictException: If username or email already exists
    """
    # Check if email already exists
    existing_user = db.query(User).filter(User.email == user_data.email).first()
    if existing_user:
        logger.warning(f"Registration attempt with existing email: {user_data.email}")
        raise ConflictException(detail="Email already registered")

    # Check if username already exists
    existing_user = db.query(User).filter(User.username == user_data.username).first()
    if existing_user:
        logger.warning(f"Registration attempt with existing username: {user_data.username}")
        raise ConflictException(detail="Username already taken")

    # Create new user
    hashed_password = get_password_hash(user_data.password)
    new_user = User(
        email=user_data.email,
        username=user_data.username,
        full_name=user_data.full_name,
        hashed_password=hashed_password,
        is_active=True,
        is_superuser=False
    )

    db.add(new_user)
    db.commit()
    db.refresh(new_user)

    logger.info(f"New user registered: {new_user.username} (ID: {new_user.id})")
    return new_user


@router.post("/login", response_model=Token)
@limiter.limit(RateLimitTier.AUTH_LOGIN)
async def login(request: Request, login_data: UserLogin, db: Session = Depends(get_db)):
    """
    Login and get access token

    Args:
        login_data: User login credentials
        db: Database session

    Returns:
        Token: JWT tokens (access and refresh)

    Raises:
        UnauthorizedException: If credentials are invalid
    """
    # Find user by username
    user = db.query(User).filter(User.username == login_data.username).first()

    if not user or not verify_password(login_data.password, user.hashed_password):
        logger.warning(f"Failed login attempt for username: {login_data.username}")
        raise UnauthorizedException(detail="Incorrect username or password")

    if not user.is_active:
        logger.warning(f"Login attempt by inactive user: {login_data.username}")
        raise UnauthorizedException(detail="User account is inactive")

    # Create tokens
    token_data = {"user_id": user.id, "username": user.username}
    access_token = create_access_token(token_data)
    refresh_token = create_refresh_token(token_data)

    logger.info(f"User logged in: {user.username} (ID: {user.id})")

    return {
        "access_token": access_token,
        "refresh_token": refresh_token,
        "token_type": "bearer"
    }


@router.post("/refresh", response_model=Token)
@limiter.limit(RateLimitTier.AUTH_REFRESH)
async def refresh_token(request: Request, refresh_token: str, db: Session = Depends(get_db)):
    """
    Refresh access token using refresh token

    Args:
        refresh_token: JWT refresh token
        db: Database session

    Returns:
        Token: New JWT tokens

    Raises:
        UnauthorizedException: If refresh token is invalid
    """
    # Decode refresh token
    payload = decode_token(refresh_token)
    if payload is None:
        logger.warning("Invalid refresh token")
        raise UnauthorizedException(detail="Invalid or expired refresh token")

    # Check token type
    if payload.get("type") != "refresh":
        logger.warning("Wrong token type for refresh")
        raise UnauthorizedException(detail="Invalid token type")

    # Get user
    user_id = payload.get("user_id")
    user = db.query(User).filter(User.id == user_id).first()

    if not user or not user.is_active:
        logger.warning(f"Refresh token for invalid/inactive user: {user_id}")
        raise UnauthorizedException(detail="User not found or inactive")

    # Create new tokens
    token_data = {"user_id": user.id, "username": user.username}
    new_access_token = create_access_token(token_data)
    new_refresh_token = create_refresh_token(token_data)

    logger.info(f"Token refreshed for user: {user.username} (ID: {user.id})")

    return {
        "access_token": new_access_token,
        "refresh_token": new_refresh_token,
        "token_type": "bearer"
    }
