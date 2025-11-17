"""
Authentication endpoints
Handles user registration, login, and token refresh
"""
from fastapi import APIRouter, Depends, status, Request, Response
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
from app.config import get_settings
from datetime import datetime, timedelta

logger = get_logger(__name__)
router = APIRouter()
settings = get_settings()

# Account lockout constants
MAX_FAILED_ATTEMPTS = 5  # Lock after 5 failed attempts
LOCKOUT_DURATION_MINUTES = 30  # Lock for 30 minutes
FAILED_ATTEMPT_RESET_MINUTES = 15  # Reset failed attempts after 15 minutes of inactivity


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

    # Audit log
    log_from_request(
        db=db,
        request=request,
        action=AuditAction.USER_CREATE,
        user_id=new_user.id,
        username=new_user.username,
        resource_type="user",
        resource_id=str(new_user.id),
        description=f"User registered: {new_user.username}",
        metadata={"email": new_user.email, "full_name": new_user.full_name}
    )

    logger.info(f"New user registered: {new_user.username} (ID: {new_user.id})")
    return new_user


@router.post("/login", response_model=Token)
@limiter.limit(RateLimitTier.AUTH_LOGIN)
async def login(request: Request, response: Response, login_data: UserLogin, db: Session = Depends(get_db)):
    """
    Login and get access token (stored in httpOnly cookies)

    Implements account lockout policy:
    - Lock account after 5 failed attempts
    - Lockout duration: 30 minutes
    - Reset failed attempts after 15 minutes of inactivity

    Args:
        login_data: User login credentials
        db: Database session
        response: FastAPI Response for setting cookies

    Returns:
        Token: JWT tokens (access and refresh) - also set as httpOnly cookies

    Raises:
        UnauthorizedException: If credentials are invalid or account locked
    """
    # Find user by username
    user = db.query(User).filter(User.username == login_data.username).first()

    if not user:
        logger.warning(f"Failed login attempt for non-existent username: {login_data.username}")
        raise UnauthorizedException(detail="Incorrect username or password")

    # Check if account is locked
    now = datetime.utcnow()
    if user.locked_until and user.locked_until > now:
        time_remaining = (user.locked_until - now).total_seconds() / 60
        logger.warning(f"Login attempt for locked account: {login_data.username}")
        raise UnauthorizedException(
            detail=f"Account is locked due to too many failed login attempts. "
                   f"Please try again in {int(time_remaining)} minutes."
        )

    # If lockout period has passed, reset lockout
    if user.locked_until and user.locked_until <= now:
        user.locked_until = None
        user.failed_login_attempts = 0
        db.commit()

    # Reset failed attempts if enough time has passed since last failure
    if user.last_failed_login:
        time_since_last_failure = now - user.last_failed_login
        if time_since_last_failure > timedelta(minutes=FAILED_ATTEMPT_RESET_MINUTES):
            user.failed_login_attempts = 0
            db.commit()

    # Verify password
    if not verify_password(login_data.password, user.hashed_password):
        # Increment failed login attempts
        user.failed_login_attempts += 1
        user.last_failed_login = now

        # Lock account if max attempts reached
        if user.failed_login_attempts >= MAX_FAILED_ATTEMPTS:
            user.locked_until = now + timedelta(minutes=LOCKOUT_DURATION_MINUTES)
            db.commit()
            logger.warning(f"Account locked due to {MAX_FAILED_ATTEMPTS} failed attempts: {login_data.username}")
            raise UnauthorizedException(
                detail=f"Account has been locked due to too many failed login attempts. "
                       f"Please try again in {LOCKOUT_DURATION_MINUTES} minutes."
            )

        db.commit()
        remaining_attempts = MAX_FAILED_ATTEMPTS - user.failed_login_attempts
        logger.warning(
            f"Failed login attempt for {login_data.username}. "
            f"Attempts: {user.failed_login_attempts}/{MAX_FAILED_ATTEMPTS}"
        )
        raise UnauthorizedException(
            detail=f"Incorrect username or password. "
                   f"{remaining_attempts} attempts remaining before account lockout."
        )

    if not user.is_active:
        logger.warning(f"Login attempt by inactive user: {login_data.username}")

        # Audit log inactive user login attempt
        log_from_request(
            db=db,
            request=request,
            action=AuditAction.LOGIN_FAILURE,
            user_id=user.id,
            username=user.username,
            severity=AuditSeverity.WARNING,
            status="failure",
            description=f"Login attempt by inactive user {user.username}",
            error_message="User account is inactive"
        )

        raise UnauthorizedException(detail="User account is inactive")

    # Successful login - reset failed attempts
    user.failed_login_attempts = 0
    user.last_failed_login = None
    user.locked_until = None
    db.commit()

    # Create tokens
    token_data = {"user_id": user.id, "username": user.username}
    access_token = create_access_token(token_data)
    refresh_token = create_refresh_token(token_data)

    # Set httpOnly cookies for secure token storage
    is_production = settings.ENVIRONMENT == 'production'

    # Access token cookie (15 minutes)
    response.set_cookie(
        key="access_token",
        value=access_token,
        httponly=True,
        secure=is_production,  # HTTPS only in production
        samesite="lax",  # CSRF protection
        max_age=15 * 60,  # 15 minutes in seconds
        path="/"
    )

    # Refresh token cookie (7 days)
    response.set_cookie(
        key="refresh_token",
        value=refresh_token,
        httponly=True,
        secure=is_production,  # HTTPS only in production
        samesite="lax",  # CSRF protection
        max_age=7 * 24 * 60 * 60,  # 7 days in seconds
        path="/api/auth"  # Only send with auth endpoints
    )

    logger.info(f"User logged in: {user.username} (ID: {user.id})")

    return {
        "access_token": access_token,
        "refresh_token": refresh_token,
        "token_type": "bearer"
    }


