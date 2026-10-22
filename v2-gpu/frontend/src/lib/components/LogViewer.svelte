<!-- Log Viewer - Visualización y gestión de logs del sistema -->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { logger, type LogEntry, type LogLevel, type LogFilter, type LogStatistics } from '../utils/Logger.js';

  interface Props {
    visible?: boolean;
  }

  let {
    visible = false
  }: Props = $props();

  let logs = $state<LogEntry[]>([]);
  let filteredLogs = $state<LogEntry[]>([]);
  let statistics = $state<LogStatistics | null>(null);
  let refreshInterval: number | null = null;

  // Filtros
  let levelFilter = $state<LogLevel[]>(['INFO', 'WARN', 'ERROR']);
  let moduleFilter = $state<string[]>([]);
  let searchTerm = $state('');
  let showOnlyErrors = $state(false);
  let autoScroll = $state(true);
  let maxDisplayLogs = $state(100);

  // Estado de la interfaz
  let selectedTab = $state<'logs' | 'stats' | 'config'>('logs');
  let expandedLog = $state<string | null>(null);
  let availableModules = $state<string[]>([]);

  // Configuración del logger
  let loggerConfig = $state(logger.getConfig());

  onMount(() => {
    refreshData();
    
    // Auto-refresh cada 2 segundos
    refreshInterval = setInterval(refreshData, 2000);
  });

  onDestroy(() => {
    if (refreshInterval) {
      clearInterval(refreshInterval);
    }
  });

  function refreshData() {
    logs = logger.getLogs();
    statistics = logger.getStatistics();
    
    // Extraer módulos únicos
    const modules = new Set<string>();
    logs.forEach(log => {
      if (log.module) modules.add(log.module);
    });
    availableModules = Array.from(modules).sort();
    
    applyFilters();
  }

  function applyFilters() {
    const filter: LogFilter = {
      level: showOnlyErrors ? ['ERROR'] : levelFilter.length > 0 ? levelFilter : undefined,
      module: moduleFilter.length > 0 ? moduleFilter : undefined,
      searchTerm: searchTerm.trim() || undefined
    };

    filteredLogs = logger.filterLogs(filter).slice(-maxDisplayLogs);
    
    // Auto-scroll si está habilitado
    if (autoScroll) {
      setTimeout(scrollToBottom, 100);
    }
  }

  function scrollToBottom() {
    const logContainer = document.querySelector('.log-entries');
    if (logContainer) {
      logContainer.scrollTop = logContainer.scrollHeight;
    }
  }

  function toggleLevel(level: LogLevel) {
    if (levelFilter.includes(level)) {
      levelFilter = levelFilter.filter(l => l !== level);
    } else {
      levelFilter = [...levelFilter, level];
    }
    applyFilters();
  }

  function toggleModule(module: string) {
    if (moduleFilter.includes(module)) {
      moduleFilter = moduleFilter.filter(m => m !== module);
    } else {
      moduleFilter = [...moduleFilter, module];
    }
    applyFilters();
  }

  function clearLogs() {
    logger.clearLogs();
    refreshData();
  }

  function exportLogs() {
    const filter: LogFilter = {
      level: showOnlyErrors ? ['ERROR'] : levelFilter.length > 0 ? levelFilter : undefined,
      module: moduleFilter.length > 0 ? moduleFilter : undefined,
      searchTerm: searchTerm.trim() || undefined
    };

    const exportData = logger.exportLogs(filter);
    const blob = new Blob([exportData], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    
    const a = document.createElement('a');
    a.href = url;
    a.download = `docking_logs_${new Date().toISOString().slice(0, 19).replace(/:/g, '-')}.json`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  }

  function updateLoggerConfig() {
    logger.updateConfig(loggerConfig);
    refreshData();
  }

  function formatTimestamp(timestamp: Date): string {
    return new Intl.DateTimeFormat('es-ES', {
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
      fractionalSecondDigits: 3
    }).format(timestamp);
  }

  function formatDuration(ms: number): string {
    if (ms < 1000) return `${ms.toFixed(1)}ms`;
    if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
    return `${(ms / 60000).toFixed(1)}min`;
  }

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
  }

  function getLevelIcon(level: LogLevel): string {
    switch (level) {
      case 'DEBUG': return '🐛';
      case 'INFO': return 'ℹ️';
      case 'WARN': return '⚠️';
      case 'ERROR': return '❌';
      default: return '📝';
    }
  }

  function getLevelColor(level: LogLevel): string {
    switch (level) {
      case 'DEBUG': return '#6b7280';
      case 'INFO': return '#3b82f6';
      case 'WARN': return '#f59e0b';
      case 'ERROR': return '#ef4444';
      default: return '#6b7280';
    }
  }

  function toggleLogExpansion(logId: string) {
    expandedLog = expandedLog === logId ? null : logId;
  }

  // Aplicar filtros cuando cambien
  $effect(() => {
    if (searchTerm !== undefined) {
      applyFilters();
    }
  });

  $effect(() => {
    if (showOnlyErrors !== undefined) {
      applyFilters();
    }
  });
