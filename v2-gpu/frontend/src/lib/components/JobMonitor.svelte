<!-- Job Monitor - Monitoreo en tiempo real de trabajos de docking -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../stores/api.svelte';
  import { dockingService } from '../services/DockingService.js';
  import { notificationService } from '../services/NotificationService';
  import type { JobStatus, JobProgressUpdate } from '../services/ApiService';
  import type { JobStatusInfo } from '../types/unified.js';

  interface Props {
    showCompleted?: boolean;
    maxJobs?: number;
    onJobSelect?: (job: JobStatus) => void;
  }

  let {
    showCompleted = true,
    maxJobs = 50,
    onJobSelect
  }: Props = $props();

  let isLoading = $state(false);

  let selectedJobId = $state<string | null>(null);
  let autoRefresh = $state(true);
  let refreshInterval = $state(5000); // 5 seconds
  let localJobProgress = $state<Map<string, JobProgressUpdate>>(new Map());
  let lastUpdate = $state<string>('Never');

  // Filtered jobs based on props - combine API jobs with local jobs
  const displayJobs = $derived(() => {
    console.log('📊 JobMonitor: Computing displayJobs', {
      apiJobsCount: api.state.jobs.length,
      apiJobs: api.state.jobs,
      showCompleted,
      maxJobs
    });
    
    let jobs: JobStatusInfo[] = [];
    
    // Add API jobs (convert to JobStatusInfo format)
    const apiJobs: JobStatusInfo[] = api.state.jobs.map(job => ({
      job_id: job.job_id,
      status: job.status,
      created_at: job.created_at || new Date().toISOString(),
      started_at: job.started_at,
      completed_at: job.completed_at,
      current_step: job.current_step || 'Queued',
      progress_percent: job.progress_percent || 0,
      error_message: job.error_message,
      results: job.results,
      poses_evaluated: job.poses_evaluated,
      poses_remaining: job.poses_remaining,
      estimated_completion: job.estimated_completion
    }));
    
    jobs = [...apiJobs];
    
    if (!showCompleted) {
      jobs = jobs.filter(job => 
        ['queued', 'running'].includes(job.status)
      );
    }
    
    const result = jobs.slice(0, maxJobs);
    console.log('📊 JobMonitor: Final displayJobs result:', result);
    
    return result;
  });

  // Auto-refresh jobs
  let refreshTimer: number | null = null;

  // Initialize DockingService and set up event listeners
  $effect(() => {
    const handleJobProgress = (progressData: any) => {
      const update: JobProgressUpdate = {
        job_id: progressData.jobId,
        progress_percent: progressData.progress,
        current_step: progressData.currentStep,
        poses_evaluated: progressData.posesEvaluated || 0,
        poses_remaining: progressData.posesRemaining || 0,
        estimated_completion: progressData.estimatedCompletion,
        computation_stats: progressData.computationStats,
        timestamp: progressData.timestamp || new Date().toISOString()
      };
      
      localJobProgress.set(progressData.jobId, update);
    };

    // Initialize DockingService if needed
    if (!dockingService.isInitialized) {
      dockingService.initialize().catch(error => {
        console.warn('Failed to initialize DockingService:', error);
      });
    }

    // Set up DockingService event listeners
    dockingService.on('job:progress', handleJobProgress);
    
    return () => {
      dockingService.off('job:progress', handleJobProgress);
    };
  });

  $effect(() => {
    console.log('📊 JobMonitor: Setting up auto-refresh...', { autoRefresh, refreshInterval });
    
    if (autoRefresh) {
      refreshTimer = window.setInterval(async () => {
        console.log('📊 JobMonitor: Auto-refreshing jobs...');
        isLoading = true;
        try {
          await api.loadJobs();
          lastUpdate = new Date().toLocaleTimeString();
          console.log('📊 JobMonitor: Jobs refreshed successfully');
        } catch (error) {
          console.error('❌ JobMonitor: Failed to refresh jobs:', error);
        } finally {
          isLoading = false;
        }
      }, refreshInterval);
    } else {
      if (refreshTimer) {
        clearInterval(refreshTimer);
        refreshTimer = null;
      }
    }

    return () => {
      if (refreshTimer) {
        clearInterval(refreshTimer);
      }
    };
  });

  async function manualRefresh() {
    console.log('📊 JobMonitor: Starting manual refresh...');
    isLoading = true;
    try {
      await api.loadJobs();
      lastUpdate = new Date().toLocaleTimeString();
      console.log('📊 JobMonitor: Manual refresh completed successfully');
      console.log('📊 JobMonitor: Jobs after refresh:', api.state.jobs);
    } catch (error) {
      console.error('❌ JobMonitor: Manual refresh failed:', error);
    } finally {
      isLoading = false;
    }
  }


  onMount(() => {
    console.log('📊 JobMonitor: Component mounted');
    console.log('📊 JobMonitor: API connection state:', {
      connected: api.state.connected,
      connecting: api.state.connecting,
      error: api.state.error,
      baseURL: api.state.baseURL
    });
    
    // Test direct API call and populate state if needed
    fetch('http://localhost:3001/api/jobs')
      .then(response => response.json())
      .then(data => {
        console.log('📊 JobMonitor: Direct API test successful:', data);
        
        // If API store is empty but backend has jobs, populate it
        if (api.state.jobs.length === 0 && data.jobs && data.jobs.length > 0) {
          console.log('📊 JobMonitor: Populating API store with backend jobs');
          api.state.jobs = data.jobs;
        }
      })
      .catch(error => {
        console.error('📊 JobMonitor: Direct API test failed:', error);
      });
    
    // Initial load
    manualRefresh();
  });

  function selectJob(job: JobStatusInfo) {
    selectedJobId = job.job_id;
    onJobSelect?.(job as any); // Legacy compatibility
  }

  async function cancelJob(jobId: string, event: Event) {
    event.stopPropagation();
    
    if (!confirm('Are you sure you want to cancel this job?')) {
      return;
    }
    
    await api.cancelJob(jobId);
  }

  function getStatusColor(status: string): string {
    switch (status) {
      case 'queued': return '#f59e0b';
      case 'running': return '#3b82f6';
      case 'completed': return '#10b981';
      case 'failed': return '#ef4444';
      case 'cancelled': return '#6b7280';
      default: return '#6b7280';
    }
  }

  function getStatusIcon(status: string): string {
    switch (status) {
      case 'queued': return '⏳';
      case 'running': return '🔄';
      case 'completed': return '✅';
      case 'failed': return '❌';
      case 'cancelled': return '⏹️';
      default: return '❓';
    }
  }

  function formatDuration(start?: string, end?: string): string {
    if (!start) return 'Not started';
    if (!end) return 'Running...';
    
    const startTime = new Date(start).getTime();
    const endTime = new Date(end).getTime();
    const durationMs = endTime - startTime;
    
    if (durationMs < 60000) {
      return `${Math.round(durationMs / 1000)}s`;
    } else if (durationMs < 3600000) {
      return `${Math.round(durationMs / 60000)}m`;
    } else {
      return `${Math.round(durationMs / 3600000)}h`;
    }
  }
  
  function formatTimeRemaining(estimated?: string): string {
    if (!estimated) return 'Unknown';
    
    const now = new Date().getTime();
    const estimatedTime = new Date(estimated).getTime();
    const remaining = estimatedTime - now;
    
    if (remaining <= 0) return 'Soon';
    
    if (remaining < 60000) {
      return `${Math.round(remaining / 1000)}s`;
    } else if (remaining < 3600000) {
      return `${Math.round(remaining / 60000)}m`;
    } else {
      return `${Math.round(remaining / 3600000)}h`;
    }
  }
  
  function getDetailedJobInfo(job: JobStatusInfo): string {
    const parts = [];
    
    if (job.poses_evaluated !== undefined) {
      parts.push(`${job.poses_evaluated} poses evaluated`);
    }
    
    if (job.poses_remaining !== undefined && job.poses_remaining > 0) {
      parts.push(`${job.poses_remaining} remaining`);
    }
    
    if (job.estimated_completion) {
      parts.push(`ETA: ${formatTimeRemaining(job.estimated_completion)}`);
    }
    
    return parts.join(' • ');
  }

  function formatTimeAgo(dateString: string): string {
    const date = new Date(dateString);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    
    const seconds = Math.floor(diffMs / 1000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);
    const days = Math.floor(hours / 24);
    
    if (days > 0) return `${days}d ago`;
    if (hours > 0) return `${hours}h ago`;
    if (minutes > 0) return `${minutes}m ago`;
    return `${seconds}s ago`;
  }

  function getProgressDetails(job: JobStatusInfo): { 
    progress: number; 
    update?: JobProgressUpdate;
    isActive: boolean;
  } {
    const localUpdate = localJobProgress.get(job.job_id);
    const isActive = ['queued', 'running'].includes(job.status);
    
    // Use local update if available, otherwise fall back to job data
    const progress = localUpdate?.progress_percent ?? job.progress_percent ?? 0;
    
    return {
      progress,
      update: localUpdate,
      isActive
    };
  }
