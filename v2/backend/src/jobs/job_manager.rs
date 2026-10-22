// Job Manager Implementation
// Manejo asíncrono de trabajos de docking

use super::job_types::*;
use crate::core::{pdb::PdbParser, docking::DockingEngine};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tracing::{info, error};

pub struct AsyncJobManager {
    jobs: Arc<Mutex<super::JobManager>>,
}

impl AsyncJobManager {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(Mutex::new(super::JobManager::new())),
        }
    }

    pub async fn submit_docking_job(&self, request: DockingJobRequest) -> Result<String, String> {
        let mut manager = self.jobs.lock().await;
        let job_id = manager.create_job(request);
        info!("Created new docking job: {}", job_id);
        
        // Spawn async task para procesar el job
        let jobs_clone = self.jobs.clone();
        let job_id_clone = job_id.clone();
        
        tokio::spawn(async move {
            Self::process_docking_job(jobs_clone, job_id_clone).await;
        });
        
        Ok(job_id)
    }

    async fn process_docking_job(jobs: Arc<Mutex<super::JobManager>>, job_id: String) {
        info!("Starting processing of job: {}", job_id);
        
        // Update status to running
        {
            let mut manager = jobs.lock().await;
            manager.update_job_status(&job_id, JobStatus::Running);
            manager.update_job_progress(&job_id, JobProgress {
                stage: "parsing_files".to_string(),
                percentage: 10.0,
                estimated_time_remaining: Some(30),
            });
        }

        // Get job details
        let (receptor_filename, ligand_filename) = {
            let manager = jobs.lock().await;
            if let Some(job) = manager.get_job(&job_id) {
                (job.receptor_filename.clone(), job.ligand_filename.clone())
            } else {
                error!("Job {} not found", job_id);
                return;
            }
        };

        let result = Self::execute_docking(&jobs, &job_id, &receptor_filename, &ligand_filename).await;
        
        // Complete or fail job
        let mut manager = jobs.lock().await;
        match result {
            Ok(docking_result) => {
                info!("Job {} completed successfully", job_id);
                manager.complete_job(&job_id, docking_result);
            }
            Err(error) => {
                error!("Job {} failed: {}", job_id, error);
                manager.fail_job(&job_id, error);
            }
        }
    }

    async fn execute_docking(
        jobs: &Arc<Mutex<super::JobManager>>,
        job_id: &str,
        receptor_filename: &str,
        ligand_filename: &str,
    ) -> Result<DockingJobResult, String> {
        
        let start_time = std::time::Instant::now();
        
        // Update progress - parsing
        {
            let mut manager = jobs.lock().await;
            manager.update_job_progress(job_id, JobProgress {
                stage: "parsing_receptor".to_string(),
                percentage: 20.0,
                estimated_time_remaining: Some(25),
            });
        }

        let receptor_path = format!("/app/pdb/{}", receptor_filename);
        let ligand_path = format!("/app/pdb/{}", ligand_filename);
        
        let parser = PdbParser::new();
        let parsing_start = std::time::Instant::now();
        
        // Parse receptor
        let receptor = parser.parse_file(&receptor_path)
            .map_err(|e| format!("Failed to parse receptor {}: {}", receptor_filename, e))?;
        
        // Update progress - parsing ligand
        {
            let mut manager = jobs.lock().await;
            manager.update_job_progress(job_id, JobProgress {
                stage: "parsing_ligand".to_string(),
                percentage: 30.0,
                estimated_time_remaining: Some(20),
            });
        }

        // Parse ligand
        let ligand = parser.parse_file(&ligand_path)
            .map_err(|e| format!("Failed to parse ligand {}: {}", ligand_filename, e))?;
        
        let parsing_time = parsing_start.elapsed();
        
        // Update progress - starting docking
        {
            let mut manager = jobs.lock().await;
            manager.update_job_progress(job_id, JobProgress {
                stage: "initializing_docking".to_string(),
                percentage: 40.0,
                estimated_time_remaining: Some(15),
            });
        }

        let docking_start = std::time::Instant::now();
        let docking_engine = DockingEngine::default();
        
        // Update progress - running docking
        {
            let mut manager = jobs.lock().await;
            manager.update_job_progress(job_id, JobProgress {
                stage: "running_docking".to_string(),
                percentage: 60.0,
                estimated_time_remaining: Some(10),
            });
        }

        // Execute docking
        let docking_result = docking_engine.dock_proteins(&receptor, &ligand)
            .map_err(|e| format!("Docking computation failed: {}", e))?;
        
        let docking_time = docking_start.elapsed();
        
        // Update progress - finalizing
        {
            let mut manager = jobs.lock().await;
            manager.update_job_progress(job_id, JobProgress {
                stage: "finalizing_results".to_string(),
                percentage: 90.0,
                estimated_time_remaining: Some(2),
            });
        }

        let total_time = start_time.elapsed();
        
        // Create job result
        let best_energy = docking_result.best_pose.as_ref()
            .map(|p| p.energy)
            .unwrap_or(f64::INFINITY);

        let poses_summary: Vec<PoseSummary> = docking_result.poses.iter()
            .enumerate()
            .map(|(i, pose)| PoseSummary {
                id: i,
                energy: pose.energy,
                clash_score: pose.clash_score,
                contacts: pose.contacts.len(),
            })
            .collect();

        let best_pose_data = if let Some(best_pose) = &docking_result.best_pose {
            BestPoseData {
                energy: best_pose.energy,
                contacts: best_pose.contacts.len(),
                clash_score: best_pose.clash_score,
                energy_components: {
                    let mut map = HashMap::new();
                    map.insert("van_der_waals".to_string(), best_pose.energy_components.van_der_waals);
                    map.insert("electrostatic".to_string(), best_pose.energy_components.electrostatic);
                    map.insert("hydrophobic".to_string(), best_pose.energy_components.hydrophobic);
                    map.insert("hydrogen_bonds".to_string(), best_pose.energy_components.hydrogen_bonds);
                    map.insert("desolvation".to_string(), best_pose.energy_components.desolvation);
                    map
                },
                transformation: TransformationMatrix {
                    translation: [0.0, 0.0, 0.0], // TODO: implement actual transformation
                    rotation: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0], // Identity matrix
                },
            }
        } else {
            return Err("No valid poses found".to_string());
        };

        let result = DockingJobResult {
            receptor_id: docking_result.receptor.id,
            ligand_id: docking_result.ligand.id,
            poses_count: docking_result.poses.len(),
            best_energy,
            best_pose_data,
            computation_stats: ComputationStats {
                total_time_ms: total_time.as_millis() as u64,
                parsing_time_ms: parsing_time.as_millis() as u64,
                docking_time_ms: docking_time.as_millis() as u64,
                evaluation_time_ms: docking_result.computation_stats.computation_time_ms,
                threads_used: docking_result.computation_stats.threads_used,
                simd_optimized: docking_result.computation_stats.simd_enabled,
            },
            poses_summary,
        };

        Ok(result)
    }

    pub async fn get_job(&self, job_id: &str) -> Option<DockingJob> {
        let manager = self.jobs.lock().await;
        manager.get_job(job_id).cloned()
    }

    pub async fn get_all_jobs(&self) -> Vec<DockingJob> {
        let manager = self.jobs.lock().await;
        manager.get_all_jobs().into_iter().cloned().collect()
    }

    pub async fn get_queue_status(&self) -> super::QueueStatus {
        let manager = self.jobs.lock().await;
        manager.get_queue_status()
    }

    pub async fn delete_job(&self, job_id: &str) -> Result<(), String> {
        let mut manager = self.jobs.lock().await;
        if manager.delete_job(job_id) {
            Ok(())
        } else {
            Err("Cannot delete running job or job not found".to_string())
        }
    }
}