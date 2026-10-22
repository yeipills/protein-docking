<!-- Report Viewer - Generación y visualización de reportes de docking -->
<script lang="ts">
  import { onMount } from 'svelte';
  import type { DockingResult } from '../algorithms/DockingAlgorithm.js';
  import type { PDBStructure } from '../utils/PDBParser.js';
  import { ReportGenerator, type ReportData, type ReportOptions } from '../utils/ReportGenerator.js';

  interface Props {
    dockingResult: DockingResult | null;
    receptor: PDBStructure | null;
    ligand: PDBStructure | null;
    visible?: boolean;
  }

  let {
    dockingResult,
    receptor,
    ligand,
    visible = false
  }: Props = $props();

  let reportData = $state<ReportData | null>(null);
  let isGenerating = $state(false);
  let reportOptions = $state<ReportOptions>({
    includeAllPoses: false,
    maxPosesInReport: 10,
    includeEnergyBreakdown: true,
    includeScoringDetails: true,
    includeVisualizations: true,
    includeRecommendations: true,
    format: 'html',
    language: 'es'
  });

  let selectedTab = $state<'overview' | 'analysis' | 'recommendations' | 'export'>('overview');
  let exportFormat = $state<'html' | 'json' | 'csv'>('html');
  let exportedContent = $state<string>('');

  const reportGenerator = new ReportGenerator();

  // Auto-generate report when docking result changes
  $effect(() => {
    if (dockingResult && visible) {
      generateReport();
    }
  });

  async function generateReport() {
    if (!dockingResult) return;

    isGenerating = true;
    try {
      reportGenerator.setOptions?.(reportOptions) || new ReportGenerator(reportOptions);
      reportData = await reportGenerator.generateReport(dockingResult, receptor, ligand);
      console.log('📋 Report generated successfully');
    } catch (error) {
      console.error('Error generating report:', error);
    } finally {
      isGenerating = false;
    }
  }

  function updateOptions() {
    generateReport();
  }

  function exportReport() {
    if (!reportData) return;

    try {
      switch (exportFormat) {
        case 'html':
          exportedContent = reportGenerator.exportAsHTML(reportData);
          break;
        case 'json':
          exportedContent = reportGenerator.exportAsJSON(reportData);
          break;
        case 'csv':
          exportedContent = reportGenerator.exportAsCSV(reportData);
          break;
      }
      
      // Download file
      const blob = new Blob([exportedContent], { 
        type: exportFormat === 'html' ? 'text/html' : 
             exportFormat === 'json' ? 'application/json' : 'text/csv' 
      });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `docking_report_${reportData.metadata.reportId}.${exportFormat}`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      
      console.log(`📄 Report exported as ${exportFormat.toUpperCase()}`);
    } catch (error) {
      console.error('Error exporting report:', error);
    }
  }

  function copyToClipboard() {
    if (exportedContent) {
      navigator.clipboard.writeText(exportedContent);
      console.log('📋 Report copied to clipboard');
    }
  }

  function getQualityColor(quality: string): string {
    switch (quality) {
      case 'excellent': return '#10b981';
      case 'good': return '#059669';
      case 'fair': return '#f59e0b';
      case 'poor': return '#ef4444';
      default: return '#6b7280';
    }
  }

  function getPriorityColor(priority: string): string {
    switch (priority) {
      case 'high': return '#dc2626';
      case 'medium': return '#f59e0b';
      case 'low': return '#10b981';
      default: return '#6b7280';
    }
  }

  function formatNumber(num: number, decimals: number = 2): string {
    return num.toFixed(decimals);
  }

  function formatPercentage(num: number): string {
    return (num * 100).toFixed(1) + '%';
  }
</script>

