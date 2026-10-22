<!-- Export History - Historial y gestión de exportaciones -->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { exportManager, type ExportJob } from '../utils/ExportManager.js';

  interface Props {
    visible?: boolean;
  }

  let {
    visible = false
  }: Props = $props();

  let exportHistory = $state<ExportJob[]>([]);
  let refreshInterval: number | null = null;

  onMount(() => {
    refreshHistory();
    
    // Auto-refresh cada 2 segundos
    refreshInterval = setInterval(refreshHistory, 2000);
  });

  onDestroy(() => {
    if (refreshInterval) {
      clearInterval(refreshInterval);
    }
  });

  function refreshHistory() {
    exportHistory = exportManager.getExportHistory();
  }

  function downloadFile(job: ExportJob) {
    if (!job.downloadUrl) return;
    
    const a = document.createElement('a');
    a.href = job.downloadUrl;
    a.download = job.filename;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    
    console.log(`📥 Re-downloaded: ${job.filename}`);
  }

  function clearHistory() {
    exportManager.clearHistory();
    refreshHistory();
    console.log('🗑️ Export history cleared');
  }

  function getStatusIcon(status: ExportJob['status']): string {
    switch (status) {
      case 'pending': return '⏳';
      case 'processing': return '⚙️';
      case 'completed': return '✅';
      case 'failed': return '❌';
      default: return '❓';
    }
  }

  function getStatusColor(status: ExportJob['status']): string {
    switch (status) {
      case 'pending': return '#f59e0b';
      case 'processing': return '#3b82f6';
      case 'completed': return '#10b981';
      case 'failed': return '#ef4444';
      default: return '#6b7280';
    }
  }

  function formatTimestamp(timestamp: Date): string {
    return new Intl.DateTimeFormat('es-ES', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit'
    }).format(timestamp);
  }

  function formatFileSize(bytes: number): string {
    return exportManager.formatFileSize(bytes);
  }

  function getFormatIcon(format: string): string {
    const icons: Record<string, string> = {
      json: '📄',
      csv: '📊',
      sdf: '🧪',
      pdb: '🧬',
      xml: '📝',
      xlsx: '📈',
      mol2: '⚗️',
      mae: '🔬'
    };
    return icons[format] || '📁';
  }

  // Estadísticas del historial
  const historyStats = $derived(() => {
    const total = exportHistory.length;
    const completed = exportHistory.filter(j => j.status === 'completed').length;
    const failed = exportHistory.filter(j => j.status === 'failed').length;
    const processing = exportHistory.filter(j => j.status === 'processing').length;
    const totalSize = exportHistory
      .filter(j => j.status === 'completed')
      .reduce((sum, j) => sum + j.size, 0);

    return { total, completed, failed, processing, totalSize };
  });
</script>

