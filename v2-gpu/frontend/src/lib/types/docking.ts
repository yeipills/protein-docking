/*!
 * Docking Types - Tipos TypeScript para el sistema de docking
 * 
 * Definiciones completas y estrictas para máxima type safety
 */

// Configuración de docking
export interface DockingConfiguration {
  receptor_id: string;
  ligand_id: string;
  search_space: SearchSpace;
  exhaustiveness: number;
  num_modes: number;
  energy_range: number;
  max_iterations: number;
  precision: 'low' | 'medium' | 'high' | 'ultra';
  use_gpu: boolean;
  parallel_processing: boolean;
}

// Espacio de búsqueda 3D
export interface SearchSpace {
  center: Vector3;
  size: Vector3;
}

// Vector 3D
export interface Vector3 {
  x: number;
  y: number;
  z: number;
}

// Matriz 3x3 para rotaciones
export interface Matrix3 {
  m11: number; m12: number; m13: number;
  m21: number; m22: number; m23: number;
  m31: number; m32: number; m33: number;
}

// Transformación 3D completa
export interface Transformation {
  translation: Vector3;
  rotation: Matrix3;
  rmsd: number;
}

// Trabajo de docking
export interface DockingJob {
  id: string;
  config: DockingConfiguration;
  priority: number;
  estimatedDuration?: number;
  estimatedTimeRemaining?: number;
  createdAt: Date;
  startedAt?: Date;
  completedAt?: Date;
  status?: JobStatus;
}

// Estados de trabajo
export type JobStatus = 
  | 'pending' 
  | 'running' 
  | 'completed' 
  | 'failed' 
  | 'cancelled';

// Resultado de docking
export interface DockingResult {
  id: string;
  score: number;
  energy: number;
  transformation: Transformation;
  contacts: Contact[];
  metrics: DockingMetrics;
  conformations?: Conformation[];
}

// Contacto intermolecular
export interface Contact {
  receptor_atom: AtomInfo;
  ligand_atom: AtomInfo;
  distance: number;
  interaction_type: InteractionType;
  energy: number;
}

// Información de átomo
export interface AtomInfo {
  id: number;
  element: string;
  residue: string;
  chain: string;
  position: Vector3;
  radius: number;
  charge?: number;
}

// Tipos de interacción
export type InteractionType = 
  | 'hydrogen_bond'
  | 'van_der_waals'
  | 'electrostatic'
  | 'hydrophobic'
  | 'pi_pi'
  | 'cation_pi'
  | 'metal_coordination';

// Conformación molecular
export interface Conformation {
  id: string;
  atoms: AtomInfo[];
  energy: number;
  probability: number;
  rmsd_from_reference?: number;
}

// Métricas de rendimiento del docking
export interface DockingMetrics {
  computation_time_ms: number;
  iterations: number;
  memory_used_mb: number;
  gpu_utilization?: number;
  convergence_score?: number;
  sampling_efficiency?: number;
}

// Configuración de visualización
export interface VisualizationSettings {
  representation: RepresentationType;
  color_scheme: ColorScheme;
  show_surface: boolean;
  show_contacts: boolean;
  show_hydrogen_bonds: boolean;
  transparency: number;
  quality_level: QualityLevel;
}

// Tipos de representación molecular
export type RepresentationType = 
  | 'cartoon'
  | 'spacefill'
  | 'sticks'
  | 'surface'
  | 'ribbon';

// Esquemas de color
export type ColorScheme = 
  | 'by_element'
  | 'by_residue'
  | 'by_chain'
  | 'by_secondary_structure'
  | 'by_energy'
  | 'custom';

// Niveles de calidad
export type QualityLevel = 
  | 'low'
  | 'medium' 
  | 'high'
  | 'ultra';

// Estado del sistema
export interface SystemStatus {
  cpu_cores: number;
  memory_total_gb: number;
  memory_available_gb: number;
  gpu_available: boolean;
  gpu_memory_gb?: number;
  gpu_compute_capability?: string;
  simd_support: string[];
  numa_nodes: number;
}

// Configuración de rendimiento
export interface PerformanceConfig {
  max_parallel_jobs: number;
  memory_limit_gb: number;
  use_gpu_acceleration: boolean;
  enable_simd: boolean;
  thread_pool_size?: number;
  cache_size_mb: number;
  enable_profiling: boolean;
}

// Protocolo de comunicación WebSocket
export interface WebSocketMessage {
  type: MessageType;
  payload: any;
  timestamp: number;
  id?: string;
}

export type MessageType =
  | 'job:start'
  | 'job:progress'
  | 'job:completed'
  | 'job:failed'
  | 'job:cancelled'
  | 'system:status'
  | 'performance:update'
  | 'error'
  | 'ping'
  | 'pong';

// Eventos del sistema
export interface SystemEvent {
  type: SystemEventType;
  message: string;
  severity: 'info' | 'warning' | 'error' | 'critical';
  timestamp: Date;
  details?: Record<string, any>;
}

