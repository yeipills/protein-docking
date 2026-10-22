/*!
 * System Store - Estado del sistema y capacidades
 */

import { writable, derived } from 'svelte/store';
import type { Writable, Readable } from 'svelte/store';

export interface SystemCapabilities {
  webgl: boolean;
  webgl2: boolean;
  webassembly: boolean;
  workers: boolean;
  sharedArrayBuffer: boolean;
  offscreenCanvas: boolean;
  gpu?: GPUCapabilities;
}

export interface GPUCapabilities {
  vendor: string;
  renderer: string;
  maxTextureSize: number;
  maxVertexAttribs: number;
  supportedExtensions: string[];
}

export interface SystemInfo {
  userAgent: string;
  platform: string;
  hardwareConcurrency: number;
  deviceMemory?: number;
  connection?: NetworkInformation;
}

export interface SystemState {
  capabilities: SystemCapabilities;
  info: SystemInfo;
  isOptimal: boolean;
  warnings: string[];
  recommendations: string[];
}

class SystemStore {
  private state: Writable<SystemState>;

  constructor() {
    this.state = writable<SystemState>({
      capabilities: this.getInitialCapabilities(),
      info: this.getSystemInfo(),
      isOptimal: false,
      warnings: [],
      recommendations: []
    });
  }

  get store(): Readable<SystemState> {
    return this.state;
  }

  // Store derivado para status general
  get statusStore(): Readable<'optimal' | 'good' | 'limited' | 'unsupported'> {
    return derived(this.state, ($state) => {
      const caps = $state.capabilities;
      
      if (caps.webgl2 && caps.webassembly && caps.workers && caps.offscreenCanvas) {
        return 'optimal';
      } else if (caps.webgl && caps.webassembly && caps.workers) {
        return 'good';
      } else if (caps.webgl && caps.webassembly) {
        return 'limited';
      } else {
        return 'unsupported';
      }
    });
  }

  // Capabilities iniciales (desde window.__FEATURES__)
  private getInitialCapabilities(): SystemCapabilities {
    if (typeof window === 'undefined') {
      return {
        webgl: false,
        webgl2: false,
        webassembly: false,
        workers: false,
        sharedArrayBuffer: false,
        offscreenCanvas: false
      };
    }

    return window.__FEATURES__ || {
      webgl: false,
      webgl2: false,
      webassembly: false,
      workers: false,
      sharedArrayBuffer: false,
      offscreenCanvas: false
    };
  }

  // Información del sistema
  private getSystemInfo(): SystemInfo {
    if (typeof window === 'undefined') {
      return {
        userAgent: '',
        platform: '',
        hardwareConcurrency: 1
      };
    }

    return {
      userAgent: navigator.userAgent,
      platform: navigator.platform,
      hardwareConcurrency: navigator.hardwareConcurrency || 1,
      deviceMemory: (navigator as any).deviceMemory,
      connection: (navigator as any).connection
    };
  }

  // Check completo de capacidades
  async checkCapabilities(): Promise<void> {
    if (typeof window === 'undefined') return;

    const capabilities: SystemCapabilities = {
      ...this.getInitialCapabilities()
    };

    // Check GPU capabilities si WebGL está disponible
    if (capabilities.webgl) {
      capabilities.gpu = await this.getGPUCapabilities();
    }

    // Generar warnings y recomendaciones
    const { warnings, recommendations } = this.generateAdvice(capabilities);

    // Determinar si el sistema es óptimo
    const isOptimal = this.isSystemOptimal(capabilities);

    this.state.update(state => ({
      ...state,
      capabilities,
      isOptimal,
      warnings,
      recommendations
    }));
  }

  // Obtener capacidades GPU
  private async getGPUCapabilities(): Promise<GPUCapabilities | undefined> {
    try {
      const canvas = document.createElement('canvas');
      const gl = canvas.getContext('webgl2') || canvas.getContext('webgl');
      
      if (!gl) return undefined;

      const debugInfo = gl.getExtension('WEBGL_debug_renderer_info');
      
      return {
        vendor: debugInfo ? gl.getParameter(debugInfo.UNMASKED_VENDOR_WEBGL) : 'Unknown',
        renderer: debugInfo ? gl.getParameter(debugInfo.UNMASKED_RENDERER_WEBGL) : 'Unknown',
        maxTextureSize: gl.getParameter(gl.MAX_TEXTURE_SIZE),
        maxVertexAttribs: gl.getParameter(gl.MAX_VERTEX_ATTRIBS),
        supportedExtensions: gl.getSupportedExtensions() || []
      };
    } catch (error) {
      console.warn('Failed to get GPU capabilities:', error);
      return undefined;
    }
  }

