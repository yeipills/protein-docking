<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { apiService } from '$lib/services/ApiService.js';

  // Estado reactivo
  let jobs: any[] = [];
  let queueStatus: any = null;
  let selectedJob: any = null;
  let isLoading = false;
  let error: string | null = null;
  let autoRefresh = true;
  let refreshInterval: any = null;

  // Formulario de envío de trabajos
  let jobForm = {
    receptor_filename: '2f6d.pdb',
    ligand_filename: '4jet.pdb',
    priority: 1,
    max_poses: 10,
    energy_cutoff: -5.0,
    user_id: 'demo-user',
    tags: ['test']
  };

  // Estado del formulario
  let isSubmitting = false;
  let showJobForm = false;

  onMount(() => {
    loadJobs();
    loadQueueStatus();
    
    if (autoRefresh) {
      startAutoRefresh();
    }
  });

  onDestroy(() => {
    if (refreshInterval) {
      clearInterval(refreshInterval);
    }
  });

  function startAutoRefresh() {
    if (refreshInterval) clearInterval(refreshInterval);
    refreshInterval = setInterval(() => {
      loadJobs();
      loadQueueStatus();
    }, 2000); // Refrescar cada 2 segundos
  }

  function stopAutoRefresh() {
    if (refreshInterval) {
      clearInterval(refreshInterval);
      refreshInterval = null;
    }
  }

  function toggleAutoRefresh() {
    autoRefresh = !autoRefresh;
    if (autoRefresh) {
      startAutoRefresh();
    } else {
      stopAutoRefresh();
    }
  }

  async function loadJobs() {
    try {
      isLoading = true;
      error = null;
      const response = await apiService.getAllJobs();
      
      if (response.success) {
        jobs = response.jobs || [];
      } else {
        error = response.error || 'Failed to load jobs';
      }
    } catch (err: any) {
      error = err.message || 'Network error';
      console.error('Error loading jobs:', err);
    } finally {
      isLoading = false;
    }
  }

  async function loadQueueStatus() {
    try {
      const response = await apiService.getQueueStatus();
      
      if (response.success) {
        queueStatus = response.queue;
      }
    } catch (err: any) {
      console.error('Error loading queue status:', err);
    }
  }

  async function submitJob() {
    try {
      isSubmitting = true;
      error = null;
      
      const response = await apiService.submitDockingJob(jobForm);
      
      if (response.success) {
        showJobForm = false;
        loadJobs();
        loadQueueStatus();
        console.log('Job submitted successfully:', response.job_id);
      } else {
        error = response.error || 'Failed to submit job';
      }
    } catch (err: any) {
      error = err.message || 'Network error';
      console.error('Error submitting job:', err);
    } finally {
      isSubmitting = false;
    }
  }

  async function deleteJob(jobId: string) {
    try {
      const response = await apiService.deleteJob(jobId);
      
      if (response.success) {
        loadJobs();
        loadQueueStatus();
        if (selectedJob && selectedJob.id === jobId) {
          selectedJob = null;
        }
      }
    } catch (err: any) {
      console.error('Error deleting job:', err);
    }
  }

  function selectJob(job: any) {
    selectedJob = job;
  }

  function getStatusColor(status: string): string {
    switch (status.toLowerCase()) {
      case 'queued': return 'text-yellow-600 bg-yellow-100';
      case 'running': return 'text-blue-600 bg-blue-100';
      case 'completed': return 'text-green-600 bg-green-100';
      case 'failed': return 'text-red-600 bg-red-100';
      case 'cancelled': return 'text-gray-600 bg-gray-100';
      default: return 'text-gray-600 bg-gray-100';
    }
  }

  function formatDuration(start?: string, end?: string): string {
    if (!start) return 'N/A';
    
    const startTime = new Date(start);
    const endTime = end ? new Date(end) : new Date();
    const duration = endTime.getTime() - startTime.getTime();
    
    if (duration < 1000) return `${duration}ms`;
    if (duration < 60000) return `${Math.round(duration / 1000)}s`;
    if (duration < 3600000) return `${Math.round(duration / 60000)}m`;
    return `${Math.round(duration / 3600000)}h`;
  }

  function formatTimestamp(timestamp?: string): string {
    if (!timestamp) return 'N/A';
    return new Date(timestamp).toLocaleString();
  }
</script>

