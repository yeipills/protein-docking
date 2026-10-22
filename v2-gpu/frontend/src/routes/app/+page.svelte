<!-- Ultra-Performance Protein Docking Application - Svelte 5 con Runes -->
<script lang="ts">
  import { onMount, tick } from 'svelte';
  import api from '$lib/stores/api.svelte';
  
  // Nuevos componentes integrados
  import FileUpload from '$lib/components/FileUpload.svelte';
  import FileManager from '$lib/components/FileManager.svelte';
  import DockingJobCreator from '$lib/components/DockingJobCreator.svelte';
  import JobMonitor from '$lib/components/JobMonitor.svelte';
  
  // Componentes existentes (si están disponibles)
  import ProteinViewer from '$lib/components/ProteinViewer.svelte';
  import DockingControls from '$lib/components/DockingControls.svelte';
  
  // Nuevos componentes para visualización de poses
  import PoseViewer from '$lib/components/PoseViewer.svelte';
  import PoseVisualizer from '$lib/components/PoseVisualizer.svelte';
  import ReportViewer from '$lib/components/ReportViewer.svelte';
  import DataExporter from '$lib/components/DataExporter.svelte';
  
  // Tipos
  import type { FileMetadata, JobStatus } from '$lib/services/ApiService';
  
  
  // Sistema de logging
  import { log } from '$lib/utils/Logger.js';
  
  // Svelte 5 runes para estado reactivo ultra-optimizado
  let isLoading = $state(true);
  let error = $state<string | null>(null);
  let activeTab = $state<'upload' | 'files' | 'docking' | 'jobs' | 'viewer' | 'poses' | 'reports' | 'export' | 'logs'>('upload');
  
  // Estado del visor 3D con runes optimizadas
  let selectedReceptor = $state<FileMetadata | null>(null);
  let selectedLigand = $state<FileMetadata | null>(null);
  let selectedJob = $state<JobStatus | null>(null);
  let showResults = $state(false);
  
  // Estado para visualización de poses
  let selectedDockingResult = $state<any>(null);
  let selectedPose = $state<any>(null);
  let receptorStructure = $state<any>(null);
  let ligandStructure = $state<any>(null);
  
  // Estado de UI con actualizaciones reactivas
  let lastUpdateTimestamp = $state(Date.now());
  
  // Derived values locales para computación reactiva optimizada
  const fileStats = $derived({
    total: api.state.files.length,
    receptors: api.getReceptorFiles().length,
    ligands: api.getLigandFiles().length,
    ratio: api.state.files.length > 0 ? (api.getLigandFiles().length / api.state.files.length) : 0
  });
  
  const jobStats = $derived({
    active: api.activeJobsCount,
    completed: api.state.jobs?.filter(j => j.status === 'completed').length || 0,
    failed: api.state.jobs?.filter(j => j.status === 'failed').length || 0,
    total: api.state.jobs?.length || 0
  });
  
  const dockingAnalytics = $derived({
    hasPoses: selectedDockingResult?.poses?.length > 0,
    bestEnergy: selectedDockingResult?.best_pose?.energy || null,
    avgEnergy: selectedDockingResult?.poses ? 
      selectedDockingResult.poses.reduce((sum: number, p: any) => sum + p.energy, 0) / selectedDockingResult.poses.length : null,
    poseCount: selectedDockingResult?.poses?.length || 0,
    computationEfficiency: selectedDockingResult ? 
      (selectedDockingResult.poses?.length || 0) / Math.max(selectedDockingResult.computation_time_ms || 1, 1) * 1000 : 0
  });
  
  // Acceso a derived values del store API (usando getters)
  const systemHealth = $derived(api.systemHealth);
  const performanceMetrics = $derived(api.performanceInsights);
  
  // Eliminados todos los $effect para resolver el loop infinito
  // Se moverán a onMount o eventos específicos
  
  onMount(async () => {
    log.info('Aplicación iniciando...', 'App');
    
    try {
      log.startPerf('app_initialization');
      
      // Initialize API connection with timeout
      const initPromise = api.initialize();
      const timeoutPromise = new Promise((_, reject) => 
        setTimeout(() => reject(new Error('API initialization timeout')), 10000)
      );
      
      await Promise.race([initPromise, timeoutPromise]);
      log.endPerf('app_initialization', 'App');
      log.info('API conectado exitosamente', 'App');
      isLoading = false;
    } catch (err) {
      log.error('API initialization failed', 'App', err instanceof Error ? err : undefined);
      error = err instanceof Error ? err.message : 'Backend connection failed';
      isLoading = false;
      
      // Try to continue anyway - some features might still work
      setTimeout(() => {
        if (error) {
          log.warn('Continuando con funcionalidad limitada', 'App');
          error = null;
        }
      }, 3000);
    }
  });
  
  function handleTabChange(newTab: typeof activeTab) {
    log.debug('Tab cambiado', 'Navigation', { tab: newTab, timestamp: Date.now() });
    lastUpdateTimestamp = Date.now();
    activeTab = newTab;
  }

  async function handleFileUpload(response?: any) {
    console.log('handleFileUpload called with response:', response);
    log.info('Archivo subido exitosamente', 'FileUpload', response ? { fileId: response.file_id } : undefined);
    
    // Force refresh of files list to ensure it's up to date
    try {
      console.log('Files before refresh:', api.state.files.length);
      
      // Double check: test the backend API directly
      console.log('Testing direct API call...');
      const testResponse = await fetch(api.state.baseURL + '/files?limit=100&offset=0');
      const testData = await testResponse.json();
      console.log('Direct API response:', testData);
      
      // Force a manual wait to ensure server has processed the file
      console.log('Waiting 1000ms for server processing...');
      await new Promise(resolve => setTimeout(resolve, 1000));
      
      await api.loadFiles();
      console.log('Files after refresh:', api.state.files.length, api.state.files);
      log.debug('Lista de archivos actualizada después del upload', 'FileUpload');
    } catch (error) {
      console.error('Error refreshing files:', error);
      log.error('Error actualizando lista de archivos', 'FileUpload', error instanceof Error ? error : undefined);
    }
    
    // Switch to files tab to show uploaded file
    console.log('Switching to files tab');
    handleTabChange('files');
  }
  
  function handleJobCreated(jobId: string) {
    log.info('Trabajo de docking creado', 'DockingJob', { jobId });
    // Switch to jobs tab to monitor progress
    handleTabChange('jobs');
  }

  async function handleJobCompleted(job: JobStatus) {
    log.info('Trabajo de docking completado', 'DockingJob', { 
      jobId: job.id, 
      posesCount: job.results?.poses?.length || 0,
      computationTime: job.results?.computation_time_ms 
    });
    
    // Auto-switch to poses tab if results are available
    if (job.results) {
      selectedDockingResult = job.results;
      selectedJob = job;
      
      // Load receptor and ligand structures for reports
      try {
        if (job.receptor_file_id) {
          const receptorFile = api.state.files.find(f => f.id === job.receptor_file_id);
          if (receptorFile) {
            log.debug('Cargando estructura del receptor para reportes', 'StructureLoader', { filename: receptorFile.filename });
            // receptorStructure = await loadPDBStructure(receptorFile);
          }
        }
        
        if (job.ligand_file_id) {
          const ligandFile = api.state.files.find(f => f.id === job.ligand_file_id);
          if (ligandFile) {
            log.debug('Cargando estructura del ligando para reportes', 'StructureLoader', { filename: ligandFile.filename });
            // ligandStructure = await loadPDBStructure(ligandFile);
          }
        }
      } catch (error) {
        log.error('Error cargando estructuras para reporte', 'StructureLoader', error instanceof Error ? error : undefined);
      }
      
      handleTabChange('poses');
    }
  }

  function handlePoseSelected(pose: any) {
    selectedPose = pose;
    log.debug('Pose seleccionada para visualización', 'PoseViewer', { 
      poseId: pose.id, 
      energy: pose.energy,
      score: pose.score 
    });
  }
