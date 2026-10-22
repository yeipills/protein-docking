<!--
  DockingControls.svelte - Panel de controles ultra-optimizado
  
  Features:
  - Validación en tiempo real
  - Auto-complete inteligente  
  - Presets configurables
  - Detección de capacidades del sistema
-->

<script lang="ts">
  import { api } from '../stores/api.svelte';
  import type { FileMetadata, JobStatus } from '../services/ApiService';
  
  interface Props {
    selectedReceptor?: FileMetadata | null;
    selectedLigand?: FileMetadata | null;
    selectedJob?: JobStatus | null;
    onReceptorChange?: (file: FileMetadata | null) => void;
    onLigandChange?: (file: FileMetadata | null) => void;
    onJobSelect?: (job: JobStatus | null) => void;
    onViewResultsToggle?: (show: boolean) => void;
  }
  
  let { 
    selectedReceptor = null,
    selectedLigand = null,
    selectedJob = null,
    onReceptorChange,
    onLigandChange,
    onJobSelect,
    onViewResultsToggle
  }: Props = $props();
  
  // Estado de visualización
  let showResults = $state(false);
  let viewMode = $state<'files' | 'jobs'>('files');
  let selectedFileId = $state<string | null>(null);
  let selectedJobId = $state<string | null>(null);
  
  // Opciones de representación
  const representationOptions = [
    { value: 'cartoon', label: '🧬 Cartoon', description: 'Simplified ribbon representation' },
    { value: 'spacefill', label: '🔘 Space-fill', description: 'Van der Waals spheres' },
    { value: 'sticks', label: '🥢 Sticks', description: 'Bond representation' },
    { value: 'surface', label: '🌊 Surface', description: 'Molecular surface' }
  ];
  
  // Opciones de calidad
  const qualityOptions = [
    { value: 'low', label: 'Low', description: 'Fast rendering, basic quality' },
    { value: 'medium', label: 'Medium', description: 'Balanced performance' },
    { value: 'high', label: 'High', description: 'Good quality, moderate performance' },
    { value: 'ultra', label: 'Ultra', description: 'Maximum quality, best hardware' }
  ];
  
  // Archivos disponibles del backend
  const receptorFiles = $derived(api.getReceptorFiles());
  const ligandFiles = $derived(api.getLigandFiles());
  const allJobs = $derived(api.state.jobs);
  const completedJobs = $derived(api.state.jobs.filter(job => job.status === 'completed' && job.results && job.results.length > 0));
  
  // Información del sistema desde el backend
  const systemInfo = $derived(() => {
    if (api.state.health?.capabilities) {
      const caps = api.state.health.capabilities;
      return {
        hasGPU: caps.gpu_available || false,
        cpuCores: caps.cpu_cores || 1,
        simdSupport: Array.isArray(caps.simd_support) ? caps.simd_support : [],
        memoryAllocator: caps.memory_allocator || 'System'
      };
    }
    return { hasGPU: false, cpuCores: 1, simdSupport: [], memoryAllocator: 'Unknown' };
  });
  
  function selectFile(fileId: string, type: 'receptor' | 'ligand') {
    const file = api.state.files.find(f => f.file_id === fileId);
    
    if (type === 'receptor') {
      selectedFileId = fileId;
      onReceptorChange?.(file || null);
    } else {
      selectedFileId = fileId;
      onLigandChange?.(file || null);
    }
  }
  
  function selectJob(jobId: string) {
    const job = api.state.jobs.find(j => j.job_id === jobId);
    selectedJobId = jobId;
    onJobSelect?.(job || null);
  }
  
  function toggleResultsView() {
    showResults = !showResults;
    onViewResultsToggle?.(showResults);
  }
  
  function clearSelections() {
    selectedFileId = null;
    selectedJobId = null;
    onReceptorChange?.(null);
    onLigandChange?.(null);
    onJobSelect?.(null);
  }
  
  function getFileDisplayName(file: FileMetadata): string {
    const name = file.filename;
    const atoms = file.protein_info?.total_atoms || 0;
    return `${name} (${atoms} atoms)`;
  }
  
  function getJobDisplayName(job: JobStatus): string {
    const resultCount = job.results?.length || 0;
    const score = job.results?.[0]?.score ? ` - Best: ${job.results[0].score.toFixed(2)}` : '';
    return `${job.job_id}${score} (${resultCount} poses)`;
  }
</script>

