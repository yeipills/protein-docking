/*!
 * Estado global de la aplicación compartido entre handlers
 */

use std::sync::Arc;
use crate::core::config::AppConfig;
use crate::algorithms::DockingEngine;
use crate::services::memory::MemoryPool;

/// Estado global de la aplicación
#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub db_pool: sqlx::PgPool,
    pub redis_client: redis::cluster::ClusterClient,
    pub gpu_context: Option<Arc<dyn GpuContext>>,
    pub algorithm_engine: Arc<DockingEngine>,
    pub memory_pool: Arc<MemoryPool>,
}

/// Trait para contexto GPU (permite diferentes implementaciones)
pub trait GpuContext: Send + Sync {
    fn device_count(&self) -> usize;
    fn device_name(&self, device_id: usize) -> Option<String>;
    fn memory_total(&self, device_id: usize) -> Option<usize>;
    fn memory_free(&self, device_id: usize) -> Option<usize>;
    fn is_available(&self) -> bool;
}

impl AppState {
    /// Verificar salud del sistema
    pub async fn health_check(&self) -> HealthStatus {
        let mut status = HealthStatus::default();
        
        // Check database
        status.database = match sqlx::query("SELECT 1").fetch_one(&self.db_pool).await {
            Ok(_) => ComponentStatus::Healthy,
            Err(e) => ComponentStatus::Unhealthy(e.to_string()),
        };
        
        // Check Redis
        status.cache = match self.test_redis_connection().await {
            Ok(_) => ComponentStatus::Healthy,
            Err(e) => ComponentStatus::Unhealthy(e.to_string()),
        };
        
        // Check GPU
        status.gpu = match &self.gpu_context {
            Some(ctx) if ctx.is_available() => ComponentStatus::Healthy,
            Some(_) => ComponentStatus::Unhealthy("GPU not available".to_string()),
            None => ComponentStatus::NotConfigured,
        };
        
        // Check memory pool
        status.memory = if self.memory_pool.available_capacity() > 0 {
            ComponentStatus::Healthy
        } else {
            ComponentStatus::Unhealthy("Memory pool exhausted".to_string())
        };
        
        // Overall status
        status.overall = if matches!(status.database, ComponentStatus::Healthy) &&
                              matches!(status.cache, ComponentStatus::Healthy) &&
                              matches!(status.memory, ComponentStatus::Healthy) {
            ComponentStatus::Healthy
        } else {
            ComponentStatus::Degraded
        };
        
        status
    }
    
    /// Test Redis connection
    async fn test_redis_connection(&self) -> Result<(), redis::RedisError> {
        use redis::AsyncCommands;
        
        let mut conn = self.redis_client.get_async_connection().await?;
        let _: String = conn.ping().await?;
        Ok(())
    }
    
    /// Obtener métricas del sistema
    pub fn system_metrics(&self) -> SystemMetrics {
        SystemMetrics {
            database_connections: self.db_pool.size() as usize,
            memory_pool_usage: self.memory_pool.usage_percentage(),
            gpu_memory_usage: self.gpu_context.as_ref()
                .and_then(|ctx| ctx.memory_free(0))
                .map(|free| {
                    let total = ctx.memory_total(0).unwrap_or(0);
                    if total > 0 {
                        ((total - free) as f64 / total as f64 * 100.0) as u8
                    } else {
                        0
                    }
                }),
            active_algorithms: self.algorithm_engine.active_computations(),
            uptime_seconds: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct HealthStatus {
    pub overall: ComponentStatus,
    pub database: ComponentStatus,
    pub cache: ComponentStatus,
    pub gpu: ComponentStatus,
    pub memory: ComponentStatus,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, serde::Serialize)]
pub enum ComponentStatus {
    Healthy,
    Degraded,
    Unhealthy(String),
    NotConfigured,
}

#[derive(Debug, serde::Serialize)]
pub struct SystemMetrics {
    pub database_connections: usize,
    pub memory_pool_usage: u8, // Percentage
    pub gpu_memory_usage: Option<u8>, // Percentage
    pub active_algorithms: usize,
    pub uptime_seconds: u64,
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self {
            overall: ComponentStatus::Degraded,
            database: ComponentStatus::NotConfigured,
            cache: ComponentStatus::NotConfigured,
            gpu: ComponentStatus::NotConfigured,
            memory: ComponentStatus::NotConfigured,
            timestamp: chrono::Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_health_status_default() {
        let status = HealthStatus::default();
        assert!(matches!(status.overall, ComponentStatus::Degraded));
    }
    
    #[test]
    fn test_system_metrics_serialization() {
        let metrics = SystemMetrics {
            database_connections: 10,
            memory_pool_usage: 50,
            gpu_memory_usage: Some(75),
            active_algorithms: 3,
            uptime_seconds: 3600,
        };
        
        let json = serde_json::to_string(&metrics).unwrap();
        assert!(json.contains("database_connections"));
        assert!(json.contains("50"));
    }
}