</script>

{#if visible}
  <div class="log-viewer">
    <div class="viewer-header">
      <h3>📋 System Logs</h3>
      <div class="header-actions">
        <button onclick={refreshData} class="action-btn" title="Refresh">
          🔄
        </button>
        <button onclick={exportLogs} class="action-btn" title="Export Logs">
          📥
        </button>
        <button onclick={clearLogs} class="action-btn danger" title="Clear Logs">
          🗑️
        </button>
      </div>
    </div>

    <!-- Navigation Tabs -->
    <nav class="log-tabs">
      <button 
        class="tab-btn"
        class:active={selectedTab === 'logs'}
        onclick={() => selectedTab = 'logs'}
      >
        📄 Logs ({filteredLogs.length})
      </button>
      <button 
        class="tab-btn"
        class:active={selectedTab === 'stats'}
        onclick={() => selectedTab = 'stats'}
      >
        📊 Statistics
      </button>
      <button 
        class="tab-btn"
        class:active={selectedTab === 'config'}
        onclick={() => selectedTab = 'config'}
      >
        ⚙️ Settings
      </button>
    </nav>

    <!-- Tab Content -->
    <div class="tab-content">
      {#if selectedTab === 'logs'}
        <div class="logs-section">
          <!-- Filters -->
          <div class="filters-panel">
            <div class="filter-group">
              <label>Search:</label>
              <input 
                type="text" 
                bind:value={searchTerm} 
                placeholder="Search in logs..."
                class="search-input"
              >
            </div>

            <div class="filter-group">
              <label>Levels:</label>
              <div class="level-filters">
                {#each ['DEBUG', 'INFO', 'WARN', 'ERROR'] as level}
                  <button 
                    class="level-filter-btn"
                    class:active={levelFilter.includes(level)}
                    style="border-color: {getLevelColor(level)}"
                    onclick={() => toggleLevel(level)}
                  >
                    {getLevelIcon(level)} {level}
                  </button>
                {/each}
              </div>
            </div>

            {#if availableModules.length > 0}
              <div class="filter-group">
                <label>Modules:</label>
                <div class="module-filters">
                  {#each availableModules.slice(0, 8) as module}
                    <button 
                      class="module-filter-btn"
                      class:active={moduleFilter.includes(module)}
                      onclick={() => toggleModule(module)}
                    >
                      {module}
                    </button>
                  {/each}
                </div>
              </div>
            {/if}

            <div class="filter-group">
              <label class="checkbox-label">
                <input type="checkbox" bind:checked={showOnlyErrors}>
                Errors Only
              </label>
              <label class="checkbox-label">
                <input type="checkbox" bind:checked={autoScroll}>
                Auto Scroll
              </label>
            </div>

            <div class="filter-group">
              <label>
                Max Logs: 
                <input 
                  type="number" 
                  bind:value={maxDisplayLogs} 
                  min="10" 
                  max="1000" 
                  step="10"
                  class="number-input"
                >
              </label>
            </div>
          </div>

          <!-- Log Entries -->
          <div class="log-entries">
            {#if filteredLogs.length === 0}
              <div class="no-logs">
                <div class="no-logs-icon">📝</div>
                <h4>No logs found</h4>
                <p>No logs match the current filters</p>
              </div>
            {:else}
              {#each filteredLogs as log (log.id)}
                <div class="log-entry" class:expanded={expandedLog === log.id}>
                  <div class="log-header" onclick={() => toggleLogExpansion(log.id)}>
                    <div class="log-meta">
                      <span 
                        class="log-level"
                        style="color: {getLevelColor(log.level)}"
                      >
                        {getLevelIcon(log.level)}
                      </span>
                      <span class="log-time">{formatTimestamp(log.timestamp)}</span>
                      {#if log.module}
                        <span class="log-module">[{log.module}]</span>
                      {/if}
                    </div>
                    <div class="log-message">{log.message}</div>
                    {#if log.data || log.error || log.performance}
                      <button class="expand-btn">
                        {expandedLog === log.id ? '▼' : '▶'}
                      </button>
                    {/if}
                  </div>

                  {#if expandedLog === log.id}
                    <div class="log-details">
                      {#if log.performance}
                        <div class="detail-section">
                          <h5>Performance:</h5>
                          <div class="performance-info">
                            {#if log.performance.duration}
                              <span>Duration: {formatDuration(log.performance.duration)}</span>
                            {/if}
                            {#if log.performance.memory}
                              <span>Memory: {formatBytes(log.performance.memory)}</span>
                            {/if}
                            {#if log.performance.operation}
                              <span>Operation: {log.performance.operation}</span>
                            {/if}
                          </div>
                        </div>
                      {/if}

                      {#if log.data}
                        <div class="detail-section">
                          <h5>Data:</h5>
                          <pre class="log-data">{JSON.stringify(log.data, null, 2)}</pre>
                        </div>
                      {/if}

                      {#if log.error}
                        <div class="detail-section">
                          <h5>Error:</h5>
                          <div class="error-info">
                            <strong>{log.error.name}:</strong> {log.error.message}
                          </div>
                          {#if log.stackTrace}
                            <pre class="stack-trace">{log.stackTrace}</pre>
                          {/if}
                        </div>
                      {/if}

                      <div class="detail-section">
                        <h5>Context:</h5>
                        <div class="context-info">
                          <span>Session: {log.sessionId}</span>
                          <span>Context: {log.context || 'N/A'}</span>
                          <span>ID: {log.id}</span>
                        </div>
                      </div>
                    </div>
                  {/if}
                </div>
              {/each}
            {/if}
          </div>
        </div>

      {:else if selectedTab === 'stats' && statistics}
        <div class="stats-section">
          <div class="stats-grid">
            <div class="stat-card">
              <h4>Total Entries</h4>
              <div class="stat-value">{statistics.totalEntries}</div>
            </div>
            
            <div class="stat-card">
              <h4>Session Duration</h4>
              <div class="stat-value">{formatDuration(statistics.sessionDuration)}</div>
            </div>

            <div class="stat-card">
              <h4>Avg Memory</h4>
              <div class="stat-value">{formatBytes(statistics.averageMemoryUsage)}</div>
            </div>

            <div class="stat-card">
              <h4>Errors</h4>
              <div class="stat-value error">{statistics.byLevel.ERROR}</div>
            </div>
          </div>

          <div class="charts-section">
            <div class="chart-card">
              <h4>By Level</h4>
              <div class="level-chart">
                {#each Object.entries(statistics.byLevel) as [level, count]}
                  <div class="chart-bar">
                    <span class="chart-label">{level}</span>
                    <div class="chart-bar-container">
                      <div 
                        class="chart-bar-fill"
                        style="width: {(count / statistics.totalEntries) * 100}%; background-color: {getLevelColor(level)}"
                      ></div>
                    </div>
                    <span class="chart-value">{count}</span>
                  </div>
                {/each}
              </div>
            </div>

            <div class="chart-card">
              <h4>By Module</h4>
              <div class="module-chart">
                {#each Object.entries(statistics.byModule).slice(0, 10) as [module, count]}
                  <div class="chart-bar">
                    <span class="chart-label">{module}</span>
                    <div class="chart-bar-container">
                      <div 
                        class="chart-bar-fill"
                        style="width: {(count / statistics.totalEntries) * 100}%; background-color: #3b82f6"
                      ></div>
                    </div>
                    <span class="chart-value">{count}</span>
                  </div>
                {/each}
              </div>
            </div>
          </div>

          {#if statistics.errors.length > 0}
            <div class="recent-errors">
              <h4>Recent Errors</h4>
              <div class="error-list">
                {#each statistics.errors.slice(-5) as error}
                  <div class="error-item">
                    <span class="error-time">{formatTimestamp(error.timestamp)}</span>
                    <span class="error-module">[{error.module || 'Unknown'}]</span>
                    <span class="error-message">{error.message}</span>
                  </div>
                {/each}
              </div>
            </div>
          {/if}
        </div>

      {:else if selectedTab === 'config'}
        <div class="config-section">
          <h4>Logger Configuration</h4>
          
          <div class="config-form">
            <div class="form-group">
              <label>Log Level:</label>
              <select bind:value={loggerConfig.level}>
                <option value="DEBUG">DEBUG</option>
                <option value="INFO">INFO</option>
                <option value="WARN">WARN</option>
                <option value="ERROR">ERROR</option>
              </select>
            </div>

            <div class="form-group">
              <label class="checkbox-label">
                <input type="checkbox" bind:checked={loggerConfig.enableConsole}>
                Enable Console Logging
              </label>
            </div>

            <div class="form-group">
              <label class="checkbox-label">
                <input type="checkbox" bind:checked={loggerConfig.enableStorage}>
                Enable Storage
              </label>
            </div>

            <div class="form-group">
              <label class="checkbox-label">
                <input type="checkbox" bind:checked={loggerConfig.enablePerformanceTracking}>
                Enable Performance Tracking
              </label>
            </div>

            <div class="form-group">
              <label class="checkbox-label">
                <input type="checkbox" bind:checked={loggerConfig.enableStackTraces}>
                Enable Stack Traces
              </label>
            </div>

            <div class="form-group">
              <label>
                Max Storage Entries:
                <input 
                  type="number" 
                  bind:value={loggerConfig.maxStorageEntries} 
                  min="100" 
                  max="10000" 
                  step="100"
                >
              </label>
            </div>

            <button onclick={updateLoggerConfig} class="update-config-btn">
              Update Configuration
            </button>
          </div>

          <div class="session-info">
            <h4>Session Information</h4>
            {#if logger.getSessionInfo()}
              {@const sessionInfo = logger.getSessionInfo()}
              <div class="session-details">
                <div class="session-item">
                  <strong>Session ID:</strong> 
                  <code>{sessionInfo.sessionId}</code>
                </div>
                <div class="session-item">
                  <strong>Start Time:</strong> 
                  {sessionInfo.startTime.toLocaleString()}
                </div>
                <div class="session-item">
                  <strong>Duration:</strong> 
                  {formatDuration(sessionInfo.duration)}
                </div>
              </div>
            {/if}
          </div>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .log-viewer {
    background: var(--surface-color);
    border-radius: 12px;
    border: 1px solid var(--border-color);
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .viewer-header {
    background: linear-gradient(135deg, #1f2937 0%, #111827 100%);
    color: white;
    padding: 1.5rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .viewer-header h3 {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 600;
  }

  .header-actions {
    display: flex;
    gap: 0.5rem;
  }

  .action-btn {
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
  }

  .action-btn:hover {
    background: rgba(255, 255, 255, 0.3);
    transform: scale(1.05);
  }

  .action-btn.danger:hover {
    background: #ef4444;
  }

  .log-tabs {
    display: flex;
    background: var(--background-secondary);
    border-bottom: 1px solid var(--border-color);
  }

  .tab-btn {
    flex: 1;
    padding: 1rem;
    background: none;
    border: none;
    color: var(--text-secondary);
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s ease;
    border-bottom: 2px solid transparent;
  }

  .tab-btn:hover {
    color: var(--text-primary);
    background: var(--surface-color);
  }

  .tab-btn.active {
    color: var(--primary-color);
    border-bottom-color: var(--primary-color);
    background: var(--surface-color);
  }

  .tab-content {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  /* Logs Section */
  .logs-section {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .filters-panel {
    padding: 1rem;
    background: var(--background-secondary);
    border-bottom: 1px solid var(--border-color);
    display: flex;
    flex-wrap: wrap;
    gap: 1rem;
    align-items: center;
  }

  .filter-group {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.9rem;
  }

  .filter-group label {
    font-weight: 500;
    color: var(--text-primary);
  }

  .search-input {
    padding: 0.5rem;
    border: 1px solid var(--border-color);
    border-radius: 4px;
    width: 200px;
  }

  .level-filters {
    display: flex;
    gap: 0.25rem;
  }

  .level-filter-btn {
    padding: 0.25rem 0.5rem;
    border: 1px solid;
    border-radius: 4px;
    background: var(--background-primary);
    color: var(--text-primary);
    cursor: pointer;
    font-size: 0.8rem;
    transition: all 0.2s ease;
  }

  .level-filter-btn.active {
    background: var(--primary-background);
  }

  .module-filters {
    display: flex;
    gap: 0.25rem;
    flex-wrap: wrap;
  }

  .module-filter-btn {
    padding: 0.25rem 0.5rem;
    border: 1px solid var(--border-color);
    border-radius: 4px;
    background: var(--background-primary);
    color: var(--text-primary);
    cursor: pointer;
    font-size: 0.8rem;
  }

  .module-filter-btn.active {
    background: var(--primary-color);
    color: white;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    cursor: pointer;
  }

  .number-input {
    width: 80px;
    padding: 0.25rem;
    border: 1px solid var(--border-color);
    border-radius: 4px;
  }

  .log-entries {
    flex: 1;
    overflow-y: auto;
    padding: 1rem;
  }

  .no-logs {
    text-align: center;
    padding: 4rem 2rem;
    color: var(--text-secondary);
  }

  .no-logs-icon {
    font-size: 3rem;
    margin-bottom: 1rem;
  }

  .log-entry {
    margin-bottom: 0.5rem;
    border: 1px solid var(--border-color);
    border-radius: 6px;
    background: var(--background-primary);
    transition: all 0.2s ease;
  }

  .log-entry:hover {
    border-color: var(--primary-color);
  }

  .log-entry.expanded {
    border-color: var(--primary-color);
    box-shadow: 0 2px 8px rgba(59, 130, 246, 0.1);
  }

  .log-header {
    padding: 0.75rem;
    display: flex;
    align-items: center;
    gap: 1rem;
    cursor: pointer;
  }

  .log-meta {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.85rem;
    font-family: 'Courier New', monospace;
    flex-shrink: 0;
  }

  .log-level {
    font-size: 1rem;
  }

  .log-time {
    color: var(--text-secondary);
  }

  .log-module {
    color: var(--primary-color);
    font-weight: 600;
  }

  .log-message {
    flex: 1;
    color: var(--text-primary);
    font-weight: 500;
  }

  .expand-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 0.25rem;
    border-radius: 4px;
  }

  .expand-btn:hover {
    background: var(--background-secondary);
  }

  .log-details {
    padding: 1rem;
    border-top: 1px solid var(--border-color);
    background: var(--background-secondary);
  }

  .detail-section {
    margin-bottom: 1rem;
  }

  .detail-section h5 {
    margin: 0 0 0.5rem 0;
    color: var(--text-primary);
    font-size: 0.9rem;
    font-weight: 600;
  }

  .performance-info {
    display: flex;
    gap: 1rem;
    font-size: 0.85rem;
    color: var(--text-secondary);
  }

  .log-data {
    background: var(--background-primary);
    padding: 0.75rem;
    border-radius: 4px;
    font-size: 0.8rem;
    overflow-x: auto;
    max-height: 200px;
    overflow-y: auto;
  }

  .error-info {
    color: #dc2626;
    font-weight: 500;
  }

  .stack-trace {
    background: #fef2f2;
    border: 1px solid #fecaca;
    padding: 0.75rem;
    border-radius: 4px;
    font-size: 0.8rem;
    color: #991b1b;
    overflow-x: auto;
    max-height: 200px;
    overflow-y: auto;
    margin-top: 0.5rem;
  }

  .context-info {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.85rem;
    color: var(--text-secondary);
  }

  /* Stats Section */
  .stats-section {
    padding: 2rem;
    overflow-y: auto;
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1rem;
    margin-bottom: 2rem;
  }

  .stat-card {
    background: var(--background-primary);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 1.5rem;
    text-align: center;
  }

  .stat-card h4 {
    margin: 0 0 0.5rem 0;
    color: var(--text-secondary);
    font-size: 0.9rem;
    text-transform: uppercase;
  }

  .stat-value {
    font-size: 2rem;
    font-weight: 700;
    color: var(--primary-color);
    font-family: 'Courier New', monospace;
  }

  .stat-value.error {
    color: #ef4444;
  }

  .charts-section {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 2rem;
    margin-bottom: 2rem;
  }

  .chart-card {
    background: var(--background-primary);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 1.5rem;
  }

  .chart-card h4 {
    margin: 0 0 1rem 0;
    color: var(--text-primary);
  }

  .chart-bar {
    display: grid;
    grid-template-columns: 80px 1fr 40px;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }

  .chart-label {
    font-size: 0.85rem;
    font-weight: 500;
  }

  .chart-bar-container {
    height: 20px;
    background: var(--background-secondary);
    border-radius: 4px;
    overflow: hidden;
  }

  .chart-bar-fill {
    height: 100%;
    transition: width 0.3s ease;
  }

  .chart-value {
    font-size: 0.85rem;
    font-weight: 600;
    text-align: right;
  }

  .recent-errors {
    background: var(--background-primary);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 1.5rem;
  }

  .recent-errors h4 {
    margin: 0 0 1rem 0;
    color: #ef4444;
  }

  .error-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .error-item {
    display: grid;
    grid-template-columns: 100px 120px 1fr;
    gap: 0.5rem;
    padding: 0.5rem;
    background: var(--background-secondary);
    border-radius: 4px;
    font-size: 0.85rem;
  }

  .error-time {
    color: var(--text-secondary);
    font-family: 'Courier New', monospace;
  }

  .error-module {
    color: var(--primary-color);
    font-weight: 600;
  }

  .error-message {
    color: #dc2626;
  }

  /* Config Section */
  .config-section {
    padding: 2rem;
    overflow-y: auto;
  }

  .config-form {
    background: var(--background-primary);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 1.5rem;
    margin-bottom: 2rem;
  }

  .form-group {
    margin-bottom: 1rem;
  }

  .form-group label {
    display: block;
    margin-bottom: 0.25rem;
    font-weight: 500;
    color: var(--text-primary);
  }

  .form-group select,
  .form-group input[type="number"] {
    padding: 0.5rem;
    border: 1px solid var(--border-color);
    border-radius: 4px;
    background: var(--surface-color);
    color: var(--text-primary);
  }

  .update-config-btn {
    background: var(--primary-color);
    color: white;
    border: none;
    padding: 0.75rem 1.5rem;
    border-radius: 6px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .update-config-btn:hover {
    background: var(--primary-hover);
    transform: translateY(-1px);
  }

  .session-info {
    background: var(--background-primary);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 1.5rem;
  }

  .session-info h4 {
    margin: 0 0 1rem 0;
    color: var(--text-primary);
  }

  .session-details {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .session-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.9rem;
  }

  .session-item code {
    background: var(--background-secondary);
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    font-family: 'Courier New', monospace;
    font-size: 0.8rem;
  }

  /* Responsive */
  @media (max-width: 768px) {
    .filters-panel {
      flex-direction: column;
      align-items: stretch;
    }

    .filter-group {
      flex-direction: column;
      align-items: stretch;
    }

    .search-input {
      width: 100%;
    }

    .stats-grid {
      grid-template-columns: repeat(2, 1fr);
    }

    .charts-section {
      grid-template-columns: 1fr;
    }

    .chart-bar {
      grid-template-columns: 60px 1fr 30px;
    }

    .error-item {
      grid-template-columns: 1fr;
      gap: 0.25rem;
    }
  }
</style>