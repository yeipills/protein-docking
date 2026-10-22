/*!
 * Core module - Configuración y estado central del sistema
 */

pub mod config;
pub mod error;
pub mod state;

pub use config::AppConfig;
pub use error::{AppError, Result};
pub use state::AppState;

/// Constantes de rendimiento del sistema
pub mod constants {
    /// Configuraciones de performance por defecto
    pub const DEFAULT_WORKER_THREADS: usize = 16;
    pub const DEFAULT_MEMORY_POOL_SIZE: usize = 1024 * 1024 * 512; // 512MB
    pub const DEFAULT_BATCH_SIZE: usize = 1024;
    pub const DEFAULT_TIMEOUT_MS: u64 = 30_000;
    
    /// Límites de seguridad
    pub const MAX_FILE_SIZE: usize = 100 * 1024 * 1024; // 100MB
    pub const MAX_CONCURRENT_REQUESTS: usize = 10_000;
    pub const MAX_MEMORY_PER_REQUEST: usize = 50 * 1024 * 1024; // 50MB
    
    /// Configuraciones de algoritmos
    pub const SIMD_CHUNK_SIZE: usize = 8; // Para AVX-256
    pub const GPU_BATCH_SIZE: usize = 2048;
    pub const SPATIAL_INDEX_DEPTH: usize = 16;
}