<div class="docking-controls">
  <!-- Header -->
  <section class="controls-header">
    <h3>🔬 3D Protein Viewer</h3>
    <p class="header-description">Select files or view docking results</p>
  </section>
  
  <!-- Mode Selector -->
  <section class="mode-selector">
    <div class="mode-tabs">
      <button 
        class="mode-tab"
        class:active={viewMode === 'files'}
        onclick={() => viewMode = 'files'}
      >
        📁 Files
      </button>
      <button 
        class="mode-tab"
        class:active={viewMode === 'jobs'}
        onclick={() => viewMode = 'jobs'}
      >
        📊 Results
      </button>
    </div>
  </section>

  <!-- File Selection Mode -->
  {#if viewMode === 'files'}
    <section class="file-selection">
      <div class="form-group">
        <label for="receptor">Receptor Protein</label>
        <select 
          id="receptor" 
          bind:value={selectedFileId}
          onchange={(e) => selectFile(e.target.value, 'receptor')}
        >
          <option value="">Select receptor...</option>
          {#each receptorFiles as file}
            <option value={file.file_id}>{getFileDisplayName(file)}</option>
          {/each}
        </select>
        
        {#if receptorFiles.length === 0}
          <p class="no-files">No receptor files available. Upload larger protein structures first.</p>
        {/if}
      </div>

      <div class="form-group">
        <label for="ligand">Ligand Molecule</label>
        <select 
          id="ligand" 
          bind:value={selectedFileId}
          onchange={(e) => selectFile(e.target.value, 'ligand')}
        >
          <option value="">Select ligand...</option>
          {#each ligandFiles as file}
            <option value={file.file_id}>{getFileDisplayName(file)}</option>
          {/each}
        </select>
        
        {#if ligandFiles.length === 0}
          <p class="no-files">No ligand files available. Upload smaller molecules first.</p>
        {/if}
      </div>
    </section>
  {:else}
    <!-- Job Results Mode -->
    <section class="job-selection">
      <div class="form-group">
        <label for="job">Completed Jobs with Results</label>
        <select 
          id="job" 
          bind:value={selectedJobId}
          onchange={(e) => selectJob(e.target.value)}
        >
          <option value="">Select completed job...</option>
          {#each completedJobs as job}
            <option value={job.job_id}>{getJobDisplayName(job)}</option>
          {/each}
        </select>
        
        {#if completedJobs.length === 0}
          <p class="no-jobs">No completed jobs with results available. Run some docking simulations first.</p>
        {/if}
      </div>
      
      <div class="form-group">
        <label>
          <input type="checkbox" bind:checked={showResults} onchange={toggleResultsView} />
          Show Docking Results Visualization
        </label>
        <p class="help-text">Display pose results as 3D spheres in the viewer</p>
      </div>
    </section>
  {/if}

  <!-- Viewer Settings -->
  <section class="viewer-settings">
    <h4>🎨 Visualization Settings</h4>
    
    <div class="form-group">
      <label for="representation">Representation Style</label>
      <div class="representation-grid">
        {#each representationOptions as option}
          <button 
            class="representation-btn"
            onclick={() => {}}
            title={option.description}
          >
            {option.label}
          </button>
        {/each}
      </div>
    </div>

    <div class="form-group">
      <label for="quality">Rendering Quality</label>
      <select id="quality">
        {#each qualityOptions as option}
          <option value={option.value}>{option.label} - {option.description}</option>
        {/each}
      </select>
    </div>
  </section>

  <!-- System Information -->
  <section class="system-info">
    <h4>⚡ System Information</h4>
    
    <div class="info-grid">
      <div class="info-item">
        <span class="info-label">CPU Cores:</span>
        <span class="info-value">{systemInfo.cpuCores}</span>
      </div>
      
      <div class="info-item">
        <span class="info-label">GPU Available:</span>
        <span class="info-value" class:available={systemInfo.hasGPU} class:unavailable={!systemInfo.hasGPU}>
          {systemInfo.hasGPU ? '✅ Yes' : '❌ No'}
          {#if systemInfo.hasGPU && api.state.health?.capabilities?.gpu_model}
            <span class="gpu-model">({api.state.health.capabilities.gpu_model})</span>
          {/if}
        </span>
      </div>
      
      {#if systemInfo.hasGPU && api.state.health?.capabilities?.blackwell_features}
        <div class="blackwell-section">
          <h4>🚀💎 RTX Blackwell Features</h4>
          
          <div class="info-item">
            <span class="info-label">Tensor Cores Gen 5:</span>
            <span class="info-value">
              {api.state.health.capabilities.blackwell_features.tensor_cores_enabled ? '✅ Enabled (FP4)' : '❌ Disabled'}
            </span>
          </div>
          
          <div class="info-item">
            <span class="info-label">RT Cores Gen 4:</span>
            <span class="info-value">
              {api.state.health.capabilities.blackwell_features.rt_cores_enabled ? '✅ Enabled' : '❌ Disabled'}
            </span>
          </div>
          
          <div class="info-item">
            <span class="info-label">Memory Bandwidth:</span>
            <span class="info-value">
              {api.state.health.capabilities.blackwell_features.memory_bandwidth_gbps || 'Unknown'} GB/s (GDDR7)
            </span>
          </div>
          
          <div class="info-item">
            <span class="info-label">MIG Support:</span>
            <span class="info-value">
              {api.state.health.capabilities.blackwell_features.mig_support ? '✅ Active' : '❌ Inactive'}
            </span>
          </div>
          
          <div class="info-item">
            <span class="info-label">Power Efficiency:</span>
            <span class="info-value">
              {api.state.health.capabilities.blackwell_features.power_efficiency_score || 'N/A'}/10
            </span>
          </div>
        </div>
      {/if}
      
      <div class="info-item">
        <span class="info-label">SIMD Support:</span>
        <span class="info-value">{systemInfo.simdSupport?.join(', ') || 'None'}</span>
      </div>
      
      <div class="info-item">
        <span class="info-label">Memory Allocator:</span>
        <span class="info-value">{systemInfo.memoryAllocator}</span>
      </div>
    </div>
  </section>

  <!-- Connection Status -->
  {#if api.state.connected}
    <section class="connection-status">
      <div class="status-card connected">
        <span class="status-indicator">🟢</span>
        <div class="status-info">
          <span class="status-label">Backend Connected</span>
          <span class="status-detail">{api.state.files.length} files • {api.state.jobs.length} jobs</span>
        </div>
      </div>
    </section>
  {:else}
    <section class="connection-status">
      <div class="status-card disconnected">
        <span class="status-indicator">🔴</span>
        <div class="status-info">
          <span class="status-label">Backend Disconnected</span>
          <span class="status-detail">Cannot load protein data</span>
        </div>
      </div>
    </section>
  {/if}

  <!-- Action Controls -->
  <section class="action-controls">
    <button 
      type="button" 
      class="btn btn-secondary btn-large"
      onclick={clearSelections}
    >
      🔄 Clear All
    </button>
  </section>
</div>

<style>
  .docking-controls {
    padding: 1.5rem;
    height: 100%;
    overflow-y: auto;
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 12px;
  }

  .controls-header {
    margin-bottom: 1.5rem;
    text-align: center;
  }

  .controls-header h3 {
    margin: 0 0 0.5rem 0;
    font-size: 1.25rem;
    font-weight: 600;
    color: #111827;
  }
  
  .header-description {
    margin: 0;
    font-size: 0.875rem;
    color: #6b7280;
  }

  .mode-selector {
    margin-bottom: 1.5rem;
  }
  
  .mode-tabs {
    display: flex;
    background: #f3f4f6;
    border-radius: 8px;
    padding: 0.25rem;
  }
  
  .mode-tab {
    flex: 1;
    padding: 0.75rem 1rem;
    border: none;
    background: transparent;
    color: #6b7280;
    font-size: 0.875rem;
    font-weight: 500;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.2s;
  }
  
  .mode-tab.active {
    background: white;
    color: #3b82f6;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
  }

  section {
    margin-bottom: 2rem;
    padding-bottom: 1.5rem;
    border-bottom: 1px solid var(--border-color);
  }

  section:last-child {
    border-bottom: none;
    margin-bottom: 0;
  }

  h4 {
    margin: 0 0 1rem 0;
    font-size: 1rem;
    font-weight: 600;
    color: #111827;
  }

  .form-group {
    margin-bottom: 1rem;
  }

  .form-group:last-child {
    margin-bottom: 0;
  }

  label {
    display: block;
    margin-bottom: 0.5rem;
    font-size: 0.875rem;
    font-weight: 500;
    color: #374151;
  }
  
  .no-files, .no-jobs {
    font-size: 0.75rem;
    color: #f59e0b;
    background: #fef3cd;
    padding: 0.5rem;
    border-radius: 4px;
    margin-top: 0.5rem;
  }
  
  .help-text {
    font-size: 0.75rem;
    color: #6b7280;
    margin-top: 0.25rem;
  }

  input, select {
    width: 100%;
    padding: 0.75rem;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    background: white;
    color: #111827;
    font-size: 0.875rem;
    transition: border-color 0.2s ease;
  }

  input:focus, select:focus {
    outline: none;
    border-color: #3b82f6;
    box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
  }

  input.error, select.error {
    border-color: var(--error-color);
  }

  input[type="range"] {
    padding: 0;
    height: 6px;
    background: var(--bg-tertiary);
    border-radius: 3px;
    appearance: none;
    cursor: pointer;
  }

  input[type="range"]::-webkit-slider-thumb {
    appearance: none;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--accent-primary);
    cursor: pointer;
    transition: transform 0.2s ease;
  }

  input[type="range"]::-webkit-slider-thumb:hover {
    transform: scale(1.1);
  }

  .checkbox-group label {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    cursor: pointer;
  }

  .checkbox-group input[type="checkbox"] {
    width: auto;
    margin: 0;
  }

  .representation-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.5rem;
    margin-top: 0.5rem;
  }
  
  .representation-btn {
    padding: 0.75rem;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    background: white;
    color: #374151;
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.2s;
  }
  
  .representation-btn:hover {
    border-color: #3b82f6;
    background: #eff6ff;
  }
  
  .info-grid {
    display: grid;
    gap: 0.75rem;
  }
  
  .info-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem;
    background: #f9fafb;
    border-radius: 4px;
  }
  
  .info-label {
    font-size: 0.75rem;
    color: #6b7280;
    font-weight: 500;
  }
  
  .info-value {
    font-size: 0.75rem;
    font-weight: 600;
    color: #111827;
  }
  
  .info-value.available {
    color: #10b981;
  }
  
  .info-value.unavailable {
    color: #ef4444;
  }
  
  .gpu-model {
    font-size: 0.75rem;
    color: #6b7280;
    margin-left: 0.25rem;
  }
  
  .blackwell-section {
    margin: 1rem 0;
    padding: 1rem;
    background: linear-gradient(135deg, #f0fdf4 0%, #ecfdf5 100%);
    border: 2px solid #10b981;
    border-radius: 8px;
  }
  
  .blackwell-section h4 {
    margin: 0 0 0.75rem 0;
    color: #059669;
    font-size: 1rem;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  
  .blackwell-section .info-item {
    margin: 0.5rem 0;
    border-bottom: 1px solid #d1fae5;
    padding-bottom: 0.5rem;
  }
  
  .blackwell-section .info-item:last-child {
    border-bottom: none;
    padding-bottom: 0;
  }
  
  .blackwell-section .info-label {
    color: #065f46;
    font-weight: 500;
  }
  
  .blackwell-section .info-value {
    color: #047857;
    font-weight: 600;
  }

  .error-message {
    display: block;
    margin-top: 0.25rem;
    font-size: 0.75rem;
    color: var(--error-color);
  }

  .time-estimate {
    border: none !important;
    padding-bottom: 1rem !important;
  }

  .status-card {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 1rem;
    border-radius: 8px;
    border: 1px solid #e5e7eb;
  }
  
  .status-card.connected {
    background: #f0fdf4;
    border-color: #10b981;
  }
  
  .status-card.disconnected {
    background: #fef2f2;
    border-color: #ef4444;
  }
  
  .status-indicator {
    font-size: 1rem;
  }
  
  .status-info {
    flex: 1;
  }
  
  .status-label {
    display: block;
    font-size: 0.875rem;
    font-weight: 600;
    color: #111827;
  }
  
  .status-detail {
    font-size: 0.75rem;
    color: #6b7280;
  }

  .btn {
    padding: 0.75rem 1.5rem;
    border: none;
    border-radius: 0.5rem;
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
    text-align: center;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
  }

  .btn-large {
    padding: 1rem 2rem;
    font-size: 1rem;
    width: 100%;
  }

  .btn-primary {
    background: #3b82f6;
    color: white;
  }

  .btn-primary:hover:not(:disabled) {
    background: #2563eb;
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(59, 130, 246, 0.3);
  }

  .btn-secondary {
    background: #f3f4f6;
    color: #374151;
    border: 1px solid #d1d5db;
  }

  .btn-secondary:hover:not(:disabled) {
    background: #e5e7eb;
    color: #111827;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    transform: none !important;
  }

  /* Scrollbar styling */
  .docking-controls::-webkit-scrollbar {
    width: 6px;
  }

  .docking-controls::-webkit-scrollbar-track {
    background: #f3f4f6;
  }

  .docking-controls::-webkit-scrollbar-thumb {
    background: #d1d5db;
    border-radius: 3px;
  }

  .docking-controls::-webkit-scrollbar-thumb:hover {
    background: #9ca3af;
  }
  
  /* Dark mode support */
  @media (prefers-color-scheme: dark) {
    .docking-controls {
      background: #1f2937;
      border-color: #374151;
    }
    
    .controls-header h3 {
      color: #e5e7eb;
    }
    
    .mode-tabs {
      background: #374151;
    }
    
    .mode-tab.active {
      background: #1f2937;
    }
    
    h4, label {
      color: #e5e7eb;
    }
    
    input, select {
      background: #374151;
      border-color: #4b5563;
      color: #e5e7eb;
    }
    
    .info-item {
      background: #374151;
    }
  }
</style>"