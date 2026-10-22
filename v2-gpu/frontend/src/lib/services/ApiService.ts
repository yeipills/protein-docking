// API Service - Conecta frontend Svelte con backend Rust ultra-optimizado
// Manejo de HTTP requests, WebSockets, error handling y retry logic

import axios, { type AxiosInstance, type AxiosResponse, type AxiosError } from 'axios';
import { WebSocketService } from './WebSocketService.js';

// Import tipos unificados
import type {
  ProteinInfo,
  UploadResponse,
  FileMetadata,
  FilesListResponse,
  Algorithm,
  AlgorithmsResponse,
  DockingConfig,
  CreateJobRequest,
  CreateJobResponse,
  JobStatusInfo,
  JobsListResponse,
  SystemStats,
  HealthResponse,
  PingResponse,
  DockingResult,
  JobStatus
} from '../types/unified.js';

// Re-export para compatibilidad
export type {
  ProteinInfo,
  UploadResponse,
  FileMetadata,
  FilesListResponse,
  Algorithm,
  DockingConfig,
  CreateJobRequest,
  CreateJobResponse,
  JobStatusInfo as JobStatus,
  SystemStats,
  HealthResponse,
  DockingResult
};

class ApiService {
  private client: AxiosInstance;
  private baseURL: string;
  private retryCount = 3;
  private retryDelay = 1000;

  constructor(baseURL?: string) {
    this.baseURL = baseURL || this.detectBackendURL();
    
    this.client = axios.create({
      baseURL: this.baseURL,
      timeout: 30000, // 30 seconds
      headers: {
        'Content-Type': 'application/json',
      },
    });

    this.setupInterceptors();
  }

  private detectBackendURL(): string {
    // Auto-detect backend URL based on environment
    if (typeof window !== 'undefined') {
      const hostname = window.location.hostname;
      const protocol = window.location.protocol;
      
      // Development: backend on port 8080 (start.sh script)
      if (hostname === 'localhost' || hostname === '127.0.0.1') {
        return `${protocol}//${hostname}:8080/api/v1`;
      }
      
      // Production: same domain, /api path
      return `${protocol}//${hostname}/api/v1`;
    }
    
    // Server-side fallback
    return 'http://localhost:8080/api/v1';
  }

  private setupInterceptors(): void {
    // Request interceptor
    this.client.interceptors.request.use(
      (config) => {
        console.log(`🌐 API Request: ${config.method?.toUpperCase()} ${config.url}`);
        return config;
      },
      (error) => {
        console.error('🚨 API Request Error:', error);
        return Promise.reject(error);
      }
    );

    // Response interceptor with retry logic
    this.client.interceptors.response.use(
      (response) => {
        console.log(`✅ API Response: ${response.status} ${response.config.url}`);
        return response;
      },
      async (error: AxiosError) => {
        console.error(`❌ API Error: ${error.response?.status} ${error.config?.url}`, error.message);
        
        // Retry logic for network errors
        if (this.shouldRetry(error) && error.config) {
          return this.retryRequest(error.config);
        }
        
        return Promise.reject(this.transformError(error));
      }
    );
  }

  private shouldRetry(error: AxiosError): boolean {
    // Retry on network errors, timeouts, and 5xx server errors
    return (
      !error.response ||
      error.code === 'ECONNABORTED' ||
      (error.response.status >= 500 && error.response.status < 600)
    );
  }

  private async retryRequest(config: any, attempt = 1): Promise<AxiosResponse> {
    if (attempt > this.retryCount) {
      throw new Error(`Max retries (${this.retryCount}) exceeded`);
    }

    console.log(`🔄 Retrying request (attempt ${attempt}/${this.retryCount}): ${config.url}`);
    
    // Exponential backoff
    await new Promise(resolve => setTimeout(resolve, this.retryDelay * Math.pow(2, attempt - 1)));
    
    try {
      return await this.client.request(config);
    } catch (error) {
      if (this.shouldRetry(error as AxiosError)) {
        return this.retryRequest(config, attempt + 1);
      }
      throw error;
    }
  }

  private transformError(error: AxiosError): ApiError {
    const status = error.response?.status || 0;
    const message = error.response?.data?.error || error.message || 'Unknown error';
    const details = error.response?.data?.details;

    return new ApiError(message, status, details);
  }

  // Health & System endpoints
  async ping(): Promise<PingResponse> {
    const response = await this.client.get('/ping');
    return response.data;
  }

