<!-- Test de Visualización 3D -->
<script lang="ts">
  import SimpleProteinViewer from './SimpleProteinViewer.svelte';
  import { apiService } from '$lib/services/ApiService.js';
  
  let selectedFile = $state('');
  let selectedReceptor = $state('');
  let selectedLigand = $state('');
  
  let viewerKey = $state(0);
  let parsedData = $state<any>(null);
  let dockingResult = $state<any>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  
  // Lista de archivos PDB disponibles
  const availableFiles = [
    { value: '2f6d.pdb', label: '2f6d.pdb - Small protein (4,775 atoms)' },
    { value: '4jet.pdb', label: '4jet.pdb - Medium protein (14,784 atoms)' },
    { value: '6izr.pdb', label: '6izr.pdb - Large protein (84,360 atoms)' },
    { value: '6qtl.pdb', label: '6qtl.pdb - Complex structure' },
    { value: '8e5d.pdb', label: '8e5d.pdb - Another structure' }
  ];
  
  async function testParsing() {
    if (!selectedFile) return;
    
    loading = true;
    error = null;
    parsedData = null;
    
    try {
      const result = await apiService.parsePdbFile(selectedFile);
      if (result.success) {
        parsedData = result.protein;
        viewerKey++; // Force viewer refresh
        console.log('✅ Parsed protein:', parsedData);
      } else {
        throw new Error(result.error);
      }
    } catch (err) {
      error = err instanceof Error ? err.message : 'Parsing failed';
      console.error('❌ Parsing error:', err);
    } finally {
      loading = false;
    }
  }
  
  async function testDocking() {
    if (!selectedReceptor || !selectedLigand) {
      error = 'Select both receptor and ligand';
      return;
    }
    
    if (selectedReceptor === selectedLigand) {
      error = 'Receptor and ligand must be different';
      return;
    }
    
    loading = true;
    error = null;
    dockingResult = null;
    
    try {
      const result = await apiService.dockProteins(selectedReceptor, selectedLigand);
      if (result.success) {
        dockingResult = result.docking_result;
        // Also load the receptor for visualization
        if (selectedReceptor !== selectedFile) {
          selectedFile = selectedReceptor;
          await testParsing();
        }
        viewerKey++;
        console.log('✅ Docking completed:', dockingResult);
      } else {
        throw new Error(result.error);
      }
    } catch (err) {
      error = err instanceof Error ? err.message : 'Docking failed';
      console.error('❌ Docking error:', err);
    } finally {
      loading = false;
    }
  }
  
  function clearResults() {
    selectedFile = '';
    selectedReceptor = '';
    selectedLigand = '';
    parsedData = null;
    dockingResult = null;
    error = null;
    viewerKey++;
  }
</script>

