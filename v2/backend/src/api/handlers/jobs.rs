// Jobs Handler - Sistema de trabajos de docking con WebSocket tiempo real
// Manejo de jobs, cola de procesamiento, notificaciones en vivo

use actix_web::{web, HttpResponse, Result, HttpRequest};
use actix_ws::Message;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use std::collections::HashMap;
use tracing::{info, warn, error, debug};
use uuid::Uuid;

use crate::algorithms::{DockingConfig, DockingResult, ProteinStructure, ProteinParser};
use crate::database::{get_database_pool, DockingJobModel, DockingResultModel};

/// Estado de un job de docking
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Job de docking completo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockingJob {
    pub job_id: String,
    pub user_id: Option<String>,
    pub receptor_file_id: String,
    pub ligand_file_id: String,
    pub config: DockingConfig,
    pub status: JobStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub progress_percent: f32,
    pub current_step: Option<String>,
    pub estimated_completion: Option<chrono::DateTime<chrono::Utc>>,
    pub results: Option<Vec<DockingResult>>,
    pub error_message: Option<String>,
    pub computation_stats: Option<ComputationStats>,
}

/// Estadísticas de computación
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputationStats {
    pub total_poses_evaluated: u32,
    pub poses_per_second: f64,
    pub memory_used_mb: u64,
    pub cpu_utilization_percent: f32,
    pub gpu_utilization_percent: Option<f32>,
    pub simd_acceleration_used: bool,
}

/// Request para crear nuevo job
#[derive(Debug, Deserialize)]
pub struct CreateJobRequest {
    pub receptor_file_id: String,
    pub ligand_file_id: String,
    pub config: Option<DockingConfig>,
    pub user_id: Option<String>,
    pub priority: Option<JobPriority>,
}

/// Prioridad del job
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Respuesta de creación de job
#[derive(Serialize)]
pub struct CreateJobResponse {
    pub job_id: String,
    pub status: JobStatus,
    pub estimated_duration_seconds: u64,
    pub queue_position: usize,
    pub websocket_url: String,
}

/// Update de progreso en tiempo real
#[derive(Debug, Clone, Serialize)]
pub struct JobProgressUpdate {
    pub job_id: String,
    pub status: JobStatus,
    pub progress_percent: f32,
    pub current_step: Option<String>,
    pub poses_evaluated: u32,
    pub poses_remaining: u32,
    pub estimated_completion: Option<chrono::DateTime<chrono::Utc>>,
    pub computation_stats: Option<ComputationStats>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Manager de jobs global
pub struct JobManager {
    jobs: RwLock<HashMap<String, DockingJob>>,
    job_queue: RwLock<Vec<String>>,
    active_jobs: RwLock<HashMap<String, tokio::task::JoinHandle<()>>>,
    websocket_connections: RwLock<HashMap<String, mpsc::UnboundedSender<JobProgressUpdate>>>,
    max_concurrent_jobs: usize,
}

impl JobManager {
    pub fn new(max_concurrent_jobs: usize) -> Arc<Self> {
        Arc::new(Self {
            jobs: RwLock::new(HashMap::new()),
            job_queue: RwLock::new(Vec::new()),
            active_jobs: RwLock::new(HashMap::new()),
            websocket_connections: RwLock::new(HashMap::new()),
            max_concurrent_jobs,
        })
    }
    
    /// Sincronizar job desde base de datos
    async fn sync_job_from_db(&self, job_id: &str) -> anyhow::Result<()> {
        let pool = get_database_pool()?;
        
        if let Some(model) = DockingJobModel::find_by_job_id(pool, job_id).await? {
            let domain_job = model.to_domain_model()?;
            self.jobs.write().await.insert(job_id.to_string(), domain_job);
        }
        
        Ok(())
    }
    
    /// Crear nuevo job de docking
    pub async fn create_job(&self, request: CreateJobRequest) -> anyhow::Result<DockingJob> {
        let job_id = Uuid::new_v4().to_string();
        let config = request.config.unwrap_or_default();
        let priority = request.priority.unwrap_or(JobPriority::Normal);
        
        // Guardar en base de datos primero
        let pool = get_database_pool()?;
        let model = DockingJobModel::create(
            pool,
            job_id.clone(),
            request.user_id.clone(),
            request.receptor_file_id.clone(),
            request.ligand_file_id.clone(),
            config.clone(),
            priority,
        ).await?;
        
        let job = model.to_domain_model()?;
        
        // Guardar en memoria para acceso rápido
        self.jobs.write().await.insert(job_id.clone(), job.clone());
        
        // Añadir a cola
        self.job_queue.write().await.push(job_id.clone());
        
        info!("📋 Created docking job: {}", job_id);
        
        // Intentar procesar cola
        self.process_queue().await;
        
        Ok(job)
    }
    