@router.post("/refresh", response_model=Token)
@limiter.limit(RateLimitTier.AUTH_REFRESH)
async def refresh_token_endpoint(request: Request, response: Response, db: Session = Depends(get_db)):
    """
    Refresh access token using refresh token from cookie

    Args:
        request: Request object (to read cookies)
        response: Response object (to set new cookies)
        db: Database session

    Returns:
        Token: New JWT tokens

    Raises:
        UnauthorizedException: If refresh token is invalid
    """
    # Get refresh token from cookie
    refresh_token = request.cookies.get("refresh_token")
    if not refresh_token:
        logger.warning("No refresh token in cookies")
        raise UnauthorizedException(detail="No refresh token provided")

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

    # Set new httpOnly cookies
    is_production = settings.ENVIRONMENT == 'production'

    # Access token cookie (15 minutes)
    response.set_cookie(
        key="access_token",
        value=new_access_token,
        httponly=True,
        secure=is_production,
        samesite="lax",
        max_age=15 * 60,
        path="/"
    )

    # Refresh token cookie (7 days)
    response.set_cookie(
        key="refresh_token",
        value=new_refresh_token,
        httponly=True,
        secure=is_production,
        samesite="lax",
        max_age=7 * 24 * 60 * 60,
        path="/api/auth"
    )

    logger.info(f"Token refreshed for user: {user.username} (ID: {user.id})")

    return {
        "access_token": new_access_token,
        "refresh_token": new_refresh_token,
        "token_type": "bearer"
    }


@router.post("/logout")
async def logout(response: Response):
    """
    Logout user by clearing httpOnly cookies

    Args:
        response: Response object to clear cookies

    Returns:
        dict: Success message
    """
    # Clear access token cookie
    response.delete_cookie(key="access_token", path="/")

    # Clear refresh token cookie
    response.delete_cookie(key="refresh_token", path="/api/auth")

    logger.info("User logged out")

    return {"message": "Successfully logged out"}
