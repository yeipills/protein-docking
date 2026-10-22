<!-- Docking Job Creator - Configuración y creación de trabajos de docking -->
<script lang="ts">
  import { api } from '../stores/api.svelte';
  import { notificationService } from '../services/NotificationService';
  import DockingConfiguration from './DockingConfiguration.svelte';
  import type { FileMetadata, DockingConfig, CreateJobRequest } from '../services/ApiService';
  import type { DockingParameters } from '../algorithms/DockingAlgorithm.js';

  interface Props {
    onJobCreated?: (jobId: string) => void;
  }

  let { onJobCreated }: Props = $props();

  // Form state
  let receptorFile = $state<FileMetadata | null>(null);
  let ligandFile = $state<FileMetadata | null>(null);
  let config = $state<DockingConfig>(api.getDefaultConfig());
  let dockingParams = $state<DockingParameters | null>(null);
  let isCreating = $state(false);
  let error = $state<string | null>(null);
  let showAdvancedConfig = $state(true);
  let ligandSelectionMode = $state<'auto' | 'manual'>('auto');

  // Available files
  const receptorFiles = $derived(api.getReceptorFiles());
  const ligandFiles = $derived(api.getLigandFiles());

  // Form validation
  const isValid = $derived(() => {
    const validReceptor = receptorFile !== null;
    const validLigand = ligandFile !== null;
    const differentFiles = receptorFile && ligandFile && receptorFile.file_id !== ligandFile.file_id;
    const validParams = dockingParams !== null;
    
    return validReceptor && validLigand && differentFiles && validParams;
  });

  // Estimated computation info
  const estimatedInfo = $derived(() => {
    if (!isValid || !dockingParams) return null;
    
    const receptorAtoms = receptorFile?.protein_info?.total_atoms || 0;
    const ligandAtoms = ligandFile?.protein_info?.total_atoms || 0;
    
    // Rough estimation based on atom count and poses
    const complexity = (receptorAtoms * ligandAtoms) / 10000;
    const basePosesPerSecond = dockingParams.use_gpu_acceleration ? 150 : 50;
    const adjustedRate = basePosesPerSecond / Math.max(1, complexity / 100);
    
    const estimatedSeconds = dockingParams.max_poses / adjustedRate;
    const estimatedMinutes = Math.ceil(estimatedSeconds / 60);
    
    return {
      complexity: complexity > 1000 ? 'High' : complexity > 100 ? 'Medium' : 'Low',
      estimatedTime: estimatedMinutes < 60 ? `~${estimatedMinutes} min` : `~${Math.ceil(estimatedMinutes/60)} hours`,
      posesPerSecond: Math.round(adjustedRate),
      totalPoses: dockingParams.max_poses,
    };
  });

  async function createJob(event?: SubmitEvent) {
    if (event) {
      event.preventDefault();
    }
    
    if (!isValid || isCreating) {
      return;
    }
    
    error = null;
    isCreating = true;
    
    try {
      const request: CreateJobRequest = {
        receptor_file_id: receptorFile!.file_id,
        ligand_file_id: ligandFile!.file_id,
        config: dockingParams ? {
          ...config,
          // Map DockingParameters to DockingConfig
          max_poses: dockingParams.max_poses,
          energy_threshold: dockingParams.energy_threshold,
          rmsd_threshold: dockingParams.rmsd_threshold,
          sampling_strategy: dockingParams.sampling_strategy as any,
          optimization_rounds: dockingParams.optimization_rounds,
          parallel_workers: dockingParams.parallel_workers,
          use_gpu_acceleration: dockingParams.use_gpu_acceleration,
          advanced_scoring: dockingParams.advanced_scoring
        } : config,
        priority: 'normal',
      };
      
      const response = await api.createJob(request);
      
      if (response) {
        // Mostrar notificación de éxito
        notificationService.jobStarted(response.job_id, 'Docking Job');
        notificationService.success(
          'Job Created Successfully',
          `Job ${response.job_id} added to queue (position: ${response.queue_position})`,
          {
            category: 'job',
            metadata: { jobId: response.job_id, queuePosition: response.queue_position }
          }
        );
        
        onJobCreated?.(response.job_id);
        
        // Reset form
        resetForm();
      } else {
        error = 'Failed to create job - No response received from server';
        notificationService.error('Job Creation Failed', 'No response received from server');
      }
      
    } catch (err) {
      let errorMessage = 'Failed to create job';
      if (err instanceof Error) {
        errorMessage = err.message;
      }
      
      // Check for network errors
      if (err && typeof err === 'object' && 'code' in err) {
        if (err.code === 'NETWORK_ERROR' || err.code === 'ERR_NETWORK') {
          errorMessage = 'Network error - Cannot connect to server';
        }
      }
      
      error = errorMessage;
      notificationService.error(
        'Job Creation Failed',
        errorMessage,
        { 
          category: 'job',
          metadata: { error: err }
        }
      );
    } finally {
      isCreating = false;
    }
  }

  function resetForm() {
    receptorFile = null;
    ligandFile = null;
    config = api.getDefaultConfig();
    dockingParams = null;
    error = null;
  }

  function resetToDefaults() {
    config = api.getDefaultConfig();
    dockingParams = null;
  }

  function selectFile(file: FileMetadata, type: 'receptor' | 'ligand') {
    if (type === 'receptor') {
      receptorFile = file;
    } else {
      ligandFile = file;
    }
  }

