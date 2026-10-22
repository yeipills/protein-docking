/*!
 * WASM Wrapper - Interface TypeScript para el módulo WebAssembly
 * 
 * Proporciona una API limpia y type-safe para el módulo WASM de docking
 */

import type { Vector3 as Vector3Type } from '$lib/types/docking';

// Tipos para el módulo WASM
interface WasmModule {
  DockingEngine: new () => WasmDockingEngine;
  Vector3: new (x: number, y: number, z: number) => WasmVector3;
  ScoringParameters: new () => WasmScoringParameters;
  compute_rmsd: (positions1: Float64Array, positions2: Float64Array) => number;
  benchmark_performance: (iterations: number) => number;
  memory: WebAssembly.Memory;
}

interface WasmDockingEngine {
  load_receptor_atoms(
    positions: Float64Array,
    radii: Float64Array,
    charges: Float64Array,
    elements: Uint32Array
  ): void;
  
  load_ligand_atoms(
    positions: Float64Array,
    radii: Float64Array,
    charges: Float64Array,
    elements: Uint32Array
  ): void;
  
  compute_binding_score(): number;
  optimize_conformation(maxIterations: number, learningRate: number): number;
  compute_centroids(): WasmVector3;
  apply_transformation(translation: WasmVector3, rotationMatrix: Float64Array): void;
  get_ligand_positions(): Float64Array;
  free(): void;
}

interface WasmVector3 {
  readonly x: number;
  readonly y: number;
  readonly z: number;
  magnitude(): number;
  normalize(): void;
  distance_to(other: WasmVector3): number;
  dot_product(other: WasmVector3): number;
  free(): void;
}

interface WasmScoringParameters {
  set_weights(vdw: number, elec: number, hbond: number, hydro: number, clash: number): void;
  free(): void;
}

// Clase wrapper principal
export class WasmDockingWrapper {
  private wasmModule: WasmModule | null = null;
  private engine: WasmDockingEngine | null = null;
  private isInitialized = false;
  private loadingPromise: Promise<void> | null = null;

  constructor() {
    // Inicialización lazy
  }

  async initialize(): Promise<void> {
    if (this.isInitialized) return;
    
    if (this.loadingPromise) {
      return this.loadingPromise;
    }

    this.loadingPromise = this.loadWasmModule();
    await this.loadingPromise;
  }

  private async loadWasmModule(): Promise<void> {
    try {
      // Detectar soporte WASM
      if (!this.isWasmSupported()) {
        throw new Error('WebAssembly not supported in this browser');
      }

      console.log('🔄 Loading WASM module...');
      
      // En desarrollo, cargar desde el directorio local
      // En producción, cargar desde CDN o build optimizado
      const wasmPath = import.meta.env.DEV 
        ? '/wasm/docking_wasm.wasm'
        : '/static/wasm/docking_wasm.wasm';

      // Cargar dinámicamente el módulo WASM
      const wasmInit = await import('./docking_wasm.js');
      this.wasmModule = await wasmInit.default(wasmPath);
      
      // Crear instancia del motor
      this.engine = new this.wasmModule.DockingEngine();
      
      this.isInitialized = true;
      console.log('✅ WASM module loaded successfully');
      
      // Ejecutar benchmark de performance
      const benchmarkTime = this.wasmModule.benchmark_performance(100000);
      console.log(`🚀 WASM performance: ${benchmarkTime.toFixed(2)}ms for 100k operations`);
      
    } catch (error) {
      console.error('Failed to load WASM module:', error);
      throw error;
    }
  }

  private isWasmSupported(): boolean {
    return (
      typeof WebAssembly === 'object' &&
      typeof WebAssembly.instantiate === 'function' &&
      typeof WebAssembly.Module === 'function'
    );
  }

  // API Methods

  async loadReceptorAtoms(atoms: AtomData[]): Promise<void> {
    await this.initialize();
    if (!this.engine) throw new Error('WASM engine not initialized');

    const { positions, radii, charges, elements } = this.prepareAtomData(atoms);
    
    try {
      this.engine.load_receptor_atoms(positions, radii, charges, elements);
    } catch (error) {
      console.error('Failed to load receptor atoms:', error);
      throw error;
    }
  }

  async loadLigandAtoms(atoms: AtomData[]): Promise<void> {
    await this.initialize();
    if (!this.engine) throw new Error('WASM engine not initialized');

    const { positions, radii, charges, elements } = this.prepareAtomData(atoms);
    
    try {
      this.engine.load_ligand_atoms(positions, radii, charges, elements);
    } catch (error) {
      console.error('Failed to load ligand atoms:', error);
      throw error;
    }
  }

  async computeBindingScore(): Promise<number> {
    await this.initialize();
    if (!this.engine) throw new Error('WASM engine not initialized');

    const startTime = performance.now();
    const score = this.engine.compute_binding_score();
    const endTime = performance.now();
    
    console.log(`⚡ Binding score computed in ${(endTime - startTime).toFixed(2)}ms: ${score.toFixed(3)}`);
    return score;
  }

  async optimizeConformation(maxIterations: number = 100, learningRate: number = 0.01): Promise<number> {
    await this.initialize();
    if (!this.engine) throw new Error('WASM engine not initialized');

    const startTime = performance.now();
    const finalScore = this.engine.optimize_conformation(maxIterations, learningRate);
    const endTime = performance.now();
    
    console.log(`🎯 Conformation optimized in ${(endTime - startTime).toFixed(2)}ms: ${finalScore.toFixed(3)}`);
    return finalScore;
  }

