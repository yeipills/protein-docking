/*!
 * Docking Store - Store reactivo ultra-optimizado con Svelte 5 runes
 * 
 * Features:
 * - State management con runes para zero-runtime overhead
 * - Reactive updates con minimal re-renders
 * - TypeScript estricto para mejor DX
 * - Performance monitoring integrado
 */

import type { DockingConfiguration, DockingResult, DockingJob } from '$lib/types/docking';

// Tipos para el estado del store
interface DockingState {
  currentJob: DockingJob | null;
  results: DockingResult[];
  isProcessing: boolean;
  progress: number;
  error: string | null;
  configuration: DockingConfiguration;
  history: DockingJob[];
  statistics: DockingStatistics;
}

interface DockingStatistics {
  totalJobs: number;
  successfulJobs: number;
  failedJobs: number;
  averageExecutionTime: number;
  bestScore: number;
  totalComputeTime: number;
}

interface PerformanceMetrics {
  computationsPerSecond: number;
  cpuUtilization: number;
  memoryUsage: number;
  gpuUtilization: number;
  throughput: number;
  queueLength: number;
}

// Store principal usando Svelte 5 runes
class DockingStore {
  // Estado reactivo usando runes
  #state = $state<DockingState>({
    currentJob: null,
    results: [],
    isProcessing: false,
    progress: 0,
    error: null,
    configuration: this.getDefaultConfiguration(),
    history: [],
    statistics: {
      totalJobs: 0,
      successfulJobs: 0,
      failedJobs: 0,
      averageExecutionTime: 0,
      bestScore: 0,
      totalComputeTime: 0
    }
  });

  // Métricas de rendimiento reactivas
  #performanceMetrics = $state<PerformanceMetrics>({
    computationsPerSecond: 0,
    cpuUtilization: 0,
    memoryUsage: 0,
    gpuUtilization: 0,
    throughput: 0,
    queueLength: 0
  });

  // WebSocket connection state
  #connectionState = $state<'disconnected' | 'connecting' | 'connected' | 'error'>('disconnected');

  // Configuración reactiva
  #settings = $state({
    autoSave: true,
    maxHistoryItems: 100,
    enableNotifications: true,
    performanceMode: 'balanced' as 'eco' | 'balanced' | 'performance' | 'ultra',
    enableGPU: true,
    parallelJobs: 1
  });

  constructor() {
    // Cargar configuración desde localStorage
    this.loadFromStorage();
    
    // Setup auto-save reactivo
    $effect(() => {
      if (this.#settings.autoSave) {
        this.saveToStorage();
      }
    });

    // Auto-optimización basada en performance
    $effect(() => {
      this.autoOptimizePerformance();
    });
  }

  // Getters reactivos
  get state() {
    return this.#state;
  }

  get performanceMetrics() {
    return this.#performanceMetrics;
  }

  get connectionState() {
    return this.#connectionState;
  }

  get settings() {
    return this.#settings;
  }

  // Computed properties reactivos
  get canStartJob() {
    return !this.#state.isProcessing && 
           this.#state.configuration.receptor_id && 
           this.#state.configuration.ligand_id &&
           this.#connectionState === 'connected';
  }

  get completionRate() {
    if (this.#state.statistics.totalJobs === 0) return 0;
    return (this.#state.statistics.successfulJobs / this.#state.statistics.totalJobs) * 100;
  }

  get averageScore() {
    if (this.#state.results.length === 0) return 0;
    return this.#state.results.reduce((sum, result) => sum + result.score, 0) / this.#state.results.length;
  }

  get isHighPerformance() {
    return this.#performanceMetrics.computationsPerSecond > 100 &&
           this.#performanceMetrics.cpuUtilization < 80;
  }

  // Actions del store
  updateConfiguration(config: Partial<DockingConfiguration>) {
    this.#state.configuration = { ...this.#state.configuration, ...config };
  }

  startJob(job: DockingJob) {
    if (!this.canStartJob) {
      throw new Error('Cannot start job: conditions not met');
    }

    this.#state.currentJob = job;
    this.#state.isProcessing = true;
    this.#state.progress = 0;
    this.#state.error = null;
    
    // Agregar a historial
    this.#state.history = [job, ...this.#state.history.slice(0, this.#settings.maxHistoryItems - 1)];
    
    // Actualizar estadísticas
    this.#state.statistics.totalJobs++;
  }

  updateProgress(progress: number, estimatedTimeRemaining?: number) {
    if (this.#state.currentJob) {
      this.#state.progress = Math.max(0, Math.min(100, progress));
      
      if (estimatedTimeRemaining !== undefined) {
        this.#state.currentJob.estimatedTimeRemaining = estimatedTimeRemaining;
      }
    }
  }

  completeJob(result: DockingResult) {
    if (!this.#state.currentJob) return;

    // Agregar resultado
    this.#state.results = [result, ...this.#state.results];
    
    // Actualizar estadísticas
    this.#state.statistics.successfulJobs++;
    const executionTime = Date.now() - this.#state.currentJob.createdAt.getTime();
    this.updateAverageExecutionTime(executionTime);
    
    if (result.score > this.#state.statistics.bestScore) {
      this.#state.statistics.bestScore = result.score;
    }

    // Limpiar estado actual
    this.#state.currentJob = null;
    this.#state.isProcessing = false;
    this.#state.progress = 100;
    
    // Auto-notificación
    if (this.#settings.enableNotifications) {
      this.showNotification(`Docking completed with score: ${result.score.toFixed(3)}`, 'success');
    }
  }

  failJob(error: string) {
    if (!this.#state.currentJob) return;

    this.#state.error = error;
    this.#state.isProcessing = false;
    this.#state.currentJob = null;
    this.#state.statistics.failedJobs++;

    if (this.#settings.enableNotifications) {
      this.showNotification(`Docking failed: ${error}`, 'error');
    }
  }

  cancelJob() {
    if (this.#state.currentJob) {
      this.#state.currentJob = null;
      this.#state.isProcessing = false;
      this.#state.progress = 0;
      
      if (this.#settings.enableNotifications) {
        this.showNotification('Job cancelled by user', 'warning');
      }
    }
  }

  clearResults() {
    this.#state.results = [];
  }

  clearHistory() {
    this.#state.history = [];
  }

  clearError() {
    this.#state.error = null;
  }

  // Performance metrics update
  updateMetrics(metrics: Partial<PerformanceMetrics>) {
    Object.assign(this.#performanceMetrics, metrics);
  }

  // Connection state management
  setConnectionState(state: typeof this.#connectionState) {
    this.#connectionState = state;
  }

  // Settings management
  updateSettings(settings: Partial<typeof this.#settings>) {
    Object.assign(this.#settings, settings);
  }

  // Utility methods
  private getDefaultConfiguration(): DockingConfiguration {
    return {
      receptor_id: '',
      ligand_id: '',
      search_space: {
        center: { x: 0, y: 0, z: 0 },
        size: { x: 20, y: 20, z: 20 }
      },
      exhaustiveness: 8,
      num_modes: 9,
      energy_range: 3.0,
      max_iterations: 1000,
      precision: 'high',
      use_gpu: true,
      parallel_processing: true
    };
  }

  private updateAverageExecutionTime(newTime: number) {
    const stats = this.#state.statistics;
    stats.averageExecutionTime = 
      (stats.averageExecutionTime * (stats.successfulJobs - 1) + newTime) / stats.successfulJobs;
    stats.totalComputeTime += newTime;
  }

  private autoOptimizePerformance() {
    const metrics = this.#performanceMetrics;
    
    // Auto-ajustar configuración basada en performance
    if (metrics.cpuUtilization > 90) {
      // Reducir carga
      if (this.#settings.parallelJobs > 1) {
        this.#settings.parallelJobs--;
      }
    } else if (metrics.cpuUtilization < 50 && metrics.computationsPerSecond > 0) {
      // Aumentar carga si hay capacidad
      if (this.#settings.parallelJobs < navigator.hardwareConcurrency) {
        this.#settings.parallelJobs++;
      }
    }

    // Auto-ajustar calidad basada en FPS
    if (metrics.computationsPerSecond < 30) {
      if (this.#settings.performanceMode === 'ultra') {
        this.#settings.performanceMode = 'performance';
      } else if (this.#settings.performanceMode === 'performance') {
        this.#settings.performanceMode = 'balanced';
      }
    }
  }

  private saveToStorage() {
    try {
      const dataToSave = {
        configuration: this.#state.configuration,
        settings: this.#settings,
        statistics: this.#state.statistics,
        results: this.#state.results.slice(0, 50), // Solo guardar últimos 50 resultados
        timestamp: Date.now()
      };
      
      localStorage.setItem('docking-store', JSON.stringify(dataToSave));
    } catch (error) {
      console.warn('Failed to save to localStorage:', error);
    }
  }

  private loadFromStorage() {
    try {
      const saved = localStorage.getItem('docking-store');
      if (!saved) return;

      const data = JSON.parse(saved);
      
      // Verificar que no sea muy antiguo (7 días)
      if (Date.now() - data.timestamp > 7 * 24 * 60 * 60 * 1000) {
        localStorage.removeItem('docking-store');
        return;
      }

      // Restaurar datos
      if (data.configuration) {
        this.#state.configuration = { ...this.getDefaultConfiguration(), ...data.configuration };
      }
      
      if (data.settings) {
        Object.assign(this.#settings, data.settings);
      }
      
      if (data.statistics) {
        Object.assign(this.#state.statistics, data.statistics);
      }
      
      if (data.results && Array.isArray(data.results)) {
        this.#state.results = data.results;
      }

    } catch (error) {
      console.warn('Failed to load from localStorage:', error);
      localStorage.removeItem('docking-store');
    }
  }

  private showNotification(message: string, type: 'success' | 'error' | 'warning' | 'info') {
    // En una implementación real, esto sería manejado por un servicio de notificaciones
    if ('Notification' in window && Notification.permission === 'granted') {
      new Notification('Protein Docking', {
        body: message,
        icon: '/favicon.ico',
        tag: 'docking-notification'
      });
    }
  }

  // Métodos de conveniencia para debugging y testing
  getDebugInfo() {
    return {
      state: structuredClone(this.#state),
      metrics: structuredClone(this.#performanceMetrics),
      connection: this.#connectionState,
      settings: structuredClone(this.#settings)
    };
  }

  reset() {
    this.#state = {
      currentJob: null,
      results: [],
      isProcessing: false,
      progress: 0,
      error: null,
      configuration: this.getDefaultConfiguration(),
      history: [],
      statistics: {
        totalJobs: 0,
        successfulJobs: 0,
        failedJobs: 0,
        averageExecutionTime: 0,
        bestScore: 0,
        totalComputeTime: 0
      }
    };

    this.#performanceMetrics = {
      computationsPerSecond: 0,
      cpuUtilization: 0,
      memoryUsage: 0,
      gpuUtilization: 0,
      throughput: 0,
      queueLength: 0
    };

    this.#connectionState = 'disconnected';
    
    localStorage.removeItem('docking-store');
  }
}

// Singleton instance
export const dockingStore = new DockingStore();

// Tipos exportados
export type {
  DockingState,
  DockingStatistics,
  PerformanceMetrics
};

// Utility functions
export function createDockingJob(
  id: string,
  config: DockingConfiguration,
  priority: number = 5
): DockingJob {
  return {
    id,
    config,
    priority,
    estimatedDuration: estimateJobDuration(config),
    createdAt: new Date()
  };
}

export function estimateJobDuration(config: DockingConfiguration): number {
  // Estimación simple basada en parámetros
  const baseTime = config.exhaustiveness * config.num_modes * 0.5;
  const gpuMultiplier = config.use_gpu ? 0.1 : 1.0;
  const parallelMultiplier = config.parallel_processing ? 0.5 : 1.0;
  
  return Math.round(baseTime * gpuMultiplier * parallelMultiplier * 1000); // ms
}

export function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  if (ms < 3600000) return `${(ms / 60000).toFixed(1)}m`;
  return `${(ms / 3600000).toFixed(1)}h`;
}

export function calculateThroughput(jobs: number, timeMs: number): number {
  if (timeMs === 0) return 0;
  return (jobs / timeMs) * 1000; // jobs per second
}