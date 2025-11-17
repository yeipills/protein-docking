"""
Tests for Redis caching layer

Tests cache decorators, invalidation, and service layer caching.
"""
import pytest
from unittest.mock import Mock, patch, MagicMock
from app.core.cache import (
    cache,
    invalidate_cache,
    invalidate_pattern,
    get_cache_stats,
    CacheTTL,
    _generate_cache_key,
    _serialize_value,
)


class TestCacheKeyGeneration:
    """Test cache key generation"""

    def test_generate_cache_key_simple(self):
        """Test key generation with simple arguments"""
        key = _generate_cache_key("test_prefix", 123, "abc")
        assert key.startswith("test_prefix:")
        assert len(key) > len("test_prefix:")

    def test_generate_cache_key_with_kwargs(self):
        """Test key generation with keyword arguments"""
        key1 = _generate_cache_key("test", user_id=123, status="active")
        key2 = _generate_cache_key("test", status="active", user_id=123)
        # Keys should be the same regardless of kwarg order
        assert key1 == key2

    def test_generate_cache_key_different_args(self):
        """Test that different arguments produce different keys"""
        key1 = _generate_cache_key("test", 123)
        key2 = _generate_cache_key("test", 456)
        assert key1 != key2


class TestCacheSerialization:
    """Test value serialization for caching"""

    def test_serialize_simple_dict(self):
        """Test serialization of simple dictionary"""
        value = {"id": 123, "name": "test"}
        serialized = _serialize_value(value)
        assert isinstance(serialized, str)
        assert "123" in serialized
        assert "test" in serialized

    def test_serialize_list(self):
        """Test serialization of list"""
        value = [1, 2, 3, 4, 5]
        serialized = _serialize_value(value)
        assert isinstance(serialized, str)

    def test_serialize_with_datetime(self):
        """Test serialization with datetime objects"""
        from datetime import datetime

        value = {
            "id": 1,
            "created_at": datetime(2024, 1, 1, 12, 0, 0)
        }
        serialized = _serialize_value(value)
        assert "2024-01-01" in serialized


class TestCacheDecorator:
    """Test cache decorator functionality"""

    @patch('app.core.cache.redis_client')
    def test_cache_decorator_miss_then_set(self, mock_redis):
        """Test cache miss and subsequent set"""
        mock_redis.get.return_value = None

        @cache(ttl=60, prefix="test")
        def expensive_function(x):
            return x * 2

        result = expensive_function(5)

        assert result == 10
        mock_redis.get.assert_called_once()
        mock_redis.setex.assert_called_once()

    @patch('app.core.cache.redis_client')
    def test_cache_decorator_hit(self, mock_redis):
        """Test cache hit returns cached value"""
        import json
        mock_redis.get.return_value = json.dumps({"result": 42})

        @cache(ttl=60, prefix="test")
        def expensive_function():
            return {"result": 100}  # Should not be called

        result = expensive_function()

        assert result == {"result": 42}
        mock_redis.get.assert_called_once()
        mock_redis.setex.assert_not_called()

    @patch('app.core.cache.redis_client')
    def test_cache_decorator_async(self, mock_redis):
        """Test cache decorator with async functions"""
        import asyncio
        mock_redis.get.return_value = None

        @cache(ttl=60, prefix="test")
        async def async_function(x):
            await asyncio.sleep(0.01)
            return x * 3

        result = asyncio.run(async_function(7))

        assert result == 21
        mock_redis.get.assert_called_once()
        mock_redis.setex.assert_called_once()

    @patch('app.core.cache.redis_client', None)
    def test_cache_decorator_no_redis(self):
        """Test decorator works when Redis is unavailable"""
        @cache(ttl=60, prefix="test")
        def function_without_redis(x):
            return x + 1

        result = function_without_redis(5)
        assert result == 6  # Function should execute normally


class TestCacheInvalidation:
    """Test cache invalidation functions"""

    @patch('app.core.cache.redis_client')
    def test_invalidate_cache_success(self, mock_redis):
        """Test successful cache invalidation"""
        mock_redis.delete.return_value = 1

        result = invalidate_cache("test_prefix", 123, status="active")

        assert result is True
        mock_redis.delete.assert_called_once()

    @patch('app.core.cache.redis_client')
    def test_invalidate_cache_not_found(self, mock_redis):
        """Test invalidation when key doesn't exist"""
        mock_redis.delete.return_value = 0

        result = invalidate_cache("test_prefix", 999)

        assert result is False

    @patch('app.core.cache.redis_client')
    def test_invalidate_pattern_multiple_keys(self, mock_redis):
        """Test pattern invalidation with multiple keys"""
        mock_redis.keys.return_value = ["key1", "key2", "key3"]
        mock_redis.delete.return_value = 3

        count = invalidate_pattern("user:*")

        assert count == 3
        mock_redis.keys.assert_called_once_with("user:*")
        mock_redis.delete.assert_called_once_with("key1", "key2", "key3")

    @patch('app.core.cache.redis_client')
    def test_invalidate_pattern_no_matches(self, mock_redis):
        """Test pattern invalidation with no matching keys"""
        mock_redis.keys.return_value = []

        count = invalidate_pattern("nonexistent:*")

        assert count == 0
        mock_redis.delete.assert_not_called()


