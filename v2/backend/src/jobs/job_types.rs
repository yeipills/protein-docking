// Tipos de datos para el sistema de jobs

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockingJob {
    pub id: String,
    pub receptor_filename: String,
    pub ligand_filename: String,
    pub status: JobStatus,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub progress: JobProgress,
    pub result: Option<DockingJobResult>,
    pub error: Option<String>,
    pub metadata: JobMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobProgress {
    pub stage: String,
    pub percentage: f64,
    pub estimated_time_remaining: Option<u64>, // seconds
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobMetadata {
    pub priority: i32,
    pub max_poses: usize,
    pub energy_cutoff: Option<f64>,
    pub user_id: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockingJobRequest {
    pub receptor_filename: String,
    pub ligand_filename: String,
    pub priority: Option<i32>,
    pub max_poses: Option<usize>,
    pub energy_cutoff: Option<f64>,
    pub user_id: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockingJobResult {
    pub receptor_id: String,
    pub ligand_id: String,
    pub poses_count: usize,
    pub best_energy: f64,
    pub best_pose_data: BestPoseData,
    pub computation_stats: ComputationStats,
    pub poses_summary: Vec<PoseSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BestPoseData {
    pub energy: f64,
    pub contacts: usize,
    pub clash_score: f64,
    pub energy_components: HashMap<String, f64>,
    pub transformation: TransformationMatrix,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputationStats {
    pub total_time_ms: u64,
    pub parsing_time_ms: u64,
    pub docking_time_ms: u64,
    pub evaluation_time_ms: u64,
    pub threads_used: usize,
    pub simd_optimized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoseSummary {
    pub id: usize,
    pub energy: f64,
    pub clash_score: f64,
    pub contacts: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformationMatrix {
    pub translation: [f64; 3],
    pub rotation: [f64; 9], // 3x3 rotation matrix
}

impl Default for JobProgress {
    fn default() -> Self {
        Self {
            stage: "initializing".to_string(),
            percentage: 0.0,
            estimated_time_remaining: None,
        }
    }
}

impl Default for JobMetadata {
    fn default() -> Self {
        Self {
            priority: 1,
            max_poses: 10,
            energy_cutoff: None,
            user_id: None,
            tags: Vec::new(),
        }
    }
}