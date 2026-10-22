// API Store - Estado reactivo ultra-optimizado con Svelte 5 runes
// Integra ApiService y WebSocketService con arquitectura reactiva 2025

import { 
  apiService, 
  webSocketService, 
  ApiError
} from '../services/ApiService';

// Import tipos unificados
import type {
  UploadResponse,
  FileMetadata,
  JobStatusInfo,
  CreateJobRequest,
  CreateJobResponse,
  SystemStats,
  HealthResponse,
  DockingConfig,
  JobProgressUpdate
} from '../types/unified.js';
import { notificationService } from '../services/NotificationService';
import type { ConnectionState } from '../services/WebSocketService';

// Estados reactivos con Svelte 5 runes - Arquitectura 2025
interface ApiState {
  // Connection
  connected: boolean;
  connecting: boolean;
  error: string | null;
  baseURL: string;
  latency: number | null;
  
  // Health & System
  health: HealthResponse | null;
  systemStats: SystemStats | null;
  
  // Files
  files: FileMetadata[];
  uploading: Map<string, number>; // filename -> progress
  
  // Jobs
  jobs: JobStatusInfo[];
  activeJobs: Map<string, JobStatusInfo>;
  
  // WebSocket
  wsConnected: boolean;
  wsError: string | null;
  jobUpdates: Map<string, JobProgressUpdate>;
  
  // Performance metrics
  performanceMetrics: {
    apiCallCount: number;
    totalResponseTime: number;
    avgResponseTime: number;
    errorCount: number;
    successRate: number;
  };
  
  // Cache and optimization
  cacheHitRate: number;
}

// Estado inicial optimizado
const initialState: ApiState = {
  connected: false,
  connecting: false,
  error: null,
  baseURL: apiService.getBaseURL(),
  latency: null,
  health: null,
  systemStats: null,
  files: [],
  uploading: new Map(),
  jobs: [],
  activeJobs: new Map(),
  wsConnected: false,
  wsError: null,
  jobUpdates: new Map(),
  performanceMetrics: {
    apiCallCount: 0,
    totalResponseTime: 0,
    avgResponseTime: 0,
    errorCount: 0,
    successRate: 0
  },
  cacheHitRate: 0
};

// Estado reactivo con Svelte 5 runes
let state = $state<ApiState>(initialState);

// Derived values para computación reactiva optimizada
const fileStats = $derived({
  total: state.files.length,
  receptors: state.files.filter(f => f.protein_info && f.protein_info.total_atoms > 100).length,
  ligands: state.files.filter(f => f.protein_info && f.protein_info.total_atoms <= 100).length,
  totalSize: state.files.reduce((sum, f) => sum + f.file_size, 0),
  avgSize: state.files.length > 0 ? state.files.reduce((sum, f) => sum + f.file_size, 0) / state.files.length : 0
});

const jobStats = $derived({
  total: state.jobs.length,
  active: state.activeJobs.size,
  completed: state.jobs.filter(j => j.status === 'completed').length,
  failed: state.jobs.filter(j => j.status === 'failed').length,
  cancelled: state.jobs.filter(j => j.status === 'cancelled').length,
  queued: state.jobs.filter(j => j.status === 'queued').length,
  running: state.jobs.filter(j => j.status === 'running').length,
  successRate: state.jobs.length > 0 ? 
    (state.jobs.filter(j => j.status === 'completed').length / state.jobs.length) * 100 : 0
});

const systemHealth = $derived({
  isHealthy: state.connected && state.wsConnected && !state.error && !state.wsError,
  connectivity: state.connected ? (state.wsConnected ? 'excellent' : 'partial') : 'poor',
  responseTime: state.latency || 0,
  qualityScore: (
    (state.connected ? 25 : 0) +
    (state.wsConnected ? 25 : 0) +
    (!state.error ? 25 : 0) +
    (state.performanceMetrics.successRate > 95 ? 25 : 
     state.performanceMetrics.successRate > 80 ? 15 : 
     state.performanceMetrics.successRate > 60 ? 10 : 0)
  )
});

