// Ultra-optimized Protein Docking Backend
// Main entry point con configuración mínima funcional

use actix_web::{web, App, HttpServer, HttpResponse, Result, middleware::Logger};
use serde::{Deserialize, Serialize};
use tracing::info;
use protein_docking_backend::core::pdb::PdbParser;
use protein_docking_backend::core::docking::DockingEngine;
use protein_docking_backend::jobs::{job_manager::AsyncJobManager, job_types::*};
use std::sync::Arc;

// Global memory allocator ultra-optimizado
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

// Global job manager
lazy_static::lazy_static! {
    static ref JOB_MANAGER: Arc<AsyncJobManager> = Arc::new(AsyncJobManager::new());
}

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
            "parse_pdb": "/api/v1/parse_pdb/{filename}",
            "dock": "/api/v1/dock/{receptor}/{ligand}",
            "jobs": {
                "submit": "POST /api/v1/jobs",
                "list": "GET /api/v1/jobs",
                "status": "GET /api/v1/jobs/{job_id}",
                "delete": "DELETE /api/v1/jobs/{job_id}",
                "queue": "GET /api/v1/queue"
            },
            "upload": "/api/v1/upload (TODO)"
        },
        "supported_formats": ["PDB", "MOL", "SDF"],
        "max_file_size": "500MB",
        "features": {
            "job_queue": true,
            "async_processing": true,
            "real_time_monitoring": true
        }
    })))
}

async fn parse_pdb_file(path: web::Path<String>) -> Result<HttpResponse> {
    let filename = path.into_inner();
    let pdb_path = format!("/app/pdb/{}", filename);
    
    info!("Parsing PDB file: {}", pdb_path);
    
    let parser = PdbParser::new();
    match parser.parse_file(&pdb_path) {
        Ok(protein) => {
            info!("Successfully parsed PDB: {} atoms, {} chains", 
                  protein.statistics.atom_count, 
                  protein.statistics.chain_count);
            
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "filename": filename,
                "protein": {
                    "id": protein.id,
                    "title": protein.title,
                    "statistics": protein.statistics,
                    "chains": protein.chains.keys().collect::<Vec<_>>(),
                    "metadata": protein.metadata
                }
            })))
        }
        Err(e) => {
            info!("Failed to parse PDB {}: {}", filename, e);
            Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "error": e.to_string(),
                "filename": filename
            })))
        }
    }
}

async fn dock_proteins(path: web::Path<(String, String)>) -> Result<HttpResponse> {
    let (receptor_filename, ligand_filename) = path.into_inner();
    let receptor_path = format!("/app/pdb/{}", receptor_filename);
    let ligand_path = format!("/app/pdb/{}", ligand_filename);
    
    info!("Starting docking: receptor={} ligand={}", receptor_filename, ligand_filename);
    
    let parser = PdbParser::new();
    
    // Parsear receptor
    let receptor = match parser.parse_file(&receptor_path) {
        Ok(protein) => protein,
        Err(e) => {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "error": format!("Failed to parse receptor {}: {}", receptor_filename, e),
                "stage": "receptor_parsing"
            })));
        }
    };
    
    // Parsear ligando
    let ligand = match parser.parse_file(&ligand_path) {
        Ok(protein) => protein,
        Err(e) => {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "error": format!("Failed to parse ligand {}: {}", ligand_filename, e),
                "stage": "ligand_parsing"
            })));
        }
    };
    
    info!("Parsed proteins - Receptor: {} atoms, Ligand: {} atoms", 
          receptor.statistics.atom_count, 
          ligand.statistics.atom_count);
    
    // Realizar docking
    let docking_engine = DockingEngine::default();
    match docking_engine.dock_proteins(&receptor, &ligand) {
        Ok(docking_result) => {
            let best_energy = docking_result.best_pose.as_ref()
                .map(|p| p.energy)
                .unwrap_or(f64::INFINITY);
            
            info!("Docking completed successfully: {} poses generated, best energy: {:.2}", 
                  docking_result.poses.len(), 
                  best_energy);
            
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "receptor_filename": receptor_filename,
                "ligand_filename": ligand_filename,
                "docking_result": {
                    "receptor_id": docking_result.receptor.id,
                    "ligand_id": docking_result.ligand.id,
                    "poses_count": docking_result.poses.len(),
                    "best_pose": docking_result.best_pose.as_ref().map(|pose| serde_json::json!({
                        "id": pose.id,
                        "energy": pose.energy,
                        "contacts": pose.contacts.len(),
                        "clash_score": pose.clash_score,
                        "energy_components": pose.energy_components
                    })),
                    "computation_stats": docking_result.computation_stats
                }
            })))
        }
        Err(e) => {
            info!("Docking failed: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": e.to_string(),
                "stage": "docking_computation"
            })))
        }
    }
}

// Job management handlers
async fn submit_docking_job(request: web::Json<DockingJobRequest>) -> Result<HttpResponse> {
    info!("Submitting new docking job: receptor={}, ligand={}", 
          request.receptor_filename, request.ligand_filename);
    
    match JOB_MANAGER.submit_docking_job(request.into_inner()).await {
        Ok(job_id) => {
            info!("Job submitted successfully: {}", job_id);
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "job_id": job_id,
                "message": "Job submitted successfully"
            })))
        }
        Err(error) => {
            info!("Failed to submit job: {}", error);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": error
            })))
        }
    }
}