</script>

<!-- Job Monitor Component -->
<div class="job-monitor">

  <!-- Header -->
  <div class="monitor-header">
    <div class="header-left">
      <h3>Job Monitor</h3>
      <div class="status-indicators">
        <span class="indicator">
          <span class="dot running"></span>
          {api.activeJobsCount} Active
        </span>
        <span class="indicator">
          <span class="dot completed"></span>
          {api.completedJobsCount} Completed
        </span>
        <span class="indicator">
          <span class="dot failed"></span>
          {api.failedJobsCount} Failed
        </span>
      </div>
    </div>
    
    <div class="header-controls">
      <!-- WebSocket Status -->
      <div class="ws-status" class:connected={api.state.wsConnected}>
        <span class="ws-dot"></span>
        {api.state.wsConnected ? 'Live Updates' : 'Disconnected'}
      </div>
      
      <!-- Auto-refresh Toggle -->
      <label class="auto-refresh">
        <input type="checkbox" bind:checked={autoRefresh} />
        Auto-refresh
      </label>
      
      <!-- Manual Refresh -->
      <button onclick={() => api.loadJobs()} class="refresh-btn">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
          <path d="M21.5 2v6h-6"></path>
          <path d="M2.5 22v-6h6"></path>
          <path d="M21.5 8a10 10 0 0 0-18.8-4.3"></path>
          <path d="M2.5 16a10 10 0 0 0 18.8 4.3"></path>
        </svg>
      </button>
    </div>
  </div>

  <!-- Job List -->
  <div class="job-list">
    {#if api.state.connecting}
      <div class="loading-state">
        <div class="spinner"></div>
        <p>Loading jobs...</p>
      </div>
    {:else if displayJobs.length === 0}
      <div class="empty-state">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
          <rect x="2" y="3" width="20" height="14" rx="2" ry="2"></rect>
          <line x1="8" y1="21" x2="16" y2="21"></line>
          <line x1="12" y1="17" x2="12" y2="21"></line>
        </svg>
        <h4>No jobs found</h4>
        <p>
          {showCompleted 
            ? 'No docking jobs have been created yet' 
            : 'No active jobs running'
          }
        </p>
      </div>
    {:else}
      {#each displayJobs as job (job.job_id)}
        {@const details = getProgressDetails(job)}
        <div 
          class="job-card"
          class:selected={selectedJobId === job.job_id}
          class:active={details.isActive}
          onclick={() => selectJob(job)}
          role="button"
          tabindex="0"
          onkeydown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault();
              selectJob(job);
            }
          }}
        >
          <!-- Job Header -->
          <div class="job-header">
            <div class="job-info">
              <span class="job-icon">{getStatusIcon(job.status)}</span>
              <div class="job-details">
                <h4 class="job-id">{job.job_id}</h4>
                <span class="job-time">
                  Created {formatTimeAgo(job.created_at)}
                </span>
              </div>
            </div>
            
            <div class="job-status">
              <span 
                class="status-badge"
                style="color: {getStatusColor(job.status)}"
              >
                {job.status.toUpperCase()}
              </span>
              {#if ['queued', 'running'].includes(job.status)}
                <button
                  onclick={(e) => cancelJob(job.job_id, e)}
                  class="cancel-btn"
                  title="Cancel job"
                >
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
                    <circle cx="12" cy="12" r="10"></circle>
                    <line x1="15" y1="9" x2="9" y2="15"></line>
                    <line x1="9" y1="9" x2="15" y2="15"></line>
                  </svg>
                </button>
              {/if}
            </div>
          </div>

          <!-- Progress Bar -->
          {#if details.isActive}
            <div class="progress-section">
              <div class="progress-info">
                <span class="current-step">
                  {job.current_step || 'Processing...'}
                </span>
                <span class="progress-percent">
                  {Math.round(details.progress)}%
                </span>
              </div>
              <div class="progress-bar">
                <div 
                  class="progress-fill"
                  style="width: {details.progress}%"
                ></div>
              </div>
              
              {#if details.update}
                <div class="progress-stats">
                  <span class="stat">
                    {details.update.poses_evaluated.toLocaleString()} / {details.update.poses_remaining.toLocaleString()} poses
                  </span>
                  {#if details.update.computation_stats}
                    <span class="stat">
                      {details.update.computation_stats.poses_per_second.toFixed(1)} poses/sec
                    </span>
                    <span class="stat">
                      {details.update.computation_stats.memory_used_mb}MB memory
                    </span>
                  {/if}
                </div>
              {/if}
            </div>
          {:else}
            <!-- Completed Job Info -->
            <div class="completion-info">
              <div class="completion-stats">
                <span class="stat">
                  Duration: {formatDuration(job.started_at, job.completed_at)}
                </span>
                {#if job.results}
                  <span class="stat">
                    {job.results.length} poses generated
                  </span>
                  <span class="stat">
                    Best score: {Math.min(...job.results.map(r => r.score)).toFixed(2)}
                  </span>
                {/if}
              </div>
              
              {#if job.error_message}
                <div class="error-info">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
                    <path d="M12 9v4"></path>
                    <path d="M12 17h.01"></path>
                    <path d="M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20z"></path>
                  </svg>
                  {job.error_message}
                </div>
              {/if}
            </div>
          {/if}

          <!-- Live Update Indicator -->
          {#if details.isActive && details.update}
            <div class="live-indicator">
              <span class="live-dot"></span>
              Live
            </div>
          {/if}
        </div>
      {/each}
    {/if}
  </div>

  <!-- Summary Stats -->
  {#if api.state.systemStats}
    <div class="summary-stats">
      <div class="stat-item">
        <span class="stat-label">System Load</span>
        <span class="stat-value">
          {api.state.systemStats.system_load_percent.toFixed(1)}%
        </span>
      </div>
      <div class="stat-item">
        <span class="stat-label">Avg Completion</span>
        <span class="stat-value">
          {Math.round(api.state.systemStats.avg_completion_time_seconds / 60)}min
        </span>
      </div>
      <div class="stat-item">
        <span class="stat-label">Total Jobs</span>
        <span class="stat-value">
          {api.state.systemStats.total_jobs}
        </span>
      </div>
    </div>
  {/if}
</div>

<style>
  .job-monitor {
    width: 100%;
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 12px;
    overflow: hidden;
  }


  .monitor-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem 1.5rem;
    background: #f9fafb;
    border-bottom: 1px solid #e5e7eb;
  }

  .header-left h3 {
    margin: 0 0 0.5rem;
    font-size: 1.125rem;
    font-weight: 600;
    color: #111827;
  }

  .status-indicators {
    display: flex;
    gap: 1rem;
  }

  .indicator {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.875rem;
    color: #6b7280;
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .dot.running { background: #3b82f6; }
  .dot.completed { background: #10b981; }
  .dot.failed { background: #ef4444; }

  .header-controls {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .ws-status {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.875rem;
    color: #6b7280;
  }

  .ws-status.connected {
    color: #10b981;
  }

  .ws-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #ef4444;
  }

  .ws-status.connected .ws-dot {
    background: #10b981;
    animation: pulse 2s infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }

  .auto-refresh {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.875rem;
    color: #6b7280;
    cursor: pointer;
  }

  .refresh-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    background: white;
    border: 1px solid #d1d5db;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.2s;
  }

  .refresh-btn:hover {
    background: #f3f4f6;
  }

  .refresh-btn svg {
    width: 16px;
    height: 16px;
    color: #6b7280;
  }

  /* Job List */
  .job-list {
    max-height: 600px;
    overflow-y: auto;
  }

  .loading-state,
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 3rem 1rem;
    color: #6b7280;
  }

  .loading-state .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid #e5e7eb;
    border-top-color: #3b82f6;
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin-bottom: 1rem;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .empty-state svg {
    width: 64px;
    height: 64px;
    margin-bottom: 1rem;
    color: #d1d5db;
  }

  .empty-state h4 {
    margin: 0 0 0.5rem;
    font-size: 1.125rem;
    font-weight: 600;
    color: #374151;
  }

  .empty-state p {
    margin: 0;
    font-size: 0.875rem;
  }
  

  /* Job Cards */
  .job-card {
    position: relative;
    padding: 1rem 1.5rem;
    border-bottom: 1px solid #f3f4f6;
    cursor: pointer;
    transition: all 0.2s;
  }

  .job-card:hover {
    background: #f9fafb;
  }

  .job-card.selected {
    background: #eff6ff;
    border-color: #bfdbfe;
  }

  .job-card.active {
    border-left: 4px solid #3b82f6;
  }

  .job-card:last-child {
    border-bottom: none;
  }

  .job-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.75rem;
  }

  .job-info {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .job-icon {
    font-size: 1.25rem;
  }

  .job-details {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .job-id {
    margin: 0;
    font-size: 0.875rem;
    font-weight: 600;
    color: #111827;
    font-family: monospace;
  }

  .job-time {
    font-size: 0.75rem;
    color: #6b7280;
  }

  .job-status {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .status-badge {
    font-size: 0.75rem;
    font-weight: 600;
    padding: 0.25rem 0.5rem;
    background: rgba(0, 0, 0, 0.05);
    border-radius: 4px;
  }

  .cancel-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    background: none;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    color: #6b7280;
    transition: all 0.2s;
  }

  .cancel-btn:hover {
    background: #fee2e2;
    color: #ef4444;
  }

  .cancel-btn svg {
    width: 14px;
    height: 14px;
  }

  /* Progress Section */
  .progress-section {
    margin-top: 0.75rem;
  }

  .progress-info {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }

  .current-step {
    font-size: 0.875rem;
    color: #374151;
  }

  .progress-percent {
    font-size: 0.875rem;
    font-weight: 600;
    color: #3b82f6;
  }

  .progress-bar {
    height: 6px;
    background: #e5e7eb;
    border-radius: 3px;
    overflow: hidden;
    margin-bottom: 0.5rem;
  }

  .progress-fill {
    height: 100%;
    background: #3b82f6;
    transition: width 0.3s ease;
    border-radius: 3px;
  }

  .progress-stats {
    display: flex;
    gap: 1rem;
    font-size: 0.75rem;
    color: #6b7280;
  }

  /* Completion Info */
  .completion-info {
    margin-top: 0.75rem;
  }

  .completion-stats {
    display: flex;
    gap: 1rem;
    font-size: 0.75rem;
    color: #6b7280;
    margin-bottom: 0.5rem;
  }

  .error-info {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.75rem;
    color: #ef4444;
    padding: 0.5rem;
    background: #fef2f2;
    border-radius: 4px;
  }

  .error-info svg {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
  }

  /* Live Indicator */
  .live-indicator {
    position: absolute;
    top: 1rem;
    right: 1rem;
    display: flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.75rem;
    color: #10b981;
    font-weight: 500;
  }

  .live-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #10b981;
    animation: pulse 1.5s infinite;
  }

  /* Summary Stats */
  .summary-stats {
    display: flex;
    justify-content: space-around;
    padding: 1rem;
    background: #f9fafb;
    border-top: 1px solid #e5e7eb;
  }

  .stat-item {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.25rem;
  }

  .stat-label {
    font-size: 0.75rem;
    color: #6b7280;
  }

  .stat-value {
    font-size: 0.875rem;
    font-weight: 600;
    color: #111827;
  }

  /* Responsive design */
  @media (max-width: 768px) {
    .monitor-header {
      flex-direction: column;
      align-items: flex-start;
      gap: 1rem;
    }

    .header-controls {
      width: 100%;
      justify-content: flex-end;
    }

    .status-indicators {
      flex-wrap: wrap;
      gap: 0.75rem;
    }

    .progress-stats,
    .completion-stats {
      flex-direction: column;
      gap: 0.25rem;
    }

    .summary-stats {
      flex-direction: column;
      gap: 0.75rem;
    }
  }

  /* Dark mode support */
  @media (prefers-color-scheme: dark) {
    .job-monitor {
      background: #1f2937;
      border-color: #374151;
    }

    .monitor-header {
      background: #111827;
      border-bottom-color: #374151;
    }

    .header-left h3 {
      color: #e5e7eb;
    }

    .job-card:hover {
      background: #374151;
    }

    .job-card.selected {
      background: #1e3a8a;
    }

    .job-id {
      color: #e5e7eb;
    }

    .summary-stats {
      background: #111827;
      border-top-color: #374151;
    }

    .stat-value {
      color: #e5e7eb;
    }
  }
</style>