{#if visible}
  <div class="export-history">
    <div class="history-header">
      <h3>📋 Historial de Exportaciones</h3>
      <div class="header-actions">
        <button onclick={refreshHistory} class="refresh-btn" title="Actualizar">
          🔄
        </button>
        <button onclick={clearHistory} class="clear-btn" title="Limpiar historial">
          🗑️
        </button>
      </div>
    </div>

    {#if exportHistory.length === 0}
      <div class="no-history">
        <div class="no-history-icon">📦</div>
        <h4>No hay exportaciones</h4>
        <p>Las exportaciones que realices aparecerán aquí</p>
      </div>
    {:else}
      <!-- Statistics -->
      <div class="history-stats">
        <div class="stat-item">
          <span class="stat-value">{historyStats.total}</span>
          <span class="stat-label">Total</span>
        </div>
        <div class="stat-item">
          <span class="stat-value">{historyStats.completed}</span>
          <span class="stat-label">Completadas</span>
        </div>
        <div class="stat-item">
          <span class="stat-value">{historyStats.processing}</span>
          <span class="stat-label">En Proceso</span>
        </div>
        <div class="stat-item">
          <span class="stat-value">{historyStats.failed}</span>
          <span class="stat-label">Fallidas</span>
        </div>
        <div class="stat-item">
          <span class="stat-value">{formatFileSize(historyStats.totalSize)}</span>
          <span class="stat-label">Tamaño Total</span>
        </div>
      </div>

      <!-- Export List -->
      <div class="history-list">
        {#each exportHistory as job (job.id)}
          <div class="history-item" class:failed={job.status === 'failed'}>
            <div class="item-header">
              <div class="file-info">
                <span class="format-icon">{getFormatIcon(job.format)}</span>
                <div class="file-details">
                  <span class="filename" title={job.filename}>{job.filename}</span>
                  <span class="file-meta">
                    {job.format.toUpperCase()} • {formatFileSize(job.size)}
                  </span>
                </div>
              </div>
              
              <div class="status-badge" style="background-color: {getStatusColor(job.status)}">
                <span class="status-icon">{getStatusIcon(job.status)}</span>
                <span class="status-text">{job.status.toUpperCase()}</span>
              </div>
            </div>

            <div class="item-details">
              <div class="timestamp">
                <span class="timestamp-label">Creado:</span>
                <span class="timestamp-value">{formatTimestamp(job.timestamp)}</span>
              </div>

              {#if job.status === 'failed' && job.error}
                <div class="error-message">
                  <span class="error-icon">⚠️</span>
                  <span class="error-text">{job.error}</span>
                </div>
              {/if}

              {#if job.status === 'processing'}
                <div class="processing-indicator">
                  <div class="processing-spinner"></div>
                  <span>Procesando exportación...</span>
                </div>
              {/if}
            </div>

            <div class="item-actions">
              {#if job.status === 'completed' && job.downloadUrl}
                <button 
                  onclick={() => downloadFile(job)}
                  class="download-btn"
                  title="Descargar archivo"
                >
                  📥 Descargar
                </button>
              {:else if job.status === 'failed'}
                <button 
                  class="retry-btn disabled"
                  disabled
                  title="Reintentar exportación"
                >
                  🔄 Reintentar
                </button>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
{/if}

<style>
  .export-history {
    background: var(--surface-color);
    border-radius: 12px;
    border: 1px solid var(--border-color);
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .history-header {
    background: linear-gradient(135deg, #6366f1 0%, #4f46e5 100%);
    color: white;
    padding: 1.5rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-radius: 12px 12px 0 0;
  }

  .history-header h3 {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 600;
  }

  .header-actions {
    display: flex;
    gap: 0.5rem;
  }

  .refresh-btn,
  .clear-btn {
    width: 36px;
    height: 36px;
    border: none;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.2);
    color: white;
    cursor: pointer;
    transition: all 0.2s ease;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 1rem;
  }

  .refresh-btn:hover,
  .clear-btn:hover {
    background: rgba(255, 255, 255, 0.3);
    transform: scale(1.05);
  }

  .no-history {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 4rem 2rem;
    text-align: center;
    color: var(--text-secondary);
    flex: 1;
  }

  .no-history-icon {
    font-size: 3rem;
    margin-bottom: 1rem;
    opacity: 0.5;
  }

  .no-history h4 {
    margin: 0 0 0.5rem 0;
    color: var(--text-primary);
    font-size: 1.25rem;
    font-weight: 600;
  }

  .no-history p {
    margin: 0;
    font-size: 1rem;
  }

  .history-stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(100px, 1fr));
    gap: 1rem;
    padding: 1.5rem;
    background: var(--background-secondary);
    border-bottom: 1px solid var(--border-color);
  }

  .stat-item {
    text-align: center;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .stat-value {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--primary-color);
    font-family: 'Courier New', monospace;
  }

  .stat-label {
    font-size: 0.75rem;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-weight: 500;
  }

  .history-list {
    flex: 1;
    overflow-y: auto;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .history-item {
    background: var(--background-primary);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 1.5rem;
    transition: all 0.2s ease;
  }

  .history-item:hover {
    border-color: var(--primary-color);
    box-shadow: 0 2px 8px rgba(99, 102, 241, 0.1);
  }

  .history-item.failed {
    border-color: #ef4444;
    background: rgba(239, 68, 68, 0.05);
  }

  .item-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
  }

  .file-info {
    display: flex;
    align-items: center;
    gap: 1rem;
    flex: 1;
  }

  .format-icon {
    font-size: 1.5rem;
    flex-shrink: 0;
  }

  .file-details {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    min-width: 0;
  }

  .filename {
    font-weight: 600;
    color: var(--text-primary);
    font-family: 'Courier New', monospace;
    font-size: 0.9rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .file-meta {
    font-size: 0.8rem;
    color: var(--text-secondary);
  }

  .status-badge {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    border-radius: 16px;
    color: white;
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
  }

  .status-icon {
    font-size: 0.9rem;
  }

  .item-details {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    margin-bottom: 1rem;
  }

  .timestamp {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.85rem;
  }

  .timestamp-label {
    color: var(--text-secondary);
    font-weight: 500;
  }

  .timestamp-value {
    color: var(--text-primary);
    font-family: 'Courier New', monospace;
  }

  .error-message {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    padding: 0.75rem;
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.3);
    border-radius: 6px;
  }

  .error-text {
    color: #dc2626;
    font-size: 0.85rem;
    line-height: 1.4;
  }

  .processing-indicator {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem;
    background: rgba(59, 130, 246, 0.1);
    border: 1px solid rgba(59, 130, 246, 0.3);
    border-radius: 6px;
    color: #1d4ed8;
    font-size: 0.85rem;
  }

  .processing-spinner {
    width: 16px;
    height: 16px;
    border: 2px solid rgba(59, 130, 246, 0.3);
    border-top: 2px solid #3b82f6;
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  .item-actions {
    display: flex;
    gap: 0.75rem;
    justify-content: flex-end;
  }

  .download-btn,
  .retry-btn {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 6px;
    font-size: 0.85rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s ease;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .download-btn {
    background: #10b981;
    color: white;
  }

  .download-btn:hover {
    background: #059669;
    transform: translateY(-1px);
  }

  .retry-btn {
    background: var(--text-secondary);
    color: white;
  }

  .retry-btn:hover:not(:disabled) {
    background: var(--text-primary);
    transform: translateY(-1px);
  }

  .retry-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    transform: none;
  }

  /* Responsive */
  @media (max-width: 768px) {
    .history-header {
      flex-direction: column;
      gap: 1rem;
      text-align: center;
    }

    .history-stats {
      grid-template-columns: repeat(3, 1fr);
      gap: 0.75rem;
      padding: 1rem;
    }

    .stat-value {
      font-size: 1.25rem;
    }

    .item-header {
      flex-direction: column;
      align-items: flex-start;
      gap: 1rem;
    }

    .file-info {
      width: 100%;
    }

    .status-badge {
      align-self: flex-start;
    }

    .timestamp {
      flex-direction: column;
      align-items: flex-start;
      gap: 0.25rem;
    }

    .item-actions {
      justify-content: stretch;
    }

    .download-btn,
    .retry-btn {
      flex: 1;
      justify-content: center;
    }
  }
</style>