/*!
 * Core App Initialization - Configuración y servicios principales
 */

import { systemStore } from '$stores/system';
import { performanceStore } from '$stores/performance';
import { themeStore } from '$stores/theme';

export interface AppConfig {
  apiBaseUrl: string;
  wsUrl: string;
  enableProfiling: boolean;
  enableAnalytics: boolean;
  maxFileSize: number;
  supportedFormats: string[];
}

export interface InitializationResult {
  success: boolean;
  capabilities: any;
  warnings: string[];
  config: AppConfig;
}

// Configuración por defecto
const DEFAULT_CONFIG: AppConfig = {
  apiBaseUrl: 'http://localhost:8080/api/v1',
  wsUrl: 'ws://localhost:8080/ws',
  enableProfiling: import.meta.env.DEV,
  enableAnalytics: false,
  maxFileSize: 100 * 1024 * 1024, // 100MB
  supportedFormats: ['pdb', 'stl', 'vert', 'face']
};

let appConfig: AppConfig = DEFAULT_CONFIG;
let initialized = false;

/**
 * Inicializar la aplicación principal
 */
export async function initializeApp(): Promise<InitializationResult> {
  if (initialized) {
    return {
      success: true,
      capabilities: {},
      warnings: [],
      config: appConfig
    };
  }

  console.log('🚀 Initializing Protein Docking v2...');

  try {
    // 1. Cargar configuración
    appConfig = await loadConfiguration();
    console.log('📋 Configuration loaded:', appConfig);

    // 2. Verificar capacidades del sistema
    await systemStore.checkCapabilities();
    console.log('🔧 System capabilities checked');

    // 3. Inicializar servicios de performance
    if (appConfig.enableProfiling) {
      performanceStore.startMonitoring();
      console.log('📊 Performance monitoring started');
    }

    // 4. Verificar conectividad API
    const apiHealth = await checkApiHealth();
    console.log('🌐 API health check:', apiHealth ? '✅' : '❌');

    // 5. Inicializar WebAssembly si está disponible
    if (window.__FEATURES__?.webassembly) {
      await initializeWasm();
      console.log('🦀 WebAssembly initialized');
    }

    // 6. Inicializar Web Workers
    if (window.__FEATURES__?.workers) {
      await initializeWorkers();
      console.log('👷 Web Workers initialized');
    }

    // 7. Configurar Service Worker para caching
    if ('serviceWorker' in navigator) {
      await registerServiceWorker();
      console.log('⚡ Service Worker registered');
    }

    initialized = true;

    const result: InitializationResult = {
      success: true,
      capabilities: window.__FEATURES__ || {},
      warnings: [],
      config: appConfig
    };

    console.log('✅ App initialization completed successfully');
    return result;

  } catch (error) {
    console.error('❌ App initialization failed:', error);
    
    return {
      success: false,
      capabilities: {},
      warnings: [error instanceof Error ? error.message : 'Unknown initialization error'],
      config: appConfig
    };
  }
}

/**
 * Cargar configuración de la aplicación
 */
async function loadConfiguration(): Promise<AppConfig> {
  // En desarrollo, usar configuración por defecto
  if (import.meta.env.DEV) {
    return DEFAULT_CONFIG;
  }

  try {
    // En producción, cargar desde API o variables de entorno
    const config = {
      ...DEFAULT_CONFIG,
      apiBaseUrl: import.meta.env.VITE_API_URL || DEFAULT_CONFIG.apiBaseUrl,
      wsUrl: import.meta.env.VITE_WS_URL || DEFAULT_CONFIG.wsUrl,
      enableAnalytics: import.meta.env.VITE_ENABLE_ANALYTICS === 'true'
    };

    return config;
  } catch (error) {
    console.warn('Failed to load remote config, using defaults:', error);
    return DEFAULT_CONFIG;
  }
}

/**
 * Verificar salud de la API
 */
async function checkApiHealth(): Promise<boolean> {
  try {
    const response = await fetch(`${appConfig.apiBaseUrl.replace('/api/v1', '')}/health`, {
      method: 'GET',
      headers: {
        'Accept': 'application/json'
      },
      // Timeout corto para health check
      signal: AbortSignal.timeout(5000)
    });

    return response.ok;
  } catch (error) {
    console.warn('API health check failed:', error);
    return false;
  }
}

/**
 * Inicializar WebAssembly
 */
async function initializeWasm(): Promise<void> {
  try {
    // Intentar cargar módulos WASM para algoritmos críticos
    if (typeof window !== 'undefined') {
      // El módulo WASM se cargará bajo demanda cuando se necesite
      console.log('WASM runtime ready');
    }
  } catch (error) {
    console.warn('WASM initialization failed:', error);
    throw new Error('WebAssembly initialization failed');
  }
}

/**
 * Inicializar Web Workers
 */
async function initializeWorkers(): Promise<void> {
  try {
    // Pre-verificar que los workers están disponibles
    const testWorker = new Worker(
      new URL('../workers/test-worker.ts', import.meta.url),
      { type: 'module' }
    );
    
    // Test básico
    await new Promise<void>((resolve, reject) => {
      const timeout = setTimeout(() => {
        reject(new Error('Worker test timeout'));
      }, 2000);

      testWorker.onmessage = (e) => {
        if (e.data === 'test-ok') {
          clearTimeout(timeout);
          testWorker.terminate();
          resolve();
        }
      };

      testWorker.onerror = (error) => {
        clearTimeout(timeout);
        testWorker.terminate();
        reject(error);
      };

      testWorker.postMessage('test');
    });

    console.log('Web Workers test passed');
  } catch (error) {
    console.warn('Web Workers initialization failed:', error);
    // No fatal, la app puede funcionar sin workers
  }
}

/**
 * Registrar Service Worker
 */
async function registerServiceWorker(): Promise<void> {
  try {
    const registration = await navigator.serviceWorker.register('/service-worker.js');
    
    registration.addEventListener('updatefound', () => {
      console.log('New service worker version available');
    });

    console.log('Service Worker registered:', registration);
  } catch (error) {
    console.warn('Service Worker registration failed:', error);
    // No fatal, la app puede funcionar sin SW
  }
}

/**
 * Obtener configuración actual
 */
export function getAppConfig(): AppConfig {
  return appConfig;
}

/**
 * Verificar si la app está inicializada
 */
export function isAppInitialized(): boolean {
  return initialized;
}

/**
 * Cleanup recursos al cerrar la app
 */
export function cleanupApp(): void {
  performanceStore.stopMonitoring();
  
  // Cleanup workers, conexiones, etc.
  console.log('🧹 App cleanup completed');
}