</script>

<!-- Docking Job Creator Component -->
<div class="job-creator">
  <div class="creator-header">
    <h3>🧬 Create Docking Job</h3>
    <p>Configure and start a new protein-ligand docking simulation</p>
  </div>

  <form onsubmit={createJob} class="job-form">
    <!-- File Selection -->
    <div class="form-section">
      <h4>File Selection</h4>
      
      <!-- Receptor Selection -->
      <div class="file-selector">
        <label for="receptor">Receptor Protein:</label>
        <select 
          id="receptor" 
          bind:value={receptorFile}
          required
        >
          <option value={null}>Select receptor protein...</option>
          {#each receptorFiles as file}
            <option value={file}>
              {file.filename} 
              ({file.protein_info?.total_atoms || 0} atoms)
            </option>
          {/each}
        </select>
        
        {#if receptorFile}
          <div class="file-info">
            <span class="file-details">
              📊 {receptorFile.protein_info?.total_atoms || 0} atoms, 
              {receptorFile.protein_info?.total_residues || 0} residues,
              {receptorFile.protein_info?.chains || 0} chains
            </span>
            {#if receptorFile.protein_info?.pdb_id}
              <span class="pdb-id">PDB: {receptorFile.protein_info.pdb_id}</span>
            {/if}
          </div>
        {/if}
      </div>

      <!-- Ligand Selection -->
      <div class="file-selector">
        <label for="ligand">Ligand Molecule:</label>
        <div class="selection-mode">
          <label class="radio-label">
            <input type="radio" bind:group={ligandSelectionMode} value="auto" />
            🧪 Small molecules (≤100 atoms)
          </label>
          <label class="radio-label">
            <input type="radio" bind:group={ligandSelectionMode} value="manual" />
            🧬 Any molecule (research mode)
          </label>
        </div>
        
        <select 
          id="ligand" 
          bind:value={ligandFile}
          required
        >
          <option value={null}>Select ligand molecule...</option>
          {#each (ligandSelectionMode === 'auto' ? ligandFiles : api.state.files.filter(f => f.file_id !== receptorFile?.file_id)) as file}
            <option value={file}>
              {file.filename} 
              ({file.protein_info?.total_atoms || 0} atoms)
              {file.protein_info?.total_atoms > 100 ? ' - Large molecule' : ''}
            </option>
          {/each}
        </select>
        
        {#if ligandFile}
          <div class="file-info">
            <span class="file-details">
              🧪 {ligandFile.protein_info?.total_atoms || 0} atoms,
              MW: {ligandFile.protein_info?.molecular_weight.toFixed(2) || 0} Da
            </span>
          </div>
        {/if}
      </div>


      {#if receptorFiles.length === 0 || ligandFiles.length === 0}
        <div class="no-files-warning">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
            <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path>
            <line x1="12" y1="9" x2="12" y2="13"></line>
            <line x1="12" y1="17" x2="12.01" y2="17"></line>
          </svg>
          <span>
            {#if receptorFiles.length === 0}
              No receptor proteins available. Upload protein structures with >100 atoms.
            {:else if ligandFiles.length === 0}
              No ligand molecules available. Upload small molecules with ≤100 atoms.
            {:else}
              No files available. Upload both receptor and ligand files.
            {/if}
          </span>
          
          <div class="upload-suggestions">
            <h5>💡 Need files? Try these sources:</h5>
            <ul>
              <li><strong>Small molecules:</strong> <a href="https://pubchem.ncbi.nlm.nih.gov/" target="_blank">PubChem</a> (.sdf format)</li>
              <li><strong>Proteins:</strong> <a href="https://www.rcsb.org/" target="_blank">Protein Data Bank</a> (.pdb format)</li>
              <li><strong>Drug examples:</strong> Aspirin, Caffeine, Ibuprofen from PubChem</li>
              <li><strong>Protein examples:</strong> 1A2G, 1HIV, 1HTM from PDB</li>
            </ul>
          </div>
          
          <div class="scientific-note">
            <h5>🔬 Scientific Note:</h5>
            <p>
              In molecular biology, a <strong>ligand</strong> is any molecule that binds to a receptor, 
              regardless of size. This includes small drugs (~20 atoms), peptides (~100 atoms), 
              other proteins (~1000+ atoms), or DNA/RNA segments.
            </p>
            <p>
              <strong>Auto mode</strong> follows drug discovery conventions (≤100 atoms).
              <strong>Manual mode</strong> allows any molecule as ligand for research applications.
            </p>
          </div>
        </div>
      {/if}
    </div>

    <!-- Advanced Docking Configuration -->
    <DockingConfiguration
      {receptorFile}
      {ligandFile}
      showAdvanced={showAdvancedConfig}
      onconfigChange={(event) => {
        dockingParams = event.detail;
        // Update legacy config for compatibility
        config = {
          ...config,
          max_poses: event.detail.max_poses,
          energy_threshold: event.detail.energy_threshold,
          rmsd_threshold: event.detail.rmsd_threshold,
          use_gpu_acceleration: event.detail.use_gpu_acceleration,
          advanced_scoring: event.detail.advanced_scoring,
          parallel_workers: event.detail.parallel_workers,
          optimization_rounds: event.detail.optimization_rounds
        };
      }}
      onstartDocking={(event) => {
        dockingParams = event.detail;
        createJob();
      }}
      onresetToDefaults={() => {
        resetToDefaults();
      }}
    />


    <!-- Estimation Info -->
    {#if estimatedInfo}
      <div class="estimation-section">
        <h4>Estimated Computation</h4>
        <div class="estimation-grid">
          <div class="estimation-item">
            <span class="label">⚡ Complexity:</span>
            <span class="value complexity-{estimatedInfo.complexity?.toLowerCase() || 'low'}">
              {estimatedInfo.complexity || 'Unknown'}
            </span>
          </div>
          <div class="estimation-item">
            <span class="label">⏱️ Est. Time:</span>
            <span class="value">{estimatedInfo.estimatedTime || 'Unknown'}</span>
          </div>
          <div class="estimation-item">
            <span class="label">🔄 Speed:</span>
            <span class="value">{estimatedInfo.posesPerSecond || 0} poses/sec</span>
          </div>
          <div class="estimation-item">
            <span class="label">🎯 Total Poses:</span>
            <span class="value">{estimatedInfo.totalPoses?.toLocaleString() || '0'}</span>
          </div>
        </div>
      </div>
    {/if}

    <!-- Error Display -->
    {#if error}
      <div class="error-message">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
          <path d="M12 9v4"></path>
          <path d="M12 17h.01"></path>
          <path d="M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20z"></path>
        </svg>
        {error}
      </div>
    {/if}

    <!-- Form Actions -->
    <div class="form-actions">
      <button type="button" onclick={resetForm} class="reset-form-btn">
        Reset Form
      </button>
      
      <button 
        type="submit" 
        class="submit-btn"
        disabled={!isValid || isCreating}
      >
        {#if isCreating}
          <div class="button-spinner"></div>
          Creating Job...
        {:else}
          Create Docking Job
        {/if}
      </button>
    </div>
  </form>
</div>

<style>
  .job-creator {
    width: 100%;
    max-width: 800px;
    margin: 0 auto;
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 12px;
    overflow: hidden;
  }

  .creator-header {
    padding: 1.5rem;
    background: #f9fafb;
    border-bottom: 1px solid #e5e7eb;
  }

  .creator-header h3 {
    margin: 0 0 0.5rem;
    font-size: 1.25rem;
    font-weight: 600;
    color: #111827;
  }

  .creator-header p {
    margin: 0;
    color: #6b7280;
    font-size: 0.875rem;
  }


  .job-form {
    padding: 1.5rem;
  }

  .form-section {
    margin-bottom: 2rem;
  }

  .form-section:last-child {
    margin-bottom: 0;
  }

  .form-section h4 {
    margin: 0 0 1rem;
    font-size: 1.125rem;
    font-weight: 600;
    color: #111827;
  }


  .reset-btn {
    padding: 0.5rem 1rem;
    background: #f3f4f6;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    color: #374151;
    font-size: 0.875rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .reset-btn:hover {
    background: #e5e7eb;
  }

  /* File Selection */
  .file-selector {
    margin-bottom: 1.5rem;
  }

  .file-selector label {
    display: block;
    margin-bottom: 0.5rem;
    font-weight: 500;
    color: #374151;
  }

  .file-selector select {
    width: 100%;
    padding: 0.75rem;
    border: 1px solid #d1d5db;
    border-radius: 8px;
    font-size: 0.875rem;
    background: white;
  }

  .file-selector select:focus {
    outline: none;
    border-color: #3b82f6;
    box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
  }

  .selection-mode {
    display: flex;
    gap: 1rem;
    margin: 0.5rem 0;
    padding: 0.5rem;
    background: #f8fafc;
    border-radius: 4px;
  }

  .radio-label {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.75rem;
    color: #475569;
    cursor: pointer;
  }

  .radio-label input[type="radio"] {
    margin: 0;
    width: auto;
  }

  .file-info {
    margin-top: 0.5rem;
    padding: 0.75rem;
    background: #f9fafb;
    border-radius: 6px;
    font-size: 0.875rem;
  }

  .file-details {
    color: #374151;
  }

  .pdb-id {
    display: block;
    margin-top: 0.25rem;
    color: #3b82f6;
    font-weight: 500;
  }

  .no-files-warning {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 1rem;
    background: #fef3cd;
    border: 1px solid #f59e0b;
    border-radius: 8px;
    color: #92400e;
    font-size: 0.875rem;
    margin-top: 1rem;
  }

  .no-files-warning svg {
    width: 16px;
    height: 16px;
    flex-shrink: 0;
  }


  .upload-suggestions {
    margin-top: 1rem;
    padding: 1rem;
    background: #f1f5f9;
    border-radius: 6px;
  }

  .upload-suggestions h5 {
    margin: 0 0 0.5rem 0;
    color: #1e293b;
  }

  .upload-suggestions ul {
    margin: 0;
    padding-left: 1.5rem;
    color: #475569;
  }

  .upload-suggestions li {
    margin-bottom: 0.25rem;
    font-size: 0.8rem;
  }

  .upload-suggestions a {
    color: #3b82f6;
    text-decoration: none;
    font-weight: 500;
  }

  .upload-suggestions a:hover {
    text-decoration: underline;
  }

  .scientific-note {
    margin-top: 1rem;
    padding: 1rem;
    background: #eff6ff;
    border: 1px solid #3b82f6;
    border-radius: 6px;
  }

  .scientific-note h5 {
    margin: 0 0 0.5rem 0;
    color: #1e40af;
    font-size: 0.875rem;
  }

  .scientific-note p {
    margin: 0 0 0.5rem 0;
    color: #1e40af;
    font-size: 0.8rem;
    line-height: 1.4;
  }

  .scientific-note p:last-child {
    margin-bottom: 0;
  }

  /* Configuration Grid */
  .config-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 1.5rem;
  }


  .input-help {
    font-size: 0.75rem;
    color: #6b7280;
  }


  /* Estimation Section */
  .estimation-section {
    padding: 1rem;
    background: #f0f9ff;
    border: 1px solid #0ea5e9;
    border-radius: 8px;
    margin-bottom: 1.5rem;
  }

  .estimation-section h4 {
    margin: 0 0 1rem;
    color: #0c4a6e;
  }

  .estimation-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 1rem;
  }

  .estimation-item {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .estimation-item .label {
    font-size: 0.75rem;
    color: #0369a1;
    font-weight: 500;
  }

  .estimation-item .value {
    font-size: 0.875rem;
    font-weight: 600;
    color: #0c4a6e;
  }

  .complexity-low { color: #059669; }
  .complexity-medium { color: #d97706; }
  .complexity-high { color: #dc2626; }

  /* Error Message */
  .error-message {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 1rem;
    background: #fef2f2;
    border: 1px solid #f87171;
    border-radius: 8px;
    color: #b91c1c;
    font-size: 0.875rem;
    margin-bottom: 1.5rem;
  }

  .error-message svg {
    width: 16px;
    height: 16px;
    flex-shrink: 0;
  }

  /* Form Actions */
  .form-actions {
    display: flex;
    gap: 1rem;
    justify-content: flex-end;
    padding-top: 1.5rem;
    border-top: 1px solid #e5e7eb;
  }

  .reset-form-btn {
    padding: 0.75rem 1.5rem;
    background: white;
    border: 1px solid #d1d5db;
    border-radius: 8px;
    color: #374151;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }

  .reset-form-btn:hover {
    background: #f9fafb;
    border-color: #9ca3af;
  }

  .submit-btn {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1.5rem;
    background: #3b82f6;
    border: none;
    border-radius: 8px;
    color: white;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }

  .submit-btn:hover:not(:disabled) {
    background: #2563eb;
  }

  .submit-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .button-spinner {
    width: 16px;
    height: 16px;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-top-color: white;
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  /* Responsive design */
  @media (max-width: 768px) {
    .config-grid {
      grid-template-columns: 1fr;
    }

    .job-settings {
      grid-template-columns: 1fr;
    }

    .estimation-grid {
      grid-template-columns: repeat(2, 1fr);
    }

    .form-actions {
      flex-direction: column;
    }
  }

  /* Dark mode support */
  @media (prefers-color-scheme: dark) {
    .job-creator {
      background: #1f2937;
      border-color: #374151;
    }

    .creator-header {
      background: #111827;
      border-bottom-color: #374151;
    }

    .creator-header h3 {
      color: #e5e7eb;
    }

    .form-section h4 {
      color: #e5e7eb;
    }

    .file-selector select {
      background: #374151;
      border-color: #4b5563;
      color: #e5e7eb;
    }

    .file-info {
      background: #374151;
    }

    .estimation-section {
      background: #1e3a8a;
      border-color: #3b82f6;
    }
  }
</style>