    /// Obtener job por ID
    pub async fn get_job(&self, job_id: &str) -> Option<DockingJob> {
        // Buscar en memoria primero
        if let Some(job) = self.jobs.read().await.get(job_id).cloned() {
            return Some(job);
        }
        
        // Si no está en memoria, buscar en base de datos
        if let Ok(()) = self.sync_job_from_db(job_id).await {
            return self.jobs.read().await.get(job_id).cloned();
        }
        
        None
    }
    
    /// Listar jobs con filtros
    pub async fn list_jobs(
        &self, 
        user_id: Option<&str>, 
        status: Option<JobStatus>,
        limit: usize,
        offset: usize
    ) -> Vec<DockingJob> {
        let jobs = self.jobs.read().await;
        let mut filtered_jobs: Vec<_> = jobs.values()
            .filter(|job| {
                if let Some(uid) = user_id {
                    if job.user_id.as_deref() != Some(uid) {
                        return false;
                    }
                }
                if let Some(ref status_filter) = status {
                    if !matches!(&job.status, status_filter) {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();
        
        // Ordenar por fecha de creación descendente
        filtered_jobs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        // Aplicar paginación
        filtered_jobs.into_iter().skip(offset).take(limit).collect()
    }
    
    /// Cancelar job
    pub async fn cancel_job(&self, job_id: &str) -> anyhow::Result<()> {
        let mut jobs = self.jobs.write().await;
        
        if let Some(job) = jobs.get_mut(job_id) {
            match job.status {
                JobStatus::Queued => {
                    job.status = JobStatus::Cancelled;
                    job.completed_at = Some(chrono::Utc::now());
                    
                    // Remover de cola
                    let mut queue = self.job_queue.write().await;
                    queue.retain(|id| id != job_id);
                    
                    info!("❌ Cancelled queued job: {}", job_id);
                },
                JobStatus::Running => {
                    // Cancelar tarea activa
                    let mut active_jobs = self.active_jobs.write().await;
                    if let Some(handle) = active_jobs.remove(job_id) {
                        handle.abort();
                    }
                    
                    job.status = JobStatus::Cancelled;
                    job.completed_at = Some(chrono::Utc::now());
                    
                    info!("❌ Cancelled running job: {}", job_id);
                    
                    // Notificar via WebSocket
                    self.send_progress_update(job_id, job).await;
                },
                _ => {
                    anyhow::bail!("Cannot cancel job in status: {:?}", job.status);
                }
            }
        } else {
            anyhow::bail!("Job not found: {}", job_id);
        }
        
        Ok(())
    }
    
    /// Procesar cola de jobs
    async fn process_queue(&self) {
        let active_count = self.active_jobs.read().await.len();
        
        if active_count >= self.max_concurrent_jobs {
            debug!("Max concurrent jobs reached: {}/{}", active_count, self.max_concurrent_jobs);
            return;
        }
        
        let next_job_id = {
            let mut queue = self.job_queue.write().await;
            queue.pop()
        };
        
        if let Some(job_id) = next_job_id {
            let manager = Arc::new(self);
            let job_id_clone = job_id.clone();
            
            // Crear tarea de procesamiento
            let handle = tokio::spawn(async move {
                if let Err(e) = manager.execute_job(&job_id_clone).await {
                    error!("Job execution failed: {} - {}", job_id_clone, e);
                    manager.mark_job_failed(&job_id_clone, &e.to_string()).await;
                }
                
                // Remover de jobs activos
                manager.active_jobs.write().await.remove(&job_id_clone);
                
                // Procesar siguiente job en cola
                manager.process_queue().await;
            });
            
            // Registrar job activo
            self.active_jobs.write().await.insert(job_id, handle);
        }
    }
    
    /// Ejecutar job de docking
    async fn execute_job(&self, job_id: &str) -> anyhow::Result<()> {
        info!("🚀 Starting job execution: {}", job_id);
        
        // Marcar como running
        {
            let mut jobs = self.jobs.write().await;
            if let Some(job) = jobs.get_mut(job_id) {
                job.status = JobStatus::Running;
                job.started_at = Some(chrono::Utc::now());
                job.current_step = Some("Initializing docking engine".to_string());
                job.progress_percent = 5.0;
            }
        }
        
        // Enviar update inicial
        if let Some(job) = self.get_job(job_id).await {
            self.send_progress_update(job_id, &job).await;
        }
        
        // TODO: Aquí iría la integración real con el docking engine
        // Por ahora simulamos el procesamiento
        self.simulate_docking_process(job_id).await?;
        
        info!("✅ Job completed: {}", job_id);
        Ok(())
    }
    
    /// Simular proceso de docking (placeholder)
    async fn simulate_docking_process(&self, job_id: &str) -> anyhow::Result<()> {
        let steps = vec![
            ("Loading protein structures", 10.0),
            ("Preparing receptor", 20.0),
            ("Preparing ligand", 30.0),
            ("Generating initial poses", 40.0),
            ("Evaluating poses", 60.0),
            ("Optimizing top poses", 80.0),
            ("Finalizing results", 95.0),
            ("Completed", 100.0),
        ];
        
        for (step, progress) in steps {
            // Simular tiempo de procesamiento
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            
            // Actualizar progreso
            {
                let mut jobs = self.jobs.write().await;
                if let Some(job) = jobs.get_mut(job_id) {
                    job.current_step = Some(step.to_string());
                    job.progress_percent = progress;
                    
                    if progress >= 100.0 {
                        job.status = JobStatus::Completed;
                        job.completed_at = Some(chrono::Utc::now());
                        
                        // Simular resultados
                        job.results = Some(vec![
                            DockingResult {
                                pose_id: 1,
                                score: -127.45,
                                rmsd: 1.23,
                                transformation: crate::algorithms::docking_core::Transform3D {
                                    translation: [1.0, 2.0, 3.0],
                                    rotation: [1.0, 0.0, 0.0, 0.0],
                                    center_of_rotation: [0.0, 0.0, 0.0],
                                },
                                energy_breakdown: crate::algorithms::docking_core::EnergyBreakdown {
                                    electrostatic: -45.2,
                                    van_der_waals: -67.1,
                                    hydrogen_bonds: -15.15,
                                    shape_complementarity: 0.0,
                                    solvation: 0.0,
                                    entropy: 0.0,
                                    total: -127.45,
                                },
                                computation_time_ms: 2340,
                                iterations_used: 1000,
                                convergence_reached: true,
                            }
                        ]);
                        
                        job.computation_stats = Some(ComputationStats {
                            total_poses_evaluated: 10000,
                            poses_per_second: 150.5,
                            memory_used_mb: 512,
                            cpu_utilization_percent: 85.2,
                            gpu_utilization_percent: Some(92.1),
                            simd_acceleration_used: true,
                        });
                    }
                }
            }
            
            // Enviar update via WebSocket
            if let Some(job) = self.get_job(job_id).await {
                self.send_progress_update(job_id, &job).await;
            }
        }
        
        Ok(())
    }
    
    /// Marcar job como fallido
    async fn mark_job_failed(&self, job_id: &str, error_message: &str) {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(job_id) {
            job.status = JobStatus::Failed;
            job.completed_at = Some(chrono::Utc::now());
            job.error_message = Some(error_message.to_string());
        }
        
        // Enviar update via WebSocket
        if let Some(job) = jobs.get(job_id) {
            self.send_progress_update(job_id, job).await;
        }
    }
    
    /// Enviar update de progreso via WebSocket
    async fn send_progress_update(&self, job_id: &str, job: &DockingJob) {
        let update = JobProgressUpdate {
            job_id: job_id.to_string(),
            status: job.status.clone(),
            progress_percent: job.progress_percent,
            current_step: job.current_step.clone(),
            poses_evaluated: job.computation_stats.as_ref()
                .map(|s| s.total_poses_evaluated)
                .unwrap_or(0),
            poses_remaining: job.config.max_poses.saturating_sub(
                job.computation_stats.as_ref()
                    .map(|s| s.total_poses_evaluated)
                    .unwrap_or(0)
            ),
            estimated_completion: job.estimated_completion,
            computation_stats: job.computation_stats.clone(),
            timestamp: chrono::Utc::now(),
        };
        
        // Enviar a todas las conexiones WebSocket para este job
        let connections = self.websocket_connections.read().await;
        if let Some(sender) = connections.get(job_id) {
            if let Err(_) = sender.send(update) {
                debug!("WebSocket connection closed for job: {}", job_id);
            }
        }
    }
    
    /// Registrar conexión WebSocket
    pub async fn register_websocket(&self, job_id: &str, sender: mpsc::UnboundedSender<JobProgressUpdate>) {
        self.websocket_connections.write().await.insert(job_id.to_string(), sender);
        info!("🔌 WebSocket registered for job: {}", job_id);
    }
    
    /// Desregistrar conexión WebSocket
    pub async fn unregister_websocket(&self, job_id: &str) {
        self.websocket_connections.write().await.remove(job_id);
        info!("🔌 WebSocket unregistered for job: {}", job_id);
    }
    
    /// Obtener estadísticas del sistema
    pub async fn get_system_stats(&self) -> SystemStats {
        let jobs = self.jobs.read().await;
        let active_jobs = self.active_jobs.read().await;
        let queue = self.job_queue.read().await;
        
        let mut stats = SystemStats {
            total_jobs: jobs.len(),
            queued_jobs: queue.len(),
            running_jobs: active_jobs.len(),
            completed_jobs: 0,
            failed_jobs: 0,
            cancelled_jobs: 0,
            avg_completion_time_seconds: 0.0,
            system_load_percent: 0.0,
        };
        
        let mut completion_times = Vec::new();
        
        for job in jobs.values() {
            match job.status {
                JobStatus::Completed => {
                    stats.completed_jobs += 1;
                    if let (Some(started), Some(completed)) = (job.started_at, job.completed_at) {
                        let duration = completed.signed_duration_since(started);
                        completion_times.push(duration.num_seconds() as f64);
                    }
                },
                JobStatus::Failed => stats.failed_jobs += 1,
                JobStatus::Cancelled => stats.cancelled_jobs += 1,
                _ => {}
            }
        }
        
        if !completion_times.is_empty() {
            stats.avg_completion_time_seconds = completion_times.iter().sum::<f64>() / completion_times.len() as f64;
        }
        
        // Calcular carga del sistema
        stats.system_load_percent = (active_jobs.len() as f64 / self.max_concurrent_jobs as f64) * 100.0;
        
        stats
    }
}

/// Estadísticas del sistema de jobs
#[derive(Debug, Serialize)]
pub struct SystemStats {
    pub total_jobs: usize,
    pub queued_jobs: usize,
    pub running_jobs: usize,
    pub completed_jobs: usize,
    pub failed_jobs: usize,
    pub cancelled_jobs: usize,
    pub avg_completion_time_seconds: f64,
    pub system_load_percent: f64,
}

// Handlers HTTP

/// Crear nuevo job de docking
pub async fn create_docking_job(
    job_manager: web::Data<Arc<JobManager>>,
    request: web::Json<CreateJobRequest>,
) -> Result<HttpResponse> {
    match job_manager.create_job(request.into_inner()).await {
        Ok(job) => {
            let queue_position = job_manager.job_queue.read().await
                .iter()
                .position(|id| id == &job.job_id)
                .unwrap_or(0);
            
            let response = CreateJobResponse {
                job_id: job.job_id.clone(),
                status: job.status,
                estimated_duration_seconds: 60, // Placeholder
                queue_position,
                websocket_url: format!("/ws/jobs/{}", job.job_id),
            };
            
            Ok(HttpResponse::Created().json(response))
        },
        Err(e) => {
            error!("Failed to create job: {}", e);
            Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Failed to create job",
                "details": e.to_string()
            })))
        }
    }
}

/// Obtener información de job
pub async fn get_job_info(
    job_manager: web::Data<Arc<JobManager>>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let job_id = path.into_inner();
    
    match job_manager.get_job(&job_id).await {
        Some(job) => Ok(HttpResponse::Ok().json(job)),
        None => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Job not found",
            "job_id": job_id
        })))
    }
}