<div class="visualization-test">
  <h2>🔬 3D Visualization Test</h2>
  <p>Test the 3D molecular viewer with real PDB data and docking results</p>
  
  <div class="test-controls">
    <!-- PDB Parsing Test -->
    <div class="control-section">
      <h3>🧬 Single Protein Visualization</h3>
      <div class="control-row">
        <select bind:value={selectedFile} class="file-select">
          <option value="">Select PDB file...</option>
          {#each availableFiles as file}
            <option value={file.value}>{file.label}</option>
          {/each}
        </select>
        
        <button 
          onclick={testParsing}
          disabled={!selectedFile || loading}
          class="test-button primary"
        >
          {loading ? '⏳ Loading...' : '📊 Visualize'}
        </button>
      </div>
      
      {#if parsedData}
        <div class="protein-info">
          <h4>📋 Protein Information</h4>
          <div class="info-grid">
            <span>Title:</span><span>{parsedData.title || parsedData.id}</span>
            <span>Atoms:</span><span>{parsedData.statistics.atom_count.toLocaleString()}</span>
            <span>Chains:</span><span>{parsedData.statistics.chain_count}</span>
            <span>Mass:</span><span>{(parsedData.statistics.mass / 1000).toFixed(1)} kDa</span>
            <span>Center:</span><span>[{parsedData.statistics.center_of_mass.map(c => c.toFixed(1)).join(', ')}]</span>
          </div>
        </div>
      {/if}
    </div>
    
    <!-- Docking Test -->
    <div class="control-section">
      <h3>⚗️ Molecular Docking Visualization</h3>
      <div class="control-row">
        <select bind:value={selectedReceptor} class="file-select">
          <option value="">Select receptor...</option>
          {#each availableFiles as file}
            <option value={file.value}>{file.label}</option>
          {/each}
        </select>
        
        <select bind:value={selectedLigand} class="file-select">
          <option value="">Select ligand...</option>
          {#each availableFiles as file}
            <option value={file.value}>{file.label}</option>
          {/each}
        </select>
        
        <button 
          onclick={testDocking}
          disabled={!selectedReceptor || !selectedLigand || selectedReceptor === selectedLigand || loading}
          class="test-button success"
        >
          {loading ? '⚗️ Docking...' : '🚀 Dock & Visualize'}
        </button>
      </div>
      
      {#if dockingResult}
        <div class="docking-info">
          <h4>🎯 Docking Results</h4>
          <div class="info-grid">
            <span>Poses found:</span><span>{dockingResult.poses_count}</span>
            {#if dockingResult.best_pose}
              <span>Best energy:</span><span class="energy">{dockingResult.best_pose.energy.toFixed(2)} kcal/mol</span>
              <span>Contacts:</span><span>{dockingResult.best_pose.contacts}</span>
            {/if}
            <span>Computation time:</span><span>{dockingResult.computation_stats.computation_time_ms} ms</span>
            <span>SIMD enabled:</span><span class="simd">{dockingResult.computation_stats.simd_enabled ? '✅' : '❌'}</span>
          </div>
        </div>
      {/if}
    </div>
    
    <div class="control-section">
      <button onclick={clearResults} class="test-button secondary">
        🧹 Clear All
      </button>
    </div>
  </div>
  
  {#if error}
    <div class="error-box">
      <h4>❌ Error</h4>
      <p>{error}</p>
    </div>
  {/if}
  
  <!-- 3D Viewer -->
  <div class="viewer-section">
    <h3>🖼️ 3D Molecular Viewer</h3>
    {#key viewerKey}
      <SimpleProteinViewer 
        filename={selectedFile}
        dockingResult={dockingResult}
        showBoth={!!dockingResult}
        autoRotate={true}
      />
    {/key}
    
    <div class="viewer-instructions">
      <h4>🎮 Viewer Controls</h4>
      <ul>
        <li><strong>🎯 Reset View:</strong> Return camera to default position</li>
        <li><strong>🔄 Toggle Rotation:</strong> Start/stop automatic rotation</li>
        <li><strong>🔍 Fit to View:</strong> Adjust camera to fit the molecule</li>
        <li><strong>Mouse:</strong> Click and drag to rotate (if Three.js is available)</li>
        <li><strong>Wheel:</strong> Zoom in and out</li>
      </ul>
    </div>
  </div>
  
  <!-- Test Status -->
  <div class="test-status">
    <h3>📊 Test Status</h3>
    <div class="status-grid">
      <div class="status-item" class:active={selectedFile}>
        <span class="status-dot" class:green={parsedData}></span>
        <span>PDB Parsing: {parsedData ? 'Success' : selectedFile ? 'Loading...' : 'Not tested'}</span>
      </div>
      <div class="status-item" class:active={selectedReceptor && selectedLigand}>
        <span class="status-dot" class:green={dockingResult}></span>
        <span>Molecular Docking: {dockingResult ? 'Success' : (selectedReceptor && selectedLigand) ? 'Ready' : 'Not configured'}</span>
      </div>
      <div class="status-item" class:active={parsedData || dockingResult}>
        <span class="status-dot" class:green={parsedData || dockingResult}></span>
        <span>3D Visualization: {parsedData || dockingResult ? 'Active' : 'No data'}</span>
      </div>
    </div>
  </div>
</div>

<style>
  .visualization-test {
    padding: 2rem;
    max-width: 1200px;
    margin: 0 auto;
  }

  .visualization-test h2 {
    margin: 0 0 0.5rem 0;
    color: #1f2937;
    font-size: 1.75rem;
  }

  .visualization-test > p {
    margin: 0 0 2rem 0;
    color: #6b7280;
    font-size: 1rem;
  }

  .test-controls {
    display: flex;
    flex-direction: column;
    gap: 2rem;
    margin-bottom: 2rem;
  }

  .control-section {
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 12px;
    padding: 1.5rem;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }

  .control-section h3 {
    margin: 0 0 1rem 0;
    color: #374151;
    font-size: 1.25rem;
    font-weight: 600;
  }

  .control-row {
    display: flex;
    gap: 1rem;
    align-items: center;
    flex-wrap: wrap;
  }

  .file-select {
    flex: 1;
    min-width: 200px;
    padding: 0.75rem;
    border: 1px solid #d1d5db;
    border-radius: 8px;
    font-size: 0.9rem;
    background: white;
  }

  .file-select:focus {
    outline: none;
    border-color: #3b82f6;
    box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
  }

  .test-button {
    padding: 0.75rem 1.5rem;
    border: none;
    border-radius: 8px;
    font-size: 0.9rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
    white-space: nowrap;
  }

  .test-button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
    transform: none !important;
  }

  .test-button.primary {
    background: #3b82f6;
    color: white;
  }

  .test-button.primary:hover:not(:disabled) {
    background: #2563eb;
    transform: translateY(-1px);
  }

  .test-button.success {
    background: #10b981;
    color: white;
  }

  .test-button.success:hover:not(:disabled) {
    background: #059669;
    transform: translateY(-1px);
  }

  .test-button.secondary {
    background: #6b7280;
    color: white;
  }

  .test-button.secondary:hover:not(:disabled) {
    background: #4b5563;
    transform: translateY(-1px);
  }

  .protein-info,
  .docking-info {
    margin-top: 1rem;
    padding: 1rem;
    background: #f9fafb;
    border-radius: 8px;
    border: 1px solid #e5e7eb;
  }

  .protein-info h4,
  .docking-info h4 {
    margin: 0 0 0.75rem 0;
    color: #374151;
    font-size: 1rem;
    font-weight: 600;
  }

  .info-grid {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 0.5rem 1rem;
    font-size: 0.875rem;
  }

  .info-grid span:nth-child(odd) {
    font-weight: 500;
    color: #6b7280;
  }

  .info-grid span:nth-child(even) {
    color: #1f2937;
    font-family: 'Courier New', monospace;
  }

  .info-grid .energy {
    color: #dc2626;
    font-weight: 600;
  }

  .info-grid .simd {
    font-weight: 600;
  }

  .error-box {
    background: #fef2f2;
    border: 1px solid #fecaca;
    border-radius: 8px;
    padding: 1rem;
    margin-bottom: 2rem;
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

  .viewer-section {
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 12px;
    padding: 1.5rem;
    margin-bottom: 2rem;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }

  .viewer-section h3 {
    margin: 0 0 1rem 0;
    color: #374151;
    font-size: 1.25rem;
    font-weight: 600;
  }

  .viewer-instructions {
    margin-top: 1rem;
    padding: 1rem;
    background: #f0f9ff;
    border-radius: 8px;
    border: 1px solid #bae6fd;
  }

  .viewer-instructions h4 {
    margin: 0 0 0.75rem 0;
    color: #0369a1;
    font-size: 1rem;
  }

  .viewer-instructions ul {
    margin: 0;
    padding-left: 1.5rem;
    color: #0c4a6e;
  }

  .viewer-instructions li {
    margin-bottom: 0.5rem;
  }

  .test-status {
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 12px;
    padding: 1.5rem;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }

  .test-status h3 {
    margin: 0 0 1rem 0;
    color: #374151;
    font-size: 1.25rem;
    font-weight: 600;
  }

  .status-grid {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .status-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem;
    border-radius: 6px;
    background: #f9fafb;
    transition: all 0.2s;
  }

  .status-item.active {
    background: #f0f9ff;
    border: 1px solid #bae6fd;
  }

  .status-dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: #9ca3af;
    transition: all 0.2s;
  }

  .status-dot.green {
    background: #10b981;
    box-shadow: 0 0 8px rgba(16, 185, 129, 0.3);
  }

  /* Responsive */
  @media (max-width: 768px) {
    .visualization-test {
      padding: 1rem;
    }

    .control-row {
      flex-direction: column;
      align-items: stretch;
    }

    .test-button {
      width: 100%;
    }

    .info-grid {
      grid-template-columns: 1fr;
      gap: 0.25rem;
    }

    .info-grid span:nth-child(odd) {
      font-weight: 600;
    }
  }
</style>