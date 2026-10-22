<!--
  DockingEngine.svelte - Componente principal del motor de docking
  
  Optimizaciones Svelte 5:
  - Runes reactivos para máximo rendimiento
  - Zero-runtime overhead
  - Compilación optimizada
  - Event delegation automático
-->

<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { DockingService } from '$lib/services/DockingService';
  import { performanceStore } from '$lib/stores/performance';
  import { systemStore } from '$lib/stores/system';
  import ProteinViewer from './ProteinViewer.svelte';
  import DockingControls from './DockingControls.svelte';
  import PerformanceMonitor from './PerformanceMonitor.svelte';
  import ProgressPanel from './ProgressPanel.svelte';
  import ResultsPanel from './ResultsPanel.svelte';
  import type { DockingConfiguration, DockingResult, DockingJob } from '$lib/types/docking';
  
  // Svelte 5 runes para reactividad ultra-optimizada
  let dockingService = $state<DockingService>();
  let currentJob = $state<DockingJob | null>(null);
  let results = $state<DockingResult[]>([]);
  let isProcessing = $state(false);
  let progress = $state(0);
  let estimatedTimeRemaining = $state<number | null>(null);
  
  // Configuración reactiva
  let configuration = $state<DockingConfiguration>({
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
  });
  
  // Métricas de rendimiento en tiempo real
  let performanceMetrics = $state({
    computationsPerSecond: 0,
    cpuUtilization: 0,
    memoryUsage: 0,
    gpuUtilization: 0,
    throughput: 0
  });
  
  // Estado de conexión WebSocket
  let connectionStatus = $state<'connected' | 'connecting' | 'disconnected'>('disconnected');
  
  onMount(async () => {
    try {
      // Inicializar servicio de docking
      dockingService = new DockingService();
      await dockingService.initialize();
      
      // Configurar listeners de eventos
      setupEventListeners();
      
      // Conectar a WebSocket para actualizaciones en tiempo real
      await connectWebSocket();
      
      // Inicializar monitoreo de rendimiento
      startPerformanceMonitoring();
      
      connectionStatus = 'connected';
      
    } catch (error) {
      console.error('Error initializing docking engine:', error);
      systemStore.addError('Failed to initialize docking engine');
    }
  });
  
  onDestroy(() => {
    cleanup();
  });
  
  function setupEventListeners() {
    if (!dockingService) return;
    
    // Progreso de trabajo
    dockingService.on('job:progress', (data) => {
      progress = data.progress;
      estimatedTimeRemaining = data.estimatedTimeRemaining;
      currentJob = data.job;
    });
    
    // Trabajo completado
    dockingService.on('job:completed', (result: DockingResult) => {
      results = [...results, result];
      isProcessing = false;
      progress = 100;
      currentJob = null;
      
      // Notificación de éxito
      systemStore.addNotification({
        type: 'success',
        message: `Docking completed with score: ${result.score.toFixed(3)}`,
        duration: 5000
      });
    });
    
    // Error en trabajo
    dockingService.on('job:error', (error) => {
      isProcessing = false;
      currentJob = null;
      systemStore.addError(`Docking failed: ${error.message}`);
    });
    
    // Métricas de rendimiento
    dockingService.on('performance:update', (metrics) => {
      performanceMetrics = { ...metrics };
      performanceStore.updateMetrics(metrics);
    });
  }
  
  async function connectWebSocket() {
    if (!dockingService) return;
    
    connectionStatus = 'connecting';
    
    try {
      await dockingService.connectWebSocket();
      connectionStatus = 'connected';
    } catch (error) {
      connectionStatus = 'disconnected';
      console.error('WebSocket connection failed:', error);
    }
  }
  
  function startPerformanceMonitoring() {
    // Actualizar métricas cada segundo
    setInterval(() => {
      if (dockingService && isProcessing) {
        const metrics = dockingService.getPerformanceMetrics();
        performanceMetrics = { ...metrics };
        performanceStore.updateMetrics(metrics);
      }
    }, 1000);
  }
  
  async function startDocking() {
    if (!dockingService || isProcessing) return;
    
    try {
      // Validar configuración
      const validationErrors = validateConfiguration(configuration);
      if (validationErrors.length > 0) {
        systemStore.addError(`Configuration errors: ${validationErrors.join(', ')}`);
        return;
      }
      
      isProcessing = true;
      progress = 0;
      
      // Iniciar trabajo de docking
      const job = await dockingService.startDocking(configuration);
      currentJob = job;
      
      systemStore.addNotification({
        type: 'info',
        message: 'Docking started - using GPU acceleration',
        duration: 3000
      });
      
    } catch (error) {
      isProcessing = false;
      systemStore.addError(`Failed to start docking: ${error.message}`);
    }
  }
  
  async function stopDocking() {
    if (!dockingService || !currentJob) return;
    
    try {
      await dockingService.stopDocking(currentJob.id);
      isProcessing = false;
      currentJob = null;
      progress = 0;
      
      systemStore.addNotification({
        type: 'warning',
        message: 'Docking stopped by user',
        duration: 3000
      });
      
    } catch (error) {
      systemStore.addError(`Failed to stop docking: ${error.message}`);
    }
  }
  
  function validateConfiguration(config: DockingConfiguration): string[] {
    const errors: string[] = [];
    
    if (!config.receptor_id.trim()) {
      errors.push('Receptor ID is required');
    }
    
    if (!config.ligand_id.trim()) {
      errors.push('Ligand ID is required');
    }
    
    if (config.exhaustiveness < 1 || config.exhaustiveness > 32) {
      errors.push('Exhaustiveness must be between 1 and 32');
    }
    
    if (config.num_modes < 1 || config.num_modes > 20) {
      errors.push('Number of modes must be between 1 and 20');
    }
    
    return errors;
  }
  
  function clearResults() {
    results = [];
    systemStore.addNotification({
      type: 'info',
      message: 'Results cleared',
      duration: 2000
    });
  }
  
  async function exportResults(format: 'json' | 'csv' | 'pdb') {
    if (results.length === 0) {
      systemStore.addError('No results to export');
      return;
    }
    
    try {
      await dockingService?.exportResults(results, format);
      
      systemStore.addNotification({
        type: 'success',
        message: `Results exported as ${format.toUpperCase()}`,
        duration: 3000
      });
      
    } catch (error) {
      systemStore.addError(`Export failed: ${error.message}`);
    }
  }
  
  function cleanup() {
    if (dockingService) {
      dockingService.disconnect();
    }
  }
  
  // Computed properties usando Svelte 5 derivados
  $effect(() => {
    // Auto-save configuration
    if (typeof window !== 'undefined') {
      localStorage.setItem('docking-config', JSON.stringify(configuration));
    }
  });
  
  $effect(() => {
    // Auto-detect optimal settings based on system capabilities
    if (dockingService && connectionStatus === 'connected') {
      const systemCapabilities = dockingService.getSystemCapabilities();
      
      if (systemCapabilities.hasGPU) {
        configuration.use_gpu = true;
        configuration.parallel_processing = true;
      }
      
      if (systemCapabilities.cpuCores >= 8) {
        configuration.exhaustiveness = Math.min(16, systemCapabilities.cpuCores);
      }
    }
  });