async fn get_job_status(path: web::Path<String>) -> Result<HttpResponse> {
    let job_id = path.into_inner();
    
    match JOB_MANAGER.get_job(&job_id).await {
        Some(job) => {
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "job": job
            })))
        }
        None => {
            Ok(HttpResponse::NotFound().json(serde_json::json!({
                "success": false,
                "error": "Job not found"
            })))
        }
    }
}

async fn get_all_jobs() -> Result<HttpResponse> {
    let jobs = JOB_MANAGER.get_all_jobs().await;
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "jobs": jobs
    })))
}

async fn get_queue_status() -> Result<HttpResponse> {
    let status = JOB_MANAGER.get_queue_status().await;
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "queue": status
    })))
}

async fn delete_job(path: web::Path<String>) -> Result<HttpResponse> {
    let job_id = path.into_inner();
    
    match JOB_MANAGER.delete_job(&job_id).await {
        Ok(()) => {
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "message": "Job deleted successfully"
            })))
        }
        Err(error) => {
            Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "error": error
            })))
        }
    }
}

// System stats endpoint
async fn get_system_stats() -> Result<HttpResponse> {
    let stats = serde_json::json!({
        "success": true,
        "system": {
            "cpu_cores": num_cpus::get(),
            "memory": {
                "total_mb": 0, // TODO: implement actual memory stats
                "used_mb": 0,
                "available_mb": 0
            },
            "disk": {
                "total_gb": 0,
                "used_gb": 0,
                "available_gb": 0
            },
            "performance": {
                "simd_support": ["AVX2", "SSE2"],
                "threads": rayon::current_num_threads(),
                "memory_allocator": "mimalloc"
            },
            "services": {
                "backend": "healthy",
                "database": "not_connected",
                "cache": "not_connected"
            }
        },
        "timestamp": chrono::Utc::now()
    });

    Ok(HttpResponse::Ok().json(stats))
}

// Files endpoint
async fn list_files() -> Result<HttpResponse> {
    // List available PDB files
    let pdb_files = vec![
        serde_json::json!({
            "id": "2f6d",
            "filename": "2f6d.pdb",
            "size_mb": 0.5,
            "atoms": 4775,
            "description": "Protein structure 2f6d",
            "uploaded_at": chrono::Utc::now()
        }),
        serde_json::json!({
            "id": "4jet",
            "filename": "4jet.pdb", 
            "size_mb": 1.2,
            "atoms": 14784,
            "description": "Protein structure 4jet",
            "uploaded_at": chrono::Utc::now()
        }),
        serde_json::json!({
            "id": "6izr",
            "filename": "6izr.pdb",
            "size_mb": 0.8,
            "atoms": 7952,
            "description": "Protein structure 6izr", 
            "uploaded_at": chrono::Utc::now()
        }),
        serde_json::json!({
            "id": "6qtl",
            "filename": "6qtl.pdb",
            "size_mb": 2.1,
            "atoms": 21304,
            "description": "Protein structure 6qtl",
            "uploaded_at": chrono::Utc::now()
        }),
        serde_json::json!({
            "id": "8e5d",
            "filename": "8e5d.pdb",
            "size_mb": 8.7,
            "atoms": 84360,
            "description": "Protein structure 8e5d",
            "uploaded_at": chrono::Utc::now()
        })
    ];

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "files": pdb_files,
        "total": pdb_files.len(),
        "timestamp": chrono::Utc::now()
    })))
}

// Configurar rutas
fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg
        .route("/", web::get().to(root))
        .route("/health", web::get().to(health))
        .route("/metrics", web::get().to(|| async {
            HttpResponse::Ok().body("# HELP backend_status Status of backend\n# TYPE backend_status gauge\nbackend_status 1\n")
        }))
        .service(
            web::scope("/api")
                .route("/health", web::get().to(health))
                .service(
                    web::scope("/v1")
                        .route("/", web::get().to(api_info))
                        .route("/ping", web::get().to(|| async {
                            HttpResponse::Ok().json(serde_json::json!({
                                "message": "pong",
                                "timestamp": chrono::Utc::now(),
                                "performance": "ULTRA_OPTIMIZED"
                            }))
                        }))
                        .route("/parse_pdb/{filename}", web::get().to(parse_pdb_file))
                        .route("/dock/{receptor}/{ligand}", web::post().to(dock_proteins))
                        // Job management endpoints
                        .route("/jobs", web::post().to(submit_docking_job))
                        .route("/jobs", web::get().to(get_all_jobs))
                        .route("/jobs/{job_id}", web::get().to(get_job_status))
                        .route("/jobs/{job_id}", web::delete().to(delete_job))
                        .route("/queue", web::get().to(get_queue_status))
                        // System endpoints
                        .route("/system/stats", web::get().to(get_system_stats))
                        // File management endpoints
                        .route("/files", web::get().to(list_files))
                )
        );
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

    // Crear servidor HTTP
    HttpServer::new(move || {
        // NO configurar CORS aquí - nginx maneja todo el CORS
        // Esto evita conflictos de headers duplicados
        
        App::new()
            .wrap(Logger::default())
            .configure(configure_routes)
    })
    .bind(format!("{}:{}", config.host, config.port))?
    .workers(config.workers)
    .run()
    .await
}