  async computeCentroids(): Promise<Vector3Type> {
    await this.initialize();
    if (!this.engine) throw new Error('WASM engine not initialized');

    const wasmVector = this.engine.compute_centroids();
    const result = {
      x: wasmVector.x,
      y: wasmVector.y,
      z: wasmVector.z
    };
    
    // Limpiar memoria WASM
    wasmVector.free();
    
    return result;
  }

  async applyTransformation(translation: Vector3Type, rotationMatrix: number[]): Promise<void> {
    await this.initialize();
    if (!this.engine || !this.wasmModule) throw new Error('WASM engine not initialized');

    if (rotationMatrix.length !== 9) {
      throw new Error('Rotation matrix must have 9 elements');
    }

    const wasmTranslation = new this.wasmModule.Vector3(translation.x, translation.y, translation.z);
    const rotationArray = new Float64Array(rotationMatrix);
    
    try {
      this.engine.apply_transformation(wasmTranslation, rotationArray);
    } finally {
      wasmTranslation.free();
    }
  }

  async getLigandPositions(): Promise<Float64Array> {
    await this.initialize();
    if (!this.engine) throw new Error('WASM engine not initialized');

    return this.engine.get_ligand_positions();
  }

  async computeRMSD(positions1: Float64Array, positions2: Float64Array): Promise<number> {
    await this.initialize();
    if (!this.wasmModule) throw new Error('WASM module not initialized');

    return this.wasmModule.compute_rmsd(positions1, positions2);
  }

  // Utility methods

  private prepareAtomData(atoms: AtomData[]): {
    positions: Float64Array;
    radii: Float64Array;
    charges: Float64Array;
    elements: Uint32Array;
  } {
    const positions = new Float64Array(atoms.length * 3);
    const radii = new Float64Array(atoms.length);
    const charges = new Float64Array(atoms.length);
    const elements = new Uint32Array(atoms.length);

    for (let i = 0; i < atoms.length; i++) {
      const atom = atoms[i];
      
      positions[i * 3] = atom.position.x;
      positions[i * 3 + 1] = atom.position.y;
      positions[i * 3 + 2] = atom.position.z;
      
      radii[i] = atom.radius;
      charges[i] = atom.charge || 0;
      elements[i] = this.getElementNumber(atom.element);
    }

    return { positions, radii, charges, elements };
  }

  private getElementNumber(element: string): number {
    // Mapeo simple de elementos a números
    const elementMap: Record<string, number> = {
      'H': 1, 'C': 6, 'N': 7, 'O': 8, 'F': 9,
      'P': 15, 'S': 16, 'Cl': 17, 'Fe': 26, 'Zn': 30
    };
    
    return elementMap[element] || 6; // Default a carbono
  }

  getMemoryUsage(): number {
    if (!this.wasmModule?.memory) return 0;
    
    const pages = this.wasmModule.memory.buffer.byteLength / (64 * 1024); // 64KB per page
    return pages * 64; // KB
  }

  isInitialized(): boolean {
    return this.isInitialized;
  }

  dispose(): void {
    if (this.engine) {
      this.engine.free();
      this.engine = null;
    }
    
    this.wasmModule = null;
    this.isInitialized = false;
    this.loadingPromise = null;
  }
}

// Tipos auxiliares
export interface AtomData {
  position: Vector3Type;
  radius: number;
  charge?: number;
  element: string;
  residueId?: number;
}

// Singleton instance
export const wasmDocking = new WasmDockingWrapper();

// Utility functions
export async function benchmarkWasmPerformance(): Promise<{
  wasmTime: number;
  jsTime: number;
  speedup: number;
}> {
  const iterations = 1000000;
  
  // Benchmark JavaScript
  const jsStart = performance.now();
  let jsSum = 0;
  for (let i = 0; i < iterations; i++) {
    jsSum += Math.sin(i) * Math.cos(i);
  }
  const jsTime = performance.now() - jsStart;
  
  // Benchmark WASM
  await wasmDocking.initialize();
  const wasmTime = wasmDocking.wasmModule?.benchmark_performance(iterations) || 0;
  
  const speedup = jsTime / wasmTime;
  
  return {
    wasmTime,
    jsTime,
    speedup
  };
}

export function getWasmCapabilities(): {
  supported: boolean;
  simdSupported: boolean;
  threadsSupported: boolean;
  memorySize: number;
} {
  const supported = typeof WebAssembly !== 'undefined';
  let simdSupported = false;
  let threadsSupported = false;
  let memorySize = 0;

  if (supported) {
    // Check SIMD support
    try {
      simdSupported = WebAssembly.validate(new Uint8Array([
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00
      ]));
    } catch {
      simdSupported = false;
    }

    // Check threads support
    threadsSupported = typeof SharedArrayBuffer !== 'undefined';
    
    // Get available memory
    if ('memory' in performance && 'deviceMemory' in (performance as any)) {
      memorySize = (performance as any).deviceMemory * 1024; // MB
    }
  }

  return {
    supported,
    simdSupported,
    threadsSupported,
    memorySize
  };
}