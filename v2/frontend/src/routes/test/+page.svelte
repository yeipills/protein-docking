<!-- Página de Test para Probar Frontend-Backend Communication -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { apiService } from '$lib/services/ApiService.js';
  import PdbUploadTest from '$lib/components/PdbUploadTest.svelte';
  import DockingTest from '$lib/components/DockingTest.svelte';
  
  let backendStatus = $state<'checking' | 'connected' | 'disconnected'>('checking');
  let healthData = $state<any>(null);
  let connectionLatency = $state<number>(0);
  
  onMount(async () => {
    await checkBackendConnection();
  });
  
  async function checkBackendConnection() {
    backendStatus = 'checking';
    
    try {
      const startTime = performance.now();
      
      // Test basic connection
      await apiService.ping();
      
      // Get health information
      healthData = await apiService.getHealth();
      
      const endTime = performance.now();
      connectionLatency = Math.round(endTime - startTime);
      
      backendStatus = 'connected';
      console.log('✅ Backend connection successful', healthData);
      
    } catch (error) {
      backendStatus = 'disconnected';
      console.error('❌ Backend connection failed', error);
    }
  }
</script>

<svelte:head>
  <title>Frontend-Backend Test - Protein Docking v2</title>
  <meta name="description" content="Test page for frontend-backend communication" />
</svelte:head>