  async getHealth(): Promise<HealthResponse> {
    const response = await this.client.get('/health');
    return response.data;
  }

  async getSystemStats(): Promise<SystemStats> {
    const response = await this.client.get('/system/stats');
    return response.data;
  }

  // File upload & management
  async uploadProteinFile(
    file: File,
    onProgress?: (percent: number) => void
  ): Promise<UploadResponse> {
    const formData = new FormData();
    formData.append('file', file);

    const response = await this.client.post('/upload', formData, {
      headers: {
        'Content-Type': 'multipart/form-data',
      },
      onUploadProgress: (progressEvent) => {
        if (onProgress && progressEvent.total) {
          const percent = Math.round((progressEvent.loaded * 100) / progressEvent.total);
          onProgress(percent);
        }
      },
    });

    return response.data;
  }

  async getFileInfo(fileId: string): Promise<FileMetadata> {
    const response = await this.client.get(`/files/${fileId}`);
    return response.data;
  }

  async listFiles(limit = 50, offset = 0): Promise<FilesListResponse> {
    const response = await this.client.get('/files', {
      params: { limit, offset }
    });
    return response.data;
  }

  async downloadFile(fileId: string): Promise<Blob> {
    const response = await this.client.get(`/files/${fileId}/download`, {
      responseType: 'blob'
    });
    return response.data;
  }

  async deleteFile(fileId: string): Promise<{ message: string }> {
    const response = await this.client.delete(`/files/${fileId}`);
    return response.data;
  }

  // Job management
  async createDockingJob(request: CreateJobRequest): Promise<CreateJobResponse> {
    const response = await this.client.post('/jobs', request);
    return response.data;
  }

  async getJobInfo(jobId: string): Promise<JobStatusInfo & { 
    results?: DockingResult[];
    computation_stats?: any;
  }> {
    const response = await this.client.get(`/jobs/${jobId}`);
    return response.data;
  }

  async listJobs(
    userId?: string,
    status?: string,
    limit = 50,
    offset = 0
  ): Promise<JobsListResponse> {
    const params: any = { limit, offset };
    if (userId) params.user_id = userId;
    if (status) params.status = status;

    console.log('🔍 ApiService: Requesting jobs with params:', params);
    console.log('🔍 ApiService: Request URL:', this.client.defaults.baseURL + '/jobs');
    
    const response = await this.client.get('/jobs', { params });
    console.log('📋 ApiService: Jobs response status:', response.status);
    console.log('📋 ApiService: Jobs response data:', response.data);
    
    return response.data;
  }

  async cancelJob(jobId: string): Promise<{ message: string }> {
    const response = await this.client.post(`/jobs/${jobId}/cancel`);
    return response.data;
  }

  // Configuration & defaults
  getDefaultDockingConfig(): DockingConfig {
    return {
      max_poses: 1000,
      energy_threshold: -5.0,
      rmsd_threshold: 2.0,
      sampling_strategy: 'Hybrid',
      optimization_rounds: 5,
      parallel_workers: 4,
      use_gpu_acceleration: true,
      advanced_scoring: true,
    };
  }

  // WebSocket URL helper
  getWebSocketURL(path: string): string {
    const wsProtocol = this.baseURL.startsWith('https') ? 'wss' : 'ws';
    const baseWS = this.baseURL.replace('http', 'ws').replace('/api', '');
    return `${baseWS}${path}`;
  }

  // Connection testing
  async testConnection(): Promise<boolean> {
    try {
      await this.ping();
      return true;
    } catch (error) {
      console.error('🚨 Backend connection failed:', error);
      return false;
    }
  }

  // Update base URL (for switching environments)
  updateBaseURL(newBaseURL: string): void {
    this.baseURL = newBaseURL;
    this.client.defaults.baseURL = newBaseURL;
  }

  // Get current base URL
  getBaseURL(): string {
    return this.baseURL;
  }
}

// Custom error class
export class ApiError extends Error {
  constructor(
    message: string,
    public status: number = 0,
    public details?: any
  ) {
    super(message);
    this.name = 'ApiError';
  }

  get isNetworkError(): boolean {
    return this.status === 0;
  }

  get isClientError(): boolean {
    return this.status >= 400 && this.status < 500;
  }

  get isServerError(): boolean {
    return this.status >= 500;
  }
}

// Singleton instances
export const apiService = new ApiService();
export const webSocketService = new WebSocketService();

// Export classes for testing with custom URLs
export { ApiService, WebSocketService };