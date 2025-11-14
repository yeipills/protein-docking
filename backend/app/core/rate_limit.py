"""
Rate Limiting Configuration
Granular rate limiting for different endpoint types
"""
from slowapi import Limiter
from slowapi.util import get_remote_address
from functools import wraps
from fastapi import Request
from typing import Callable
import logging

logger = logging.getLogger(__name__)

# Initialize limiter
limiter = Limiter(key_func=get_remote_address)


class RateLimitTier:
    """Rate limit tiers for different endpoint types"""

    # Public endpoints (no auth required)
    PUBLIC = "100/hour"

    # Authentication endpoints
    AUTH_LOGIN = "5/minute"
    AUTH_REGISTER = "3/minute"
    AUTH_REFRESH = "10/minute"

    # Standard authenticated requests
    AUTHENTICATED = "1000/hour"

    # File upload endpoints (more restrictive)
    UPLOAD = "10/minute"
    UPLOAD_LARGE = "5/minute"

    # Compute-intensive operations
    DOCKING_JOB = "20/hour"
    ANALYSIS = "50/hour"

    # Admin operations
    ADMIN = "500/hour"

    # Health checks (very permissive)
    HEALTH = "60/minute"


def get_user_identifier(request: Request) -> str:
    """
    Get user identifier for rate limiting
    Prefer authenticated user ID over IP address
    """
    # Try to get user from auth token
    if hasattr(request.state, 'user') and request.state.user:
        user_id = getattr(request.state.user, 'id', None)
        if user_id:
            return f"user:{user_id}"

    # Fallback to IP address
    return get_remote_address(request)


def create_user_limiter():
    """Create a limiter that uses user ID when available"""
    return Limiter(key_func=get_user_identifier)


# User-aware limiter
user_limiter = create_user_limiter()


def adaptive_rate_limit(
    authenticated_limit: str,
    unauthenticated_limit: str
):
    """
    Adaptive rate limiting decorator
    Different limits for authenticated vs unauthenticated users

    Args:
        authenticated_limit: Rate limit for authenticated users (e.g., "100/minute")
        unauthenticated_limit: Rate limit for unauthenticated users (e.g., "10/minute")

    Usage:
        @app.get("/endpoint")
        @adaptive_rate_limit("100/minute", "10/minute")
        async def my_endpoint():
            pass
    """
    def decorator(func: Callable):
        @wraps(func)
        async def wrapper(request: Request, *args, **kwargs):
            # Check if user is authenticated
            is_authenticated = (
                hasattr(request.state, 'user') and
                request.state.user is not None
            )

            # Select appropriate limit
            limit = authenticated_limit if is_authenticated else unauthenticated_limit

            # Apply rate limit
            limiter_instance = user_limiter if is_authenticated else limiter
            rate_limit_decorator = limiter_instance.limit(limit)

            # Execute function with rate limit
            return await rate_limit_decorator(func)(request, *args, **kwargs)

        return wrapper
    return decorator


def log_rate_limit_hit(request: Request, limit: str):
    """Log when a rate limit is hit"""
    identifier = get_user_identifier(request)
    logger.warning(
        f"Rate limit exceeded: {identifier} - "
        f"Endpoint: {request.url.path} - "
        f"Limit: {limit}"
    )


# Endpoint-specific rate limit decorators
def rate_limit_auth_login(func: Callable):
    """Rate limit for login endpoints"""
    return limiter.limit(RateLimitTier.AUTH_LOGIN)(func)


def rate_limit_auth_register(func: Callable):
    """Rate limit for registration endpoints"""
    return limiter.limit(RateLimitTier.AUTH_REGISTER)(func)


def rate_limit_upload(func: Callable):
    """Rate limit for file upload endpoints"""
    return user_limiter.limit(RateLimitTier.UPLOAD)(func)


def rate_limit_docking(func: Callable):
    """Rate limit for docking job endpoints"""
    return user_limiter.limit(RateLimitTier.DOCKING_JOB)(func)


def rate_limit_health(func: Callable):
    """Rate limit for health check endpoints"""
    return limiter.limit(RateLimitTier.HEALTH)(func)


# Rate limit bypass for specific users/IPs (whitelist)
RATE_LIMIT_WHITELIST = set([
    # Add IPs or user IDs that should bypass rate limiting
    # "192.168.1.1",
    # "user:admin-id"
])


def is_whitelisted(request: Request) -> bool:
    """Check if request should bypass rate limiting"""
    identifier = get_user_identifier(request)
    return identifier in RATE_LIMIT_WHITELIST


def conditional_rate_limit(limit: str):
    """
    Rate limit that can be bypassed for whitelisted users

    Usage:
        @app.get("/endpoint")
        @conditional_rate_limit("100/minute")
        async def my_endpoint():
            pass
    """
    def decorator(func: Callable):
        @wraps(func)
        async def wrapper(request: Request, *args, **kwargs):
            # Skip rate limiting for whitelisted users
            if is_whitelisted(request):
                return await func(request, *args, **kwargs)

            # Apply rate limit
            rate_limited = limiter.limit(limit)(func)
            return await rate_limited(request, *args, **kwargs)

        return wrapper
    return decorator


# Rate limit storage backends
class RateLimitConfig:
    """Rate limit configuration"""

    def __init__(self):
        self.storage_uri = None  # Will use in-memory by default
        self.strategy = "fixed-window"  # or "moving-window"

    @classmethod
    def from_redis(cls, redis_url: str):
        """Configure rate limiting with Redis backend"""
        config = cls()
        config.storage_uri = redis_url
        return config

    @classmethod
    def from_memcached(cls, memcached_url: str):
        """Configure rate limiting with Memcached backend"""
        config = cls()
        config.storage_uri = memcached_url
        return config


# Export commonly used items
__all__ = [
    'limiter',
    'user_limiter',
    'RateLimitTier',
    'adaptive_rate_limit',
    'rate_limit_auth_login',
    'rate_limit_auth_register',
    'rate_limit_upload',
    'rate_limit_docking',
    'rate_limit_health',
    'conditional_rate_limit',
    'get_user_identifier',
    'is_whitelisted',
    'RateLimitConfig',
]
