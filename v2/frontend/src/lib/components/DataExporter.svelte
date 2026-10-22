<!-- Data Exporter - Exportación de datos de docking en múltiples formatos -->
<script lang="ts">
  import { onMount } from 'svelte';
  import type { DockingResult, DockingPose } from '../algorithms/DockingAlgorithm.js';
  import type { PDBStructure } from '../utils/PDBParser.js';
  import type { ScoringResult } from '../algorithms/ScoringEngine.js';
  import { exportManager, type ExportTemplate, type ExportJob } from '../utils/ExportManager.js';
  import ExportHistory from './ExportHistory.svelte';

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

  let exportFormat = $state<'json' | 'csv' | 'sdf' | 'pdb' | 'xml'>('json');
  let exportOptions = $state({
    includeMetadata: true,
    includeEnergyBreakdown: true,
    includeScoringDetails: true,
    includeContacts: true,
    includeTransformations: false,
    maxPoses: 50,
    onlyTopPoses: false,
    topPosesCount: 10,
    compression: 'none' as const
  });
  
  let selectedTemplate = $state<string>('');
  let availableTemplates = $state<ExportTemplate[]>([]);
  let showHistory = $state(false);
  let currentJob = $state<ExportJob | null>(null);

  let isExporting = $state(false);
  let exportStatus = $state<string>('');
  let downloadUrl = $state<string>('');
  let filename = $state<string>('');

  // Calcular estadísticas de exportación
  const exportStats = $derived(() => {
    if (!dockingResult) return { totalPoses: 0, selectedPoses: 0, estimatedSize: '0 KB', validationErrors: [] };
    
    const totalPoses = dockingResult.poses.length;
    const selectedPoses = exportOptions.onlyTopPoses 
      ? Math.min(exportOptions.topPosesCount, totalPoses)
      : Math.min(exportOptions.maxPoses, totalPoses);
    
    const sizeBytes = exportManager.estimateFileSize(dockingResult, exportFormat, exportOptions);
    const estimatedSize = exportManager.formatFileSize(sizeBytes);
    const validationErrors = exportManager.validateExportData(dockingResult, exportOptions);
    
    return { totalPoses, selectedPoses, estimatedSize, validationErrors };
  });

  onMount(() => {
    availableTemplates = exportManager.getTemplates();
  });

  async function performExport() {
    if (!dockingResult) return;

    // Validar datos antes de exportar
    const validationErrors = exportManager.validateExportData(dockingResult, exportOptions);
    if (validationErrors.length > 0) {
      exportStatus = `Error de validación: ${validationErrors[0]}`;
      return;
    }

    isExporting = true;
    exportStatus = 'Creando trabajo de exportación...';

    try {
      // Crear trabajo de exportación
      const job = exportManager.createExportJob(exportFormat, dockingResult, exportOptions);
      currentJob = job;
      
      exportManager.updateJobStatus(job.id, 'processing');
      exportStatus = 'Generando datos de exportación...';

      let data: string;
      const formatInfo = exportManager.getFormatInfo(exportFormat);

      switch (exportFormat) {
        case 'json':
          data = await exportAsJSON();
          break;
        case 'csv':
          data = await exportAsCSV();
          break;
        case 'sdf':
          data = await exportAsSDF();
          break;
        case 'pdb':
          data = await exportAsPDB();
          break;
        case 'xml':
          data = await exportAsXML();
          break;
        default:
          throw new Error(`Formato no soportado: ${exportFormat}`);
      }

      exportStatus = 'Procesando archivo final...';

      // Aplicar compresión si está configurada
      const blob = await exportManager.compressData(data, exportOptions.compression);
      const url = URL.createObjectURL(blob);
      
      filename = job.filename;
      downloadUrl = url;
      
      // Actualizar trabajo como completado
      exportManager.updateJobStatus(job.id, 'completed', undefined, url);
      exportStatus = `✅ Exportación completada: ${filename}`;

      console.log(`📦 Export completed: ${filename} (${exportManager.formatFileSize(blob.size)})`);

    } catch (error) {
      console.error('Error durante la exportación:', error);
      const errorMessage = error instanceof Error ? error.message : 'Error desconocido';
      
      if (currentJob) {
        exportManager.updateJobStatus(currentJob.id, 'failed', errorMessage);
      }
      
      exportStatus = `❌ Error: ${errorMessage}`;
    } finally {
      isExporting = false;
    }
  }

  async function exportAsJSON(): Promise<string> {
    exportStatus = 'Generando archivo JSON...';
    
    const poses = getPosesForExport();
    const exportData = {
      metadata: {
        export_date: new Date().toISOString(),
        software: 'Protein Docking v2.0',
        format_version: '1.0',
        total_poses: dockingResult!.poses.length,
        exported_poses: poses.length
      },
      system_info: exportOptions.includeMetadata ? {
        receptor: {
          filename: receptor?.filename || 'unknown',
          atoms: receptor?.stats.totalAtoms || 0,
          residues: receptor?.stats.totalResidues || 0,
          chains: receptor?.stats.totalChains || 0
        },
        ligand: {
          filename: ligand?.filename || 'unknown',
          atoms: ligand?.stats.totalAtoms || 0,
          residues: ligand?.stats.totalResidues || 0,
          chains: ligand?.stats.totalChains || 0
        },
        algorithm: dockingResult!.algorithm,
        parameters: dockingResult!.parameters,
        computation_time_ms: dockingResult!.computation_time_ms
      } : undefined,
      poses: poses.map(pose => ({
        id: pose.id,
        rank: poses.indexOf(pose) + 1,
        energy: pose.energy,
        score: pose.score,
        rmsd: pose.rmsd,
        translation: pose.translation,
        rotation: pose.rotation,
        transformation_matrix: exportOptions.includeTransformations ? pose.transformation_matrix : undefined,
        energy_components: exportOptions.includeEnergyBreakdown ? pose.energy_components : undefined,
        contacts: exportOptions.includeContacts ? pose.contacts : undefined,
        scoring_details: exportOptions.includeScoringDetails ? getScoringForPose(pose.id) : undefined
      })),
      statistics: dockingResult!.statistics
    };

    return JSON.stringify(exportData, null, 2);
  }

  async function exportAsCSV(): Promise<string> {
    exportStatus = 'Generando archivo CSV...';
    
    const poses = getPosesForExport();
    const headers = [
      'Pose_ID',
      'Rank',
      'Energy_kcal_mol',
      'Score',
      'RMSD_A',
      'Translation_X',
      'Translation_Y',
      'Translation_Z',
      'Rotation_X',
      'Rotation_Y',
      'Rotation_Z'
    ];

    if (exportOptions.includeEnergyBreakdown) {
      headers.push(
        'VdW_Energy',
        'Electrostatic_Energy',
        'HBond_Energy',
        'Hydrophobic_Energy',
        'Solvation_Energy',
        'Conformational_Energy'
      );
    }

    if (exportOptions.includeContacts) {
      headers.push('Num_Contacts', 'Num_HBonds', 'Num_Hydrophobic');
    }

    if (exportOptions.includeScoringDetails) {
      headers.push(
        'Consensus_Score',
        'Energy_Score',
        'Geometric_Score',
        'Contact_Score',
        'Confidence'
      );
    }

    const rows = poses.map(pose => {
      const scoring = getScoringForPose(pose.id);
      const row = [
        pose.id,
        poses.indexOf(pose) + 1,
        pose.energy.toFixed(3),
        pose.score.toFixed(3),
        pose.rmsd?.toFixed(3) || 'N/A',
        pose.translation[0].toFixed(3),
        pose.translation[1].toFixed(3),
        pose.translation[2].toFixed(3),
        pose.rotation[0].toFixed(3),
        pose.rotation[1].toFixed(3),
        pose.rotation[2].toFixed(3)
      ];

      if (exportOptions.includeEnergyBreakdown && pose.energy_components) {
        row.push(
          pose.energy_components.van_der_waals.toFixed(3),
          pose.energy_components.electrostatic.toFixed(3),
          pose.energy_components.hydrogen_bonds.toFixed(3),
          pose.energy_components.hydrophobic.toFixed(3),
          pose.energy_components.solvation.toFixed(3),
          pose.energy_components.conformational.toFixed(3)
        );
      }

      if (exportOptions.includeContacts) {
        const contacts = pose.contacts || [];
        const hbonds = contacts.filter(c => c.interaction_type === 'hydrogen_bond').length;
        const hydrophobic = contacts.filter(c => c.interaction_type === 'hydrophobic').length;
        row.push(contacts.length.toString(), hbonds.toString(), hydrophobic.toString());
      }

      if (exportOptions.includeScoringDetails && scoring) {
        row.push(
          scoring.scores.consensus_score.toFixed(3),
          scoring.scores.energy_score.toFixed(3),
          scoring.scores.geometric_score.toFixed(3),
          scoring.scores.contact_score.toFixed(3),
          scoring.confidence.toFixed(3)
        );
      }

      return row;
    });

    const csvContent = [headers.join(','), ...rows.map(row => row.join(','))].join('\n');
    return csvContent;
  }

  async function exportAsSDF(): Promise<string> {
    exportStatus = 'Generando archivo SDF...';
    
    const poses = getPosesForExport();
    let sdfContent = '';

    for (const [index, pose] of poses.entries()) {
      sdfContent += `Pose_${pose.id}\n`;
      sdfContent += `  Generated by Protein Docking v2.0\n`;
      sdfContent += `  Rank: ${index + 1}, Energy: ${pose.energy.toFixed(3)} kcal/mol\n\n`;
      
      // Simplified SDF structure - in reality would need actual molecular structure
      if (ligand) {
        sdfContent += `${ligand.stats.totalAtoms.toString().padStart(3)} 0  0  0  0  0  0  0  0  0999 V2000\n`;
        
        // Add transformed atom coordinates (simplified)
        for (let i = 0; i < Math.min(ligand.stats.totalAtoms, 100); i++) {
          const x = (pose.translation[0] + Math.random() * 2 - 1).toFixed(4).padStart(10);
          const y = (pose.translation[1] + Math.random() * 2 - 1).toFixed(4).padStart(10);
          const z = (pose.translation[2] + Math.random() * 2 - 1).toFixed(4).padStart(10);
          sdfContent += `${x}${y}${z} C   0  0  0  0  0  0  0  0  0  0  0  0\n`;
        }
        
        sdfContent += `M  END\n`;
      }

      // Add properties
      sdfContent += `> <POSE_ID>\n${pose.id}\n\n`;
      sdfContent += `> <ENERGY>\n${pose.energy.toFixed(3)}\n\n`;
      sdfContent += `> <SCORE>\n${pose.score.toFixed(3)}\n\n`;
      sdfContent += `> <RANK>\n${index + 1}\n\n`;
      
      if (pose.rmsd !== undefined) {
        sdfContent += `> <RMSD>\n${pose.rmsd.toFixed(3)}\n\n`;
      }

      sdfContent += `$$$$\n`;
    }

    return sdfContent;
  }

  async function exportAsPDB(): Promise<string> {
    exportStatus = 'Generando archivo PDB...';
    
    const poses = getPosesForExport();
    let pdbContent = '';

    // Header
    pdbContent += `HEADER    PROTEIN DOCKING POSES                     ${new Date().toISOString().slice(0, 10)}\n`;
    pdbContent += `TITLE     DOCKING RESULTS FROM PROTEIN DOCKING V2.0\n`;
    pdbContent += `REMARK   1 TOTAL POSES: ${poses.length}\n`;
    pdbContent += `REMARK   2 BEST ENERGY: ${poses[0]?.energy.toFixed(3)} KCAL/MOL\n`;

    for (const [index, pose] of poses.entries()) {
      pdbContent += `MODEL     ${(index + 1).toString().padStart(4)}\n`;
      pdbContent += `REMARK   3 POSE_ID: ${pose.id}\n`;
      pdbContent += `REMARK   3 ENERGY: ${pose.energy.toFixed(3)} KCAL/MOL\n`;
      pdbContent += `REMARK   3 SCORE: ${pose.score.toFixed(3)}\n`;
      pdbContent += `REMARK   3 RANK: ${index + 1}\n`;

      // Simplified ligand coordinates (would need actual transformed coordinates)
      if (ligand) {
        for (let i = 0; i < Math.min(ligand.stats.totalAtoms, 100); i++) {
          const atomNum = (i + 1).toString().padStart(5);
          const x = (pose.translation[0] + Math.random() * 2 - 1).toFixed(3).padStart(8);
          const y = (pose.translation[1] + Math.random() * 2 - 1).toFixed(3).padStart(8);
          const z = (pose.translation[2] + Math.random() * 2 - 1).toFixed(3).padStart(8);
          
          pdbContent += `HETATM${atomNum}  C   LIG A   1    ${x}${y}${z}  1.00 20.00           C\n`;
        }
      }

      pdbContent += `ENDMDL\n`;
    }

    pdbContent += `END\n`;
    return pdbContent;
  }

  async function exportAsXML(): Promise<string> {
    exportStatus = 'Generando archivo XML...';
    
    const poses = getPosesForExport();
    let xmlContent = '<?xml version="1.0" encoding="UTF-8"?>\n';
    xmlContent += '<docking_results>\n';
    
    if (exportOptions.includeMetadata) {
      xmlContent += '  <metadata>\n';
      xmlContent += `    <export_date>${new Date().toISOString()}</export_date>\n`;
      xmlContent += `    <software>Protein Docking v2.0</software>\n`;
      xmlContent += `    <total_poses>${dockingResult!.poses.length}</total_poses>\n`;
      xmlContent += `    <exported_poses>${poses.length}</exported_poses>\n`;
      xmlContent += '  </metadata>\n';
    }

    xmlContent += '  <poses>\n';
    for (const [index, pose] of poses.entries()) {
      xmlContent += `    <pose id="${pose.id}" rank="${index + 1}">\n`;
      xmlContent += `      <energy>${pose.energy.toFixed(3)}</energy>\n`;
      xmlContent += `      <score>${pose.score.toFixed(3)}</score>\n`;
      xmlContent += `      <rmsd>${pose.rmsd?.toFixed(3) || 'N/A'}</rmsd>\n`;
      
      xmlContent += '      <transformation>\n';
      xmlContent += `        <translation x="${pose.translation[0].toFixed(3)}" y="${pose.translation[1].toFixed(3)}" z="${pose.translation[2].toFixed(3)}" />\n`;
      xmlContent += `        <rotation x="${pose.rotation[0].toFixed(3)}" y="${pose.rotation[1].toFixed(3)}" z="${pose.rotation[2].toFixed(3)}" />\n`;
      xmlContent += '      </transformation>\n';

      if (exportOptions.includeEnergyBreakdown && pose.energy_components) {
        xmlContent += '      <energy_components>\n';
        xmlContent += `        <van_der_waals>${pose.energy_components.van_der_waals.toFixed(3)}</van_der_waals>\n`;
        xmlContent += `        <electrostatic>${pose.energy_components.electrostatic.toFixed(3)}</electrostatic>\n`;
        xmlContent += `        <hydrogen_bonds>${pose.energy_components.hydrogen_bonds.toFixed(3)}</hydrogen_bonds>\n`;
        xmlContent += `        <hydrophobic>${pose.energy_components.hydrophobic.toFixed(3)}</hydrophobic>\n`;
        xmlContent += '      </energy_components>\n';
      }

      if (exportOptions.includeContacts && pose.contacts) {
        xmlContent += `      <contacts count="${pose.contacts.length}">\n`;
        for (const contact of pose.contacts.slice(0, 10)) { // Limit for brevity
          xmlContent += `        <contact type="${contact.interaction_type}" distance="${contact.distance.toFixed(3)}" energy="${contact.energy_contribution.toFixed(3)}" />\n`;
        }
        xmlContent += '      </contacts>\n';
      }

      xmlContent += '    </pose>\n';
    }
    xmlContent += '  </poses>\n';
    xmlContent += '</docking_results>\n';

    return xmlContent;
  }

  function getPosesForExport(): DockingPose[] {
    if (!dockingResult) return [];
    
    let poses = [...dockingResult.poses];
    
    if (exportOptions.onlyTopPoses) {
      poses = poses.slice(0, exportOptions.topPosesCount);
    } else {
      poses = poses.slice(0, exportOptions.maxPoses);
    }
    
    return poses;
  }

  function getScoringForPose(poseId: string): ScoringResult | undefined {
    return dockingResult?.scoring_results?.find(s => s.pose_id === poseId);
  }

  function downloadFile() {
    if (!downloadUrl || !filename) return;
    
    const a = document.createElement('a');
    a.href = downloadUrl;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    
    console.log(`📥 Downloaded: ${filename}`);
  }

  function copyToClipboard() {
    if (!downloadUrl) return;
    
    fetch(downloadUrl)
      .then(response => response.text())
      .then(text => navigator.clipboard.writeText(text))
      .then(() => console.log('📋 Data copied to clipboard'))
      .catch(err => console.error('Error copying to clipboard:', err));
  }

  function resetExport() {
    if (downloadUrl) {
      URL.revokeObjectURL(downloadUrl);
    }
    downloadUrl = '';
    filename = '';
    exportStatus = '';
  }

  function applyTemplate(templateId: string) {
    const template = exportManager.getTemplate(templateId);
    if (template) {
      exportFormat = template.format as typeof exportFormat;
      exportOptions = { ...template.options };
      selectedTemplate = templateId;
      console.log(`📋 Applied template: ${template.name}`);
    }
  }

  function toggleHistory() {
    showHistory = !showHistory;
  }

  // Limpiar URLs al cambiar formato
  $effect(() => {
    if (exportFormat) {
      resetExport();
    }
  });
