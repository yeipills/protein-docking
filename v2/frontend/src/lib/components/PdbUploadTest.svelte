<!-- Componente Simple para Probar PDB Parsing -->
<script lang="ts">
  import { apiService } from '$lib/services/ApiService.js';
  
  let selectedFile: string = '';
  let parsing = false;
  let result: any = null;
  let error: string | null = null;
  
  // Lista de archivos PDB disponibles (asumiendo que están en /app/pdb en el backend)
  const availableFiles = [
    '2f6d.pdb',
    '4jet.pdb', 
    '6izr.pdb',
    '6qtl.pdb',
    '8e5d.pdb'
  ];
  
  async function parsePdbFile() {
    if (!selectedFile) {
      error = 'Selecciona un archivo PDB';
      return;
    }
    
    parsing = true;
    error = null;
    result = null;
    
    try {
      console.log(`Parsing PDB file: ${selectedFile}`);
      const response = await apiService.parsePdbFile(selectedFile);
      console.log('Parse response:', response);
      result = response;
    } catch (err) {
      console.error('Parse error:', err);
      error = err instanceof Error ? err.message : 'Error parsing PDB file';
    } finally {
      parsing = false;
    }
  }
</script>

<div class="pdb-test-container">
  <h3>🧬 PDB Parser Test</h3>
  
  <div class="form-section">
    <label for="pdb-select">Seleccionar archivo PDB:</label>
    <select 
      id="pdb-select" 
      bind:value={selectedFile}
      class="file-select"
    >
      <option value="">-- Selecciona un archivo --</option>
      {#each availableFiles as filename}
        <option value={filename}>{filename}</option>
      {/each}
    </select>
    
    <button 
      onclick={parsePdbFile}
      disabled={!selectedFile || parsing}
      class="parse-button"
      class:loading={parsing}
    >
      {parsing ? '⏳ Parsing...' : '🔍 Parse PDB'}
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
      <h4>✅ Parse Results</h4>
      
      {#if result.success}
        <div class="success-content">
          <h5>Protein Information</h5>
          <div class="protein-info">
            <p><strong>ID:</strong> {result.protein.id}</p>
            <p><strong>Title:</strong> {result.protein.title}</p>
            <p><strong>Atoms:</strong> {result.protein.statistics.atom_count}</p>
            <p><strong>Chains:</strong> {result.protein.chains.length}</p>
            <p><strong>Mass:</strong> {result.protein.statistics.mass.toFixed(2)} Da</p>
            <p><strong>Center:</strong> [{result.protein.statistics.center_of_mass.map(c => c.toFixed(2)).join(', ')}]</p>
          </div>
          
          <h5>Bounding Box</h5>
          <div class="bounding-box">
            <p><strong>Min:</strong> [{result.protein.statistics.bounding_box.min.map(c => c.toFixed(2)).join(', ')}]</p>
            <p><strong>Max:</strong> [{result.protein.statistics.bounding_box.max.map(c => c.toFixed(2)).join(', ')}]</p>
            <p><strong>Size:</strong> [{result.protein.statistics.bounding_box.size.map(c => c.toFixed(2)).join(', ')}]</p>
          </div>
          
          <h5>Chains</h5>
          <div class="chains-info">
            {#each result.protein.chains as chain}
              <span class="chain-tag">Chain {chain}</span>
            {/each}
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
  .pdb-test-container {
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 12px;
    padding: 2rem;
    margin: 1rem 0;
  }

  .pdb-test-container h3 {
    margin: 0 0 1.5rem 0;
    color: #1f2937;
    font-size: 1.25rem;
    font-weight: 600;
  }

  .form-section {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    margin-bottom: 1.5rem;
  }

  label {
    font-weight: 500;
    color: #374151;
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

  .parse-button {
    padding: 0.75rem 1.5rem;
    background: #3b82f6;
    color: white;
    border: none;
    border-radius: 8px;
    font-size: 1rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }

  .parse-button:hover:not(:disabled) {
    background: #2563eb;
    transform: translateY(-1px);
  }

  .parse-button:disabled {
    background: #9ca3af;
    cursor: not-allowed;
    transform: none;
  }

  .parse-button.loading {
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
    padding: 1.5rem;
    margin-top: 1rem;
  }

  .result-box h4 {
    margin: 0 0 1rem 0;
    color: #059669;
    font-size: 1rem;
  }

  .result-box h5 {
    margin: 1rem 0 0.5rem 0;
    color: #065f46;
    font-size: 0.9rem;
    font-weight: 600;
  }

  .protein-info p,
  .bounding-box p {
    margin: 0.25rem 0;
    font-family: 'Courier New', monospace;
    font-size: 0.875rem;
    color: #374151;
  }

  .chains-info {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .chain-tag {
    background: #3b82f6;
    color: white;
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 600;
  }

  .success-content {
    color: #065f46;
  }

  .error-content {
    color: #dc2626;
  }

  /* Responsive */
  @media (max-width: 768px) {
    .pdb-test-container {
      padding: 1rem;
    }

    .form-section {
      gap: 0.75rem;
    }
  }
</style>