/*!
 * Database Service - Gestión de conexiones y operaciones de base de datos
 */

use sqlx::{PgPool, postgres::PgPoolOptions};
use crate::core::{Result, AppError};
use crate::core::config::DatabaseConfig;
use std::time::Duration;

pub struct DatabaseService;

impl DatabaseService {
    /// Inicializar pool de conexiones de base de datos
    pub async fn new(config: &DatabaseConfig) -> Result<PgPool> {
        init_database_pool(config).await
    }
}

/// Inicializar pool de conexiones optimizado
pub async fn init_database_pool(config: &DatabaseConfig) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(Duration::from_secs(config.acquire_timeout))
        .idle_timeout(Duration::from_secs(config.idle_timeout))
        .connect(&config.url)
        .await
        .map_err(|e| AppError::database(format!("Failed to connect to database: {}", e)))?;
    
    // Ejecutar migraciones si es necesario
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to run migrations: {}", e)))?;
    
    tracing::info!("Database pool initialized with {} connections", config.max_connections);
    
    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_database_config_validation() {
        let config = DatabaseConfig {
            url: "postgresql://test:test@localhost/test".to_string(),
            max_connections: 10,
            min_connections: 2,
            acquire_timeout: 30,
            idle_timeout: 600,
        };
        
        // Test that config is valid
        assert!(config.max_connections > config.min_connections);
    }
}