{#if visible}
  <div class="report-viewer">
    <div class="report-header">
      <h2>📋 Reporte de Resultados</h2>
      {#if dockingResult}
        <div class="header-info">
          <span class="info-item">
            {dockingResult.poses.length} poses
          </span>
          <span class="info-item">
            {formatNumber(dockingResult.computation_time_ms / 1000)} s
          </span>
          <span class="info-item">
            {dockingResult.algorithm}
          </span>
        </div>
      {/if}
    </div>

    {#if !dockingResult}
      <div class="no-data">
        <div class="no-data-icon">📄</div>
        <h3>No hay datos disponibles</h3>
        <p>Complete un cálculo de docking para generar un reporte</p>
      </div>
    {:else if isGenerating}
      <div class="loading-state">
        <div class="loading-spinner"></div>
        <h3>Generando reporte...</h3>
        <p>Analizando resultados y compilando información</p>
      </div>
    {:else if reportData}
      <div class="report-content">
        <!-- Navigation Tabs -->
        <nav class="report-tabs">
          <button 
            class="tab-btn"
            class:active={selectedTab === 'overview'}
            onclick={() => selectedTab = 'overview'}
          >
            📊 Resumen
          </button>
          <button 
            class="tab-btn"
            class:active={selectedTab === 'analysis'}
            onclick={() => selectedTab = 'analysis'}
          >
            🔬 Análisis
          </button>
          <button 
            class="tab-btn"
            class:active={selectedTab === 'recommendations'}
            onclick={() => selectedTab = 'recommendations'}
          >
            💡 Recomendaciones
          </button>
          <button 
            class="tab-btn"
            class:active={selectedTab === 'export'}
            onclick={() => selectedTab = 'export'}
          >
            📤 Exportar
          </button>
        </nav>

        <!-- Tab Content -->
        <div class="tab-content">
          {#if selectedTab === 'overview'}
            <div class="overview-section">
              <!-- Executive Summary -->
              <div class="summary-card">
                <h3>Resumen Ejecutivo</h3>
                <div class="quality-assessment">
                  <div class="quality-badge" style="background-color: {getQualityColor(reportData.executiveSummary.qualityAssessment.overall)}">
                    {reportData.executiveSummary.qualityAssessment.overall.toUpperCase()}
                  </div>
                  <div class="quality-score">
                    {reportData.executiveSummary.qualityAssessment.score}/100
                  </div>
                </div>
                <p class="summary-text">{reportData.executiveSummary.overview}</p>
                <div class="quality-reasoning">
                  <strong>Evaluación:</strong> {reportData.executiveSummary.qualityAssessment.reasoning}
                </div>
              </div>

              <!-- Key Findings -->
              <div class="findings-card">
                <h3>Hallazgos Principales</h3>
                <ul class="findings-list">
                  {#each reportData.executiveSummary.keyFindings as finding}
                    <li>{finding}</li>
                  {/each}
                </ul>
              </div>

              <!-- Best Pose -->
              <div class="best-pose-card">
                <h3>Mejor Pose</h3>
                <div class="pose-metrics">
                  <div class="metric">
                    <span class="metric-label">ID:</span>
                    <span class="metric-value">{reportData.executiveSummary.bestPose.id}</span>
                  </div>
                  <div class="metric">
                    <span class="metric-label">Energía:</span>
                    <span class="metric-value energy">{formatNumber(reportData.executiveSummary.bestPose.energy)} kcal/mol</span>
                  </div>
                  <div class="metric">
                    <span class="metric-label">Consensus:</span>
                    <span class="metric-value score">{formatPercentage(reportData.executiveSummary.bestPose.consensusScore)}</span>
                  </div>
                  <div class="metric">
                    <span class="metric-label">Confianza:</span>
                    <span class="metric-value">{reportData.executiveSummary.bestPose.confidence}</span>
                  </div>
                </div>
                <p class="pose-summary">{reportData.executiveSummary.bestPose.summary}</p>
              </div>

              <!-- System Info -->
              <div class="system-info-card">
                <h3>Información del Sistema</h3>
                <div class="system-grid">
                  <div class="system-item">
                    <h4>Receptor</h4>
                    <p><strong>{reportData.systemInfo.receptor.name}</strong></p>
                    <p>{reportData.systemInfo.receptor.atoms} átomos, {reportData.systemInfo.receptor.residues} residuos</p>
                  </div>
                  <div class="system-item">
                    <h4>Ligando</h4>
                    <p><strong>{reportData.systemInfo.ligand.name}</strong></p>
                    <p>{reportData.systemInfo.ligand.atoms} átomos, ~{reportData.systemInfo.ligand.molecularWeight} Da</p>
                  </div>
                  <div class="system-item">
                    <h4>Algoritmo</h4>
                    <p><strong>{reportData.systemInfo.algorithm.name}</strong></p>
                    <p>Tiempo: {formatNumber(reportData.systemInfo.algorithm.computationTime / 1000)} s</p>
                  </div>
                </div>
              </div>
            </div>

          {:else if selectedTab === 'analysis'}
            <div class="analysis-section">
              <!-- Results Analysis -->
              <div class="results-card">
                <h3>Análisis de Resultados</h3>
                <div class="stats-grid">
                  <div class="stat-item">
                    <span class="stat-value">{reportData.resultsAnalysis.posesGenerated}</span>
                    <span class="stat-label">Poses Generadas</span>
                  </div>
                  <div class="stat-item">
                    <span class="stat-value">{reportData.resultsAnalysis.posesEvaluated}</span>
                    <span class="stat-label">Poses Evaluadas</span>
                  </div>
                  <div class="stat-item">
                    <span class="stat-value">{formatNumber(reportData.resultsAnalysis.successRate)}%</span>
                    <span class="stat-label">Tasa de Éxito</span>
                  </div>
                </div>

                <div class="energy-stats">
                  <h4>Distribución de Energías</h4>
                  <div class="range-info">
                    <span>Mín: {formatNumber(reportData.resultsAnalysis.energyRange.min)} kcal/mol</span>
                    <span>Media: {formatNumber(reportData.resultsAnalysis.energyRange.mean)} kcal/mol</span>
                    <span>Máx: {formatNumber(reportData.resultsAnalysis.energyRange.max)} kcal/mol</span>
                  </div>
                </div>

                <div class="quality-distribution">
                  <h4>Distribución de Calidad</h4>
                  <div class="quality-bars">
                    <div class="quality-bar excellent">
                      <span class="bar-label">Excelente</span>
                      <div class="bar" style="width: {(reportData.resultsAnalysis.qualityDistribution.excellent / dockingResult.poses.length) * 100}%"></div>
                      <span class="bar-count">{reportData.resultsAnalysis.qualityDistribution.excellent}</span>
                    </div>
                    <div class="quality-bar good">
                      <span class="bar-label">Bueno</span>
                      <div class="bar" style="width: {(reportData.resultsAnalysis.qualityDistribution.good / dockingResult.poses.length) * 100}%"></div>
                      <span class="bar-count">{reportData.resultsAnalysis.qualityDistribution.good}</span>
                    </div>
                    <div class="quality-bar fair">
                      <span class="bar-label">Regular</span>
                      <div class="bar" style="width: {(reportData.resultsAnalysis.qualityDistribution.fair / dockingResult.poses.length) * 100}%"></div>
                      <span class="bar-count">{reportData.resultsAnalysis.qualityDistribution.fair}</span>
                    </div>
                    <div class="quality-bar poor">
                      <span class="bar-label">Pobre</span>
                      <div class="bar" style="width: {(reportData.resultsAnalysis.qualityDistribution.poor / dockingResult.poses.length) * 100}%"></div>
                      <span class="bar-count">{reportData.resultsAnalysis.qualityDistribution.poor}</span>
                    </div>
                  </div>
                </div>
              </div>

              <!-- Top Poses Analysis -->
              <div class="poses-analysis-card">
                <h3>Análisis de Poses Principales</h3>
                <div class="poses-table">
                  {#each reportData.poseAnalysis.slice(0, 5) as poseAnalysis}
                    <div class="pose-row">
                      <div class="pose-header">
                        <span class="pose-rank">#{poseAnalysis.rank}</span>
                        <span class="pose-id">{poseAnalysis.pose.id}</span>
                        <span class="pose-energy">{formatNumber(poseAnalysis.pose.energy)} kcal/mol</span>
                      </div>
                      <div class="pose-details">
                        <div class="pose-strengths">
                          <strong>Fortalezas:</strong>
                          <ul>
                            {#each poseAnalysis.analysis.strengths as strength}
                              <li>{strength}</li>
                            {/each}
                          </ul>
                        </div>
                        {#if poseAnalysis.analysis.weaknesses.length > 0}
                          <div class="pose-weaknesses">
                            <strong>Debilidades:</strong>
                            <ul>
                              {#each poseAnalysis.analysis.weaknesses as weakness}
                                <li>{weakness}</li>
                              {/each}
                            </ul>
                          </div>
                        {/if}
                        <div class="pose-recommendation">
                          <strong>Recomendación:</strong> {poseAnalysis.analysis.recommendation}
                        </div>
                      </div>
                    </div>
                  {/each}
                </div>
              </div>
            </div>

          {:else if selectedTab === 'recommendations'}
            <div class="recommendations-section">
              <h3>Recomendaciones</h3>
              {#if reportData.recommendations.length > 0}
                <div class="recommendations-list">
                  {#each reportData.recommendations as recommendation}
                    <div class="recommendation-card">
                      <div class="rec-header">
                        <h4>{recommendation.title}</h4>
                        <div class="rec-badges">
                          <span class="priority-badge" style="background-color: {getPriorityColor(recommendation.priority)}">
                            {recommendation.priority}
                          </span>
                          <span class="type-badge">{recommendation.type}</span>
                        </div>
                      </div>
                      <p class="rec-description">{recommendation.description}</p>
                      <div class="rec-rationale">
                        <strong>Justificación:</strong> {recommendation.rationale}
                      </div>
                      <div class="rec-next-steps">
                        <strong>Próximos pasos:</strong>
                        <ol>
                          {#each recommendation.nextSteps as step}
                            <li>{step}</li>
                          {/each}
                        </ol>
                      </div>
                    </div>
                  {/each}
                </div>
              {:else}
                <div class="no-recommendations">
                  <p>No se generaron recomendaciones específicas para estos resultados.</p>
                </div>
              {/if}
            </div>

          {:else if selectedTab === 'export'}
            <div class="export-section">
              <h3>Exportar Reporte</h3>
              
              <!-- Export Options -->
              <div class="export-options">
                <h4>Opciones de Exportación</h4>
                <div class="options-grid">
                  <label class="option-item">
                    <input type="checkbox" bind:checked={reportOptions.includeAllPoses}>
                    Incluir todas las poses
                  </label>
                  <label class="option-item">
                    <input type="checkbox" bind:checked={reportOptions.includeEnergyBreakdown}>
                    Desglose de energía
                  </label>
                  <label class="option-item">
                    <input type="checkbox" bind:checked={reportOptions.includeScoringDetails}>
                    Detalles de scoring
                  </label>
                  <label class="option-item">
                    <input type="checkbox" bind:checked={reportOptions.includeRecommendations}>
                    Recomendaciones
                  </label>
                </div>
                
                <div class="numeric-options">
                  <label>
                    Máximo de poses en reporte:
                    <input type="number" bind:value={reportOptions.maxPosesInReport} min="1" max="100">
                  </label>
                </div>
                
                <button onclick={updateOptions} class="update-btn">
                  Actualizar Reporte
                </button>
              </div>

              <!-- Export Formats -->
              <div class="export-formats">
                <h4>Formato de Exportación</h4>
                <div class="format-options">
                  <label>
                    <input type="radio" bind:group={exportFormat} value="html">
                    HTML (Página web completa)
                  </label>
                  <label>
                    <input type="radio" bind:group={exportFormat} value="json">
                    JSON (Datos estructurados)
                  </label>
                  <label>
                    <input type="radio" bind:group={exportFormat} value="csv">
                    CSV (Tabla de poses)
                  </label>
                </div>
                
                <div class="export-actions">
                  <button onclick={exportReport} class="export-btn">
                    📥 Descargar {exportFormat.toUpperCase()}
                  </button>
                  {#if exportedContent}
                    <button onclick={copyToClipboard} class="copy-btn">
                      📋 Copiar al Portapapeles
                    </button>
                  {/if}
                </div>
              </div>

              <!-- Preview -->
              {#if exportedContent && exportFormat === 'html'}
                <div class="preview-section">
                  <h4>Vista Previa</h4>
                  <div class="preview-container">
                    <iframe srcdoc={exportedContent} title="Report Preview"></iframe>
                  </div>
                </div>
              {/if}
            </div>
          {/if}
        </div>
      </div>
    {/if}
  </div>
{/if}

<style>
  .report-viewer {
    background: var(--surface-color);
    border-radius: 12px;
    border: 1px solid var(--border-color);
    overflow: hidden;
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .report-header {
    background: linear-gradient(135deg, #3b82f6 0%, #1d4ed8 100%);
    color: white;
    padding: 1.5rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .report-header h2 {
    margin: 0;
    font-size: 1.5rem;
    font-weight: 600;
  }

  .header-info {
    display: flex;
    gap: 1rem;
    font-size: 0.9rem;
  }

  .info-item {
    background: rgba(255, 255, 255, 0.2);
    padding: 0.25rem 0.75rem;
    border-radius: 12px;
  }

  .no-data, .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 4rem 2rem;
    text-align: center;
    color: var(--text-secondary);
  }

  .no-data-icon {
    font-size: 4rem;
    margin-bottom: 1rem;
  }

  .loading-spinner {
    width: 40px;
    height: 40px;
    border: 4px solid rgba(59, 130, 246, 0.3);
    border-top: 4px solid var(--primary-color);
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin-bottom: 1rem;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  .report-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .report-tabs {
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
    padding: 2rem;
    overflow-y: auto;
  }

  /* Overview Section */
  .overview-section {
    display: grid;
    gap: 1.5rem;
  }

  .summary-card, .findings-card, .best-pose-card, .system-info-card {
    background: var(--background-primary);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 1.5rem;
  }

  .summary-card h3, .findings-card h3, .best-pose-card h3, .system-info-card h3 {
    margin: 0 0 1rem 0;
    color: var(--text-primary);
    font-size: 1.2rem;
  }

  .quality-assessment {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .quality-badge {
    color: white;
    padding: 0.5rem 1rem;
    border-radius: 6px;
    font-weight: 600;
    font-size: 0.85rem;
  }

  .quality-score {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--primary-color);
  }

  .summary-text {
    margin: 1rem 0;
    line-height: 1.6;
  }

  .quality-reasoning {
    background: var(--background-secondary);
    padding: 1rem;
    border-radius: 6px;
    font-size: 0.9rem;
  }

  .findings-list {
    margin: 0;
    padding-left: 1.5rem;
  }

  .findings-list li {
    margin-bottom: 0.5rem;
  }

  .pose-metrics {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .metric {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .metric-label {
    font-size: 0.85rem;
    color: var(--text-secondary);
    font-weight: 500;
  }

  .metric-value {
    font-weight: 600;
    font-size: 1.1rem;
  }

  .metric-value.energy {
    color: #dc2626;
  }

  .metric-value.score {
    color: #059669;
  }

  .pose-summary {
    background: var(--background-secondary);
    padding: 1rem;
    border-radius: 6px;
    margin: 0;
    font-size: 0.9rem;
  }

  .system-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1rem;
  }

  .system-item h4 {
    margin: 0 0 0.5rem 0;
    color: var(--primary-color);
    font-size: 1rem;
  }

  .system-item p {
    margin: 0.25rem 0;
    font-size: 0.9rem;
  }

  /* Analysis Section */
  .analysis-section {
    display: grid;
    gap: 1.5rem;
  }

  .results-card, .poses-analysis-card {
    background: var(--background-primary);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 1.5rem;
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 1rem;
    margin-bottom: 2rem;
  }

  .stat-item {
    text-align: center;
    padding: 1rem;
    background: var(--background-secondary);
    border-radius: 8px;
  }

  .stat-value {
    display: block;
    font-size: 2rem;
    font-weight: 700;
    color: var(--primary-color);
  }

  .stat-label {
    font-size: 0.85rem;
    color: var(--text-secondary);
    margin-top: 0.25rem;
  }

  .energy-stats h4 {
    margin: 0 0 0.5rem 0;
  }

  .range-info {
    display: flex;
    gap: 1rem;
    font-size: 0.9rem;
    color: var(--text-secondary);
  }

  .quality-distribution h4 {
    margin: 1.5rem 0 0.5rem 0;
  }

  .quality-bars {
    display: grid;
    gap: 0.5rem;
  }

  .quality-bar {
    display: grid;
    grid-template-columns: 80px 1fr 40px;
    align-items: center;
    gap: 0.5rem;
  }

  .bar-label {
    font-size: 0.85rem;
    font-weight: 500;
  }

  .bar {
    height: 20px;
    border-radius: 4px;
    transition: width 0.3s ease;
  }

  .quality-bar.excellent .bar { background: #10b981; }
  .quality-bar.good .bar { background: #059669; }
  .quality-bar.fair .bar { background: #f59e0b; }
  .quality-bar.poor .bar { background: #ef4444; }

  .bar-count {
    font-size: 0.85rem;
    font-weight: 600;
    text-align: right;
  }

  .poses-table {
    display: grid;
    gap: 1rem;
  }

  .pose-row {
    background: var(--background-secondary);
    border-radius: 8px;
    padding: 1rem;
  }

  .pose-header {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 0.75rem;
    font-weight: 600;
  }

  .pose-rank {
    background: var(--primary-color);
    color: white;
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    font-size: 0.85rem;
  }

  .pose-id {
    font-family: 'Courier New', monospace;
    color: var(--text-secondary);
  }

  .pose-energy {
    color: #dc2626;
    margin-left: auto;
  }

  .pose-details {
    display: grid;
    gap: 0.75rem;
    font-size: 0.9rem;
  }

  .pose-strengths ul, .pose-weaknesses ul {
    margin: 0.25rem 0 0 1rem;
    padding: 0;
  }

  .pose-strengths li {
    color: #059669;
  }

  .pose-weaknesses li {
    color: #dc2626;
  }

  .pose-recommendation {
    background: var(--background-primary);
    padding: 0.75rem;
    border-radius: 6px;
    border-left: 4px solid var(--primary-color);
  }

  /* Recommendations Section */
  .recommendations-section {
    display: grid;
    gap: 1.5rem;
  }

  .recommendations-list {
    display: grid;
    gap: 1rem;
  }

  .recommendation-card {
    background: var(--background-primary);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 1.5rem;
  }

  .rec-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 1rem;
  }

  .rec-header h4 {
    margin: 0;
    color: var(--text-primary);
  }

  .rec-badges {
    display: flex;
    gap: 0.5rem;
  }

  .priority-badge, .type-badge {
    padding: 0.25rem 0.75rem;
    border-radius: 12px;
    font-size: 0.75rem;
    font-weight: 600;
    color: white;
  }

  .type-badge {
    background: var(--text-secondary);
  }

  .rec-description {
    margin: 0 0 1rem 0;
    line-height: 1.6;
  }

  .rec-rationale, .rec-next-steps {
    margin: 1rem 0;
    font-size: 0.9rem;
  }

  .rec-next-steps ol {
    margin: 0.5rem 0 0 1rem;
    padding: 0;
  }

  .rec-next-steps li {
    margin-bottom: 0.25rem;
  }

  /* Export Section */
  .export-section {
    display: grid;
    gap: 2rem;
  }

  .export-options, .export-formats {
    background: var(--background-primary);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 1.5rem;
  }

  .export-options h4, .export-formats h4 {
    margin: 0 0 1rem 0;
    color: var(--text-primary);
  }

  .options-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 0.75rem;
    margin-bottom: 1rem;
  }

  .option-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
  }

  .numeric-options {
    margin: 1rem 0;
  }

  .numeric-options label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .numeric-options input {
    width: 80px;
    padding: 0.25rem 0.5rem;
    border: 1px solid var(--border-color);
    border-radius: 4px;
  }

  .format-options {
    display: grid;
    gap: 0.75rem;
    margin-bottom: 1.5rem;
  }

  .format-options label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
  }

  .update-btn, .export-btn, .copy-btn {
    padding: 0.75rem 1.5rem;
    border: none;
    border-radius: 6px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .update-btn {
    background: var(--primary-color);
    color: white;
  }

  .export-btn {
    background: #059669;
    color: white;
  }

  .copy-btn {
    background: var(--text-secondary);
    color: white;
    margin-left: 0.5rem;
  }

  .update-btn:hover, .export-btn:hover, .copy-btn:hover {
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  }

  .export-actions {
    display: flex;
    align-items: center;
  }

  .preview-section {
    background: var(--background-primary);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 1.5rem;
  }

  .preview-section h4 {
    margin: 0 0 1rem 0;
  }

  .preview-container {
    border: 1px solid var(--border-color);
    border-radius: 6px;
    overflow: hidden;
    height: 400px;
  }

  .preview-container iframe {
    width: 100%;
    height: 100%;
    border: none;
  }

  /* Responsive */
  @media (max-width: 768px) {
    .report-header {
      flex-direction: column;
      gap: 1rem;
      text-align: center;
    }

    .header-info {
      justify-content: center;
    }

    .tab-content {
      padding: 1rem;
    }

    .pose-metrics, .stats-grid, .system-grid {
      grid-template-columns: 1fr;
    }

    .pose-header {
      flex-direction: column;
      align-items: flex-start;
      gap: 0.5rem;
    }

    .pose-energy {
      margin-left: 0;
    }

    .rec-header {
      flex-direction: column;
      gap: 0.5rem;
    }

    .export-actions {
      flex-direction: column;
      gap: 0.5rem;
      align-items: stretch;
    }

    .copy-btn {
      margin-left: 0;
    }
  }
</style>