</script>

<!-- Template ultra-optimizado -->
<div class="docking-engine" class:processing={isProcessing}>
  <!-- Header con estado de conexión -->
  <header class="engine-header">
    <h1>🧬 Protein Docking Engine v2.0</h1>
    <div class="connection-status" class:connected={connectionStatus === 'connected'}>
      <span class="status-indicator"></span>
      {connectionStatus.charAt(0).toUpperCase() + connectionStatus.slice(1)}
    </div>
  </header>
  
  <!-- Panel principal con layout responsivo -->
  <main class="engine-main">
    <!-- Panel izquierdo: Controles -->
    <aside class="controls-panel">
      <DockingControls 
        bind:configuration={configuration}
        {isProcessing}
        onStart={startDocking}
        onStop={stopDocking}
        onClear={clearResults}
        onExport={exportResults}
      />
      
      <!-- Monitor de rendimiento -->
      <PerformanceMonitor 
        metrics={performanceMetrics}
        {connectionStatus}
      />
    </aside>
    
    <!-- Panel central: Visualizador 3D -->
    <section class="viewer-panel">
      <ProteinViewer 
        {configuration}
        {results}
        {isProcessing}
      />
      
      <!-- Panel de progreso superpuesto -->
      {#if isProcessing && currentJob}
        <ProgressPanel 
          {progress}
          {estimatedTimeRemaining}
          job={currentJob}
          onCancel={stopDocking}
        />
      {/if}
    </section>
    
    <!-- Panel derecho: Resultados -->
    <aside class="results-panel">
      <ResultsPanel 
        {results}
        onExport={exportResults}
        onClear={clearResults}
      />
    </aside>
  </main>
</div>

<style>
  .docking-engine {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg-primary);
    color: var(--text-primary);
    font-family: 'Inter', system-ui, sans-serif;
  }
  
  .engine-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem 2rem;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-color);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  }
  
  .engine-header h1 {
    margin: 0;
    font-size: 1.5rem;
    font-weight: 600;
    background: linear-gradient(135deg, var(--accent-primary), var(--accent-secondary));
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }
  
  .connection-status {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    border-radius: 2rem;
    background: var(--bg-tertiary);
    font-size: 0.875rem;
    font-weight: 500;
    transition: all 0.3s ease;
  }
  
  .connection-status.connected {
    background: rgba(34, 197, 94, 0.1);
    color: rgb(34, 197, 94);
  }
  
  .status-indicator {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--error-color);
    transition: background-color 0.3s ease;
  }
  
  .connection-status.connected .status-indicator {
    background: rgb(34, 197, 94);
    animation: pulse 2s infinite;
  }
  
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }
  
  .engine-main {
    display: grid;
    grid-template-columns: 320px 1fr 320px;
    flex: 1;
    gap: 1px;
    background: var(--border-color);
  }
  
  .controls-panel,
  .results-panel {
    background: var(--bg-secondary);
    overflow-y: auto;
    border-right: 1px solid var(--border-color);
  }
  
  .results-panel {
    border-right: none;
    border-left: 1px solid var(--border-color);
  }
  
  .viewer-panel {
    position: relative;
    background: var(--bg-primary);
    overflow: hidden;
  }
  
  .processing {
    cursor: wait;
  }
  
  /* Responsive design */
  @media (max-width: 1200px) {
    .engine-main {
      grid-template-columns: 280px 1fr 280px;
    }
  }
  
  @media (max-width: 768px) {
    .engine-main {
      grid-template-columns: 1fr;
      grid-template-rows: auto 1fr auto;
    }
    
    .controls-panel,
    .results-panel {
      max-height: 300px;
    }
  }
  
  /* Optimizaciones de rendimiento */
  .viewer-panel {
    contain: layout style paint;
    will-change: transform;
  }
  
  .controls-panel,
  .results-panel {
    contain: layout style;
  }
  
  /* Smooth transitions */
  * {
    transition: opacity 0.2s ease, transform 0.2s ease;
  }
  
  /* Hardware acceleration hints */
  .engine-header,
  .viewer-panel {
    transform: translateZ(0);
  }
</style>"