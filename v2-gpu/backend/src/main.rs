// Ultra-optimized Protein Docking Backend
// Main entry point con configuración mínima funcional

use actix_web::{web, App, HttpServer, HttpResponse, Result, middleware::Logger};
use serde::{Deserialize, Serialize};
use tracing::info;
use std::sync::Arc;

#[cfg(feature = "postgres")]
use protein_docking_backend::database::{initialize_database_pool, DatabaseConfig};
#[cfg(not(feature = "postgres"))]
use protein_docking_backend::database::{initialize_simple_pool, simple_health_check};
use protein_docking_backend::api::handlers::jobs::JobManager;

// Import handlers implementados
use protein_docking_backend::api::handlers::{
    upload::{upload_protein_file, get_file_info, download_protein_file, delete_file, list_uploaded_files},
    jobs::{create_docking_job, list_jobs, get_job_info, cancel_job, get_system_stats},
    websocket::{websocket_jobs_handler, websocket_job_handler, websocket_status},
};

// Global memory allocator ultra-optimizado
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

// Configuración básica
#[derive(Debug, Serialize, Deserialize)]
struct AppConfig {
    host: String,
    port: u16,
    workers: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            workers: num_cpus::get(),
        }
    }
}

// Respuesta básica para health check
#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
    timestamp: chrono::DateTime<chrono::Utc>,
    capabilities: SystemCapabilities,
}

#[derive(Serialize)]
struct SystemCapabilities {
    cpu_cores: usize,
    simd_support: Vec<String>,
    memory_allocator: String,
}

// Handlers básicos
async fn health() -> Result<HttpResponse> {
    let mut simd_support = Vec::new();
    
    #[cfg(target_arch = "x86_64")]
    {
        if std::arch::is_x86_feature_detected!("avx2") {
            simd_support.push("AVX2".to_string());
        }
        if std::arch::is_x86_feature_detected!("avx512f") {
            simd_support.push("AVX-512".to_string());
        }
        if std::arch::is_x86_feature_detected!("sse2") {
            simd_support.push("SSE2".to_string());
        }
    }
    
    let response = HealthResponse {
        status: "healthy".to_string(),
        version: "2.0.0".to_string(),
        timestamp: chrono::Utc::now(),
        capabilities: SystemCapabilities {
            cpu_cores: num_cpus::get(),
            simd_support,
            memory_allocator: "mimalloc".to_string(),
        },
    };
    
    Ok(HttpResponse::Ok().json(response))
}

async fn root() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "🧬 Protein Docking v2 - Ultra Performance Backend",
        "status": "active",
        "performance_mode": "ULTRA",
        "features": {
            "simd_vectorization": true,
            "memory_pooling": true,
            "parallel_processing": true,
            "ultra_optimized": true
        },
        "endpoints": {
            "health": "/health",
            "api": "/api/v1"
        }
    })))
}

async fn api_info() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "api_version": "v1",
        "endpoints": {
            "ping": "/api/v1/ping",
            "dock": "/api/v1/dock (TODO)",
            "upload": "/api/v1/upload (TODO)",
            "jobs": "/api/v1/jobs (TODO)"
        },
        "supported_formats": ["PDB", "MOL", "SDF"],
        "max_file_size": "500MB"
    })))
}

// Configurar rutas
fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg
        .route("/", web::get().to(root))
        .route("/health", web::get().to(health))
        .service(
            web::scope("/api/v1")
                .route("/", web::get().to(api_info))
                .route("/ping", web::get().to(|| async {
                    HttpResponse::Ok().json(serde_json::json!({
                        "message": "pong",
                        "timestamp": chrono::Utc::now(),
                        "performance": "ULTRA_OPTIMIZED"
                    }))
                }))
                // Endpoints de archivos (implementados)
                .route("/upload", web::post().to(upload_protein_file))
                .route("/files/{file_id}", web::get().to(get_file_info))
                .route("/files/{file_id}/download", web::get().to(download_protein_file))
                .route("/files/{file_id}", web::delete().to(delete_file))
                .route("/files", web::get().to(list_uploaded_files))
                // Endpoints de jobs (implementados)
                .route("/jobs", web::post().to(create_docking_job))
                .route("/jobs", web::get().to(list_jobs))
                .route("/jobs/{job_id}", web::get().to(get_job_info))
                .route("/jobs/{job_id}/cancel", web::post().to(cancel_job))
                .route("/system/stats", web::get().to(get_system_stats))
        )
        // WebSocket endpoints (implementados)
        .route("/ws/jobs", web::get().to(websocket_jobs_handler))
        .route("/ws/jobs/{job_id}", web::get().to(websocket_job_handler))
        .route("/ws/status", web::get().to(websocket_status));
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Configurar logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_target(false)
        .with_thread_ids(true)
        .init();

    // Configuración
    let config = AppConfig::default();
    
    info!("🚀 Iniciando Protein Docking Backend v2.0.0");
    info!("📍 Host: {}:{}", config.host, config.port);
    info!("⚡ Workers: {}", config.workers);
    info!("🧬 Performance mode: ULTRA");
    info!("🧠 Memory allocator: mimalloc");
    
    // Inicializar base de datos
    #[cfg(feature = "postgres")]
    {
        info!("📊 Initializing PostgreSQL database...");
        let db_config = DatabaseConfig::default();
        let _pool = match initialize_database_pool(db_config).await {
            Ok(pool) => {
                info!("✅ Database connected successfully");
                pool
            },
            Err(e) => {
                tracing::warn!("⚠️ Database connection failed: {}. Running with simple storage.", e);
                let _ = initialize_simple_pool()?;
                return start_server_simple(config).await;
            }
        };
    }
    
    #[cfg(not(feature = "postgres"))]
    {
        info!("📊 Initializing simple in-memory storage...");
        let _pool = initialize_simple_pool()?;
        info!("✅ Simple storage initialized");
    }
    
    // Inicializar job manager
    let job_manager = JobManager::new(4); // Max 4 jobs concurrentes
    
    // System capabilities check
    #[cfg(target_arch = "x86_64")]
    {
        if std::arch::is_x86_feature_detected!("avx2") {
            info!("✅ AVX2 SIMD support detected");
        }
        if std::arch::is_x86_feature_detected!("avx512f") {
            info!("✅ AVX-512 SIMD support detected");
        }
    }
    
    info!("🧵 Rayon thread pool size: {}", rayon::current_num_threads());

    // Crear servidor HTTP con dependencies
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(job_manager.clone()))
            .wrap(Logger::default())
            .configure(configure_routes)
    })
    .bind(format!("{}:{}", config.host, config.port))?
    .workers(config.workers)
    .run()
    .await
}

/// Fallback: iniciar servidor con almacenamiento simple
async fn start_server_simple(config: AppConfig) -> std::io::Result<()> {
    info!("📊 Running with simple in-memory storage");
    
    let job_manager = JobManager::new(2); // Reduced capacity
    
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(job_manager.clone()))
            .wrap(Logger::default())
            .configure(configure_routes)
    })
    .bind(format!("{}:{}", config.host, config.port))?
    .workers(config.workers)
    .run()
    .await
}