const performanceInsights = $derived({
  isOptimal: state.performanceMetrics.avgResponseTime < 100 && state.performanceMetrics.successRate > 95,
  bottlenecks: {
    highLatency: state.latency && state.latency > 500,
    lowSuccessRate: state.performanceMetrics.successRate < 90,
    manyErrors: state.performanceMetrics.errorCount > 10
  },
  recommendations: [
    ...(state.latency && state.latency > 500 ? ['Consider network optimization'] : []),
    ...(state.performanceMetrics.successRate < 90 ? ['Check error handling'] : []),
    ...(state.performanceMetrics.errorCount > 10 ? ['Investigate API errors'] : [])
  ]
});

// Funciones de utilidad optimizadas
function setError(error: string | null) {
  state.error = error;
  if (error) {
    state.performanceMetrics.errorCount++;
    // updatePerformanceMetrics se llama solo en trackApiCall
  }
}

function setConnecting(connecting: boolean) {
  state.connecting = connecting;
}

function setConnected(connected: boolean) {
  state.connected = connected;
  if (connected) {
    state.error = null;
  }
  // Remover updatePerformanceMetrics aquí para evitar loops
}

function updatePerformanceMetrics() {
  // Solo actualizar métricas en API calls, no en cada cambio de estado
  if (state.performanceMetrics.apiCallCount > 0) {
    state.performanceMetrics.avgResponseTime = 
      state.performanceMetrics.totalResponseTime / state.performanceMetrics.apiCallCount;
    
    const successfulCalls = state.performanceMetrics.apiCallCount - state.performanceMetrics.errorCount;
    state.performanceMetrics.successRate = 
      (successfulCalls / state.performanceMetrics.apiCallCount) * 100;
  }
  
  // Remover update frequency que causa loops reactivos
}

function trackApiCall(responseTime: number, success: boolean = true) {
  state.performanceMetrics.apiCallCount++;
  state.performanceMetrics.totalResponseTime += responseTime;
  
  if (!success) {
    state.performanceMetrics.errorCount++;
  }
  
  updatePerformanceMetrics();
}

