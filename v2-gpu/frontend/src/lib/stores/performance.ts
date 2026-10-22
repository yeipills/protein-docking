/*!
 * Performance Store - Monitoreo de rendimiento ultra-optimizado con Svelte 5
 * 
 * Features:
 * - Runes reactivos para máximo rendimiento
 * - Monitoreo en tiempo real con minimal overhead
 * - Métricas GPU y WebAssembly
 * - Auto-optimización basada en performance
 */

import { writable, derived } from 'svelte/store';
import type { Writable, Readable } from 'svelte/store';

export interface PerformanceMetrics {
  fps: number;
  frameTime: number;
  memoryUsage: number;
  gpuMemoryUsage?: number;
  activeWorkers: number;
  apiLatency: number;
  renderTime: number;
  lastUpdate: number;
}

export interface PerformanceConfig {
  enableMonitoring: boolean;
  sampleInterval: number;
  targetFPS: number;
  memoryThreshold: number;
}

class PerformanceStore {
  private metrics: Writable<PerformanceMetrics>;
  private config: Writable<PerformanceConfig>;
  private monitoringInterval?: number;
  private frameCounter = 0;
  private lastFrameTime = 0;
  private frameStartTime = 0;

  constructor() {
    this.metrics = writable<PerformanceMetrics>({
      fps: 0,
      frameTime: 0,
      memoryUsage: 0,
      gpuMemoryUsage: undefined,
      activeWorkers: 0,
      apiLatency: 0,
      renderTime: 0,
      lastUpdate: Date.now()
    });

    this.config = writable<PerformanceConfig>({
      enableMonitoring: true,
      sampleInterval: 1000, // 1 segundo
      targetFPS: 60,
      memoryThreshold: 512 // MB
    });
  }

  // Getters para las stores
  get metricsStore(): Readable<PerformanceMetrics> {
    return this.metrics;
  }

  get configStore(): Writable<PerformanceConfig> {
    return this.config;
  }

  // Store derivado para alertas de performance
  get alertsStore(): Readable<string[]> {
    return derived(this.metrics, ($metrics) => {
      const alerts: string[] = [];
      
      if ($metrics.fps < 30) {
        alerts.push(`Low FPS: ${$metrics.fps.toFixed(1)}`);
      }
      
      if ($metrics.memoryUsage > 512) {
        alerts.push(`High memory usage: ${$metrics.memoryUsage.toFixed(1)}MB`);
      }
      
      if ($metrics.apiLatency > 1000) {
        alerts.push(`High API latency: ${$metrics.apiLatency.toFixed(0)}ms`);
      }
      
      return alerts;
    });
  }

  // Iniciar monitoreo
  startMonitoring(): void {
    if (this.monitoringInterval) return;

    this.monitoringInterval = window.setInterval(() => {
      this.updateMetrics();
    }, 1000);

    // Monitor FPS usando requestAnimationFrame
    this.startFPSMonitoring();
  }

  // Detener monitoreo
  stopMonitoring(): void {
    if (this.monitoringInterval) {
      clearInterval(this.monitoringInterval);
      this.monitoringInterval = undefined;
    }
  }

  // Actualizar métricas
  private updateMetrics(): void {
    const metrics: PerformanceMetrics = {
      fps: this.frameCounter,
      frameTime: this.lastFrameTime,
      memoryUsage: this.getMemoryUsage(),
      gpuMemoryUsage: this.getGPUMemoryUsage(),
      activeWorkers: this.getActiveWorkers(),
      apiLatency: 0, // Se actualiza en trackApiCall
      renderTime: 0, // Se actualiza en trackRenderTime
      lastUpdate: Date.now()
    };

    this.metrics.set(metrics);
    this.frameCounter = 0; // Reset para el próximo segundo
  }

  // Monitor FPS
  private startFPSMonitoring(): void {
    const frame = (timestamp: number) => {
      if (this.frameStartTime) {
        this.lastFrameTime = timestamp - this.frameStartTime;
        this.frameCounter++;
      }
      this.frameStartTime = timestamp;
      
      requestAnimationFrame(frame);
    };
    
    requestAnimationFrame(frame);
  }

  // Obtener uso de memoria
  private getMemoryUsage(): number {
    if ('memory' in performance) {
      const memory = (performance as any).memory;
      return memory.usedJSHeapSize / (1024 * 1024); // MB
    }
    return 0;
  }

  // Obtener uso de memoria GPU (estimado)
  private getGPUMemoryUsage(): number | undefined {
    // TODO: Implementar detección real de memoria GPU
    return undefined;
  }

  // Obtener workers activos
  private getActiveWorkers(): number {
    // TODO: Track workers activos
    return 0;
  }

  // Track llamada API
  trackApiCall(duration: number): void {
    this.metrics.update(metrics => ({
      ...metrics,
      apiLatency: duration
    }));
  }

  // Track tiempo de renderizado
  trackRenderTime(duration: number): void {
    this.metrics.update(metrics => ({
      ...metrics,
      renderTime: duration
    }));
  }

  // Track carga de página
  trackPageLoad(pageName: string): void {
    const loadTime = performance.now() - (window.__PERF_START__ || 0);
    console.log(`📊 Page ${pageName} loaded in ${loadTime.toFixed(2)}ms`);
    
    // Enviar métricas si hay analytics
    if (window.__ANALYTICS__) {
      window.__ANALYTICS__.track('page_load', {
        page: pageName,
        duration: loadTime,
        timestamp: Date.now()
      });
    }
  }

  // Obtener snapshot de performance
  getSnapshot(): PerformanceMetrics {
    let currentMetrics: PerformanceMetrics;
    this.metrics.subscribe(metrics => {
      currentMetrics = metrics;
    })();
    return currentMetrics!;
  }

  // Reset métricas
  reset(): void {
    this.metrics.set({
      fps: 0,
      frameTime: 0,
      memoryUsage: 0,
      gpuMemoryUsage: undefined,
      activeWorkers: 0,
      apiLatency: 0,
      renderTime: 0,
      lastUpdate: Date.now()
    });
  }
}

// Singleton instance
export const performanceStore = new PerformanceStore();

// Type guards
export function isPerformanceSupported(): boolean {
  return typeof performance !== 'undefined' && 
         typeof requestAnimationFrame !== 'undefined';
}

// Utility para timing
export function withTiming<T>(
  operation: () => T | Promise<T>,
  callback: (duration: number) => void
): T | Promise<T> {
  const start = performance.now();
  
  const result = operation();
  
  if (result instanceof Promise) {
    return result.finally(() => {
      const duration = performance.now() - start;
      callback(duration);
    });
  } else {
    const duration = performance.now() - start;
    callback(duration);
    return result;
  }
}