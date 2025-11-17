"""
Redis caching layer for improved performance
Provides decorators and utilities for caching expensive operations
"""
import redis
import json
from functools import wraps
from typing import Any, Callable, Optional
import hashlib
from datetime import datetime

from app.config import get_settings
from app.core.logging import get_logger
from app.core.metrics import cache_operations

settings = get_settings()
logger = get_logger(__name__)

# Create Redis client with connection pooling
try:
    redis_client = redis.from_url(
        settings.REDIS_URL,
        decode_responses=True,
        socket_connect_timeout=5,
        socket_timeout=5,
        retry_on_timeout=True,
        health_check_interval=30,
    )
    # Test connection
    redis_client.ping()
    logger.info("Redis cache client initialized successfully")
except Exception as e:
    logger.warning(f"Redis cache unavailable: {e}. Caching will be disabled.")
    redis_client = None


def _serialize_value(value: Any) -> str:
    """
    Serialize a value for caching, handling SQLAlchemy models.

    Args:
        value: Value to serialize

    Returns:
        str: JSON serialized value
    """
    def default_serializer(obj):
        """Custom serializer for non-JSON types"""
        # Handle datetime objects
        if isinstance(obj, datetime):
            return obj.isoformat()

        # Handle SQLAlchemy models (check for __dict__)
        if hasattr(obj, '__dict__') and hasattr(obj, '__table__'):
            # Convert SQLAlchemy model to dict
            result = {}
            for column in obj.__table__.columns:
                value = getattr(obj, column.name)
                if isinstance(value, datetime):
                    result[column.name] = value.isoformat()
                else:
                    result[column.name] = value
            return result

        # Handle lists of SQLAlchemy models
        if isinstance(obj, list) and obj and hasattr(obj[0], '__table__'):
            return [default_serializer(item) for item in obj]

        # Default: convert to string
        return str(obj)

    return json.dumps(value, default=default_serializer)


def _generate_cache_key(prefix: str, *args, **kwargs) -> str:
    """
    Generate a unique cache key from function arguments.

    Args:
        prefix: Cache key prefix (usually function name)
        *args: Positional arguments
        **kwargs: Keyword arguments

    Returns:
        str: Unique cache key
    """
    # Filter out SQLAlchemy sessions and non-serializable objects
    serializable_args = []
    for arg in args:
        if not hasattr(arg, '__table__'):  # Skip SQLAlchemy sessions
            serializable_args.append(str(arg))

    serializable_kwargs = {
        k: str(v) for k, v in kwargs.items()
        if not hasattr(v, '__table__')
    }

    # Create a string representation of args and kwargs
    key_data = f"{str(serializable_args)}:{str(sorted(serializable_kwargs.items()))}"

    # Hash the data to create a shorter key
    key_hash = hashlib.md5(key_data.encode()).hexdigest()

    return f"{prefix}:{key_hash}"


def _calculate_hit_rate() -> float:
    """
    Calculate cache hit rate from Prometheus metrics.

    Returns:
        float: Hit rate as a percentage (0-100)
    """
    try:
        from app.core.metrics import cache_operations

        # Get the metric values
        hits = cache_operations.labels(operation="hit")._value.get()
        misses = cache_operations.labels(operation="miss")._value.get()

        total = hits + misses
        if total == 0:
            return 0.0

        return round((hits / total) * 100, 2)
    except Exception:
        return 0.0


