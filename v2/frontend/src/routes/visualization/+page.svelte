<!-- Página de Test de Visualización 3D -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { apiService } from '$lib/services/ApiService.js';
  import VisualizationTest from '$lib/components/VisualizationTest.svelte';
  
  let backendStatus = $state<'checking' | 'connected' | 'disconnected'>('checking');
  let healthData = $state<any>(null);
  
  onMount(async () => {
    await checkBackendConnection();
  });
  
  async function checkBackendConnection() {
    backendStatus = 'checking';
    
    try {
      await apiService.ping();
      healthData = await apiService.getHealth();
      backendStatus = 'connected';
    } catch (error) {
      backendStatus = 'disconnected';
      console.error('Backend connection failed:', error);
    }
  }
</script>

<svelte:head>
  <title>3D Visualization Test - Protein Docking</title>
  <meta name="description" content="Test 3D molecular visualization with real protein docking data" />
</svelte:head>

<div class="visualization-page">
  <header class="page-header">
    <h1>🔬 3D Molecular Visualization</h1>
    <p>Test the 3D viewer with real protein structures and docking results</p>
    
    <!-- Connection Status -->
    <div class="connection-banner" class:connected={backendStatus === 'connected'} class:disconnected={backendStatus === 'disconnected'}>
      {#if backendStatus === 'checking'}
        <span class="status-icon">⏳</span>
        <span>Checking backend connection...</span>
      {:else if backendStatus === 'connected'}
        <span class="status-icon">✅</span>
        <span>Backend connected - 3D visualization ready</span>
        {#if healthData}
          <span class="backend-info">
            | {healthData.capabilities.cpu_cores} cores
            | {healthData.capabilities.simd_support.join(', ')} SIMD
            | {healthData.capabilities.memory_allocator}
          </span>
        {/if}
      {:else}
        <span class="status-icon">❌</span>
        <span>Backend disconnected - visualization unavailable</span>
        <button onclick={checkBackendConnection} class="retry-button">Retry</button>
      {/if}
    </div>
  </header>
  
  {#if backendStatus === 'connected'}
    <!-- Main Test Component -->
    <VisualizationTest />
  {:else}
    <!-- Offline State -->
    <div class="offline-state">
      <div class="offline-icon">🔌</div>
      <h2>Backend Required</h2>
      <p>The 3D molecular visualization requires a connection to the protein docking backend.</p>
      
      <div class="troubleshooting">
        <h3>Troubleshooting</h3>
        <ul>
          <li>Make sure the backend is running: <code>cargo run</code></li>
          <li>Check if services are up: <code>docker compose up -d</code></li>
          <li>Verify backend health: <code>curl http://localhost:8080/health</code></li>
          <li>Check network connectivity and firewall settings</li>
        </ul>
      </div>
      
      <button onclick={checkBackendConnection} class="reconnect-button">
        🔄 Try Reconnecting
      </button>
    </div>
  {/if}
  
  <!-- Technical Information -->
  <footer class="page-footer">
    <h3>🛠️ Technical Details</h3>
    <div class="tech-info">
      <div class="tech-section">
        <h4>3D Rendering</h4>
        <ul>
          <li>Primary: Three.js WebGL renderer</li>
          <li>Fallback: HTML/CSS 3D animations</li>
          <li>Instanced rendering for performance</li>
          <li>Automatic LOD (Level of Detail)</li>
        </ul>
      </div>
      
      <div class="tech-section">
        <h4>Data Sources</h4>
        <ul>
          <li>Real PDB files from Protein Data Bank</li>
          <li>Ultra-optimized Rust PDB parser</li>
          <li>SIMD-accelerated docking algorithms</li>
          <li>Parallel processing with Rayon</li>
        </ul>
      </div>
      
      <div class="tech-section">
        <h4>Features</h4>
        <ul>
          <li>Multiple representation modes</li>
          <li>Interactive camera controls</li>
          <li>Real-time molecular data</li>
          <li>Docking result visualization</li>
        </ul>
      </div>
    </div>
  </footer>
</div>

<style>
  .visualization-page {
    min-height: 100vh;
    background: #f8fafc;
  }

  .page-header {
    background: white;
    border-bottom: 1px solid #e5e7eb;
    padding: 2rem;
    text-align: center;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }

  .page-header h1 {
    margin: 0 0 0.5rem 0;
    font-size: 2.5rem;
    font-weight: 700;
    background: linear-gradient(45deg, #3b82f6, #1d4ed8);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }

  .page-header > p {
    margin: 0 0 1.5rem 0;
    font-size: 1.125rem;
    color: #6b7280;
  }

  .connection-banner {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.75rem;
    padding: 1rem;
    border-radius: 8px;
    font-size: 0.875rem;
    font-weight: 500;
    flex-wrap: wrap;
    background: #fef3c7;
    border: 1px solid #f59e0b;
    color: #92400e;
  }

  .connection-banner.connected {
    background: #d1fae5;
    border-color: #10b981;
    color: #065f46;
  }

  .connection-banner.disconnected {
    background: #fee2e2;
    border-color: #ef4444;
    color: #991b1b;
  }

  .status-icon {
    font-size: 1rem;
  }

  .backend-info {
    font-size: 0.75rem;
    opacity: 0.8;
    font-family: 'Courier New', monospace;
  }

  .retry-button {
    padding: 0.25rem 0.75rem;
    background: rgba(255, 255, 255, 0.8);
    border: 1px solid currentColor;
    border-radius: 4px;
    color: inherit;
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .retry-button:hover {
    background: rgba(255, 255, 255, 1);
  }

  .offline-state {
    padding: 4rem 2rem;
    text-align: center;
    max-width: 600px;
    margin: 0 auto;
  }

  .offline-icon {
    font-size: 4rem;
    margin-bottom: 1.5rem;
  }

  .offline-state h2 {
    margin: 0 0 1rem 0;
    color: #374151;
    font-size: 1.75rem;
    font-weight: 600;
  }

  .offline-state > p {
    margin: 0 0 2rem 0;
    color: #6b7280;
    font-size: 1.125rem;
    line-height: 1.6;
  }

  .troubleshooting {
    background: #fef3c7;
    border: 1px solid #f59e0b;
    border-radius: 8px;
    padding: 1.5rem;
    margin-bottom: 2rem;
    text-align: left;
  }

  .troubleshooting h3 {
    margin: 0 0 1rem 0;
    color: #92400e;
    font-size: 1.125rem;
  }

  .troubleshooting ul {
    margin: 0;
    padding-left: 1.5rem;
    color: #92400e;
  }

  .troubleshooting li {
    margin-bottom: 0.5rem;
  }

  .troubleshooting code {
    background: rgba(146, 64, 14, 0.1);
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    font-family: 'Courier New', monospace;
    font-size: 0.875rem;
  }

  .reconnect-button {
    padding: 0.75rem 2rem;
    background: #3b82f6;
    color: white;
    border: none;
    border-radius: 8px;
    font-size: 1rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }

  .reconnect-button:hover {
    background: #2563eb;
    transform: translateY(-1px);
  }

  .page-footer {
    background: white;
    border-top: 1px solid #e5e7eb;
    padding: 2rem;
    margin-top: 2rem;
  }

  .page-footer h3 {
    margin: 0 0 1.5rem 0;
    text-align: center;
    color: #374151;
    font-size: 1.5rem;
    font-weight: 600;
  }

  .tech-info {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 2rem;
    max-width: 1200px;
    margin: 0 auto;
  }

  .tech-section h4 {
    margin: 0 0 1rem 0;
    color: #1f2937;
    font-size: 1.125rem;
    font-weight: 600;
  }

  .tech-section ul {
    margin: 0;
    padding-left: 1.5rem;
    color: #6b7280;
  }

  .tech-section li {
    margin-bottom: 0.5rem;
    line-height: 1.5;
  }

  /* Responsive */
  @media (max-width: 768px) {
    .page-header {
      padding: 1rem;
    }

    .page-header h1 {
      font-size: 2rem;
    }

    .connection-banner {
      flex-direction: column;
      gap: 0.5rem;
      text-align: center;
    }

    .offline-state {
      padding: 2rem 1rem;
    }

    .tech-info {
      grid-template-columns: 1fr;
      gap: 1.5rem;
    }

    .page-footer {
      padding: 1rem;
    }
  }
</style>