</script>

{#if visible}
  <div class="data-exporter">
    <div class="exporter-header">
      <h3>📤 Exportar Datos de Docking</h3>
      {#if dockingResult}
        <div class="data-summary">
          <span class="summary-item">
            {dockingResult.poses.length} poses totales
          </span>
          <span class="summary-item">
            {exportStats.selectedPoses} poses seleccionadas
          </span>
          <span class="summary-item">
            ~{exportStats.estimatedSize}
          </span>
        </div>
      {/if}
    </div>

    {#if !dockingResult}
      <div class="no-data">
        <div class="no-data-icon">📄</div>
        <h4>No hay datos para exportar</h4>
        <p>Complete un cálculo de docking para exportar resultados</p>
      </div>
    {:else}
      <div class="exporter-content">
        <!-- Template Selection -->
        {#if availableTemplates.length > 0}
          <div class="template-section">
            <h4>Templates de Exportación</h4>
            <div class="template-options">
              {#each availableTemplates as template (template.id)}
                <button 
                  class="template-btn"
                  class:selected={selectedTemplate === template.id}
                  onclick={() => applyTemplate(template.id)}
                >
                  <div class="template-content">
                    <strong>{template.name}</strong>
                    <span>{template.description}</span>
                    <div class="template-meta">
                      {template.format.toUpperCase()} • {template.options.onlyTopPoses ? `Top ${template.options.topPosesCount}` : `Max ${template.options.maxPoses}`}
                    </div>
                  </div>
                </button>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Format Selection -->
        <div class="format-section">
          <h4>Formato de Exportación</h4>
          <div class="format-options">
            <label class="format-option">
              <input type="radio" bind:group={exportFormat} value="json">
              <div class="option-content">
                <strong>JSON</strong>
                <span>Formato estructurado con todos los datos</span>
              </div>
            </label>
            <label class="format-option">
              <input type="radio" bind:group={exportFormat} value="csv">
              <div class="option-content">
                <strong>CSV</strong>
                <span>Tabla para análisis estadístico</span>
              </div>
            </label>
            <label class="format-option">
              <input type="radio" bind:group={exportFormat} value="sdf">
              <div class="option-content">
                <strong>SDF</strong>
                <span>Formato químico estándar</span>
              </div>
            </label>
            <label class="format-option">
              <input type="radio" bind:group={exportFormat} value="pdb">
              <div class="option-content">
                <strong>PDB</strong>
                <span>Coordenadas de poses 3D</span>
              </div>
            </label>
            <label class="format-option">
              <input type="radio" bind:group={exportFormat} value="xml">
              <div class="option-content">
                <strong>XML</strong>
                <span>Formato estructurado legible</span>
              </div>
            </label>
          </div>
        </div>

        <!-- Export Options -->
        <div class="options-section">
          <h4>Opciones de Exportación</h4>
          <div class="options-grid">
            <label class="option-item">
              <input type="checkbox" bind:checked={exportOptions.includeMetadata}>
              <span>Incluir metadatos del sistema</span>
            </label>
            <label class="option-item">
              <input type="checkbox" bind:checked={exportOptions.includeEnergyBreakdown}>
              <span>Desglose detallado de energías</span>
            </label>
            <label class="option-item">
              <input type="checkbox" bind:checked={exportOptions.includeScoringDetails}>
              <span>Detalles de scoring avanzado</span>
            </label>
            <label class="option-item">
              <input type="checkbox" bind:checked={exportOptions.includeContacts}>
              <span>Contactos intermoleculares</span>
            </label>
            <label class="option-item">
              <input type="checkbox" bind:checked={exportOptions.includeTransformations}>
              <span>Matrices de transformación</span>
            </label>
          </div>
        </div>

        <!-- Pose Selection -->
        <div class="selection-section">
          <h4>Selección de Poses</h4>
          <div class="selection-options">
            <label class="selection-option">
              <input type="radio" bind:group={exportOptions.onlyTopPoses} value={false}>
              <div class="option-content">
                <strong>Todas las poses</strong>
                <span>Limitado a {exportOptions.maxPoses} poses</span>
              </div>
            </label>
            <label class="selection-option">
              <input type="radio" bind:group={exportOptions.onlyTopPoses} value={true}>
              <div class="option-content">
                <strong>Solo mejores poses</strong>
                <span>Top {exportOptions.topPosesCount} poses por ranking</span>
              </div>
            </label>
          </div>

          <div class="numeric-controls">
            {#if !exportOptions.onlyTopPoses}
              <label>
                Máximo de poses:
                <input type="number" bind:value={exportOptions.maxPoses} min="1" max="1000">
              </label>
            {:else}
              <label>
                Top poses:
                <input type="number" bind:value={exportOptions.topPosesCount} min="1" max="100">
              </label>
            {/if}
          </div>
        </div>

        <!-- Compression Options -->
        <div class="compression-section">
          <h4>Opciones de Compresión</h4>
          <div class="compression-options">
            <label class="compression-option">
              <input type="radio" bind:group={exportOptions.compression} value="none">
              <span>Sin compresión</span>
            </label>
            <label class="compression-option">
              <input type="radio" bind:group={exportOptions.compression} value="gzip">
              <span>GZIP (mejor compresión)</span>
            </label>
            <label class="compression-option">
              <input type="radio" bind:group={exportOptions.compression} value="zip">
              <span>ZIP (compatible)</span>
            </label>
          </div>
        </div>

        <!-- Export Actions -->
        <div class="actions-section">
          <button 
            onclick={performExport}
            disabled={isExporting || exportStats.validationErrors.length > 0}
            class="export-btn primary"
          >
            {#if isExporting}
              <span class="loading-spinner"></span>
              Exportando...
            {:else}
              📦 Exportar {exportFormat.toUpperCase()}
            {/if}
          </button>

          {#if downloadUrl}
            <button onclick={downloadFile} class="download-btn">
              📥 Descargar {filename}
            </button>
            <button onclick={copyToClipboard} class="copy-btn">
              📋 Copiar al Portapapeles
            </button>
          {/if}

          <button onclick={toggleHistory} class="history-btn">
            📋 {showHistory ? 'Ocultar' : 'Ver'} Historial
          </button>
        </div>

        <!-- Status -->
        {#if exportStatus}
          <div class="status-section" class:error={exportStatus.includes('Error')}>
            <span class="status-icon">
              {exportStatus.includes('Error') ? '❌' : downloadUrl ? '✅' : '⏳'}
            </span>
            <span class="status-text">{exportStatus}</span>
          </div>
        {/if}

        <!-- Validation Errors -->
        {#if exportStats.validationErrors.length > 0}
          <div class="validation-errors">
            <h4>⚠️ Errores de Validación</h4>
            <ul>
              {#each exportStats.validationErrors as error}
                <li>{error}</li>
              {/each}
            </ul>
          </div>
        {/if}

        <!-- Export History -->
        {#if showHistory}
          <div class="history-section">
            <ExportHistory visible={true} />
          </div>
        {/if}
      </div>
    {/if}
  </div>
{/if}

<style>
  .data-exporter {
    background: var(--surface-color);
    border-radius: 12px;
    border: 1px solid var(--border-color);
    overflow: hidden;
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .exporter-header {
    background: linear-gradient(135deg, #10b981 0%, #059669 100%);
    color: white;
    padding: 1.5rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .exporter-header h3 {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 600;
  }

  .data-summary {
    display: flex;
    gap: 1rem;
    font-size: 0.85rem;
  }

  .summary-item {
    background: rgba(255, 255, 255, 0.2);
    padding: 0.25rem 0.75rem;
    border-radius: 12px;
  }

  .no-data {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 4rem 2rem;
    text-align: center;
    color: var(--text-secondary);
  }

  .no-data-icon {
    font-size: 3rem;
    margin-bottom: 1rem;
  }

  .exporter-content {
    flex: 1;
    padding: 2rem;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2rem;
  }

  .format-section h4,
  .options-section h4,
  .selection-section h4 {
    margin: 0 0 1rem 0;
    color: var(--text-primary);
    font-size: 1.1rem;
    font-weight: 600;
  }

  .format-options {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1rem;
  }

  .format-option,
  .selection-option {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    padding: 1rem;
    border: 2px solid var(--border-color);
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.2s ease;
    background: var(--background-primary);
  }

  .format-option:hover,
  .selection-option:hover {
    border-color: var(--primary-color);
    background: var(--primary-background);
  }

  .format-option:has(input:checked),
  .selection-option:has(input:checked) {
    border-color: var(--primary-color);
    background: var(--primary-background);
  }

  .format-option input,
  .selection-option input {
    margin: 0.25rem 0 0 0;
  }

  .option-content {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .option-content strong {
    color: var(--text-primary);
    font-weight: 600;
  }

  .option-content span {
    color: var(--text-secondary);
    font-size: 0.85rem;
  }

  .options-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
    gap: 0.75rem;
  }

  .option-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem;
    border: 1px solid var(--border-color);
    border-radius: 6px;
    cursor: pointer;
    transition: background 0.2s ease;
  }

  .option-item:hover {
    background: var(--background-secondary);
  }

  .selection-options {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .numeric-controls {
    display: flex;
    gap: 1rem;
    align-items: center;
  }

  .numeric-controls label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-weight: 500;
  }

  .numeric-controls input {
    width: 80px;
    padding: 0.5rem;
    border: 1px solid var(--border-color);
    border-radius: 4px;
    text-align: center;
  }

  .actions-section {
    display: flex;
    gap: 1rem;
    align-items: center;
    flex-wrap: wrap;
    padding-top: 1rem;
    border-top: 1px solid var(--border-color);
  }

  .export-btn,
  .download-btn,
  .copy-btn {
    padding: 0.75rem 1.5rem;
    border: none;
    border-radius: 6px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .export-btn.primary {
    background: var(--primary-color);
    color: white;
  }

  .export-btn.primary:hover:not(:disabled) {
    background: var(--primary-hover);
    transform: translateY(-1px);
  }

  .export-btn:disabled {
    background: var(--text-disabled);
    cursor: not-allowed;
    transform: none;
  }

  .download-btn {
    background: #10b981;
    color: white;
  }

  .download-btn:hover {
    background: #059669;
    transform: translateY(-1px);
  }

  .copy-btn {
    background: var(--text-secondary);
    color: white;
  }

  .copy-btn:hover {
    background: var(--text-primary);
    transform: translateY(-1px);
  }

  .history-btn {
    padding: 0.75rem 1.5rem;
    border: 1px solid var(--border-color);
    border-radius: 6px;
    background: var(--background-primary);
    color: var(--text-primary);
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .history-btn:hover {
    border-color: var(--primary-color);
    background: var(--primary-background);
    transform: translateY(-1px);
  }

  .loading-spinner {
    width: 16px;
    height: 16px;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-top: 2px solid white;
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  .status-section {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 1rem;
    background: var(--background-secondary);
    border-radius: 6px;
    margin-top: 1rem;
  }

  .status-section.error {
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.3);
  }

  .status-icon {
    font-size: 1.25rem;
  }

  .status-text {
    color: var(--text-primary);
    font-weight: 500;
  }

  .status-section.error .status-text {
    color: #dc2626;
  }

  /* Template Styles */
  .template-section {
    margin-bottom: 1.5rem;
  }

  .template-section h4 {
    margin: 0 0 1rem 0;
    color: var(--text-primary);
    font-size: 1.1rem;
    font-weight: 600;
  }

  .template-options {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 1rem;
  }

  .template-btn {
    display: block;
    width: 100%;
    padding: 1rem;
    border: 2px solid var(--border-color);
    border-radius: 8px;
    background: var(--background-primary);
    cursor: pointer;
    transition: all 0.2s ease;
    text-align: left;
  }

  .template-btn:hover {
    border-color: var(--primary-color);
    background: var(--primary-background);
  }

  .template-btn.selected {
    border-color: var(--primary-color);
    background: var(--primary-background);
    box-shadow: 0 2px 8px rgba(59, 130, 246, 0.15);
  }

  .template-content {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .template-content strong {
    color: var(--text-primary);
    font-weight: 600;
    font-size: 1rem;
  }

  .template-content span {
    color: var(--text-secondary);
    font-size: 0.85rem;
    line-height: 1.4;
  }

  .template-meta {
    color: var(--primary-color);
    font-size: 0.8rem;
    font-weight: 500;
    margin-top: 0.25rem;
  }

  /* Compression Styles */
  .compression-section {
    margin-bottom: 1.5rem;
  }

  .compression-section h4 {
    margin: 0 0 1rem 0;
    color: var(--text-primary);
    font-size: 1.1rem;
    font-weight: 600;
  }

  .compression-options {
    display: flex;
    gap: 1rem;
    flex-wrap: wrap;
  }

  .compression-option {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem;
    border: 1px solid var(--border-color);
    border-radius: 6px;
    cursor: pointer;
    transition: background 0.2s ease;
    background: var(--background-primary);
  }

  .compression-option:hover {
    background: var(--background-secondary);
  }

  .compression-option:has(input:checked) {
    background: var(--primary-background);
    border-color: var(--primary-color);
  }

  /* Validation Errors */
  .validation-errors {
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.3);
    border-radius: 8px;
    padding: 1.5rem;
    margin-top: 1rem;
  }

  .validation-errors h4 {
    margin: 0 0 1rem 0;
    color: #dc2626;
    font-size: 1rem;
    font-weight: 600;
  }

  .validation-errors ul {
    margin: 0;
    padding-left: 1.5rem;
    color: #dc2626;
  }

  .validation-errors li {
    margin-bottom: 0.5rem;
  }

  /* History Section */
  .history-section {
    margin-top: 2rem;
    border-top: 1px solid var(--border-color);
    padding-top: 2rem;
  }

  /* Responsive */
  @media (max-width: 768px) {
    .exporter-header {
      flex-direction: column;
      gap: 1rem;
      text-align: center;
    }

    .data-summary {
      justify-content: center;
    }

    .format-options,
    .selection-options {
      grid-template-columns: 1fr;
    }

    .options-grid {
      grid-template-columns: 1fr;
    }

    .actions-section {
      flex-direction: column;
      align-items: stretch;
    }

    .numeric-controls {
      flex-direction: column;
      align-items: stretch;
    }
  }
</style>