/// Listar jobs
pub async fn list_jobs(
    job_manager: web::Data<Arc<JobManager>>,
    query: web::Query<ListJobsQuery>,
) -> Result<HttpResponse> {
    let jobs = job_manager.list_jobs(
        query.user_id.as_deref(),
        query.status.clone(),
        query.limit.unwrap_or(50),
        query.offset.unwrap_or(0),
    ).await;
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "jobs": jobs,
        "total": jobs.len(),
        "limit": query.limit.unwrap_or(50),
        "offset": query.offset.unwrap_or(0)
    })))
}

#[derive(Deserialize)]
pub struct ListJobsQuery {
    pub user_id: Option<String>,
    pub status: Option<JobStatus>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Cancelar job
pub async fn cancel_job(
    job_manager: web::Data<Arc<JobManager>>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let job_id = path.into_inner();
    
    match job_manager.cancel_job(&job_id).await {
        Ok(()) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "message": "Job cancelled successfully",
            "job_id": job_id
        }))),
        Err(e) => Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Failed to cancel job",
            "details": e.to_string(),
            "job_id": job_id
        })))
    }
}

/// Obtener estadísticas del sistema
pub async fn get_system_stats(
    job_manager: web::Data<Arc<JobManager>>,
) -> Result<HttpResponse> {
    let stats = job_manager.get_system_stats().await;
    Ok(HttpResponse::Ok().json(stats))
}