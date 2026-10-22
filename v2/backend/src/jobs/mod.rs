// Job Management System para Protein Docking
// Gestiona trabajos de docking con seguimiento en tiempo real

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub mod job_manager;
pub mod job_types;

use job_types::*;

// Sistema de gestión de trabajos global
lazy_static::lazy_static! {
    pub static ref JOB_MANAGER: Arc<Mutex<JobManager>> = Arc::new(Mutex::new(JobManager::new()));
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobManager {
    jobs: HashMap<String, DockingJob>,
    job_queue: Vec<String>,
    running_jobs: Vec<String>,
}

impl JobManager {
    pub fn new() -> Self {
        Self {
            jobs: HashMap::new(),
            job_queue: Vec::new(),
            running_jobs: Vec::new(),
        }
    }

    pub fn create_job(&mut self, request: DockingJobRequest) -> String {
        let job_id = Uuid::new_v4().to_string();
        let job = DockingJob {
            id: job_id.clone(),
            receptor_filename: request.receptor_filename,
            ligand_filename: request.ligand_filename,
            status: JobStatus::Queued,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            progress: JobProgress {
                stage: "queued".to_string(),
                percentage: 0.0,
                estimated_time_remaining: None,
            },
            result: None,
            error: None,
            metadata: JobMetadata {
                priority: request.priority.unwrap_or(1),
                max_poses: request.max_poses.unwrap_or(10),
                energy_cutoff: request.energy_cutoff,
                user_id: request.user_id,
                tags: request.tags.unwrap_or_default(),
            },
        };

        self.jobs.insert(job_id.clone(), job);
        self.job_queue.push(job_id.clone());
        
        job_id
    }

    pub fn get_job(&self, job_id: &str) -> Option<&DockingJob> {
        self.jobs.get(job_id)
    }

    pub fn update_job_status(&mut self, job_id: &str, status: JobStatus) {
        if let Some(job) = self.jobs.get_mut(job_id) {
            job.status = status.clone();
            
            match status {
                JobStatus::Running => {
                    job.started_at = Some(Utc::now());
                    if let Some(pos) = self.job_queue.iter().position(|id| id == job_id) {
                        self.job_queue.remove(pos);
                    }
                    self.running_jobs.push(job_id.to_string());
                }
                JobStatus::Completed | JobStatus::Failed => {
                    job.completed_at = Some(Utc::now());
                    if let Some(pos) = self.running_jobs.iter().position(|id| id == job_id) {
                        self.running_jobs.remove(pos);
                    }
                }
                _ => {}
            }
        }
    }

    pub fn update_job_progress(&mut self, job_id: &str, progress: JobProgress) {
        if let Some(job) = self.jobs.get_mut(job_id) {
            job.progress = progress;
        }
    }

    pub fn complete_job(&mut self, job_id: &str, result: DockingJobResult) {
        if let Some(job) = self.jobs.get_mut(job_id) {
            job.result = Some(result);
            job.status = JobStatus::Completed;
            job.completed_at = Some(Utc::now());
            job.progress.percentage = 100.0;
            job.progress.stage = "completed".to_string();
            
            if let Some(pos) = self.running_jobs.iter().position(|id| id == job_id) {
                self.running_jobs.remove(pos);
            }
        }
    }

    pub fn fail_job(&mut self, job_id: &str, error: String) {
        if let Some(job) = self.jobs.get_mut(job_id) {
            job.error = Some(error);
            job.status = JobStatus::Failed;
            job.completed_at = Some(Utc::now());
            
            if let Some(pos) = self.running_jobs.iter().position(|id| id == job_id) {
                self.running_jobs.remove(pos);
            }
        }
    }

    pub fn get_all_jobs(&self) -> Vec<&DockingJob> {
        self.jobs.values().collect()
    }

    pub fn get_jobs_by_status(&self, status: JobStatus) -> Vec<&DockingJob> {
        self.jobs.values()
            .filter(|job| job.status == status)
            .collect()
    }

    pub fn get_queue_status(&self) -> QueueStatus {
        QueueStatus {
            total_jobs: self.jobs.len(),
            queued: self.job_queue.len(),
            running: self.running_jobs.len(),
            completed: self.get_jobs_by_status(JobStatus::Completed).len(),
            failed: self.get_jobs_by_status(JobStatus::Failed).len(),
            queue_position: self.job_queue.clone(),
            running_jobs: self.running_jobs.clone(),
        }
    }

    pub fn delete_job(&mut self, job_id: &str) -> bool {
        if let Some(job) = self.jobs.get(job_id) {
            match job.status {
                JobStatus::Running => false, // No se puede eliminar trabajo en curso
                _ => {
                    self.jobs.remove(job_id);
                    // Remover de colas si existe
                    if let Some(pos) = self.job_queue.iter().position(|id| id == job_id) {
                        self.job_queue.remove(pos);
                    }
                    if let Some(pos) = self.running_jobs.iter().position(|id| id == job_id) {
                        self.running_jobs.remove(pos);
                    }
                    true
                }
            }
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStatus {
    pub total_jobs: usize,
    pub queued: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
    pub queue_position: Vec<String>,
    pub running_jobs: Vec<String>,
}