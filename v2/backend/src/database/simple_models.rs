// Simplified Database Models - Sin SQLx macros para compilación inicial
// Versión simplificada que compila sin base de datos activa

use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::algorithms::{ProteinStructure, DockingConfig, DockingResult};
use crate::api::handlers::jobs::{JobStatus, JobPriority, ComputationStats};

/// Modelo simplificado de archivo de proteína
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleProteinFileModel {
    pub id: Uuid,
    pub file_id: String,
    pub original_filename: String,
    pub file_path: String,
    pub file_size: i64,
    pub content_type: String,
    pub upload_timestamp: DateTime<Utc>,
    pub protein_structure: Option<ProteinStructure>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Modelo simplificado de job de docking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleDockingJobModel {
    pub id: Uuid,
    pub job_id: String,
    pub user_id: Option<String>,
    pub receptor_file_id: String,
    pub ligand_file_id: String,
    pub config: DockingConfig,
    pub status: JobStatus,
    pub priority: JobPriority,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    pub progress_percent: f32,
    pub current_step: Option<String>,
    pub estimated_completion: Option<DateTime<Utc>>,
    pub results: Option<Vec<DockingResult>>,
    pub error_message: Option<String>,
    pub computation_stats: Option<ComputationStats>,
}

impl SimpleDockingJobModel {
    /// Crear nuevo job simulado
    pub fn create(
        job_id: String,
        user_id: Option<String>,
        receptor_file_id: String,
        ligand_file_id: String,
        config: DockingConfig,
        priority: JobPriority,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            job_id,
            user_id,
            receptor_file_id,
            ligand_file_id,
            config,
            status: JobStatus::Queued,
            priority,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            updated_at: Utc::now(),
            progress_percent: 0.0,
            current_step: Some("Validating input files".to_string()),
            estimated_completion: None,
            results: None,
            error_message: None,
            computation_stats: None,
        }
    }
    
    /// Convertir a modelo de dominio
    pub fn to_domain_model(&self) -> crate::api::handlers::jobs::DockingJob {
        crate::api::handlers::jobs::DockingJob {
            job_id: self.job_id.clone(),
            user_id: self.user_id.clone(),
            receptor_file_id: self.receptor_file_id.clone(),
            ligand_file_id: self.ligand_file_id.clone(),
            config: self.config.clone(),
            status: self.status.clone(),
            created_at: self.created_at,
            started_at: self.started_at,
            completed_at: self.completed_at,
            progress_percent: self.progress_percent,
            current_step: self.current_step.clone(),
            estimated_completion: self.estimated_completion,
            results: self.results.clone(),
            error_message: self.error_message.clone(),
            computation_stats: self.computation_stats.clone(),
        }
    }
}

/// Estadísticas simplificadas
#[derive(Debug, Serialize)]
pub struct SimpleJobStatistics {
    pub total_jobs: i64,
    pub queued_jobs: i64,
    pub running_jobs: i64,
    pub completed_jobs: i64,
    pub failed_jobs: i64,
    pub cancelled_jobs: i64,
}

impl Default for SimpleJobStatistics {
    fn default() -> Self {
        Self {
            total_jobs: 0,
            queued_jobs: 0,
            running_jobs: 0,
            completed_jobs: 0,
            failed_jobs: 0,
            cancelled_jobs: 0,
        }
    }
}