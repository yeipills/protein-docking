/*!
 * Job models - Gestión de trabajos y tareas
 */

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: Uuid,
    pub job_type: JobType,
    pub status: JobStatus,
    pub progress: u8, // 0-100
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub error_message: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobType {
    DockingComputation,
    FileValidation,
    ResultsGeneration,
    Benchmark,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl Job {
    pub fn new(job_type: JobType) -> Self {
        Self {
            id: Uuid::new_v4(),
            job_type,
            status: JobStatus::Pending,
            progress: 0,
            created_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            error_message: None,
            metadata: serde_json::Value::Null,
        }
    }
    
    pub fn start(&mut self) {
        self.status = JobStatus::Running;
        self.started_at = Some(chrono::Utc::now());
    }
    
    pub fn complete(&mut self) {
        self.status = JobStatus::Completed;
        self.progress = 100;
        self.completed_at = Some(chrono::Utc::now());
    }
    
    pub fn fail(&mut self, error: String) {
        self.status = JobStatus::Failed;
        self.error_message = Some(error);
        self.completed_at = Some(chrono::Utc::now());
    }
    
    pub fn duration(&self) -> Option<chrono::Duration> {
        match (self.started_at, self.completed_at) {
            (Some(start), Some(end)) => Some(end - start),
            _ => None,
        }
    }
}