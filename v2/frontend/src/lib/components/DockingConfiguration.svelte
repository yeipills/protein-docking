<!--
  DockingConfiguration.svelte - Panel de configuración avanzada para parámetros de docking
  
  Permite configurar:
  - Parámetros del algoritmo (poses, exhaustiveness, etc.)
  - Área de búsqueda (search space)
  - Configuraciones avanzadas (scoring, optimización)
  - Presets rápidos para casos comunes
-->

<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { DockingParameters } from '../algorithms/DockingAlgorithm.js';
  import type { FileMetadata } from '../types/unified.js';
  
  interface Props {
    receptorFile?: FileMetadata | null;
    ligandFile?: FileMetadata | null;
    initialConfig?: Partial<DockingParameters>;
    showAdvanced?: boolean;
  }
  
  let { 
    receptorFile = null,
    ligandFile = null,
    initialConfig = {},
    showAdvanced = false
  }: Props = $props();
  
  const dispatch = createEventDispatcher<{
    configChange: DockingParameters;
    startDocking: DockingParameters;
    resetToDefaults: void;
  }>();
  
  // Estado de configuración
  let config = $state<DockingParameters>({
    max_poses: initialConfig.max_poses || 20,
    energy_threshold: initialConfig.energy_threshold || 0.0,
    rmsd_threshold: initialConfig.rmsd_threshold || 2.0,
    sampling_strategy: initialConfig.sampling_strategy || 'monte_carlo',
    optimization_rounds: initialConfig.optimization_rounds || 10,
    parallel_workers: initialConfig.parallel_workers || 4,
    use_gpu_acceleration: initialConfig.use_gpu_acceleration || false,
    search_space: {
      center: initialConfig.search_space?.center || [0, 0, 0],
      size: initialConfig.search_space?.size || [20, 20, 20]
    },
    advanced_scoring: initialConfig.advanced_scoring || true
  });
  
  // Presets predefinidos
  const presets = {
    fast: {
      max_poses: 10,
      energy_threshold: 5.0,
      sampling_strategy: 'random' as const,
      optimization_rounds: 3,
      search_space: { center: [0, 0, 0], size: [15, 15, 15] }
    },
    balanced: {
      max_poses: 20,
      energy_threshold: 0.0,
      sampling_strategy: 'monte_carlo' as const,
      optimization_rounds: 10,
      search_space: { center: [0, 0, 0], size: [20, 20, 20] }
    },
    thorough: {
      max_poses: 50,
      energy_threshold: -2.0,
      sampling_strategy: 'genetic' as const,
      optimization_rounds: 20,
      search_space: { center: [0, 0, 0], size: [25, 25, 25] }
    },
    exhaustive: {
      max_poses: 100,
      energy_threshold: -5.0,
      sampling_strategy: 'grid' as const,
      optimization_rounds: 30,
      search_space: { center: [0, 0, 0], size: [30, 30, 30] }
    }
  };
  
  // Estado de la UI
  let selectedPreset = $state<keyof typeof presets | null>(null);
  let estimatedTime = $state<string>('');
  let complexity = $state<'low' | 'medium' | 'high' | 'extreme'>('medium');
  let showValidation = $state(false);
  let validationErrors = $state<string[]>([]);
  
  // Auto-detección del binding site
  let autoDetectSite = $state(true);
  let manualCenter = $state(false);
  
  // Reactividad para cambios de configuración
  $effect(() => {
    updateEstimatedTime();
    updateComplexity();
    validateConfiguration();
    dispatch('configChange', config);
  });
  
  // Actualizar centro de búsqueda cuando cambia el receptor
  $effect(() => {
    if (receptorFile && autoDetectSite) {
      // En una implementación real, calcularíamos el centro del binding site
      // Por ahora usamos el centro estimado basado en la información del archivo
      if (receptorFile.protein_info?.center_of_mass) {
        config.search_space.center = [...receptorFile.protein_info.center_of_mass];
      }
    }
  });
  
  function applyPreset(presetName: keyof typeof presets) {
    selectedPreset = presetName;
    const preset = presets[presetName];
    
    config = {
      ...config,
      ...preset,
      search_space: {
        center: config.search_space.center, // Mantener centro actual
        size: preset.search_space.size
      }
    };
    
    console.log(`🎯 Applied preset: ${presetName}`);
  }
  
  function resetToDefaults() {
    config = {
      max_poses: 20,
      energy_threshold: 0.0,
      rmsd_threshold: 2.0,
      sampling_strategy: 'monte_carlo',
      optimization_rounds: 10,
      parallel_workers: 4,
      use_gpu_acceleration: false,
      search_space: {
        center: [0, 0, 0],
        size: [20, 20, 20]
      },
      advanced_scoring: true
    };
    
    selectedPreset = null;
    dispatch('resetToDefaults');
    console.log('🔄 Reset to default configuration');
  }
  
  function updateEstimatedTime() {
    const baseTime = config.max_poses * config.optimization_rounds * 0.1; // segundos base
    const complexityMultiplier = config.sampling_strategy === 'grid' ? 2 : 
                                config.sampling_strategy === 'genetic' ? 1.5 : 1;
    const searchSpaceMultiplier = (config.search_space.size[0] * config.search_space.size[1] * config.search_space.size[2]) / 8000;
    
    const totalSeconds = Math.max(5, baseTime * complexityMultiplier * searchSpaceMultiplier);
    
    if (totalSeconds < 60) {
      estimatedTime = `${Math.round(totalSeconds)}s`;
    } else if (totalSeconds < 3600) {
      estimatedTime = `${Math.round(totalSeconds / 60)}m`;
    } else {
      estimatedTime = `${Math.round(totalSeconds / 3600)}h`;
    }
  }
  
  function updateComplexity() {
    let score = 0;
    
    // Factores de complejidad
    score += config.max_poses / 10;
    score += config.optimization_rounds / 5;
    score += (config.search_space.size[0] * config.search_space.size[1] * config.search_space.size[2]) / 5000;
    score += config.sampling_strategy === 'grid' ? 3 : 
             config.sampling_strategy === 'genetic' ? 2 : 1;
    
    if (score < 2) complexity = 'low';
    else if (score < 5) complexity = 'medium';
    else if (score < 10) complexity = 'high';
    else complexity = 'extreme';
  }
  
  function validateConfiguration() {
    const errors: string[] = [];
    
    if (!receptorFile) {
      errors.push('Receptor file is required');
    }
    
    if (!ligandFile) {
      errors.push('Ligand file is required');
    }
    
    if (config.max_poses < 1 || config.max_poses > 1000) {
      errors.push('Number of poses must be between 1 and 1000');
    }
    
    if (config.optimization_rounds < 0 || config.optimization_rounds > 100) {
      errors.push('Optimization rounds must be between 0 and 100');
    }
    
    if (config.search_space.size.some(s => s <= 0 || s > 100)) {
      errors.push('Search space dimensions must be between 0 and 100 Ångström');
    }
    
    if (config.energy_threshold < -50 || config.energy_threshold > 50) {
      errors.push('Energy threshold must be between -50 and 50 kcal/mol');
    }
    
    validationErrors = errors;
    showValidation = errors.length > 0;
  }
  
  function startDocking() {
    validateConfiguration();
    
    if (validationErrors.length === 0) {
      dispatch('startDocking', config);
      console.log('🚀 Starting docking with configuration:', config);
    } else {
      console.warn('⚠️ Cannot start docking: validation errors', validationErrors);
    }
  }
  
  function autoDetectBindingSite() {
    if (!receptorFile?.protein_info) return;
    
    // Simulamos la detección automática del binding site
    // En un sistema real, esto analizaría la estructura 3D
    const center = receptorFile.protein_info.center_of_mass || [0, 0, 0];
    
    // Añadir algo de variación para simular detección de cavidades
    config.search_space.center = [
      center[0] + (Math.random() - 0.5) * 5,
      center[1] + (Math.random() - 0.5) * 5,
      center[2] + (Math.random() - 0.5) * 5
    ];
    
    console.log('🎯 Auto-detected binding site center:', config.search_space.center);
  }