  // Generar consejos basado en capacidades
  private generateAdvice(capabilities: SystemCapabilities): {
    warnings: string[];
    recommendations: string[];
  } {
    const warnings: string[] = [];
    const recommendations: string[] = [];

    if (!capabilities.webgl2) {
      warnings.push('WebGL 2 not supported - some features may be limited');
      recommendations.push('Update your browser to the latest version');
    }

    if (!capabilities.webassembly) {
      warnings.push('WebAssembly not supported - performance will be reduced');
      recommendations.push('Use a modern browser that supports WebAssembly');
    }

    if (!capabilities.workers) {
      warnings.push('Web Workers not supported - UI may freeze during computations');
      recommendations.push('Enable JavaScript and Web Workers in browser settings');
    }

    if (!capabilities.sharedArrayBuffer) {
      warnings.push('SharedArrayBuffer not available - parallel processing limited');
      recommendations.push('Enable site isolation in browser security settings');
    }

    if (!capabilities.offscreenCanvas) {
      warnings.push('OffscreenCanvas not supported - rendering performance may be limited');
      recommendations.push('Update to a browser that supports OffscreenCanvas');
    }

    // GPU-specific advice
    if (capabilities.gpu) {
      if (capabilities.gpu.maxTextureSize < 4096) {
        warnings.push('Limited GPU texture size - large molecular structures may not render properly');
      }

      if (capabilities.gpu.renderer.includes('Software')) {
        warnings.push('Software rendering detected - GPU acceleration unavailable');
        recommendations.push('Install proper GPU drivers or enable hardware acceleration');
      }
    }

    return { warnings, recommendations };
  }

  // Verificar si el sistema es óptimo
  private isSystemOptimal(capabilities: SystemCapabilities): boolean {
    return capabilities.webgl2 &&
           capabilities.webassembly &&
           capabilities.workers &&
           capabilities.offscreenCanvas &&
           (capabilities.gpu?.maxTextureSize || 0) >= 4096;
  }

  // Obtener recomendaciones de configuración
  getOptimalSettings(): Record<string, any> {
    let currentState: SystemState;
    this.state.subscribe(state => currentState = state)();

    const settings: Record<string, any> = {
      enableGPU: currentState!.capabilities.webgl2,
      enableWorkers: currentState!.capabilities.workers,
      enableWASM: currentState!.capabilities.webassembly,
      maxWorkers: Math.min(currentState!.info.hardwareConcurrency, 8),
      renderQuality: currentState!.isOptimal ? 'high' : 'medium'
    };

    return settings;
  }

  // Test de benchmark básico
  async runBenchmark(): Promise<{
    cpuScore: number;
    gpuScore: number;
    memoryScore: number;
  }> {
    const results = {
      cpuScore: 0,
      gpuScore: 0,
      memoryScore: 0
    };

    try {
      // CPU benchmark - operaciones matemáticas
      const cpuStart = performance.now();
      let sum = 0;
      for (let i = 0; i < 1000000; i++) {
        sum += Math.sqrt(i) * Math.sin(i);
      }
      results.cpuScore = 1000 / (performance.now() - cpuStart);

      // Memory benchmark
      const memStart = performance.now();
      const arrays: number[][] = [];
      for (let i = 0; i < 100; i++) {
        arrays.push(new Array(10000).fill(Math.random()));
      }
      results.memoryScore = 1000 / (performance.now() - memStart);

      // GPU benchmark si está disponible
      if (this.getInitialCapabilities().webgl) {
        results.gpuScore = await this.benchmarkGPU();
      }

    } catch (error) {
      console.warn('Benchmark failed:', error);
    }

    return results;
  }

  // Benchmark GPU
  private async benchmarkGPU(): Promise<number> {
    try {
      const canvas = document.createElement('canvas');
      canvas.width = 512;
      canvas.height = 512;
      
      const gl = canvas.getContext('webgl2') || canvas.getContext('webgl');
      if (!gl) return 0;

      const start = performance.now();
      
      // Render simple triangles para medir performance
      for (let i = 0; i < 1000; i++) {
        gl.clearColor(Math.random(), Math.random(), Math.random(), 1);
        gl.clear(gl.COLOR_BUFFER_BIT);
      }
      
      gl.finish(); // Esperar que termine
      
      return 1000 / (performance.now() - start);
    } catch (error) {
      return 0;
    }
  }
}

// Singleton instance
const _systemStore = new SystemStore();

// Export como store válido de Svelte
export const systemStore = _systemStore.store;

// Export métodos como funciones independientes  
export const checkSystemCapabilities = () => _systemStore.checkCapabilities();
export const getOptimalSettings = () => _systemStore.getOptimalSettings();
export const runSystemBenchmark = () => _systemStore.runBenchmark();

// Type augmentation para window
declare global {
  interface Window {
    __FEATURES__?: SystemCapabilities;
    __ANALYTICS__?: {
      track: (event: string, data: any) => void;
    };
    __PERF_START__?: number;
  }

  interface Navigator {
    deviceMemory?: number;
    connection?: NetworkInformation;
  }

  interface NetworkInformation {
    effectiveType: string;
    downlink: number;
    rtt: number;
  }
}