<div class="job-monitor p-6">
  <!-- Header -->
  <div class="flex justify-between items-center mb-6">
    <div>
      <h2 class="text-2xl font-bold text-gray-900">🧬 Job Monitor</h2>
      <p class="text-gray-600">Monitoreo en tiempo real de trabajos de docking molecular</p>
    </div>
    
    <div class="flex space-x-3">
      <button
        class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
        on:click={() => showJobForm = !showJobForm}
      >
        Nuevo Job
      </button>
      
      <button
        class="px-4 py-2 {autoRefresh ? 'bg-green-600 hover:bg-green-700' : 'bg-gray-600 hover:bg-gray-700'} text-white rounded-lg"
        on:click={toggleAutoRefresh}
      >
        {autoRefresh ? '⏸️ Pausar Auto-Refresh' : '▶️ Auto-Refresh'}
      </button>
      
      <button
        class="px-4 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-700"
        on:click={() => { loadJobs(); loadQueueStatus(); }}
        disabled={isLoading}
      >
        {isLoading ? '🔄 Actualizando...' : '🔄 Refresh'}
      </button>
    </div>
  </div>

  <!-- Error Display -->
  {#if error}
    <div class="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4">
      <strong>❌ Error:</strong> {error}
    </div>
  {/if}

  <!-- Queue Status -->
  {#if queueStatus}
    <div class="grid grid-cols-2 md:grid-cols-5 gap-4 mb-6">
      <div class="bg-white p-4 rounded-lg border shadow-sm">
        <div class="text-2xl font-bold text-gray-900">{queueStatus.total_jobs}</div>
        <div class="text-sm text-gray-600">📊 Total Jobs</div>
      </div>
      
      <div class="bg-yellow-50 p-4 rounded-lg border shadow-sm">
        <div class="text-2xl font-bold text-yellow-600">{queueStatus.queued}</div>
        <div class="text-sm text-gray-600">⏳ En Cola</div>
      </div>
      
      <div class="bg-blue-50 p-4 rounded-lg border shadow-sm">
        <div class="text-2xl font-bold text-blue-600">{queueStatus.running}</div>
        <div class="text-sm text-gray-600">🏃 Ejecutándose</div>
      </div>
      
      <div class="bg-green-50 p-4 rounded-lg border shadow-sm">
        <div class="text-2xl font-bold text-green-600">{queueStatus.completed}</div>
        <div class="text-sm text-gray-600">✅ Completados</div>
      </div>
      
      <div class="bg-red-50 p-4 rounded-lg border shadow-sm">
        <div class="text-2xl font-bold text-red-600">{queueStatus.failed}</div>
        <div class="text-sm text-gray-600">❌ Fallidos</div>
      </div>
    </div>
  {/if}

  <!-- Job Submission Form -->
  {#if showJobForm}
    <div class="bg-white p-6 rounded-lg border mb-6 shadow-sm">
      <h3 class="text-lg font-semibold mb-4">🆕 Enviar Nuevo Trabajo de Docking</h3>
      
      <form on:submit|preventDefault={submitJob} class="grid grid-cols-2 gap-4">
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-1">Archivo Receptor</label>
          <select bind:value={jobForm.receptor_filename} class="w-full border rounded-lg px-3 py-2">
            <option value="2f6d.pdb">2f6d.pdb (4,775 atoms)</option>
            <option value="4jet.pdb">4jet.pdb (14,784 atoms)</option>
            <option value="6izr.pdb">6izr.pdb</option>
            <option value="6qtl.pdb">6qtl.pdb</option>
            <option value="8e5d.pdb">8e5d.pdb</option>
          </select>
        </div>
        
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-1">Archivo Ligando</label>
          <select bind:value={jobForm.ligand_filename} class="w-full border rounded-lg px-3 py-2">
            <option value="2f6d.pdb">2f6d.pdb (4,775 atoms)</option>
            <option value="4jet.pdb">4jet.pdb (14,784 atoms)</option>
            <option value="6izr.pdb">6izr.pdb</option>
            <option value="6qtl.pdb">6qtl.pdb</option>
            <option value="8e5d.pdb">8e5d.pdb</option>
          </select>
        </div>
        
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-1">Prioridad (1-10)</label>
          <input type="number" bind:value={jobForm.priority} min="1" max="10" class="w-full border rounded-lg px-3 py-2">
        </div>
        
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-1">Max Poses</label>
          <input type="number" bind:value={jobForm.max_poses} min="1" max="1000" class="w-full border rounded-lg px-3 py-2">
        </div>
        
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-1">Energy Cutoff (kcal/mol)</label>
          <input type="number" bind:value={jobForm.energy_cutoff} step="0.1" class="w-full border rounded-lg px-3 py-2">
        </div>
        
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-1">User ID</label>
          <input type="text" bind:value={jobForm.user_id} class="w-full border rounded-lg px-3 py-2">
        </div>
        
        <div class="col-span-2 flex justify-end space-x-3">
          <button
            type="button"
            class="px-4 py-2 border border-gray-300 rounded-lg hover:bg-gray-50"
            on:click={() => showJobForm = false}
          >
            Cancelar
          </button>
          <button
            type="submit"
            class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
            disabled={isSubmitting}
          >
            {isSubmitting ? '⏳ Enviando...' : '🚀 Enviar Job'}
          </button>
        </div>
      </form>
    </div>
  {/if}

  <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
    <!-- Jobs List -->
    <div class="bg-white rounded-lg border shadow-sm">
      <div class="p-4 border-b">
        <h3 class="text-lg font-semibold">📋 Trabajos ({jobs.length})</h3>
      </div>
      
      <div class="p-4 space-y-3 max-h-96 overflow-y-auto">
        {#if jobs.length === 0}
          <div class="text-center text-gray-500 py-8">
            📭 No hay trabajos disponibles
          </div>
        {:else}
          {#each jobs as job}
            <div
              class="border rounded-lg p-4 hover:shadow-md transition-shadow cursor-pointer {selectedJob && selectedJob.id === job.id ? 'border-blue-500 bg-blue-50' : ''}"
              on:click={() => selectJob(job)}
            >
              <div class="flex justify-between items-start mb-2">
                <div class="flex-1">
                  <div class="font-medium text-sm font-mono">{job.id.substring(0, 8)}...</div>
                  <div class="text-xs text-gray-600">
                    🧬 {job.receptor_filename} → {job.ligand_filename}
                  </div>
                </div>
                
                <div class="flex items-center space-x-2">
                  <span class="px-2 py-1 rounded-full text-xs font-semibold {getStatusColor(job.status)}">
                    {job.status}
                  </span>
                  
                  {#if job.status !== 'Running' && job.status !== 'running'}
                    <button
                      class="text-red-600 hover:text-red-800 text-xs"
                      on:click|stopPropagation={() => deleteJob(job.id)}
                      title="Eliminar job"
                    >
                      🗑️
                    </button>
                  {/if}
                </div>
              </div>
              
              {#if job.status === 'Running' && job.progress}
                <div class="mt-2">
                  <div class="text-xs text-gray-600 mb-1">
                    {job.progress.stage} - {job.progress.percentage.toFixed(1)}%
                  </div>
                  <div class="w-full bg-gray-200 rounded-full h-2">
                    <div 
                      class="bg-blue-600 h-2 rounded-full transition-all duration-300" 
                      style="width: {job.progress.percentage}%"
                    ></div>
                  </div>
                  {#if job.progress.estimated_time_remaining}
                    <div class="text-xs text-gray-500 mt-1">
                      ⏱️ ETA: {job.progress.estimated_time_remaining}s
                    </div>
                  {/if}
                </div>
              {/if}
              
              <div class="text-xs text-gray-500 mt-2">
                📅 Created: {formatTimestamp(job.created_at)}
                {#if job.started_at}
                  | ⏱️ Duration: {formatDuration(job.started_at, job.completed_at)}
                {/if}
              </div>
            </div>
          {/each}
        {/if}
      </div>
    </div>

    <!-- Job Details -->
    <div class="bg-white rounded-lg border shadow-sm">
      <div class="p-4 border-b">
        <h3 class="text-lg font-semibold">🔍 Detalles del Job</h3>
      </div>
      
      <div class="p-4">
        {#if selectedJob}
          <div class="space-y-4">
            <div>
              <h4 class="font-medium text-gray-900">ℹ️ Información del Job</h4>
              <dl class="mt-2 text-sm">
                <div class="grid grid-cols-2 gap-x-4 gap-y-2">
                  <dt class="text-gray-600">ID:</dt>
                  <dd class="font-mono">{selectedJob.id}</dd>
                  
                  <dt class="text-gray-600">Status:</dt>
                  <dd>
                    <span class="px-2 py-1 rounded-full text-xs font-semibold {getStatusColor(selectedJob.status)}">
                      {selectedJob.status}
                    </span>
                  </dd>
                  
                  <dt class="text-gray-600">Receptor:</dt>
                  <dd>🧬 {selectedJob.receptor_filename}</dd>
                  
                  <dt class="text-gray-600">Ligando:</dt>
                  <dd>🧬 {selectedJob.ligand_filename}</dd>
                  
                  <dt class="text-gray-600">Prioridad:</dt>
                  <dd>{selectedJob.metadata.priority}</dd>
                  
                  <dt class="text-gray-600">Max Poses:</dt>
                  <dd>{selectedJob.metadata.max_poses}</dd>
                </div>
              </dl>
            </div>
            
            <div>
              <h4 class="font-medium text-gray-900">⏰ Tiempos</h4>
              <dl class="mt-2 text-sm">
                <div class="grid grid-cols-1 gap-y-2">
                  <div class="flex justify-between">
                    <dt class="text-gray-600">Creado:</dt>
                    <dd>{formatTimestamp(selectedJob.created_at)}</dd>
                  </div>
                  
                  {#if selectedJob.started_at}
                    <div class="flex justify-between">
                      <dt class="text-gray-600">Iniciado:</dt>
                      <dd>{formatTimestamp(selectedJob.started_at)}</dd>
                    </div>
                  {/if}
                  
                  {#if selectedJob.completed_at}
                    <div class="flex justify-between">
                      <dt class="text-gray-600">Completado:</dt>
                      <dd>{formatTimestamp(selectedJob.completed_at)}</dd>
                    </div>
                  {/if}
                  
                  {#if selectedJob.started_at}
                    <div class="flex justify-between">
                      <dt class="text-gray-600">Duración:</dt>
                      <dd><strong>{formatDuration(selectedJob.started_at, selectedJob.completed_at)}</strong></dd>
                    </div>
                  {/if}
                </div>
              </dl>
            </div>

            {#if selectedJob.result}
              <div>
                <h4 class="font-medium text-gray-900">🎯 Resultados</h4>
                <dl class="mt-2 text-sm">
                  <div class="grid grid-cols-2 gap-x-4 gap-y-2">
                    <dt class="text-gray-600">Poses:</dt>
                    <dd><strong>{selectedJob.result.poses_count}</strong></dd>
                    
                    <dt class="text-gray-600">Mejor Energía:</dt>
                    <dd><strong>{selectedJob.result.best_energy.toFixed(2)}</strong> kcal/mol</dd>
                    
                    <dt class="text-gray-600">Contactos:</dt>
                    <dd>{selectedJob.result.best_pose_data.contacts}</dd>
                    
                    <dt class="text-gray-600">Clash Score:</dt>
                    <dd>{selectedJob.result.best_pose_data.clash_score.toFixed(3)}</dd>
                  </div>
                </dl>
                
                {#if selectedJob.result.computation_stats}
                  <div class="mt-4">
                    <h5 class="font-medium text-gray-800">⚡ Estadísticas de Performance</h5>
                    <dl class="mt-2 text-xs">
                      <div class="grid grid-cols-2 gap-x-4 gap-y-1">
                        <dt class="text-gray-600">Tiempo Total:</dt>
                        <dd><strong>{selectedJob.result.computation_stats.total_time_ms}ms</strong></dd>
                        
                        <dt class="text-gray-600">Threads:</dt>
                        <dd>{selectedJob.result.computation_stats.threads_used}</dd>
                        
                        <dt class="text-gray-600">SIMD:</dt>
                        <dd>{selectedJob.result.computation_stats.simd_optimized ? '✅ Sí' : '❌ No'}</dd>
                      </div>
                    </dl>
                  </div>
                {/if}
              </div>
            {/if}

            {#if selectedJob.error}
              <div class="bg-red-50 p-3 rounded">
                <h4 class="font-medium text-red-900">❌ Error</h4>
                <p class="text-sm text-red-700 mt-1">{selectedJob.error}</p>
              </div>
            {/if}
          </div>
        {:else}
          <div class="text-center text-gray-500 py-8">
            🔍 Selecciona un trabajo para ver los detalles
          </div>
        {/if}
      </div>
    </div>
  </div>
</div>