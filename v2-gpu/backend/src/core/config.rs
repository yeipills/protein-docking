/*!
 * Configuración del sistema usando variables de entorno
 */

use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub performance: PerformanceConfig,
    pub security: SecurityConfig,
    pub algorithms: AlgorithmConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: usize,
    pub max_connections: usize,
    pub keep_alive: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout: u64,
    pub idle_timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub cluster_urls: Vec<String>,
    pub max_connections: u32,
    pub pool_timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub memory_pool_size: usize,
    pub thread_count: usize,
    pub enable_simd: bool,
    pub enable_gpu: bool,
    pub batch_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub max_file_size: usize,
    pub allowed_file_types: Vec<String>,
    pub rate_limit_per_minute: u32,
    pub enable_cors: bool,
    pub cors_origins: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmConfig {
    pub spatial_index_depth: usize,
    pub convergence_threshold: f64,
    pub max_iterations: usize,
    pub enable_ml_predictions: bool,
}

impl AppConfig {
    /// Cargar configuración desde variables de entorno
    pub fn from_env() -> Result<Self, config::ConfigError> {
        let mut settings = config::Config::builder();
        
        // Configuraciones por defecto
        settings = settings
            // Server defaults
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 8080)?
            .set_default("server.workers", num_cpus::get())?
            .set_default("server.max_connections", 10000)?
            .set_default("server.keep_alive", 30)?
            
            // Database defaults
            .set_default("database.url", "postgresql://localhost:5432/protein_docking")?
            .set_default("database.max_connections", 50)?
            .set_default("database.min_connections", 5)?
            .set_default("database.acquire_timeout", 30)?
            .set_default("database.idle_timeout", 600)?
            
            // Redis defaults
            .set_default("redis.cluster_urls", vec!["redis://localhost:6379"])?
            .set_default("redis.max_connections", 20)?
            .set_default("redis.pool_timeout", 5)?
            
            // Performance defaults
            .set_default("performance.memory_pool_size", 512 * 1024 * 1024)? // 512MB
            .set_default("performance.thread_count", num_cpus::get())?
            .set_default("performance.enable_simd", true)?
            .set_default("performance.enable_gpu", cfg!(feature = "gpu"))?
            .set_default("performance.batch_size", 1024)?
            
            // Security defaults
            .set_default("security.max_file_size", 100 * 1024 * 1024)? // 100MB
            .set_default("security.allowed_file_types", vec!["stl", "pdb", "face", "vert"])?
            .set_default("security.rate_limit_per_minute", 1000)?
            .set_default("security.enable_cors", true)?
            .set_default("security.cors_origins", vec!["http://localhost:3000"])?
            
            // Algorithm defaults
            .set_default("algorithms.spatial_index_depth", 16)?
            .set_default("algorithms.convergence_threshold", 1e-6)?
            .set_default("algorithms.max_iterations", 10000)?
            .set_default("algorithms.enable_ml_predictions", true)?;
        
        // Sobreescribir con variables de entorno
        settings = settings.add_source(
            config::Environment::with_prefix("PROTEIN_DOCKING")
                .separator("_")
                .try_parsing(true)
        );
        
        // Build y deserializar
        let config = settings.build()?;
        config.try_deserialize()
    }
    
    /// Validar configuración
    pub fn validate(&self) -> Result<(), String> {
        if self.server.port == 0 {
            return Err("Server port cannot be 0".to_string());
        }
        
        if self.database.max_connections == 0 {
            return Err("Database max_connections cannot be 0".to_string());
        }
        
        if self.performance.memory_pool_size < 1024 * 1024 {
            return Err("Memory pool size too small (minimum 1MB)".to_string());
        }
        
        if self.security.max_file_size > 1024 * 1024 * 1024 {
            return Err("Max file size too large (maximum 1GB)".to_string());
        }
        
        Ok(())
    }
    
    /// Mostrar configuración para debugging (sin datos sensibles)
    pub fn debug_safe(&self) -> String {
        format!(
            "AppConfig {{ server: {}:{}, workers: {}, performance: {{ threads: {}, simd: {}, gpu: {} }} }}",
            self.server.host,
            self.server.port,
            self.server.workers,
            self.performance.thread_count,
            self.performance.enable_simd,
            self.performance.enable_gpu
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        // Set minimal env vars for testing
        env::set_var("PROTEIN_DOCKING_DATABASE_URL", "postgresql://test:test@localhost/test");
        
        let config = AppConfig::from_env().expect("Should load default config");
        assert!(config.validate().is_ok());
        assert_eq!(config.server.host, "0.0.0.0");
        assert!(config.server.port > 0);
    }
    
    #[test]
    fn test_config_validation() {
        let mut config = AppConfig::from_env().unwrap();
        
        // Test invalid port
        config.server.port = 0;
        assert!(config.validate().is_err());
        
        // Test invalid memory pool
        config.server.port = 8080;
        config.performance.memory_pool_size = 1024; // Too small
        assert!(config.validate().is_err());
    }
}