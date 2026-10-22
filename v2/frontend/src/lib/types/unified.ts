/*!
 * Unified Types - Tipos unificados entre Frontend y Backend
 * 
 * Este archivo define los tipos que deben ser consistentes
 * entre el frontend TypeScript y el backend mock/real.
 */

// ====================================================================
// PROTEIN MANAGEMENT TYPES
// ====================================================================

export interface ProteinInfo {
  name: string;
  pdb_id?: string | null;
  total_atoms: number;
  total_residues: number;
  chains: number;
  molecular_weight: number;
  center_of_mass: [number, number, number];
  bounding_box_size: [number, number, number];
}

export interface FileMetadata {
  file_id: string;
  filename: string;
  file_size: number;
  content_type: string;
  upload_timestamp: string;
  protein_info?: ProteinInfo;
  status: 'uploaded' | 'processing' | 'ready' | 'error';
}

export interface UploadResponse {
  file_id: string;
  filename: string;
  file_size: number;
  protein_info: ProteinInfo;
  upload_time_ms: number;
  status: 'uploaded' | 'processing' | 'ready' | 'error';
}

export interface FilesListResponse {
  files: FileMetadata[];
  total: number;
}

// ====================================================================
// ALGORITHM TYPES
// ====================================================================

export interface Algorithm {
  name: string;
  version: string;
  description: string;
  supported: boolean;
}

export type AlgorithmsResponse = Algorithm[];

// ====================================================================
// DOCKING CONFIGURATION TYPES
// ====================================================================

export type SamplingStrategy = 'Grid3D' | 'MonteCarlo' | 'GeneticAlgorithm' | 'Hybrid';
export type JobPriority = 'low' | 'normal' | 'high' | 'critical';
export type JobStatus = 'queued' | 'running' | 'completed' | 'failed' | 'cancelled';

export interface DockingConfig {
  max_poses: number;
  energy_threshold: number;
  rmsd_threshold: number;
  sampling_strategy: SamplingStrategy;
  optimization_rounds: number;
  parallel_workers: number;
  use_gpu_acceleration: boolean;
  advanced_scoring: boolean;
}

export interface CreateJobRequest {
  receptor_file_id: string;
  ligand_file_id: string;
  config?: Partial<DockingConfig>;
  user_id?: string;
  priority?: JobPriority;
}

export interface CreateJobResponse {
  job_id: string;
  status: JobStatus;
  estimated_duration_seconds: number;
  queue_position: number;
  websocket_url: string;
}

// ====================================================================
// JOB MANAGEMENT TYPES
// ====================================================================

export interface JobStatusInfo {
  job_id: string;
  status: JobStatus;
  progress_percent: number;
  current_step?: string;
  estimated_completion?: string;
  error_message?: string;
  created_at: string;
  started_at?: string;
  completed_at?: string;
  results?: DockingResult[];
  poses_evaluated?: number;
  poses_remaining?: number;
}

export interface JobsListResponse {
  jobs: JobStatusInfo[];
  total: number;
}

// ====================================================================
// WEBSOCKET TYPES
// ====================================================================

export interface JobProgressUpdate {
  job_id: string;
  status: JobStatus;
  progress_percent: number;
  current_step?: string;
  poses_evaluated: number;
  poses_remaining: number;
  estimated_completion?: string;
  computation_stats?: {
    total_poses_evaluated: number;
    poses_per_second: number;
    memory_used_mb: number;
    cpu_utilization_percent: number;
    gpu_utilization_percent?: number;
    simd_acceleration_used: boolean;
  };
  timestamp: string;
}

export interface ConnectionState {
  connected: boolean;
  error?: string | null;
  reconnectAttempts?: number;
}

// ====================================================================
// SYSTEM STATUS TYPES
// ====================================================================

export interface SystemStats {
  total_jobs: number;
  queued_jobs: number;
  running_jobs: number;
  completed_jobs: number;
  failed_jobs: number;
  cancelled_jobs: number;
  avg_completion_time_seconds: number;
  system_load_percent: number;
}

export interface HealthResponse {
  status: string;
  version: string;
  timestamp: string;
  capabilities: {
    cpu_cores: number;
    simd_support: string[];
    memory_allocator: string;
  };
}

// ====================================================================
// API RESPONSE TYPES
// ====================================================================

export interface ApiResponse<T = any> {
  success: boolean;
  data?: T;
  error?: string;
  timestamp: string;
}

export interface PingResponse {
  message: string;
  timestamp: string;
  performance: string;
}

// ====================================================================
// DOCKING RESULTS TYPES (for future use)
// ====================================================================

export interface Vector3 {
  x: number;
  y: number;
  z: number;
}

export interface DockingPose {
  pose_id: number;
  score: number;
  rmsd?: number;
  transformation: {
    translation: [number, number, number];
    rotation: [number, number, number, number];
    center_of_rotation: [number, number, number];
  };
  energy_breakdown: {
    electrostatic: number;
    van_der_waals: number;
    hydrogen_bonds: number;
    shape_complementarity: number;
    solvation: number;
    entropy: number;
    total: number;
  };
  computation_time_ms: number;
  iterations_used?: number;
  convergence_reached: boolean;
}

export interface DockingResult {
  job_id: string;
  poses: DockingPose[];
  best_pose: DockingPose;
  total_computation_time_ms: number;
  poses_generated: number;
  poses_converged: number;
  final_stats: {
    total_poses_evaluated: number;
    poses_per_second: number;
    memory_used_mb: number;
    cpu_utilization_percent: number;
    gpu_utilization_percent?: number;
    simd_acceleration_used: boolean;
  };
}

// ====================================================================
// LEGACY COMPATIBILITY EXPORTS
// ====================================================================

// Re-export for backward compatibility with existing code
export type { 
  ProteinInfo as LegacyProteinInfo,
  FileMetadata as LegacyFileMetadata,
  UploadResponse as LegacyUploadResponse
};