export type SystemEventType =
  | 'job_started'
  | 'job_completed'
  | 'job_failed'
  | 'performance_warning'
  | 'memory_warning'
  | 'gpu_error'
  | 'connection_lost'
  | 'connection_restored';

// Configuración de exportación
export interface ExportConfig {
  format: ExportFormat;
  include_metadata: boolean;
  include_visualizations: boolean;
  compression: 'none' | 'gzip' | 'brotli';
  quality: QualityLevel;
}

export type ExportFormat = 
  | 'json'
  | 'csv'
  | 'pdb'
  | 'mol2'
  | 'sdf'
  | 'xml'
  | 'pdf_report';

// Preset de configuración
export interface ConfigurationPreset {
  id: string;
  name: string;
  description: string;
  configuration: DockingConfiguration;
  performance_config: PerformanceConfig;
  visualization_settings: VisualizationSettings;
  tags: string[];
  created_at: Date;
  usage_count: number;
}

// Proyecto de docking
export interface DockingProject {
  id: string;
  name: string;
  description: string;
  created_at: Date;
  updated_at: Date;
  jobs: DockingJob[];
  results: DockingResult[];
  configuration: DockingConfiguration;
  tags: string[];
  collaborators: string[];
  is_public: boolean;
}

// Estadísticas del proyecto
export interface ProjectStatistics {
  total_jobs: number;
  successful_jobs: number;
  failed_jobs: number;
  average_score: number;
  best_score: number;
  total_computation_time: number;
  success_rate: number;
  performance_metrics: {
    jobs_per_hour: number;
    average_job_duration: number;
    resource_utilization: number;
  };
}

// Alertas y notificaciones
export interface NotificationConfig {
  email_enabled: boolean;
  push_enabled: boolean;
  sound_enabled: boolean;
  job_completion: boolean;
  job_failure: boolean;
  performance_warnings: boolean;
  system_alerts: boolean;
}

export interface Notification {
  id: string;
  type: 'success' | 'warning' | 'error' | 'info';
  title: string;
  message: string;
  timestamp: Date;
  read: boolean;
  actions?: NotificationAction[];
}

export interface NotificationAction {
  label: string;
  action: string;
  style: 'primary' | 'secondary' | 'danger';
}

// Filtros y búsqueda
export interface SearchFilters {
  score_range?: [number, number];
  energy_range?: [number, number];
  date_range?: [Date, Date];
  job_status?: JobStatus[];
  tags?: string[];
  text_query?: string;
}

export interface SortConfig {
  field: string;
  direction: 'asc' | 'desc';
}

// Paginación
export interface PaginationConfig {
  page: number;
  page_size: number;
  total_items: number;
  total_pages: number;
}

// API Response Types
export interface ApiResponse<T = any> {
  success: boolean;
  data?: T;
  error?: string;
  message?: string;
  timestamp: number;
}

export interface PaginatedResponse<T = any> extends ApiResponse<T[]> {
  pagination: PaginationConfig;
}

// Validación
export interface ValidationResult {
  valid: boolean;
  errors: ValidationError[];
  warnings: ValidationWarning[];
}

export interface ValidationError {
  field: string;
  message: string;
  code: string;
}

export interface ValidationWarning {
  field: string;
  message: string;
  suggestion?: string;
}

// Utility types
export type PartialExcept<T, K extends keyof T> = Partial<T> & Pick<T, K>;
export type RequiredFields<T, K extends keyof T> = T & Required<Pick<T, K>>;

// Constants
export const DEFAULT_SEARCH_SPACE: SearchSpace = {
  center: { x: 0, y: 0, z: 0 },
  size: { x: 20, y: 20, z: 20 }
};

export const DEFAULT_DOCKING_CONFIG: DockingConfiguration = {
  receptor_id: '',
  ligand_id: '',
  search_space: DEFAULT_SEARCH_SPACE,
  exhaustiveness: 8,
  num_modes: 9,
  energy_range: 3.0,
  max_iterations: 1000,
  precision: 'high',
  use_gpu: true,
  parallel_processing: true
};

export const INTERACTION_COLORS: Record<InteractionType, string> = {
  hydrogen_bond: '#00ff00',
  van_der_waals: '#ffff00',
  electrostatic: '#ff0000',
  hydrophobic: '#ffa500',
  pi_pi: '#800080',
  cation_pi: '#008080',
  metal_coordination: '#c0c0c0'
};

export const ELEMENT_COLORS: Record<string, string> = {
  C: '#909090',
  N: '#3050f8',
  O: '#ff0d0d',
  H: '#ffffff',
  S: '#ffff30',
  P: '#ff8000',
  F: '#90e050',
  Cl: '#1ff01f',
  Br: '#a62929',
  I: '#940094',
  Fe: '#e06633',
  Zn: '#7d80b0'
};