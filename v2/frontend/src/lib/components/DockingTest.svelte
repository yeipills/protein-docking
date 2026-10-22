<!-- Componente Simple para Probar Molecular Docking -->
<script lang="ts">
  import { apiService } from '$lib/services/ApiService.js';
  
  let selectedReceptor: string = '';
  let selectedLigand: string = '';
  let docking = false;
  let result: any = null;
  let error: string | null = null;
  
  // Lista de archivos PDB disponibles
  const availableFiles = [
    '2f6d.pdb',
    '4jet.pdb', 
    '6izr.pdb',
    '6qtl.pdb',
    '8e5d.pdb'
  ];
  
  async function performDocking() {
    if (!selectedReceptor || !selectedLigand) {
      error = 'Selecciona ambos archivos: receptor y ligando';
      return;
    }
    
    if (selectedReceptor === selectedLigand) {
      error = 'El receptor y ligando deben ser archivos diferentes';
      return;
    }
    
    docking = true;
    error = null;
    result = null;
    
    try {
      console.log(`Starting docking: receptor=${selectedReceptor}, ligand=${selectedLigand}`);
      const response = await apiService.dockProteins(selectedReceptor, selectedLigand);
      console.log('Docking response:', response);
      result = response;
    } catch (err) {
      console.error('Docking error:', err);
      error = err instanceof Error ? err.message : 'Error performing docking';
    } finally {
      docking = false;
    }
  }
</script>

