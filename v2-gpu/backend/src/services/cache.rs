/*!
 * Cache Service - Redis cluster management
 */

use redis::cluster::{ClusterClient, ClusterConnection};
use crate::core::{Result, AppError};
use crate::core::config::RedisConfig;

pub struct CacheService;

impl CacheService {
    pub async fn new(config: &RedisConfig) -> Result<ClusterClient> {
        init_redis_cluster(config).await
    }
}

/// Inicializar Redis cluster
pub async fn init_redis_cluster(config: &RedisConfig) -> Result<ClusterClient> {
    let client = ClusterClient::new(config.cluster_urls.clone())
        .map_err(|e| AppError::cache(format!("Failed to create Redis client: {}", e)))?;
    
    // Test connection
    let mut conn = client.get_async_connection().await
        .map_err(|e| AppError::cache(format!("Failed to connect to Redis cluster: {}", e)))?;
    
    // Ping test
    use redis::AsyncCommands;
    let _: String = conn.ping().await
        .map_err(|e| AppError::cache(format!("Redis ping failed: {}", e)))?;
    
    tracing::info!("Redis cluster initialized with {} nodes", config.cluster_urls.len());
    
    Ok(client)
}