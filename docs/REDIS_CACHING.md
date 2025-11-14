# Redis Caching Layer

## Overview

The Protein Docking Platform includes a comprehensive Redis-based caching system to improve performance and reduce database load. This document explains how to use and maintain the caching layer.

## Table of Contents

- [Architecture](#architecture)
- [Quick Start](#quick-start)
- [Using the Cache Decorator](#using-the-cache-decorator)
- [Service Layer Pattern](#service-layer-pattern)
- [Cache Invalidation](#cache-invalidation)
- [Monitoring](#monitoring)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

## Architecture

### Components

1. **Redis Client** (`app/core/cache.py`)
   - Connection pooling
   - Automatic fallback when Redis is unavailable
   - Health checks and timeouts

2. **Cache Decorators** (`@cache`)
   - Function-level caching
   - Support for both sync and async functions
   - Automatic serialization/deserialization

3. **Service Layer** (`app/services/`)
   - Business logic with built-in caching
   - Reusable cached functions
   - Separation of concerns

4. **Metrics** (Prometheus integration)
   - Cache hits/misses
   - Hit rate calculation
   - Memory usage tracking

5. **Auto-invalidation** (Celery tasks)
   - Automatic cache clearing on data updates
   - Pattern-based invalidation

## Quick Start

### Basic Usage

```python
from app.core.cache import cache, CacheTTL

@cache(ttl=CacheTTL.SHORT, prefix="user:profile")
def get_user_profile(user_id: int):
    # Expensive database query
    return db.query(User).filter(User.id == user_id).first()

# First call - cache miss, executes query
profile = get_user_profile(123)

# Second call - cache hit, returns cached data
profile = get_user_profile(123)  # Fast!
```

### Async Functions

```python
@cache(ttl=CacheTTL.MEDIUM, prefix="job:results")
async def get_job_results(job_id: int):
    result = await expensive_async_operation(job_id)
    return result
```

## Using the Cache Decorator

### Parameters

- **ttl** (int): Time to live in seconds
- **prefix** (str): Cache key prefix for organization

### TTL Constants

Use predefined TTL constants from `CacheTTL` class:

```python
CacheTTL.VERY_SHORT = 60      # 1 minute - rapidly changing data
CacheTTL.SHORT = 300          # 5 minutes - default
CacheTTL.MEDIUM = 1800        # 30 minutes - semi-static data
CacheTTL.LONG = 3600          # 1 hour - mostly static data
CacheTTL.VERY_LONG = 86400    # 24 hours - very static data
```

### Example: Different TTLs

```python
# User's active jobs - changes frequently
@cache(ttl=CacheTTL.VERY_SHORT, prefix="jobs:active")
def get_active_jobs(user_id: int):
    return db.query(Job).filter(
        Job.user_id == user_id,
        Job.status == JobStatus.PROCESSING
    ).all()

# Protein metadata - rarely changes
@cache(ttl=CacheTTL.LONG, prefix="protein:metadata")
def get_protein_metadata(protein_id: int):
    return db.query(Protein).filter(Protein.id == protein_id).first()
```

## Service Layer Pattern

The recommended approach is to create service modules with cacheable business logic.

### Example: Job Service

**File: `app/services/job_service.py`**

```python
from app.core.cache import cache, invalidate_pattern, CacheTTL

@cache(ttl=CacheTTL.SHORT, prefix="job:stats")
def get_user_job_stats(user_id: int, db: Session) -> Dict:
    """Get aggregated job statistics for a user"""
    total_jobs = db.query(Job).filter(Job.user_id == user_id).count()

    stats = {"total": total_jobs, "by_status": {}}
    for status in JobStatus:
        count = db.query(Job).filter(
            Job.user_id == user_id,
            Job.status == status
        ).count()
        stats["by_status"][status.value] = count

    return stats

def invalidate_user_job_cache(user_id: int):
    """Invalidate all job caches for a user"""
    pattern = f"cache:app.services.job_service.get_user_job_stats:*{user_id}*"
    invalidate_pattern(pattern)
```

### Using Services in Endpoints

```python
from app.services.job_service import get_user_job_stats, invalidate_user_job_cache

@router.get("/stats")
async def get_stats(
    current_user: User = Depends(get_current_user),
    db: Session = Depends(get_db)
):
    """Get user's job statistics (cached)"""
    return get_user_job_stats(current_user.id, db)

@router.post("/jobs")
async def create_job(...):
    # Create job logic here

    # Invalidate user's job cache
    invalidate_user_job_cache(current_user.id)

    return job
```

## Cache Invalidation

### Strategies

1. **Individual Key Invalidation**
   ```python
   from app.core.cache import invalidate_cache

   invalidate_cache("job:details", job_id=123)
   ```

2. **Pattern-Based Invalidation** (Recommended)
   ```python
   from app.core.cache import invalidate_pattern

   # Invalidate all caches for a user
   invalidate_pattern(f"user:{user_id}:*")

   # Invalidate all job caches
   invalidate_pattern("cache:*job*")
   ```

3. **Automatic Invalidation in Celery Tasks**

   Cache is automatically invalidated when jobs complete:

   ```python
   # In app/tasks/protein_tasks.py
   from app.services.job_service import invalidate_job_cache
   from app.services.protein_service import invalidate_protein_cache

   @celery_app.task
   def process_part_one(job_id: int):
       # ... processing logic ...

       # Invalidate caches after completion
       invalidate_job_cache(job_id, user_id)
       invalidate_protein_cache(protein_id, user_id)
   ```

### When to Invalidate

Invalidate caches when:
- ✅ Data is created
- ✅ Data is updated
- ✅ Data is deleted
- ✅ Relationships change
- ✅ Jobs complete or fail

### Invalidation Best Practices

```python
# ✅ Good - Invalidate after commit
db.commit()
invalidate_user_cache(user_id)

# ❌ Bad - Invalidate before commit
invalidate_user_cache(user_id)
db.commit()  # Could fail, leaving stale data

# ✅ Good - Use patterns for related data
invalidate_pattern(f"user:{user_id}:*")  # All user caches

# ❌ Bad - Too broad pattern
invalidate_pattern("*")  # Clears entire cache!
```

## Monitoring

### Prometheus Metrics

The caching system exports metrics to Prometheus:

- `cache_operations_total{operation="hit|miss|set|invalidate|error"}` - Counter of cache operations
- `cache_keys_total` - Current number of keys in cache
- `cache_memory_bytes` - Memory used by cache

### Health Check Endpoint

Check cache status:

```bash
curl http://localhost:5000/health/cache
```

Response:
```json
{
  "service": "protein-docking-api",
  "cache": {
    "status": "available",
    "used_memory": "2.5M",
    "used_memory_bytes": 2621440,
    "total_keys": 150,
    "connected_clients": 5,
    "uptime_days": 7,
    "hit_rate": 85.5
  }
}
```

### Grafana Dashboards

Monitor cache performance in Grafana:

1. **Cache Hit Rate**
   ```promql
   rate(cache_operations_total{operation="hit"}[5m]) /
   (rate(cache_operations_total{operation="hit"}[5m]) +
    rate(cache_operations_total{operation="miss"}[5m]))
   ```

2. **Cache Operations Rate**
   ```promql
   rate(cache_operations_total[5m])
   ```

3. **Cache Memory Usage**
   ```promql
   cache_memory_bytes
   ```

### Logging

Cache operations are logged at DEBUG level:

```
[DEBUG] Cache HIT: cache:app.services.job_service.get_user_job_stats:a1b2c3d4
[DEBUG] Cache MISS: cache:app.services.protein_service.get_protein_details:e5f6g7h8
[DEBUG] Cache SET: cache:app.services.job_service.get_recent_jobs:i9j0k1l2 (TTL: 60s)
[INFO]  Cache PATTERN INVALIDATED: user:123:* (5 keys)
```

## Best Practices

### 1. Choose Appropriate TTLs

```python
# Frequently changing data - short TTL
@cache(ttl=CacheTTL.VERY_SHORT)  # 1 minute
def get_job_status(job_id: int):
    pass

# Static reference data - long TTL
@cache(ttl=CacheTTL.VERY_LONG)  # 24 hours
def get_system_config():
    pass
```

### 2. Use Meaningful Prefixes

```python
# ✅ Good - Clear, hierarchical prefixes
@cache(prefix="user:profile")
@cache(prefix="job:results")
@cache(prefix="protein:metadata")

# ❌ Bad - Unclear prefixes
@cache(prefix="data")
@cache(prefix="stuff")
```

### 3. Cache at the Service Layer

```python
# ✅ Good - Cache in service layer
# app/services/job_service.py
@cache(ttl=CacheTTL.SHORT)
def get_job_stats(user_id: int, db: Session):
    return calculate_stats(user_id, db)

# app/api/jobs.py
@router.get("/stats")
def stats_endpoint(...):
    return get_job_stats(user.id, db)

# ❌ Bad - Cache in endpoint (harder to test/reuse)
@router.get("/stats")
@cache(ttl=CacheTTL.SHORT)  # Don't do this
def stats_endpoint(...):
    pass
```

### 4. Handle Redis Unavailability

The cache system automatically falls back when Redis is unavailable:

```python
# No special handling needed - cache decorator handles it
@cache(ttl=60)
def my_function():
    return expensive_operation()

# If Redis is down:
# - Function executes normally
# - Warning is logged
# - No exception is raised
```

### 5. Don't Cache Everything

**What to Cache:**
- ✅ Expensive database aggregations
- ✅ Complex calculations
- ✅ External API calls
- ✅ Frequently accessed, rarely changed data

**What NOT to Cache:**
- ❌ Simple database lookups (already fast)
- ❌ Data that changes every request
- ❌ User-specific sensitive data (consider security)
- ❌ Large binary files (use CDN instead)

### 6. Cache Warming (Optional)

For critical data, pre-populate cache on startup:

```python
# app/core/cache_warmer.py
def warm_cache():
    """Pre-populate cache with frequently accessed data"""
    logger.info("Warming cache...")

    # Cache most active users' data
    popular_users = get_popular_users()
    for user_id in popular_users:
        get_user_job_stats(user_id, db)

    logger.info("Cache warming complete")

# Call from startup event
@app.on_event("startup")
async def startup_event():
    warm_cache()
```

## Troubleshooting

### Issue: Cache Not Working

**Symptoms:** All requests show cache misses

**Solutions:**
1. Check Redis is running:
   ```bash
   docker-compose ps redis
   ```

2. Test Redis connection:
   ```bash
   curl http://localhost:5000/health/cache
   ```

3. Check logs for Redis errors:
   ```bash
   docker-compose logs backend | grep -i redis
   ```

### Issue: Stale Data Returned

**Symptoms:** Updated data not reflected in API responses

**Solutions:**
1. Verify cache invalidation is being called:
   ```python
   # Add logging
   logger.info(f"Invalidating cache for user {user_id}")
   invalidate_user_cache(user_id)
   ```

2. Check invalidation pattern matches cache prefix:
   ```python
   # Cache key: cache:app.services.job_service.get_job_stats:abc123
   # Pattern must match: cache:*job_service.get_job_stats*
   ```

3. Manually clear cache if needed:
   ```bash
   docker-compose exec redis redis-cli FLUSHDB
   ```

### Issue: High Memory Usage

**Symptoms:** Redis memory grows continuously

**Solutions:**
1. Review TTLs - ensure data expires:
   ```python
   # ❌ Bad - Never expires
   @cache(ttl=999999999)

   # ✅ Good - Reasonable TTL
   @cache(ttl=CacheTTL.LONG)
   ```

2. Check for memory leaks in patterns:
   ```bash
   # Connect to Redis
   docker-compose exec redis redis-cli

   # Check memory usage
   INFO memory

   # List largest keys
   MEMORY USAGE key_name
   ```

3. Enable Redis eviction policy (in production):
   ```yaml
   # docker-compose.yml
   redis:
     command: redis-server --maxmemory 256mb --maxmemory-policy allkeys-lru
   ```

### Issue: Cache Hit Rate Low

**Symptoms:** Most requests are cache misses

**Solutions:**
1. Check TTL isn't too short:
   ```python
   # ❌ Too short - data expires before reuse
   @cache(ttl=1)

   # ✅ Better
   @cache(ttl=CacheTTL.SHORT)
   ```

2. Verify cache keys are consistent:
   ```python
   # ❌ Bad - Different keys each time
   get_data(user_id=123)
   get_data(123)  # Different signature!

   # ✅ Good - Consistent
   get_data(user_id=123)
   get_data(user_id=123)
   ```

3. Check if data access patterns benefit from caching:
   - If data is rarely accessed twice, caching won't help
   - If data changes faster than TTL, caching is ineffective

## Advanced Topics

### Custom Serialization

For complex objects, implement custom serialization:

```python
from dataclasses import dataclass, asdict

@dataclass
class ComplexResult:
    id: int
    data: dict
    metadata: dict

@cache(ttl=CacheTTL.MEDIUM)
def get_complex_data(id: int) -> dict:
    result = ComplexResult(
        id=id,
        data={"key": "value"},
        metadata={"timestamp": datetime.now()}
    )
    # Convert to dict for caching
    return asdict(result)
```

### Cache Stampede Prevention

For very expensive operations, consider using cache locks:

```python
from redis.lock import Lock

def get_expensive_data_with_lock(key: str):
    lock_key = f"lock:{key}"

    # Try to get from cache
    cached = redis_client.get(key)
    if cached:
        return cached

    # Acquire lock to prevent stampede
    lock = Lock(redis_client, lock_key, timeout=10)
    if lock.acquire(blocking=True):
        try:
            # Double-check cache
            cached = redis_client.get(key)
            if cached:
                return cached

            # Compute value
            value = expensive_computation()
            redis_client.setex(key, 300, value)
            return value
        finally:
            lock.release()
```

### Distributed Cache Warming

For multi-instance deployments, coordinate cache warming:

```python
def warm_cache_distributed():
    """Warm cache only on one instance"""
    lock_key = "cache:warming:lock"
    lock = Lock(redis_client, lock_key, timeout=300)

    if lock.acquire(blocking=False):
        try:
            warm_cache()
        finally:
            lock.release()
    else:
        logger.info("Another instance is warming cache")
```

## Summary

- ✅ Use `@cache` decorator for expensive operations
- ✅ Apply caching at the service layer, not endpoints
- ✅ Choose appropriate TTLs for your data
- ✅ Always invalidate cache when data changes
- ✅ Monitor cache hit rate and memory usage
- ✅ Handle Redis unavailability gracefully
- ✅ Test cache behavior in your unit tests

## See Also

- [Observability Guide](./OBSERVABILITY.md) - Monitoring cache metrics
- [API Documentation](./API.md) - API endpoints
- [Testing Guide](./TESTING.md) - Testing cached functions

---

**Last Updated:** 2024-01-14
**Version:** 2.1.0