<div class="docking-test-container">
  <h3>⚗️ Molecular Docking Test</h3>
  
  <div class="form-section">
    <div class="file-selection">
      <div class="file-group">
        <label for="receptor-select">Receptor (proteína grande):</label>
        <select 
          id="receptor-select" 
          bind:value={selectedReceptor}
          class="file-select"
        >
          <option value="">-- Selecciona receptor --</option>
          {#each availableFiles as filename}
            <option value={filename}>{filename}</option>
          {/each}
        </select>
      </div>
      
      <div class="file-group">
        <label for="ligand-select">Ligando (molécula pequeña):</label>
        <select 
          id="ligand-select" 
          bind:value={selectedLigand}
          class="file-select"
        >
          <option value="">-- Selecciona ligando --</option>
          {#each availableFiles as filename}
            <option value={filename}>{filename}</option>
          {/each}
        </select>
      </div>
    </div>
    
    <button 
      onclick={performDocking}
      disabled={!selectedReceptor || !selectedLigand || selectedReceptor === selectedLigand || docking}
      class="dock-button"
      class:loading={docking}
    >
      {docking ? '⚗️ Docking en progreso...' : '🚀 Start Docking'}
    </button>
  </div>
  
  {#if error}
    <div class="error-box">
      <h4>❌ Error</h4>
      <p>{error}</p>
    </div>
  {/if}
  
  {#if result}
    <div class="result-box">
      <h4>✅ Docking Results</h4>
      
      {#if result.success}
        <div class="success-content">
          <div class="docking-overview">
            <h5>Experiment Overview</h5>
            <div class="overview-grid">
              <div class="overview-item">
                <span class="label">Receptor:</span>
                <span class="value">{result.receptor_filename}</span>
              </div>
              <div class="overview-item">
                <span class="label">Ligand:</span>
                <span class="value">{result.ligand_filename}</span>
              </div>
              <div class="overview-item">
                <span class="label">Poses Found:</span>
                <span class="value">{result.docking_result.poses_count}</span>
              </div>
            </div>
          </div>
          
          {#if result.docking_result.best_pose}
            <div class="best-pose">
              <h5>🏆 Best Pose</h5>
              <div class="pose-details">
                <div class="energy-info">
                  <p><strong>Energy:</strong> {result.docking_result.best_pose.energy.toFixed(3)} kcal/mol</p>
                  <p><strong>Contacts:</strong> {result.docking_result.best_pose.contacts}</p>
                  <p><strong>Clash Score:</strong> {result.docking_result.best_pose.clash_score.toFixed(3)}</p>
                </div>
                
                <div class="energy-components">
                  <h6>Energy Components</h6>
                  <div class="components-grid">
                    <div class="component-item">
                      <span>van der Waals:</span>
                      <span>{result.docking_result.best_pose.energy_components.van_der_waals.toFixed(3)}</span>
                    </div>
                    <div class="component-item">
                      <span>Electrostatic:</span>
                      <span>{result.docking_result.best_pose.energy_components.electrostatic.toFixed(3)}</span>
                    </div>
                    <div class="component-item">
                      <span>Hydrophobic:</span>
                      <span>{result.docking_result.best_pose.energy_components.hydrophobic.toFixed(3)}</span>
                    </div>
                    <div class="component-item">
                      <span>H-bonds:</span>
                      <span>{result.docking_result.best_pose.energy_components.hydrogen_bonds.toFixed(3)}</span>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          {/if}
          
          <div class="computation-stats">
            <h5>⚡ Performance Statistics</h5>
            <div class="stats-grid">
              <div class="stat-item">
                <span class="stat-label">Computation Time:</span>
                <span class="stat-value">{result.docking_result.computation_stats.computation_time_ms} ms</span>
              </div>
              <div class="stat-item">
                <span class="stat-label">Poses/Second:</span>
                <span class="stat-value">{result.docking_result.computation_stats.poses_per_second.toFixed(1)}</span>
              </div>
              <div class="stat-item">
                <span class="stat-label">Threads Used:</span>
                <span class="stat-value">{result.docking_result.computation_stats.threads_used}</span>
              </div>
              <div class="stat-item">
                <span class="stat-label">SIMD Enabled:</span>
                <span class="stat-value">{result.docking_result.computation_stats.simd_enabled ? '✅' : '❌'}</span>
              </div>
            </div>
          </div>
        </div>
      {:else}
        <div class="error-content">
          <p>{result.error}</p>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .docking-test-container {
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 12px;
    padding: 2rem;
    margin: 1rem 0;
  }

  .docking-test-container h3 {
    margin: 0 0 1.5rem 0;
    color: #1f2937;
    font-size: 1.25rem;
    font-weight: 600;
  }

  .form-section {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    margin-bottom: 1.5rem;
  }

  .file-selection {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
  }

  .file-group {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  label {
    font-weight: 500;
    color: #374151;
    font-size: 0.9rem;
  }

  .file-select {
    padding: 0.75rem;
    border: 1px solid #d1d5db;
    border-radius: 8px;
    font-size: 1rem;
    background: white;
  }

  .file-select:focus {
    outline: none;
    border-color: #3b82f6;
    box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
  }

  .dock-button {
    padding: 1rem 2rem;
    background: linear-gradient(45deg, #059669, #10b981);
    color: white;
    border: none;
    border-radius: 8px;
    font-size: 1rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
    text-align: center;
  }

  .dock-button:hover:not(:disabled) {
    background: linear-gradient(45deg, #047857, #059669);
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(16, 185, 129, 0.3);
  }

  .dock-button:disabled {
    background: #9ca3af;
    cursor: not-allowed;
    transform: none;
    box-shadow: none;
  }

  .dock-button.loading {
    opacity: 0.8;
  }

  .error-box {
    background: #fef2f2;
    border: 1px solid #fecaca;
    border-radius: 8px;
    padding: 1rem;
    margin-top: 1rem;
  }

  .error-box h4 {
    margin: 0 0 0.5rem 0;
    color: #dc2626;
    font-size: 1rem;
  }

  .error-box p {
    margin: 0;
    color: #b91c1c;
  }

  .result-box {
    background: #f0fdf4;
    border: 1px solid #bbf7d0;
    border-radius: 8px;
    padding: 2rem;
    margin-top: 1rem;
  }

  .result-box h4 {
    margin: 0 0 1.5rem 0;
    color: #059669;
    font-size: 1.1rem;
  }

  .result-box h5 {
    margin: 1.5rem 0 0.75rem 0;
    color: #065f46;
    font-size: 1rem;
    font-weight: 600;
    border-bottom: 2px solid #bbf7d0;
    padding-bottom: 0.5rem;
  }

  .result-box h6 {
    margin: 1rem 0 0.5rem 0;
    color: #065f46;
    font-size: 0.9rem;
    font-weight: 600;
  }

  .overview-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1rem;
    margin-top: 0.75rem;
  }

  .overview-item {
    display: flex;
    justify-content: space-between;
    padding: 0.75rem;
    background: rgba(255, 255, 255, 0.7);
    border-radius: 6px;
    border: 1px solid #bbf7d0;
  }

  .label {
    font-weight: 500;
    color: #374151;
  }

  .value {
    font-weight: 600;
    color: #059669;
    font-family: 'Courier New', monospace;
  }

  .best-pose {
    background: rgba(16, 185, 129, 0.1);
    padding: 1.5rem;
    border-radius: 8px;
    margin-top: 1rem;
  }

  .energy-info p {
    margin: 0.25rem 0;
    font-family: 'Courier New', monospace;
    font-size: 0.9rem;
  }

  .components-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 0.5rem;
    margin-top: 0.5rem;
  }

  .component-item {
    display: flex;
    justify-content: space-between;
    padding: 0.5rem;
    background: rgba(255, 255, 255, 0.7);
    border-radius: 4px;
    font-size: 0.8rem;
    font-family: 'Courier New', monospace;
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 1rem;
    margin-top: 0.75rem;
  }

  .stat-item {
    display: flex;
    justify-content: space-between;
    padding: 0.75rem;
    background: rgba(59, 130, 246, 0.1);
    border-radius: 6px;
    border: 1px solid #bfdbfe;
  }

  .stat-label {
    font-weight: 500;
    color: #374151;
  }

  .stat-value {
    font-weight: 600;
    color: #1d4ed8;
    font-family: 'Courier New', monospace;
  }

  .success-content {
    color: #065f46;
  }

  .error-content {
    color: #dc2626;
  }

  /* Responsive */
  @media (max-width: 768px) {
    .docking-test-container {
      padding: 1rem;
    }

    .file-selection {
      grid-template-columns: 1fr;
    }

    .overview-grid,
    .stats-grid {
      grid-template-columns: 1fr;
    }

    .components-grid {
      grid-template-columns: 1fr;
    }
  }
</style>