// API Functions
export const api = {
  // Estado reactivo (readonly)
  get state() {
    return state;
  },

  // Connection management optimizado
  async testConnection(): Promise<boolean> {
    const startTime = performance.now();
    setConnecting(true);
    
    try {
      const isConnected = await apiService.testConnection();
      const responseTime = performance.now() - startTime;
      
      state.latency = responseTime;
      trackApiCall(responseTime, isConnected);
      setConnected(isConnected);
      
      if (isConnected) {
        console.log(`✅ Backend connection successful (${responseTime.toFixed(1)}ms)`);
      } else {
        setError('Backend no disponible');
      }
      
      return isConnected;
    } catch (error) {
      const responseTime = performance.now() - startTime;
      trackApiCall(responseTime, false);
      
      const message = error instanceof ApiError ? error.message : 'Error de conexión';
      setError(message);
      setConnected(false);
      return false;
    } finally {
      setConnecting(false);
    }
  },

  async initialize(): Promise<void> {
    console.log('🚀 Initializing API connection...');
    
    // Test connection
    const connected = await this.testConnection();
    
    if (connected) {
      // Load initial data - but don't fail if some endpoints are unavailable
      const initPromises = [
        this.loadHealth().catch(e => console.warn('Health check failed:', e)),
        this.loadSystemStats().catch(e => console.warn('System stats failed:', e)),
        this.loadFiles().catch(e => console.warn('Files load failed:', e)),
        this.loadJobs().catch(e => console.warn('Jobs load failed:', e)),
      ];
      
      await Promise.allSettled(initPromises);
      
      // Connect WebSocket
      this.connectWebSocket();
      
      console.log('✅ API initialization completed');
    } else {
      throw new Error('Backend connection test failed');
    }
  },

  // Health & System
  async loadHealth(): Promise<void> {
    try {
      state.health = await apiService.getHealth();
    } catch (error) {
      console.error('Failed to load health:', error);
    }
  },

  async loadSystemStats(): Promise<void> {
    try {
      state.systemStats = await apiService.getSystemStats();
    } catch (error) {
      console.error('Failed to load system stats:', error);
    }
  },

  async ping(): Promise<{ message: string; timestamp: string; performance: string } | null> {
    try {
      return await apiService.ping();
    } catch (error) {
      console.error('Ping failed:', error);
      return null;
    }
  },

  // File management
  async uploadFile(
    file: File,
    onProgress?: (percent: number) => void
  ): Promise<UploadResponse | null> {
    const filename = file.name;
    
    try {
      // Set upload progress
      state.uploading.set(filename, 0);
      
      const response = await apiService.uploadProteinFile(file, (percent) => {
        state.uploading.set(filename, percent);
        onProgress?.(percent);
      });
      
      // Clear upload progress
      state.uploading.delete(filename);
      
      // Reload files list to get the actual file from server
      await this.loadFiles();
      
      console.log('✅ File uploaded successfully:', response.file_id);
      return response;
      
    } catch (error) {
      state.uploading.delete(filename);
      console.error('❌ File upload failed:', error);
      throw error;
    }
  },

  async loadFiles(): Promise<void> {
    try {
      console.log('loadFiles: Requesting files from server...');
      const response = await apiService.listFiles(100, 0);
      console.log('loadFiles: Server response:', response);
      
      // Force reactivity by creating a new array instead of replacing
      state.files = [...response.files];
      
      console.log('loadFiles: Updated state.files to:', state.files);
      console.log('loadFiles: Files array length:', state.files.length);
    } catch (error) {
      console.error('Failed to load files:', error);
    }
  },

  async deleteFile(fileId: string): Promise<boolean> {
    try {
      await apiService.deleteFile(fileId);
      state.files = state.files.filter(f => f.file_id !== fileId);
      console.log('✅ File deleted:', fileId);
      return true;
    } catch (error) {
      console.error('❌ Failed to delete file:', error);
      return false;
    }
  },

  async downloadFile(fileId: string, filename: string): Promise<void> {
    try {
      const blob = await apiService.downloadFile(fileId);
      
      // Create download link
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = filename;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      
      console.log('✅ File downloaded:', filename);
    } catch (error) {
      console.error('❌ Failed to download file:', error);
      throw error;
    }
  },

  // Job management
  async createJob(request: CreateJobRequest): Promise<CreateJobResponse | null> {
    try {
      console.log('🚀 API Store: Creating job with request:', request);
      console.log('🌐 API Store: Backend connected:', state.connected);
      console.log('🔗 API Store: WebSocket connected:', state.wsConnected);
      
      const response = await apiService.createDockingJob(request);
      console.log('📥 API Store: Received createJob response:', response);
      
      // Add to jobs list
      const jobStatus: JobStatusInfo = {
        job_id: response.job_id,
        status: 'queued',
        progress_percent: 0,
        current_step: 'Initializing',
        created_at: new Date().toISOString(),
        poses_evaluated: 0,
        poses_remaining: response.estimated_duration_seconds || 0
      };
      
      state.jobs.unshift(jobStatus);
      state.activeJobs.set(response.job_id, jobStatus);
      console.log('📋 API Store: Added job to state. Total jobs:', state.jobs.length);
      
      // Subscribe to WebSocket updates
      if (state.wsConnected) {
        webSocketService.subscribeToJob(response.job_id);
        console.log('🔗 API Store: Subscribed to WebSocket updates for job:', response.job_id);
      } else {
        console.warn('⚠️ API Store: WebSocket not connected, cannot subscribe to job updates');
      }
      
      console.log('✅ Job created successfully:', response.job_id);
      return response;
      
    } catch (error) {
      console.error('❌ API Store: Failed to create job:', error);
      
      // Log additional error details
      if (error instanceof Error) {
        console.error('❌ API Store: Error name:', error.name);
        console.error('❌ API Store: Error message:', error.message);
        console.error('❌ API Store: Error stack:', error.stack);
      }
      
      // Check for axios error details
      if (error && typeof error === 'object' && 'response' in error) {
        const axiosError = error as any;
        console.error('❌ API Store: HTTP Status:', axiosError.response?.status);
        console.error('❌ API Store: HTTP Data:', axiosError.response?.data);
        console.error('❌ API Store: HTTP Headers:', axiosError.response?.headers);
      }
      
      return null;
    }
  },

  async loadJobs(): Promise<void> {
    try {
      console.log('🔍 API Store: Loading jobs from backend...');
      const response = await apiService.listJobs(undefined, undefined, 50, 0);
      console.log('📋 API Store: Received jobs response:', response);
      console.log('📋 API Store: Jobs count:', response.jobs.length);
      
      state.jobs = response.jobs;
      
      // Update active jobs map
      state.activeJobs.clear();
      response.jobs.forEach(job => {
        if (job.status === 'queued' || job.status === 'running') {
          state.activeJobs.set(job.job_id, job);
        }
      });
      
      console.log('📋 API Store: Updated jobs in state. Total:', state.jobs.length);
      console.log('📋 API Store: Active jobs:', state.activeJobs.size);
      
    } catch (error) {
      console.error('❌ API Store: Failed to load jobs:', error);
    }
  },

  async cancelJob(jobId: string): Promise<boolean> {
    try {
      await apiService.cancelJob(jobId);
      
      // Update job status
      const job = state.activeJobs.get(jobId);
      if (job) {
        job.status = 'cancelled';
        state.activeJobs.delete(jobId);
      }
      
      // Update jobs list
      const jobIndex = state.jobs.findIndex(j => j.job_id === jobId);
      if (jobIndex >= 0) {
        state.jobs[jobIndex].status = 'cancelled';
      }
      
      console.log('✅ Job cancelled:', jobId);
      return true;
      
    } catch (error) {
      console.error('❌ Failed to cancel job:', error);
      return false;
    }
  },

  async getJobDetails(jobId: string): Promise<JobStatusInfo | null> {
    try {
      const job = await apiService.getJobInfo(jobId);
      
      // Update in state
      const jobIndex = state.jobs.findIndex(j => j.job_id === jobId);
      if (jobIndex >= 0) {
        state.jobs[jobIndex] = job;
      }
      
      if (job.status === 'queued' || job.status === 'running') {
        state.activeJobs.set(jobId, job);
      } else {
        state.activeJobs.delete(jobId);
      }
      
      return job;
      
    } catch (error) {
      console.error('Failed to get job details:', error);
      return null;
    }
  },

  // WebSocket management
  connectWebSocket(): void {
    console.log('🔌 Connecting WebSocket...');
    
    // Setup connection state callback
    webSocketService.onConnectionChange((wsState: ConnectionState) => {
      state.wsConnected = wsState.connected;
      state.wsError = wsState.error || null;
      
      if (wsState.connected) {
        console.log('✅ WebSocket connected');
        notificationService.connectionRestored();
        
        // Subscribe to active jobs
        state.activeJobs.forEach((_, jobId) => {
          webSocketService.subscribeToJob(jobId);
        });
      } else if (wsState.error) {
        console.log('❌ WebSocket connection lost');
        notificationService.connectionLost();
      }
    });
    
    // Setup job progress callback
    webSocketService.onJobProgress((update: JobProgressUpdate) => {
      state.jobUpdates.set(update.job_id, update);
      
      // Find notification ID for this job
      const existingNotifications = notificationService.getStats();
      let jobNotificationId: string | undefined;
      
      // Update job in active jobs
      const activeJob = state.activeJobs.get(update.job_id);
      if (activeJob) {
        activeJob.status = update.status;
        activeJob.progress_percent = update.progress_percent;
        activeJob.current_step = update.current_step;
        
        // Handle job status changes with notifications
        if (update.status === 'completed') {
          // Find and complete the progress notification
          if (jobNotificationId) {
            notificationService.jobCompleted(jobNotificationId, update.job_id, update.poses_generated);
          } else {
            notificationService.success(
              'Job Completed',
              `Job ${update.job_id} finished successfully`,
              { category: 'job', metadata: { jobId: update.job_id } }
            );
          }
          state.activeJobs.delete(update.job_id);
          
        } else if (update.status === 'failed') {
          notificationService.jobFailed(update.job_id, update.error_message || 'Unknown error');
          state.activeJobs.delete(update.job_id);
          
        } else if (update.status === 'running' && jobNotificationId) {
          // Update progress notification
          notificationService.updateProgress(
            jobNotificationId, 
            update.progress_percent, 
            update.current_step || `${update.progress_percent.toFixed(1)}% complete`
          );
        }
      }
      
      // Update in jobs list
      const jobIndex = state.jobs.findIndex(j => j.job_id === update.job_id);
      if (jobIndex >= 0) {
        Object.assign(state.jobs[jobIndex], update);
      }
    });
    
    // Setup error callback
    webSocketService.onError((error: string) => {
      state.wsError = error;
      console.error('🚨 WebSocket error:', error);
    });
    
    // Connect
    webSocketService.connect();
  },

  disconnectWebSocket(): void {
    webSocketService.disconnect();
    state.wsConnected = false;
    state.wsError = null;
  },

  // Configuration
  getDefaultConfig(): DockingConfig {
    return apiService.getDefaultDockingConfig();
  },

  // Utility functions
  updateBaseURL(newURL: string): void {
    apiService.updateBaseURL(newURL);
    webSocketService.updateBaseURL(newURL.replace('/api/v1', ''));
    state.baseURL = newURL;
  },

  // Refresh functions
  async refresh(): Promise<void> {
    console.log('🔄 Refreshing data...');
    
    await Promise.all([
      this.loadHealth(),
      this.loadSystemStats(),
      this.loadFiles(),
      this.loadJobs(),
    ]);
  },

  // Error handling
  clearError(): void {
    state.error = null;
  },

  clearWSError(): void {
    state.wsError = null;
  },

  // Getters optimizados con derived values
  get isLoading(): boolean {
    return state.connecting;
  },

  get hasError(): boolean {
    return !!state.error || !!state.wsError;
  },

  get uploadProgress(): Map<string, number> {
    return state.uploading;
  },

  get activeJobsCount(): number {
    return jobStats.active;
  },

  get completedJobsCount(): number {
    return jobStats.completed;
  },

  get failedJobsCount(): number {
    return jobStats.failed;
  },

  // Nuevos getters avanzados
  get fileStats() {
    return fileStats;
  },

  get jobStats() {
    return jobStats;
  },

  get systemHealth() {
    return systemHealth;
  },

  get performanceInsights() {
    return performanceInsights;
  },

  get isOptimalPerformance(): boolean {
    return performanceInsights.isOptimal;
  },

  get hasPerformanceIssues(): boolean {
    return Object.values(performanceInsights.bottlenecks).some(issue => issue);
  },

  get qualityScore(): number {
    return systemHealth.qualityScore;
  },

  // File helpers optimizados
  getFileById(fileId: string): FileMetadata | undefined {
    return state.files.find(f => f.file_id === fileId);
  },

  getReceptorFiles(): FileMetadata[] {
    return state.files.filter(f => f.protein_info && f.protein_info.total_atoms > 100);
  },

  getLigandFiles(): FileMetadata[] {
    return state.files.filter(f => f.protein_info && f.protein_info.total_atoms <= 100);
  },

  // Advanced file analytics
  getFilesByType(type: 'receptor' | 'ligand' | 'unknown'): FileMetadata[] {
    switch (type) {
      case 'receptor':
        return this.getReceptorFiles();
      case 'ligand':
        return this.getLigandFiles();
      case 'unknown':
        return state.files.filter(f => !f.protein_info || f.protein_info.total_atoms === 0);
      default:
        return [];
    }
  },

  getFilesUploadedToday(): FileMetadata[] {
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    return state.files.filter(f => new Date(f.upload_timestamp) >= today);
  },

  // Job helpers
  getJobById(jobId: string): JobStatusInfo | undefined {
    return state.jobs.find(j => j.job_id === jobId);
  },

  getJobUpdate(jobId: string): JobProgressUpdate | undefined {
    return state.jobUpdates.get(jobId);
  },

  getActiveJobs(): JobStatusInfo[] {
    return Array.from(state.activeJobs.values());
  },

  getCompletedJobs(): JobStatusInfo[] {
    return state.jobs.filter(j => j.status === 'completed');
  },
};

// Auto-initialize API when in browser
if (typeof window !== 'undefined') {
  // Auto-initialize with enhanced error handling
  setTimeout(() => {
    api.initialize()
      .then(() => {
        console.log('🚀 API Store initialized with Svelte 5 runes');
      })
      .catch(error => {
        console.error('❌ API initialization failed:', error);
      });
  }, 100);
  
  // Setup performance monitoring interval (traditional approach)
  setInterval(() => {
    if (state.connected) {
      api.loadSystemStats().catch(console.warn);
    }
  }, 30000);
}

// Export enhanced API with Svelte 5 features
// Export functions that return derived values (not the derived values directly)
export const getFileStats = () => fileStats;
export const getJobStats = () => jobStats;
export const getSystemHealth = () => systemHealth;
export const getPerformanceInsights = () => performanceInsights;

export default api;