<div class="test-page">
  <header class="test-header">
    <h1>🧪 Frontend-Backend Integration Test</h1>
    <p>Test the communication between Svelte frontend and Rust backend</p>
  </header>
  
  <!-- Connection Status -->
  <section class="connection-section">
    <h2>📡 Backend Connection</h2>
    
    <div class="connection-card">
      <div class="connection-status" class:checking={backendStatus === 'checking'} class:connected={backendStatus === 'connected'} class:disconnected={backendStatus === 'disconnected'}>
        <div class="status-indicator">
          {#if backendStatus === 'checking'}
            <div class="spinner"></div>
            <span>Checking connection...</span>
          {:else if backendStatus === 'connected'}
            ✅ <span>Connected to backend</span>
          {:else}
            ❌ <span>Backend disconnected</span>
          {/if}
        </div>
        
        {#if backendStatus === 'connected'}
          <div class="connection-details">
            <p><strong>Latency:</strong> {connectionLatency}ms</p>
            <p><strong>Backend URL:</strong> {apiService.getBaseURL()}</p>
          </div>
        {/if}
        
        <button onclick={checkBackendConnection} class="refresh-button">
          🔄 Refresh
        </button>
      </div>
      
      {#if healthData}
        <div class="health-info">
          <h3>🏥 System Health</h3>
          <div class="health-grid">
            <div class="health-item">
              <span class="health-label">Status:</span>
              <span class="health-value">{healthData.status}</span>
            </div>
            <div class="health-item">
              <span class="health-label">Version:</span>
              <span class="health-value">{healthData.version}</span>
            </div>
            <div class="health-item">
              <span class="health-label">CPU Cores:</span>
              <span class="health-value">{healthData.capabilities.cpu_cores}</span>
            </div>
            <div class="health-item">
              <span class="health-label">SIMD Support:</span>
              <span class="health-value">{healthData.capabilities.simd_support.join(', ') || 'None'}</span>
            </div>
            <div class="health-item">
              <span class="health-label">Memory Allocator:</span>
              <span class="health-value">{healthData.capabilities.memory_allocator}</span>
            </div>
          </div>
        </div>
      {/if}
    </div>
  </section>
  
  <!-- PDB Parsing Test -->
  {#if backendStatus === 'connected'}
    <section class="test-section">
      <h2>📋 PDB Parser Test</h2>
      <p>Test parsing of protein structure files using the Rust backend parser</p>
      <PdbUploadTest />
    </section>
    
    <!-- Molecular Docking Test -->
    <section class="test-section">
      <h2>⚗️ Molecular Docking Test</h2>
      <p>Test the protein docking algorithm with real protein-protein interactions</p>
      <DockingTest />
    </section>
  {:else}
    <section class="test-section disabled">
      <h2>🚫 Tests Unavailable</h2>
      <p>Backend connection required to run tests. Please check your backend server.</p>
      <div class="troubleshooting">
        <h3>Troubleshooting</h3>
        <ul>
          <li>Make sure the backend is running: <code>cargo run</code></li>
          <li>Check if backend is accessible: <code>curl http://localhost:8080/health</code></li>
          <li>Verify docker services are up: <code>docker-compose up -d</code></li>
          <li>Check console for detailed error messages</li>
        </ul>
      </div>
    </section>
  {/if}
  
  <footer class="test-footer">
    <p>
      This test page verifies the integration between the Svelte 5 frontend and Rust Actix Web backend.
      All tests use real PDB files and the ultra-optimized docking algorithms.
    </p>
  </footer>
</div>

<style>
  .test-page {
    max-width: 1200px;
    margin: 0 auto;
    padding: 2rem;
    background: #f8fafc;
    min-height: 100vh;
  }

  .test-header {
    text-align: center;
    margin-bottom: 3rem;
  }

  .test-header h1 {
    margin: 0 0 0.5rem 0;
    font-size: 2.5rem;
    font-weight: 700;
    background: linear-gradient(45deg, #3b82f6, #1d4ed8);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }

  .test-header p {
    margin: 0;
    font-size: 1.125rem;
    color: #6b7280;
  }

  .test-section {
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 12px;
    padding: 2rem;
    margin: 2rem 0;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }

  .test-section h2 {
    margin: 0 0 0.5rem 0;
    font-size: 1.5rem;
    font-weight: 600;
    color: #1f2937;
  }

  .test-section p {
    margin: 0 0 1.5rem 0;
    color: #6b7280;
    font-size: 1rem;
  }

  .test-section.disabled {
    background: #f9fafb;
    border-color: #d1d5db;
    opacity: 0.7;
  }

  .connection-section {
    margin: 2rem 0;
  }

  .connection-section h2 {
    margin: 0 0 1rem 0;
    font-size: 1.5rem;
    font-weight: 600;
    color: #1f2937;
  }

  .connection-card {
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 12px;
    padding: 2rem;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }

  .connection-status {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 1.5rem;
    padding: 1rem;
    border-radius: 8px;
  }

  .connection-status.checking {
    background: #fef3c7;
    border: 1px solid #f59e0b;
  }

  .connection-status.connected {
    background: #d1fae5;
    border: 1px solid #10b981;
  }

  .connection-status.disconnected {
    background: #fee2e2;
    border: 1px solid #ef4444;
  }

  .status-indicator {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    font-weight: 600;
  }

  .spinner {
    width: 20px;
    height: 20px;
    border: 2px solid #f59e0b;
    border-top: 2px solid transparent;
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  .connection-details {
    font-size: 0.875rem;
    color: #059669;
  }

  .connection-details p {
    margin: 0.25rem 0;
    font-family: 'Courier New', monospace;
  }

  .refresh-button {
    padding: 0.5rem 1rem;
    background: #f3f4f6;
    color: #374151;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    font-size: 0.875rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .refresh-button:hover {
    background: #e5e7eb;
    border-color: #9ca3af;
  }

  .health-info {
    border-top: 1px solid #e5e7eb;
    padding-top: 1.5rem;
  }

  .health-info h3 {
    margin: 0 0 1rem 0;
    font-size: 1.125rem;
    font-weight: 600;
    color: #1f2937;
  }

  .health-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1rem;
  }

  .health-item {
    display: flex;
    justify-content: space-between;
    padding: 0.75rem;
    background: #f9fafb;
    border: 1px solid #e5e7eb;
    border-radius: 6px;
  }

  .health-label {
    font-weight: 500;
    color: #6b7280;
  }

  .health-value {
    font-weight: 600;
    color: #1f2937;
    font-family: 'Courier New', monospace;
  }

  .troubleshooting {
    background: #fef3c7;
    border: 1px solid #f59e0b;
    border-radius: 8px;
    padding: 1.5rem;
    margin-top: 1.5rem;
  }

  .troubleshooting h3 {
    margin: 0 0 1rem 0;
    color: #92400e;
    font-size: 1rem;
  }

  .troubleshooting ul {
    margin: 0;
    padding-left: 1.5rem;
    color: #92400e;
  }

  .troubleshooting li {
    margin: 0.5rem 0;
  }

  .troubleshooting code {
    background: rgba(146, 64, 14, 0.1);
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    font-family: 'Courier New', monospace;
    font-size: 0.875rem;
  }

  .test-footer {
    text-align: center;
    margin: 3rem 0 1rem 0;
    padding: 2rem;
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 12px;
    color: #6b7280;
    font-size: 0.9rem;
    line-height: 1.5;
  }

  /* Responsive */
  @media (max-width: 768px) {
    .test-page {
      padding: 1rem;
    }

    .test-section {
      padding: 1.5rem;
    }

    .connection-card {
      padding: 1.5rem;
    }

    .connection-status {
      flex-direction: column;
      gap: 1rem;
      align-items: stretch;
    }

    .health-grid {
      grid-template-columns: 1fr;
    }

    .test-header h1 {
      font-size: 2rem;
    }
  }
</style>