</script>

<svelte:head>
  <title>Protein Docking v2 - Application</title>
  <meta name="description" content="Ultra-high performance protein docking with real-time backend integration" />
</svelte:head>

{#if isLoading}
  <div class="loading-screen">
    <div class="loading-spinner"></div>
    <h1>🧬 Protein Docking v2</h1>
    <p>Connecting to ultra-performance backend...</p>
    <div class="loading-progress">
      <div class="progress-bar">
        <div class="progress-fill"></div>
      </div>
      <span class="progress-text">Initializing API connection and WebSocket...</span>
    </div>
  </div>
{:else if error}
  <div class="error-screen">
    <div class="error-icon">⚠️</div>
    <h1>Connection Error</h1>
    <p>{error}</p>
    <button onclick={() => window.location.reload()} class="retry-button">
      Retry Connection
    </button>
  </div>
{:else}
  <div class="app-container">
    <!-- Header -->
    <header class="app-header">
      <div class="header-content">
        <div class="logo-section">
          <h1>🧬 Protein Docking v2</h1>
          <span class="version">Ultra Performance Edition</span>
        </div>
        
        <div class="connection-status">
          <!-- Backend Connection con estado reactivo -->
          <div class="status-indicator" class:connected={api.state.connected}>
            <span class="status-dot"></span>
            <span class="status-text">
              {api.state.connected ? 'Backend Connected' : 'Backend Disconnected'}
            </span>
            {#if api.state.connected}
              <span class="status-latency">({api.state.latency || 0}ms)</span>
            {/if}
          </div>
          
          <!-- WebSocket Status con indicador de actividad -->
          <div class="status-indicator" class:connected={api.state.wsConnected}>
            <span class="status-dot"></span>
            <span class="status-text">
              {api.state.wsConnected ? 'Live Updates' : 'No Live Updates'}
            </span>
            {#if api.state.wsConnected}
              <span class="activity-indicator" class:active={lastUpdateTimestamp > Date.now() - 5000}></span>
            {/if}
          </div>
          
          <!-- Enhanced System Info -->
          {#if api.state.health}
            <div class="system-info enhanced">
              <span class="info-item cpu" title="CPU Cores disponibles">
                🚀 {api.state.health.capabilities.cpu_cores} cores
              </span>
              <span class="info-item simd" title="Extensiones SIMD disponibles">
                ⚡ {api.state.health.capabilities.simd_support.join(', ') || 'No SIMD'}
              </span>
              <span class="info-item memory" title="Allocator de memoria">
                🧠 {api.state.health.capabilities.memory_allocator}
              </span>
              <span class="info-item performance" title="Updates por segundo">
                📊 {api.state.connected ? 'Online' : 'Offline'}
              </span>
            </div>
          {/if}
          
        </div>
      </div>
    </header>

    <!-- Navigation Tabs -->
    <nav class="tab-navigation">
      <button 
        class="tab-button"
        class:active={activeTab === 'upload'}
        onclick={() => handleTabChange('upload')}
      >
        📤 Upload Files
      </button>
      <button 
        class="tab-button"
        class:active={activeTab === 'files'}
        onclick={() => handleTabChange('files')}
      >
        📁 My Files
        <span class="badge dynamic" class:pulsing={fileStats.total !== api.state.files.length}>{fileStats.total}</span>
      </button>
      <button 
        class="tab-button"
        class:active={activeTab === 'docking'}
        onclick={() => handleTabChange('docking')}
      >
        🧪 New Docking
      </button>
      <button 
        class="tab-button"
        class:active={activeTab === 'jobs'}
        onclick={() => handleTabChange('jobs')}
      >
        📊 My Jobs
        <span class="badge active-jobs" class:urgent={jobStats.active > 5}>{jobStats.active}</span>
      </button>
      <button 
        class="tab-button"
        class:active={activeTab === 'viewer'}
        onclick={() => handleTabChange('viewer')}
      >
        🔬 Molecular Viewer
      </button>
      <button 
        class="tab-button"
        class:active={activeTab === 'poses'}
        onclick={() => handleTabChange('poses')}
      >
        🎯 Results Analysis
        {#if dockingAnalytics.hasPoses}
          <span class="badge poses" class:excellent={dockingAnalytics.poseCount > 10}>{dockingAnalytics.poseCount}</span>
        {/if}
      </button>
      <button 
        class="tab-button"
        class:active={activeTab === 'reports'}
        onclick={() => handleTabChange('reports')}
      >
        📋 Detailed Reports
        {#if dockingAnalytics.hasPoses}
          <span class="badge report-ready">1</span>
        {/if}
      </button>
      <button 
        class="tab-button"
        class:active={activeTab === 'export'}
        onclick={() => handleTabChange('export')}
      >
        📦 Export Data
        {#if dockingAnalytics.hasPoses}
          <span class="badge export-formats">5</span>
        {/if}
      </button>
    </nav>

    <!-- Main Content -->
    <main class="app-main">
      {#if activeTab === 'upload'}
        <section class="content-section">
          <div class="section-header">
            <h2>🎆 Welcome to Molecular Docking</h2>
            <p>Upload your protein structures and small molecules to discover how they might interact</p>
          </div>
          
          <FileUpload onUpload={handleFileUpload} />
          
          <div class="upload-stats enhanced">
            <div class="stat-card animated" class:highlight={fileStats.total > 0}>
              <h3>Total Files</h3>
              <span class="stat-value">{fileStats.total}</span>
              <div class="stat-trend">+{fileStats.total} uploaded</div>
            </div>
            <div class="stat-card animated" class:highlight={fileStats.receptors > 0}>
              <h3>Receptors</h3>
              <span class="stat-value">{fileStats.receptors}</span>
              <div class="stat-trend">Large proteins (>100 atoms)</div>
            </div>
            <div class="stat-card animated" class:highlight={fileStats.ligands > 0}>
              <h3>Ligands</h3>
              <span class="stat-value">{fileStats.ligands}</span>
              <div class="stat-trend">Ready for docking</div>
            </div>
          </div>
        </section>
        
      {:else if activeTab === 'files'}
        <section class="content-section">
          <div class="section-header">
            <h2>📋 Your Molecule Library</h2>
            <p>View and organize all your uploaded protein structures and molecules</p>
          </div>
          
          <FileManager />
        </section>
        
      {:else if activeTab === 'docking'}
        <section class="content-section">
          <div class="section-header">
            <h2>🎯 Start New Analysis</h2>
            <p>Select proteins and molecules to analyze their potential interactions</p>
          </div>
          
          <DockingJobCreator onJobCreated={handleJobCreated} />
        </section>
        
      {:else if activeTab === 'jobs'}
        <section class="content-section">
          <div class="section-header">
            <h2>📈 Analysis Progress</h2>
            <p>Track your docking analyses and view results as they complete</p>
          </div>
          
          <JobMonitor onJobSelect={handleJobCompleted} />
        </section>
        
      {:else if activeTab === 'poses'}
        <section class="content-section poses-layout">
          <div class="section-header">
            <h2>🔭 Interaction Analysis</h2>
            <p>Explore how your molecules interact and find the best binding configurations</p>
          </div>
          
          {#if !selectedDockingResult}
            <div class="no-results-state">
              <div class="state-icon">🎯</div>
              <h3>No Docking Results Available</h3>
              <p>Complete a docking job to view and analyze poses here.</p>
              <button 
                onclick={() => handleTabChange('docking')}
                class="primary-button"
              >
                Create Docking Job
              </button>
            </div>
          {:else}
            <div class="poses-grid">
              <!-- Pose List Panel -->
              <div class="poses-panel">
                <PoseViewer 
                  dockingResult={selectedDockingResult}
                  receptor={receptorStructure}
                  ligand={ligandStructure}
                  onPoseSelect={handlePoseSelected}
                  showEnergyDetails={true}
                  maxPosesToShow={20}
                />
              </div>
              
              <!-- 3D Visualization Panel -->
              <div class="visualization-panel">
                <PoseVisualizer 
                  receptor={receptorStructure}
                  ligand={ligandStructure}
                  selectedPose={selectedPose}
                  showReceptor={true}
                  showLigand={true}
                  showContacts={true}
                  showLabels={false}
                  representationType="spacefill"
                />
              </div>
            </div>

            <!-- Summary Statistics -->
            <div class="result-summary">
              <div class="summary-stats">
                <div class="stat-item">
                  <span class="stat-label">Total Poses:</span>
                  <span class="stat-value">{selectedDockingResult.poses?.length || 0}</span>
                </div>
                <div class="stat-item">
                  <span class="stat-label">Best Energy:</span>
                  <span class="stat-value energy">
                    {selectedDockingResult.best_pose?.energy?.toFixed(2) || 'N/A'} kcal/mol
                  </span>
                </div>
                <div class="stat-item">
                  <span class="stat-label">Total Time:</span>
                  <span class="stat-value">
                    {selectedDockingResult.computation_time ? 
                      (selectedDockingResult.computation_time / 1000).toFixed(1) : 'N/A'} s
                  </span>
                </div>
                <div class="stat-item">
                  <span class="stat-label">Algorithm:</span>
                  <span class="stat-value">{selectedDockingResult.algorithm || 'N/A'}</span>
                </div>
              </div>
            </div>
          {/if}
        </section>
        
      {:else if activeTab === 'reports'}
        <section class="content-section">
          <div class="section-header">
            <h2>📊 Scientific Reports</h2>
            <p>Get detailed scientific reports with binding scores and recommendations for your research</p>
          </div>
          
          {#if !selectedDockingResult}
            <div class="no-results-state">
              <div class="state-icon">📋</div>
              <h3>No Docking Results Available</h3>
              <p>Complete a docking job to generate detailed reports with analysis and recommendations.</p>
              <button 
                onclick={() => handleTabChange('docking')}
                class="primary-button"
              >
                Create Docking Job
              </button>
            </div>
          {:else}
            <ReportViewer 
              dockingResult={selectedDockingResult}
              receptor={receptorStructure}
              ligand={ligandStructure}
              visible={true}
            />
          {/if}
        </section>
        
      {:else if activeTab === 'export'}
        <section class="content-section">
          <div class="section-header">
            <h2>📤 Save Your Data</h2>
            <p>Download your results in various formats for publications, presentations, or further analysis</p>
          </div>
          
          <DataExporter 
            dockingResult={selectedDockingResult}
            receptor={receptorStructure}
            ligand={ligandStructure}
            visible={true}
          />
        </section>
        
        
      {:else if activeTab === 'viewer'}
        <section class="content-section viewer-layout">
          <div class="viewer-controls">
            <DockingControls 
              {selectedReceptor}
              {selectedLigand}
              {selectedJob}
              onReceptorChange={(file) => selectedReceptor = file}
              onLigandChange={(file) => selectedLigand = file}
              onJobSelect={(job) => selectedJob = job}
              onViewResultsToggle={(show) => showResults = show}
            />
          </div>
          
          <div class="viewer-main">
            <ProteinViewer 
              {selectedReceptor}
              {selectedLigand}
              {selectedJob}
              {showResults}
            />
          </div>
        </section>
      {/if}
    </main>

    <!-- Footer -->
    <footer class="app-footer">
      <div class="footer-content">
        <div class="backend-info">
          {#if api.state.systemStats}
            <span>System Load: {api.state.systemStats.system_load_percent.toFixed(1)}%</span>
            <span>Active Jobs: {api.state.systemStats.running_jobs}</span>
            <span>Total Jobs: {api.state.systemStats.total_jobs}</span>
            <span>Avg Time: {Math.round(api.state.systemStats.avg_completion_time_seconds / 60)}min</span>
          {:else}
            <span>Backend: {api.state.baseURL}</span>
            <span>Status: {api.state.connected ? '✅ Connected' : '❌ Disconnected'}</span>
          {/if}
        </div>
        
        <div class="version-info">
          <span>Frontend v2.0.0</span>
          {#if api.state.health}
            <span>Backend {api.state.health.version}</span>
          {/if}
          <span>Performance Mode: ULTRA</span>
        </div>
      </div>
    </footer>
  </div>
{/if}

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    background: #f8fafc;
    color: #1f2937;
  }

  .loading-screen,
  .error-screen {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100vh;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    text-align: center;
    color: white;
  }

  .loading-spinner {
    width: 60px;
    height: 60px;
    border: 4px solid rgba(255, 255, 255, 0.3);
    border-top: 4px solid #ffffff;
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin-bottom: 2rem;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  .loading-screen h1,
  .error-screen h1 {
    font-size: 2.5rem;
    margin: 0 0 0.5rem 0;
    font-weight: 700;
  }

  .loading-screen p,
  .error-screen p {
    font-size: 1.1rem;
    margin: 0 0 2rem 0;
    opacity: 0.9;
  }

  .loading-progress {
    width: 300px;
  }

  .progress-bar {
    width: 100%;
    height: 4px;
    background: rgba(255, 255, 255, 0.3);
    border-radius: 2px;
    overflow: hidden;
    margin-bottom: 0.5rem;
  }

  .progress-fill {
    height: 100%;
    background: #ffffff;
    border-radius: 2px;
    animation: progress 3s ease-in-out infinite;
  }

  @keyframes progress {
    0% { width: 0%; }
    50% { width: 70%; }
    100% { width: 100%; }
  }

  .progress-text {
    font-size: 0.9rem;
    opacity: 0.8;
  }

  .error-icon {
    font-size: 4rem;
    margin-bottom: 1rem;
  }

  .retry-button {
    padding: 12px 24px;
    background: #ffffff;
    color: #667eea;
    border: none;
    border-radius: 6px;
    font-size: 1rem;
    font-weight: 600;
    cursor: pointer;
    transition: transform 0.2s;
  }

  .retry-button:hover {
    transform: translateY(-2px);
  }

  /* Main App Layout */
  .app-container {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
    background: #f8fafc;
  }

  .app-header {
    background: white;
    border-bottom: 1px solid #e5e7eb;
    padding: 1rem 2rem;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }

  .header-content {
    display: flex;
    justify-content: space-between;
    align-items: center;
    max-width: 1400px;
    margin: 0 auto;
  }

  .logo-section h1 {
    margin: 0;
    font-size: 1.5rem;
    font-weight: 700;
    background: linear-gradient(45deg, #3b82f6, #1d4ed8);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }

  .version {
    font-size: 0.75rem;
    color: #6b7280;
    font-weight: 500;
  }

  .connection-status {
    display: flex;
    align-items: center;
    gap: 2rem;
  }

  .status-indicator {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.875rem;
  }

  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #ef4444;
    animation: pulse-red 2s infinite;
  }

  .status-indicator.connected .status-dot {
    background: #10b981;
    animation: pulse-green 2s infinite;
  }

  @keyframes pulse-red {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }

  @keyframes pulse-green {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.7; }
  }

  .status-text {
    color: #6b7280;
    font-weight: 500;
  }

  .status-indicator.connected .status-text {
    color: #059669;
  }

  .system-info {
    display: flex;
    gap: 1rem;
    font-size: 0.75rem;
    color: #9ca3af;
  }


  /* Tab Navigation */
  .tab-navigation {
    background: white;
    border-bottom: 1px solid #e5e7eb;
    padding: 0 2rem;
    display: flex;
    gap: 0.5rem;
    max-width: 1400px;
    margin: 0 auto;
    width: 100%;
    box-sizing: border-box;
  }

  .tab-button {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 1rem 1.5rem;
    background: none;
    border: none;
    color: #6b7280;
    font-weight: 500;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    transition: all 0.2s;
    position: relative;
  }

  .tab-button:hover {
    color: #374151;
    background: #f9fafb;
  }

  .tab-button.active {
    color: #3b82f6;
    border-bottom-color: #3b82f6;
  }

  .badge {
    background: #e5e7eb;
    color: #6b7280;
    font-size: 0.75rem;
    padding: 0.125rem 0.5rem;
    border-radius: 9999px;
    font-weight: 600;
    min-width: 1.25rem;
    text-align: center;
  }

  .badge.active-jobs {
    background: #3b82f6;
    color: white;
  }


  /* Nuevos estilos para Svelte 5 runes y estados reactivos */
  .badge.dynamic {
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .badge.pulsing {
    animation: pulseScale 0.6s ease-in-out infinite;
  }

  .badge.urgent {
    background: #ef4444;
    color: white;
    animation: pulseUrgent 1s ease-in-out infinite;
  }

  .badge.poses.excellent {
    background: #10b981;
    color: white;
    box-shadow: 0 0 10px rgba(16, 185, 129, 0.3);
  }

  .badge.report-ready {
    background: #8b5cf6;
    color: white;
  }

  .badge.export-formats {
    background: #f59e0b;
    color: white;
  }

  .status-latency {
    font-size: 0.7rem;
    color: #10b981;
    font-weight: 600;
    margin-left: 0.25rem;
  }

  .activity-indicator {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #6b7280;
    margin-left: 0.5rem;
    transition: background-color 0.3s ease;
  }

  .activity-indicator.active {
    background: #10b981;
    animation: pulseActivity 2s ease-in-out infinite;
  }

  .system-info.enhanced {
    display: flex;
    gap: 1.5rem;
    font-size: 0.75rem;
    color: #9ca3af;
  }

  .system-info .info-item {
    position: relative;
    cursor: help;
    transition: color 0.2s ease;
  }

  .system-info .info-item:hover {
    color: #6b7280;
  }

  .system-info .info-item.performance {
    color: #3b82f6;
    font-weight: 600;
  }

  /* Upload stats enhancements */
  .upload-stats.enhanced {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1rem;
    margin-top: 2rem;
  }

  .stat-card.animated {
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    position: relative;
    overflow: hidden;
  }

  .stat-card.animated::before {
    content: '';
    position: absolute;
    top: 0;
    left: -100%;
    width: 100%;
    height: 2px;
    background: linear-gradient(90deg, transparent, #3b82f6, transparent);
    transition: left 0.6s ease;
  }

  .stat-card.highlight::before {
    left: 100%;
  }

  .stat-card.highlight {
    border-color: #3b82f6;
    box-shadow: 0 4px 12px rgba(59, 130, 246, 0.15);
    transform: translateY(-2px);
  }


  .stat-trend {
    font-size: 0.75rem;
    color: #6b7280;
    margin-top: 0.25rem;
    font-weight: 500;
  }

  .stat-card.highlight .stat-trend {
    color: #3b82f6;
  }

  /* Animaciones para estados reactivos */
  @keyframes pulseScale {
    0%, 100% { transform: scale(1); }
    50% { transform: scale(1.1); }
  }

  @keyframes pulseUrgent {
    0%, 100% { 
      background: #ef4444;
      box-shadow: 0 0 0 0 rgba(239, 68, 68, 0.7);
    }
    50% { 
      background: #dc2626;
      box-shadow: 0 0 0 4px rgba(239, 68, 68, 0.3);
    }
  }

  @keyframes pulseActivity {
    0%, 100% { 
      background: #10b981;
      transform: scale(1);
    }
    50% { 
      background: #059669;
      transform: scale(1.2);
    }
  }

  /* Main Content */
  .app-main {
    flex: 1;
    padding: 2rem;
    max-width: 1400px;
    margin: 0 auto;
    width: 100%;
    box-sizing: border-box;
  }

  .content-section {
    width: 100%;
  }

  .section-header {
    margin-bottom: 2rem;
    text-align: center;
  }

  .section-header h2 {
    margin: 0 0 0.5rem;
    font-size: 2rem;
    font-weight: 700;
    color: #111827;
  }

  .section-header p {
    margin: 0;
    color: #6b7280;
    font-size: 1.125rem;
  }

  /* Upload Stats */
  .upload-stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1rem;
    margin-top: 2rem;
  }

  .stat-card {
    background: white;
    padding: 1.5rem;
    border-radius: 12px;
    border: 1px solid #e5e7eb;
    text-align: center;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }

  .stat-card h3 {
    margin: 0 0 0.5rem;
    font-size: 0.875rem;
    color: #6b7280;
    text-transform: uppercase;
    font-weight: 600;
    letter-spacing: 0.05em;
  }

  .stat-value {
    font-size: 2rem;
    font-weight: 700;
    color: #3b82f6;
  }

  /* Viewer Layout */
  .viewer-layout {
    display: grid;
    grid-template-columns: 300px 1fr;
    gap: 2rem;
    height: calc(100vh - 200px);
  }

  /* Poses Layout */
  .poses-layout {
    width: 100%;
  }

  .poses-grid {
    display: grid;
    grid-template-columns: 400px 1fr;
    gap: 2rem;
    height: 700px;
    margin-bottom: 2rem;
  }

  .poses-panel {
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 12px;
    overflow: hidden;
  }

  .visualization-panel {
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 12px;
    overflow: hidden;
  }

  .no-results-state {
    text-align: center;
    padding: 4rem 2rem;
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 12px;
    margin: 2rem 0;
  }

  .state-icon {
    font-size: 4rem;
    margin-bottom: 1.5rem;
  }

  .no-results-state h3 {
    margin: 0 0 1rem 0;
    color: #374151;
    font-size: 1.5rem;
    font-weight: 600;
  }

  .no-results-state p {
    margin: 0 0 2rem 0;
    color: #6b7280;
    font-size: 1.1rem;
  }

  .primary-button {
    padding: 0.75rem 2rem;
    background: #3b82f6;
    color: white;
    border: none;
    border-radius: 8px;
    font-size: 1rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .primary-button:hover {
    background: #2563eb;
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(59, 130, 246, 0.3);
  }

  .result-summary {
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 12px;
    padding: 2rem;
    margin-top: 2rem;
  }

  .summary-stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 2rem;
  }

  .stat-item {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    text-align: center;
    padding: 1rem;
    background: #f9fafb;
    border-radius: 8px;
  }

  .stat-label {
    font-size: 0.875rem;
    color: #6b7280;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .stat-value {
    font-size: 1.5rem;
    font-weight: 700;
    color: #111827;
    font-family: 'Courier New', monospace;
  }

  .stat-value.energy {
    color: #dc2626;
  }

  .viewer-controls {
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 12px;
    overflow: hidden;
  }

  .viewer-main {
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 12px;
    overflow: hidden;
  }

  /* Footer */
  .app-footer {
    background: white;
    border-top: 1px solid #e5e7eb;
    padding: 1rem 2rem;
    margin-top: auto;
  }

  .footer-content {
    display: flex;
    justify-content: space-between;
    align-items: center;
    max-width: 1400px;
    margin: 0 auto;
    font-size: 0.875rem;
    color: #6b7280;
  }

  .backend-info,
  .version-info {
    display: flex;
    gap: 1rem;
  }

  /* Responsive Design */
  @media (max-width: 1200px) {
    .app-main {
      padding: 1rem;
    }
    
    .viewer-layout {
      grid-template-columns: 1fr;
      grid-template-rows: auto 1fr;
      height: auto;
    }

    .poses-grid {
      grid-template-columns: 1fr;
      grid-template-rows: 400px 500px;
      height: auto;
    }
  }

  @media (max-width: 768px) {
    .header-content {
      flex-direction: column;
      gap: 1rem;
    }
    
    .connection-status {
      flex-wrap: wrap;
      gap: 1rem;
    }
    
    .tab-navigation {
      overflow-x: auto;
      padding: 0 1rem;
    }
    
    .tab-button {
      white-space: nowrap;
    }
    
    .app-main {
      padding: 1rem;
    }
    
    .section-header h2 {
      font-size: 1.5rem;
    }
    
    .footer-content {
      flex-direction: column;
      gap: 0.5rem;
      text-align: center;
    }
    
    .backend-info,
    .version-info {
      flex-wrap: wrap;
      justify-content: center;
      gap: 0.5rem;
    }
  }

  /* Dark mode support */
  @media (prefers-color-scheme: dark) {
    :global(body) {
      background: #111827;
      color: #e5e7eb;
    }
    
    .app-container {
      background: #111827;
    }
    
    .app-header,
    .tab-navigation,
    .app-footer {
      background: #1f2937;
      border-color: #374151;
    }
    
    .section-header h2 {
      color: #e5e7eb;
    }
    
    .stat-card {
      background: #1f2937;
      border-color: #374151;
    }
    
    .viewer-controls,
    .viewer-main {
      background: #1f2937;
      border-color: #374151;
    }
  }
</style>