def cache(ttl: int = 300, prefix: Optional[str] = None):
    """
    Cache decorator for functions and methods.

    Caches the return value of a function in Redis with a specified TTL.
    If Redis is unavailable, the function executes normally without caching.

    Args:
        ttl: Time to live in seconds (default: 300 = 5 minutes)
        prefix: Cache key prefix (default: function name)

    Usage:
        @cache(ttl=60)
        async def get_user_data(user_id: int):
            return expensive_database_query(user_id)

        @cache(ttl=3600, prefix="protein_list")
        def list_proteins(user_id: int):
            return db.query(Protein).filter_by(user_id=user_id).all()
    """

    def decorator(func: Callable) -> Callable:
        func_prefix = prefix or f"cache:{func.__module__}.{func.__name__}"

        @wraps(func)
        async def async_wrapper(*args, **kwargs):
            # If Redis is unavailable, execute function normally
            if redis_client is None:
                return await func(*args, **kwargs)

            # Generate cache key
            cache_key = _generate_cache_key(func_prefix, *args, **kwargs)

            try:
                # Try to get from cache
                cached_value = redis_client.get(cache_key)

                if cached_value is not None:
                    logger.debug(f"Cache HIT: {cache_key}")
                    cache_operations.labels(operation="hit").inc()
                    return json.loads(cached_value)

                logger.debug(f"Cache MISS: {cache_key}")
                cache_operations.labels(operation="miss").inc()

            except Exception as e:
                logger.warning(f"Cache read error: {e}")
                cache_operations.labels(operation="error").inc()

            # Execute function
            result = await func(*args, **kwargs)

            # Store in cache
            try:
                # Serialize result
                serialized = _serialize_value(result)
                redis_client.setex(cache_key, ttl, serialized)
                cache_operations.labels(operation="set").inc()
                logger.debug(f"Cache SET: {cache_key} (TTL: {ttl}s)")
            except Exception as e:
                logger.warning(f"Cache write error: {e}")
                cache_operations.labels(operation="error").inc()

            return result

        @wraps(func)
        def sync_wrapper(*args, **kwargs):
            # If Redis is unavailable, execute function normally
            if redis_client is None:
                return func(*args, **kwargs)

            # Generate cache key
            cache_key = _generate_cache_key(func_prefix, *args, **kwargs)

            try:
                # Try to get from cache
                cached_value = redis_client.get(cache_key)

                if cached_value is not None:
                    logger.debug(f"Cache HIT: {cache_key}")
                    cache_operations.labels(operation="hit").inc()
                    return json.loads(cached_value)

                logger.debug(f"Cache MISS: {cache_key}")
                cache_operations.labels(operation="miss").inc()

            except Exception as e:
                logger.warning(f"Cache read error: {e}")
                cache_operations.labels(operation="error").inc()

            # Execute function
            result = func(*args, **kwargs)

            # Store in cache
            try:
                # Serialize result
                serialized = _serialize_value(result)
                redis_client.setex(cache_key, ttl, serialized)
                cache_operations.labels(operation="set").inc()
                logger.debug(f"Cache SET: {cache_key} (TTL: {ttl}s)")
            except Exception as e:
                logger.warning(f"Cache write error: {e}")
                cache_operations.labels(operation="error").inc()

            return result

        # Return appropriate wrapper based on function type
        import inspect

        if inspect.iscoroutinefunction(func):
            return async_wrapper
        return sync_wrapper

    return decorator


def invalidate_cache(prefix: str, *args, **kwargs) -> bool:
    """
    Invalidate a specific cache entry.

    Args:
        prefix: Cache key prefix
        *args: Arguments used to generate the cache key
        **kwargs: Keyword arguments used to generate the cache key

    Returns:
        bool: True if cache was invalidated, False otherwise
    """
    if redis_client is None:
        return False

    cache_key = _generate_cache_key(prefix, *args, **kwargs)

    try:
        deleted = redis_client.delete(cache_key)
        if deleted:
            logger.debug(f"Cache INVALIDATED: {cache_key}")
            cache_operations.labels(operation="invalidate").inc()
        return bool(deleted)
    except Exception as e:
        logger.warning(f"Cache invalidation error: {e}")
        cache_operations.labels(operation="error").inc()
        return False


def invalidate_pattern(pattern: str) -> int:
    """
    Invalidate all cache entries matching a pattern.

    Args:
        pattern: Redis key pattern (e.g., "user:*", "protein:123:*")

    Returns:
        int: Number of keys deleted

    Example:
        # Invalidate all caches for user 123
        invalidate_pattern("user:123:*")

        # Invalidate all job caches
        invalidate_pattern("jobs:*")
    """
    if redis_client is None:
        return 0

    try:
        keys = redis_client.keys(pattern)
        if keys:
            deleted = redis_client.delete(*keys)
            logger.info(f"Cache PATTERN INVALIDATED: {pattern} ({deleted} keys)")
            cache_operations.labels(operation="invalidate").inc(deleted)
            return deleted
        return 0
    except Exception as e:
        logger.warning(f"Cache pattern invalidation error: {e}")
        cache_operations.labels(operation="error").inc()
        return 0


def get_cache_stats() -> dict:
    """
    Get Redis cache statistics and update Prometheus metrics.

    Returns:
        dict: Cache statistics including memory usage, keys count, etc.
    """
    if redis_client is None:
        return {"status": "unavailable"}

    try:
        from app.core.metrics import cache_keys_total, cache_memory_bytes

        info = redis_client.info()
        keys_count = redis_client.dbsize()
        memory_used = info.get("used_memory", 0)

        # Update Prometheus metrics
        cache_keys_total.set(keys_count)
        cache_memory_bytes.set(memory_used)

        return {
            "status": "available",
            "used_memory": info.get("used_memory_human"),
            "used_memory_bytes": memory_used,
            "total_keys": keys_count,
            "connected_clients": info.get("connected_clients"),
            "uptime_days": info.get("uptime_in_days"),
            "hit_rate": _calculate_hit_rate(),
        }
    except Exception as e:
        logger.error(f"Error getting cache stats: {e}")
        return {"status": "error", "error": str(e)}


# Pre-defined cache TTLs for common use cases
class CacheTTL:
    """Predefined cache TTL constants"""

    VERY_SHORT = 60  # 1 minute - for rapidly changing data
    SHORT = 300  # 5 minutes - default
    MEDIUM = 1800  # 30 minutes - for semi-static data
    LONG = 3600  # 1 hour - for mostly static data
    VERY_LONG = 86400  # 24 hours - for very static data