class TestCacheStats:
    """Test cache statistics"""

    @patch('app.core.cache.redis_client')
    def test_get_cache_stats_available(self, mock_redis):
        """Test getting cache stats when Redis is available"""
        mock_redis.info.return_value = {
            "used_memory_human": "1.5M",
            "used_memory": 1572864,
            "connected_clients": 5,
            "uptime_in_days": 7
        }
        mock_redis.dbsize.return_value = 150

        stats = get_cache_stats()

        assert stats["status"] == "available"
        assert stats["total_keys"] == 150
        assert stats["used_memory"] == "1.5M"
        assert "hit_rate" in stats

    @patch('app.core.cache.redis_client', None)
    def test_get_cache_stats_unavailable(self):
        """Test getting stats when Redis is unavailable"""
        stats = get_cache_stats()

        assert stats["status"] == "unavailable"


class TestCacheTTLConstants:
    """Test predefined TTL constants"""

    def test_ttl_constants_defined(self):
        """Test that TTL constants are properly defined"""
        assert CacheTTL.VERY_SHORT == 60
        assert CacheTTL.SHORT == 300
        assert CacheTTL.MEDIUM == 1800
        assert CacheTTL.LONG == 3600
        assert CacheTTL.VERY_LONG == 86400

    def test_ttl_order(self):
        """Test that TTL constants are in ascending order"""
        assert CacheTTL.VERY_SHORT < CacheTTL.SHORT
        assert CacheTTL.SHORT < CacheTTL.MEDIUM
        assert CacheTTL.MEDIUM < CacheTTL.LONG
        assert CacheTTL.LONG < CacheTTL.VERY_LONG


class TestServiceLayerCaching:
    """Test caching in service layer"""

    @patch('app.core.cache.redis_client')
    def test_job_service_caching(self, mock_redis):
        """Test that job service functions use caching"""
        from app.services.job_service import get_user_job_stats
        from app.models.job import JobStatus

        # Mock database session
        mock_db = Mock()
        mock_db.query.return_value.filter.return_value.count.return_value = 10

        mock_redis.get.return_value = None

        # Call service function
        stats = get_user_job_stats(123, mock_db)

        # Verify cache was attempted
        mock_redis.get.assert_called()
        mock_redis.setex.assert_called()

    @patch('app.core.cache.redis_client')
    def test_protein_service_caching(self, mock_redis):
        """Test that protein service functions use caching"""
        from app.services.protein_service import get_user_protein_count

        mock_db = Mock()
        mock_db.query.return_value.filter.return_value.count.return_value = 5

        mock_redis.get.return_value = None

        count = get_user_protein_count(123, mock_db)

        assert count == 5
        mock_redis.get.assert_called()
        mock_redis.setex.assert_called()


@pytest.mark.integration
class TestCacheIntegration:
    """Integration tests with real Redis (requires Redis running)"""

    @pytest.fixture
    def redis_client(self):
        """Get real Redis client for integration tests"""
        from app.core.cache import redis_client
        if redis_client is None:
            pytest.skip("Redis not available for integration tests")
        # Clear test keys before test
        redis_client.delete("integration_test:*")
        yield redis_client
        # Cleanup after test
        keys = redis_client.keys("integration_test:*")
        if keys:
            redis_client.delete(*keys)

    def test_cache_full_cycle(self, redis_client):
        """Test full cache lifecycle: set, get, invalidate"""
        from app.core.cache import cache, invalidate_cache

        call_count = 0

        @cache(ttl=60, prefix="integration_test")
        def cached_function(x):
            nonlocal call_count
            call_count += 1
            return x * 2

        # First call - cache miss
        result1 = cached_function(10)
        assert result1 == 20
        assert call_count == 1

        # Second call - cache hit
        result2 = cached_function(10)
        assert result2 == 20
        assert call_count == 1  # Should not increment

        # Invalidate cache
        invalidate_cache("cache:integration_test.test_cache_full_cycle.cached_function", 10)

        # Third call - cache miss after invalidation
        result3 = cached_function(10)
        assert result3 == 20
        assert call_count == 2