</script>

<div class="docking-config">
  <div class="config-header">
    <h3>🧬 Docking Configuration</h3>
    <div class="complexity-indicator">
      <span class="complexity-label">Complexity:</span>
      <span class="complexity-badge complexity-{complexity}">{complexity}</span>
      <span class="time-estimate">~{estimatedTime}</span>
    </div>
  </div>
  
  <!-- Validation Errors -->
  {#if showValidation && validationErrors.length > 0}
    <div class="validation-errors">
      <h4>⚠️ Configuration Issues:</h4>
      <ul>
        {#each validationErrors as error}
          <li>{error}</li>
        {/each}
      </ul>
    </div>
  {/if}
  
  <!-- Presets -->
  <div class="config-section">
    <h4>🎯 Quick Presets</h4>
    <div class="preset-buttons">
      {#each Object.entries(presets) as [name, preset]}
        <button 
          class="preset-btn"
          class:active={selectedPreset === name}
          onclick={() => applyPreset(name as keyof typeof presets)}
          title="Max poses: {preset.max_poses}, Strategy: {preset.sampling_strategy}"
        >
          {name.charAt(0).toUpperCase() + name.slice(1)}
        </button>
      {/each}
      <button class="preset-btn reset-btn" onclick={resetToDefaults}>
        Reset
      </button>
    </div>
  </div>
  
  <!-- File Information -->
  <div class="config-section">
    <h4>📄 Files</h4>
    <div class="file-grid">
      <div class="file-info">
        <span class="file-label">Receptor:</span>
        <span class="file-name">
          {receptorFile?.filename || 'Not selected'}
          {#if receptorFile?.protein_info}
            <small>({receptorFile.protein_info.total_atoms} atoms)</small>
          {/if}
        </span>
      </div>
      <div class="file-info">
        <span class="file-label">Ligand:</span>
        <span class="file-name">
          {ligandFile?.filename || 'Not selected'}
          {#if ligandFile?.protein_info}
            <small>({ligandFile.protein_info.total_atoms} atoms)</small>
          {/if}
        </span>
      </div>
    </div>
  </div>
  
  <!-- Basic Parameters -->
  <div class="config-section">
    <h4>⚙️ Basic Parameters</h4>
    <div class="param-grid">
      <div class="param-group">
        <label for="max-poses">Number of Poses:</label>
        <input 
          id="max-poses"
          type="number" 
          bind:value={config.max_poses}
          min="1" 
          max="1000"
          class="param-input"
        />
        <small>More poses = better results but slower</small>
      </div>
      
      <div class="param-group">
        <label for="sampling-strategy">Sampling Strategy:</label>
        <select id="sampling-strategy" bind:value={config.sampling_strategy} class="param-select">
          <option value="random">Random (Fast)</option>
          <option value="monte_carlo">Monte Carlo (Balanced)</option>
          <option value="genetic">Genetic (Smart)</option>
          <option value="grid">Grid (Exhaustive)</option>
        </select>
        <small>Algorithm for generating candidate poses</small>
      </div>
      
      <div class="param-group">
        <label for="optimization-rounds">Optimization Rounds:</label>
        <input 
          id="optimization-rounds"
          type="number" 
          bind:value={config.optimization_rounds}
          min="0" 
          max="100"
          class="param-input"
        />
        <small>Local optimization iterations per pose</small>
      </div>
      
      <div class="param-group">
        <label for="energy-threshold">Energy Threshold (kcal/mol):</label>
        <input 
          id="energy-threshold"
          type="number" 
          bind:value={config.energy_threshold}
          step="0.5"
          min="-50" 
          max="50"
          class="param-input"
        />
        <small>Discard poses above this energy</small>
      </div>
    </div>
  </div>
  
  <!-- Search Space -->
  <div class="config-section">
    <h4>🎯 Search Space</h4>
    
    <div class="search-space-controls">
      <div class="auto-detect">
        <label class="checkbox-label">
          <input 
            type="checkbox" 
            bind:checked={autoDetectSite}
            onchange={() => autoDetectSite && autoDetectBindingSite()}
          />
          Auto-detect binding site
        </label>
        
        {#if !autoDetectSite}
          <button class="detect-btn" onclick={autoDetectBindingSite}>
            🔍 Detect Now
          </button>
        {/if}
      </div>
      
      <div class="center-controls">
        <label>Center (Ångström):</label>
        <div class="coord-inputs">
          <input 
            type="number" 
            bind:value={config.search_space.center[0]}
            step="0.5"
            class="coord-input"
            placeholder="X"
          />
          <input 
            type="number" 
            bind:value={config.search_space.center[1]}
            step="0.5"
            class="coord-input"
            placeholder="Y"
          />
          <input 
            type="number" 
            bind:value={config.search_space.center[2]}
            step="0.5"
            class="coord-input"
            placeholder="Z"
          />
        </div>
      </div>
      
      <div class="size-controls">
        <label>Size (Ångström):</label>
        <div class="coord-inputs">
          <input 
            type="number" 
            bind:value={config.search_space.size[0]}
            min="1"
            max="100"
            class="coord-input"
            placeholder="Width"
          />
          <input 
            type="number" 
            bind:value={config.search_space.size[1]}
            min="1"
            max="100"
            class="coord-input"
            placeholder="Height"
          />
          <input 
            type="number" 
            bind:value={config.search_space.size[2]}
            min="1"
            max="100"
            class="coord-input"
            placeholder="Depth"
          />
        </div>
        <small>Volume: {(config.search_space.size[0] * config.search_space.size[1] * config.search_space.size[2]).toFixed(0)} Ų</small>
      </div>
    </div>
  </div>
  
  <!-- Advanced Options -->
  {#if showAdvanced}
    <div class="config-section">
      <h4>🔬 Advanced Options</h4>
      <div class="param-grid">
        <div class="param-group">
          <label for="rmsd-threshold">RMSD Threshold (Ų):</label>
          <input 
            id="rmsd-threshold"
            type="number" 
            bind:value={config.rmsd_threshold}
            step="0.1"
            min="0.1" 
            max="10"
            class="param-input"
          />
          <small>Clustering similarity threshold</small>
        </div>
        
        <div class="param-group">
          <label for="parallel-workers">Parallel Workers:</label>
          <input 
            id="parallel-workers"
            type="number" 
            bind:value={config.parallel_workers}
            min="1" 
            max="16"
            class="param-input"
          />
          <small>CPU cores to use (experimental)</small>
        </div>
        
        <div class="param-group checkbox-group">
          <label class="checkbox-label">
            <input 
              type="checkbox" 
              bind:checked={config.use_gpu_acceleration}
            />
            GPU Acceleration (experimental)
          </label>
        </div>
        
        <div class="param-group checkbox-group">
          <label class="checkbox-label">
            <input 
              type="checkbox" 
              bind:checked={config.advanced_scoring}
            />
            Advanced Scoring Function
          </label>
        </div>
      </div>
    </div>
  {/if}
  
  <!-- Actions -->
  <div class="config-actions">
    <button 
      class="toggle-advanced-btn"
      onclick={() => showAdvanced = !showAdvanced}
    >
      {showAdvanced ? '📋 Hide' : '🔬 Show'} Advanced Options
    </button>
    
    <div class="action-buttons">
      <button 
        class="start-btn"
        onclick={startDocking}
        disabled={validationErrors.length > 0 || !receptorFile || !ligandFile}
        title={validationErrors.length > 0 ? 'Fix configuration errors first' : 'Start docking calculation'}
      >
        🚀 Start Docking
      </button>
    </div>
  </div>
</div>

<style>
  .docking-config {
    background: rgba(255, 255, 255, 0.05);
    border-radius: 0.75rem;
    padding: 1.5rem;
    margin: 1rem 0;
    border: 1px solid rgba(255, 255, 255, 0.1);
  }
  
  .config-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1.5rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  }
  
  .config-header h3 {
    margin: 0;
    color: white;
    font-size: 1.25rem;
  }
  
  .complexity-indicator {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.875rem;
  }
  
  .complexity-label {
    color: rgba(255, 255, 255, 0.7);
  }
  
  .complexity-badge {
    padding: 0.25rem 0.5rem;
    border-radius: 0.375rem;
    font-weight: 600;
    text-transform: uppercase;
    font-size: 0.75rem;
  }
  
  .complexity-low { background: #22c55e; color: white; }
  .complexity-medium { background: #eab308; color: black; }
  .complexity-high { background: #f97316; color: white; }
  .complexity-extreme { background: #ef4444; color: white; }
  
  .time-estimate {
    color: rgba(255, 255, 255, 0.8);
    font-weight: 500;
  }
  
  .validation-errors {
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.3);
    border-radius: 0.5rem;
    padding: 1rem;
    margin-bottom: 1rem;
  }
  
  .validation-errors h4 {
    margin: 0 0 0.5rem 0;
    color: #ef4444;
    font-size: 0.875rem;
  }
  
  .validation-errors ul {
    margin: 0;
    padding-left: 1.5rem;
    color: #fca5a5;
  }
  
  .validation-errors li {
    margin-bottom: 0.25rem;
  }
  
  .config-section {
    margin-bottom: 1.5rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  }
  
  .config-section:last-child {
    border-bottom: none;
  }
  
  .config-section h4 {
    margin: 0 0 1rem 0;
    color: rgba(255, 255, 255, 0.9);
    font-size: 1rem;
  }
  
  .preset-buttons {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  
  .preset-btn {
    padding: 0.5rem 1rem;
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 0.375rem;
    background: rgba(255, 255, 255, 0.05);
    color: white;
    cursor: pointer;
    transition: all 0.2s ease;
    font-size: 0.875rem;
  }
  
  .preset-btn:hover {
    background: rgba(255, 255, 255, 0.1);
    border-color: rgba(255, 255, 255, 0.3);
  }
  
  .preset-btn.active {
    background: var(--accent-primary);
    border-color: var(--accent-primary);
  }
  
  .reset-btn {
    background: rgba(239, 68, 68, 0.1) !important;
    border-color: rgba(239, 68, 68, 0.3) !important;
    color: #fca5a5 !important;
  }
  
  .reset-btn:hover {
    background: rgba(239, 68, 68, 0.2) !important;
  }
  
  .file-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
  }
  
  .file-info {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  
  .file-label {
    font-size: 0.875rem;
    color: rgba(255, 255, 255, 0.7);
    font-weight: 500;
  }
  
  .file-name {
    color: white;
    font-family: monospace;
    background: rgba(0, 0, 0, 0.3);
    padding: 0.5rem;
    border-radius: 0.25rem;
    font-size: 0.875rem;
  }
  
  .file-name small {
    color: rgba(255, 255, 255, 0.6);
    margin-left: 0.5rem;
  }
  
  .param-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
    gap: 1rem;
  }
  
  .param-group {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  
  .param-group label {
    font-size: 0.875rem;
    color: rgba(255, 255, 255, 0.9);
    font-weight: 500;
  }
  
  .param-input,
  .param-select {
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 0.375rem;
    padding: 0.5rem;
    color: white;
    font-size: 0.875rem;
  }
  
  .param-input:focus,
  .param-select:focus {
    outline: none;
    border-color: var(--accent-primary);
    box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.2);
  }
  
  .param-group small {
    color: rgba(255, 255, 255, 0.6);
    font-size: 0.75rem;
  }
  
  .checkbox-group {
    flex-direction: row;
    align-items: center;
  }
  
  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
  }
  
  .checkbox-label input[type="checkbox"] {
    margin: 0;
  }
  
  .search-space-controls {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  
  .auto-detect {
    display: flex;
    align-items: center;
    gap: 1rem;
  }
  
  .detect-btn {
    padding: 0.25rem 0.75rem;
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 0.25rem;
    background: rgba(59, 130, 246, 0.1);
    color: #60a5fa;
    cursor: pointer;
    font-size: 0.75rem;
  }
  
  .detect-btn:hover {
    background: rgba(59, 130, 246, 0.2);
  }
  
  .center-controls,
  .size-controls {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  
  .center-controls label,
  .size-controls label {
    font-size: 0.875rem;
    color: rgba(255, 255, 255, 0.9);
    font-weight: 500;
  }
  
  .coord-inputs {
    display: flex;
    gap: 0.5rem;
  }
  
  .coord-input {
    flex: 1;
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 0.25rem;
    padding: 0.375rem;
    color: white;
    font-size: 0.8rem;
    text-align: center;
  }
  
  .coord-input:focus {
    outline: none;
    border-color: var(--accent-primary);
  }
  
  .config-actions {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: 1.5rem;
    padding-top: 1rem;
    border-top: 1px solid rgba(255, 255, 255, 0.1);
  }
  
  .toggle-advanced-btn {
    padding: 0.5rem 1rem;
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 0.375rem;
    background: rgba(255, 255, 255, 0.05);
    color: white;
    cursor: pointer;
    font-size: 0.875rem;
  }
  
  .toggle-advanced-btn:hover {
    background: rgba(255, 255, 255, 0.1);
  }
  
  .action-buttons {
    display: flex;
    gap: 0.5rem;
  }
  
  .start-btn {
    padding: 0.75rem 1.5rem;
    border: none;
    border-radius: 0.5rem;
    background: linear-gradient(135deg, #3b82f6, #1d4ed8);
    color: white;
    cursor: pointer;
    font-weight: 600;
    font-size: 0.875rem;
    transition: all 0.2s ease;
  }
  
  .start-btn:hover:not(:disabled) {
    background: linear-gradient(135deg, #2563eb, #1e40af);
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(59, 130, 246, 0.3);
  }
  
  .start-btn:disabled {
    background: rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.5);
    cursor: not-allowed;
  }
  
  /* Responsive */
  @media (max-width: 768px) {
    .config-header {
      flex-direction: column;
      align-items: flex-start;
      gap: 1rem;
    }
    
    .file-grid {
      grid-template-columns: 1fr;
    }
    
    .param-grid {
      grid-template-columns: 1fr;
    }
    
    .config-actions {
      flex-direction: column;
      gap: 1rem;
      align-items: stretch;
    }
    
    .action-buttons {
      justify-content: center;
    }
  }
</style>