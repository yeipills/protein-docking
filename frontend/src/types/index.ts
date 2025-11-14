export interface User {
  id: number
  email: string
  username: string
  is_active: boolean
  is_superuser: boolean
  jobs_count: number
  created_at: string
}

export interface LoginRequest {
  username: string
  password: string
}

export interface RegisterRequest {
  username: string
  email: string
  password: string
}

export interface AuthResponse {
  access_token: string
  refresh_token: string
  token_type: string
}

export enum JobStatus {
  PENDING = 'pending',
  PROCESSING = 'processing',
  COMPLETED = 'completed',
  FAILED = 'failed',
  CANCELLED = 'cancelled',
}

export enum JobType {
  PART_ONE = 'part_one',
  PART_TWO = 'part_two',
}

export interface Job {
  id: number
  user_id: number
  protein_id: number | null
  job_type: JobType
  status: JobStatus
  progress: number
  celery_task_id: string | null
  error_message: string | null
  created_at: string
  started_at: string | null
  completed_at: string | null
  processing_time_seconds: number | null
  input_files: string[]
  output_files: string[]
}

export interface Protein {
  id: number
  name: string
  user_id: number
  stl_file: string
  vertices_file: string
  faces_file: string
  cr_totals_file: string | null
  context_rays_file: string | null
  layer_files: Record<string, string> | null
  centroid_count: number | null
  created_at: string
  updated_at: string
}

export interface CreateJobRequest {
  protein_id: number
  job_type: JobType
}

export interface UploadProteinRequest {
  name: string
  stl_file: File
  vertices_file: File
  faces_file: File
}

export interface ApiError {
  detail: string
}

// WebSocket events
export interface SocketJobUpdate {
  job_id: number
  status: JobStatus
  progress: number
  message?: string
}
