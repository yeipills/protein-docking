<!-- Pose Viewer - Visualización interactiva de poses de docking -->
<script lang="ts">
  import { onMount } from 'svelte';
  import type { DockingResult, DockingPose } from '../algorithms/DockingAlgorithm.js';
  import type { PDBStructure } from '../utils/PDBParser.js';
  import type { EnergyComponents } from '../algorithms/EnergyCalculator.js';
  import type { ScoringResult } from '../algorithms/ScoringEngine.js';

  interface Props {
    dockingResult: DockingResult | null;
    receptor: PDBStructure | null;
    ligand: PDBStructure | null;
    onPoseSelect?: (pose: DockingPose) => void;
    showEnergyDetails?: boolean;
    maxPosesToShow?: number;
  }

  let {
    dockingResult,
    receptor,
    ligand,
    onPoseSelect,
    showEnergyDetails = true,
    maxPosesToShow = 10
  }: Props = $props();

  let selectedPoseIndex = $state(0);
  let sortBy = $state<'energy' | 'score' | 'rmsd'>('energy');
  let sortOrder = $state<'asc' | 'desc'>('asc');
  let filterMinEnergy = $state(-50);
  let filterMaxEnergy = $state(10);
  let showOnlyBest = $state(false);

  // Poses filtradas y ordenadas
  const processedPoses = $derived(() => {
    if (!dockingResult?.poses) return [];

    let poses = [...dockingResult.poses];

    // Filtrar por energía
    poses = poses.filter(pose => 
      pose.energy >= filterMinEnergy && pose.energy <= filterMaxEnergy
    );

    // Mostrar solo las mejores poses si está activado
    if (showOnlyBest) {
      poses = poses.slice(0, 5);
    }

    // Ordenar
    poses.sort((a, b) => {
      let valueA: number, valueB: number;
      
      switch (sortBy) {
        case 'energy':
          valueA = a.energy;
          valueB = b.energy;
          break;
        case 'score':
          valueA = a.score;
          valueB = b.score;
          break;
        case 'rmsd':
          valueA = a.rmsd || 0;
          valueB = b.rmsd || 0;
          break;
        default:
          valueA = a.energy;
          valueB = b.energy;
      }

      return sortOrder === 'asc' ? valueA - valueB : valueB - valueA;
    });

    return poses.slice(0, maxPosesToShow);
  });

  const selectedPose = $derived(() => 
    processedPoses[selectedPoseIndex] || null
  );

  const energyRange = $derived(() => {
    if (!dockingResult?.poses?.length) return { min: -50, max: 10 };
    
    const energies = dockingResult.poses.map(p => p.energy);
    return {
      min: Math.floor(Math.min(...energies)) - 5,
      max: Math.ceil(Math.max(...energies)) + 5
    };
  });

  // Notificar cuando se selecciona una pose
  $effect(() => {
    if (selectedPose) {
      onPoseSelect?.(selectedPose);
    }
  });

  function selectPose(index: number) {
    selectedPoseIndex = Math.max(0, Math.min(index, processedPoses.length - 1));
  }

  function getEnergyColor(energy: number): string {
    // Verde para energías bajas (favorables), rojo para altas (desfavorables)
    const normalized = Math.max(0, Math.min(1, (energy + 50) / 60)); // -50 a +10 range
    const hue = (1 - normalized) * 120; // 120° = verde, 0° = rojo
    return `hsl(${hue}, 70%, 50%)`;
  }

  function getScoreColor(score: number): string {
    // Verde para scores altos, rojo para bajos
    const hue = score * 120;
    return `hsl(${hue}, 70%, 50%)`;
  }

  function formatEnergy(energy: number): string {
    return energy.toFixed(2);
  }

  function formatScore(score: number): string {
    return (score * 100).toFixed(1);
  }

  function formatRmsd(rmsd: number | undefined): string {
    return rmsd ? rmsd.toFixed(2) : 'N/A';
  }

  function getInteractionSummary(pose: DockingPose): string {
    if (!pose.contacts?.length) return 'Sin contactos';
    
    const contactTypes = pose.contacts.reduce((acc, contact) => {
      acc[contact.interaction_type] = (acc[contact.interaction_type] || 0) + 1;
      return acc;
    }, {} as Record<string, number>);

    const summary = Object.entries(contactTypes)
      .map(([type, count]) => `${count} ${type}`)
      .join(', ');

    return summary;
  }

  function getScoringResult(pose: DockingPose): ScoringResult | null {
    if (!dockingResult?.scoring_results) return null;
    return dockingResult.scoring_results.find(s => s.pose_id === pose.id) || null;
  }

  function formatConfidence(confidence: number): string {
    if (confidence >= 0.8) return 'Alta';
    if (confidence >= 0.6) return 'Media';
    if (confidence >= 0.4) return 'Baja';
    return 'Muy baja';
  }

  function getConfidenceColor(confidence: number): string {
    if (confidence >= 0.8) return '#10b981'; // Green
    if (confidence >= 0.6) return '#f59e0b'; // Yellow
    if (confidence >= 0.4) return '#f97316'; // Orange
    return '#ef4444'; // Red
  }

  onMount(() => {
    // Auto-seleccionar la mejor pose al cargar
    if (processedPoses.length > 0) {
      selectPose(0);
    }
  });
</script>

<!-- Pose Viewer Component -->
<div class="pose-viewer">
  <div class="viewer-header">
    <h3>Poses de Docking</h3>
    {#if dockingResult}
      <div class="result-summary">
        <span class="total-poses">
          {processedPoses.length} de {dockingResult.poses.length} poses
        </span>
        <span class="best-energy">
          Mejor: {formatEnergy(dockingResult.best_pose?.energy || 0)} kcal/mol
        </span>
      </div>
    {/if}
  </div>

  {#if !dockingResult}
    <div class="no-results">
      <p>No hay resultados de docking disponibles</p>
      <small>Ejecuta un cálculo de docking para ver las poses</small>
    </div>
  {:else}
    <!-- Controles de filtrado y ordenamiento -->
    <div class="controls">
      <div class="control-group">
        <label>Ordenar por:</label>
        <select bind:value={sortBy}>
          <option value="energy">Energía</option>
          <option value="score">Score</option>
          <option value="rmsd">RMSD</option>
        </select>
        <select bind:value={sortOrder}>
          <option value="asc">Ascendente</option>
          <option value="desc">Descendente</option>
        </select>
      </div>

      <div class="control-group">
        <label>
          <input type="checkbox" bind:checked={showOnlyBest}>
          Solo mejores poses
        </label>
      </div>

      <div class="control-group energy-filter">
        <label>Filtro de energía:</label>
        <input 
          type="range" 
          bind:value={filterMinEnergy}
          min={energyRange.min}
          max={energyRange.max}
          step="1"
        >
        <span>{filterMinEnergy}</span>
        <span>a</span>
        <input 
          type="range" 
          bind:value={filterMaxEnergy}
          min={energyRange.min}
          max={energyRange.max}
          step="1"
        >
        <span>{filterMaxEnergy} kcal/mol</span>
      </div>
    </div>

    <!-- Lista de poses -->
    <div class="poses-list">
      {#each processedPoses as pose, index}
        <div 
          class="pose-item"
          class:selected={index === selectedPoseIndex}
          onclick={() => selectPose(index)}
        >
          <div class="pose-header">
            <div class="pose-rank">#{index + 1}</div>
            <div class="pose-id">{pose.id}</div>
            <div class="pose-controls">
              <button 
                class="select-btn"
                onclick={() => selectPose(index)}
                disabled={index === selectedPoseIndex}
              >
                {index === selectedPoseIndex ? 'Seleccionada' : 'Seleccionar'}
              </button>
            </div>
          </div>

          <div class="pose-metrics">
            <div class="metric">
              <span class="metric-label">Energía:</span>
              <span 
                class="metric-value energy"
                style="color: {getEnergyColor(pose.energy)}"
              >
                {formatEnergy(pose.energy)} kcal/mol
              </span>
            </div>

            <div class="metric">
              <span class="metric-label">Score:</span>
              <span 
                class="metric-value score"
                style="color: {getScoreColor(pose.score)}"
              >
                {formatScore(pose.score)}%
              </span>
            </div>

            <div class="metric">
              <span class="metric-label">RMSD:</span>
              <span class="metric-value rmsd">
                {formatRmsd(pose.rmsd)} Å
              </span>
            </div>

            <div class="metric">
              <span class="metric-label">Contactos:</span>
              <span class="metric-value contacts">
                {pose.contacts?.length || 0}
              </span>
            </div>

            <!-- Advanced scoring metrics -->
            {#if getScoringResult(pose)}
              {@const scoringResult = getScoringResult(pose)}
              <div class="metric">
                <span class="metric-label">Consensus:</span>
                <span class="metric-value consensus" style="color: {getScoreColor(scoringResult.scores.consensus_score)}">
                  {(scoringResult.scores.consensus_score * 100).toFixed(1)}%
                </span>
              </div>
              
              <div class="metric">
                <span class="metric-label">Confianza:</span>
                <span class="metric-value confidence" style="color: {getConfidenceColor(scoringResult.confidence)}">
                  {formatConfidence(scoringResult.confidence)}
                </span>
              </div>
            {/if}
          </div>

          <div class="pose-interactions">
            <small>{getInteractionSummary(pose)}</small>
          </div>

          <!-- Desglose de energía detallado -->
          {#if showEnergyDetails && pose.energy_components}
            <div class="energy-breakdown">
              <div class="breakdown-header">Desglose de energía:</div>
              <div class="breakdown-grid">
                <div class="breakdown-item">
                  <span>Van der Waals:</span>
                  <span class="energy-value">
                    {formatEnergy(pose.energy_components.van_der_waals)}
                  </span>
                </div>
                <div class="breakdown-item">
                  <span>Electrostática:</span>
                  <span class="energy-value">
                    {formatEnergy(pose.energy_components.electrostatic)}
                  </span>
                </div>
                <div class="breakdown-item">
                  <span>H-bonds:</span>
                  <span class="energy-value">
                    {formatEnergy(pose.energy_components.hydrogen_bonds)}
                  </span>
                </div>
                <div class="breakdown-item">
                  <span>Hidrofóbica:</span>
                  <span class="energy-value">
                    {formatEnergy(pose.energy_components.hydrophobic)}
                  </span>
                </div>
                {#if pose.energy_components.solvation !== 0}
                  <div class="breakdown-item">
                    <span>Solvatación:</span>
                    <span class="energy-value">
                      {formatEnergy(pose.energy_components.solvation)}
                    </span>
                  </div>
                {/if}
                {#if pose.energy_components.conformational !== 0}
                  <div class="breakdown-item">
                    <span>Conformacional:</span>
                    <span class="energy-value">
                      {formatEnergy(pose.energy_components.conformational)}
                    </span>
                  </div>
                {/if}
              </div>
            </div>
          {/if}

          <!-- Advanced scoring breakdown -->
          {#if getScoringResult(pose)}
            {@const scoringResult = getScoringResult(pose)}
            <div class="scoring-breakdown">
              <div class="breakdown-header">Desglose de scoring:</div>
              <div class="breakdown-grid">
                <div class="breakdown-item">
                  <span>Energía:</span>
                  <span class="score-value">
                    {(scoringResult.scores.energy_score * 100).toFixed(1)}%
                  </span>
                </div>
                <div class="breakdown-item">
                  <span>Geométrico:</span>
                  <span class="score-value">
                    {(scoringResult.scores.geometric_score * 100).toFixed(1)}%
                  </span>
                </div>
                <div class="breakdown-item">
                  <span>Farmacóforo:</span>
                  <span class="score-value">
                    {(scoringResult.scores.pharmacophore_score * 100).toFixed(1)}%
                  </span>
                </div>
                <div class="breakdown-item">
                  <span>Contactos:</span>
                  <span class="score-value">
                    {(scoringResult.scores.contact_score * 100).toFixed(1)}%
                  </span>
                </div>
                <div class="breakdown-item">
                  <span>Drug-like:</span>
                  <span class="score-value">
                    {(scoringResult.scores.druglikeness_score * 100).toFixed(1)}%
                  </span>
                </div>
                <div class="breakdown-item rank-item">
                  <span>Rank:</span>
                  <span class="rank-value">#{scoringResult.rank}</span>
                </div>
              </div>
            </div>
          {/if}
        </div>
      {/each}

      {#if processedPoses.length === 0}
        <div class="no-poses">
          <p>No hay poses que coincidan con los filtros</p>
          <small>Ajusta los criterios de filtrado</small>
        </div>
      {/if}
    </div>

    <!-- Navegación rápida -->
    {#if processedPoses.length > 1}
      <div class="pose-navigation">
        <button 
          onclick={() => selectPose(selectedPoseIndex - 1)}
          disabled={selectedPoseIndex === 0}
          class="nav-btn prev"
        >
          ← Anterior
        </button>
        
        <span class="pose-counter">
          {selectedPoseIndex + 1} de {processedPoses.length}
        </span>
        
        <button 
          onclick={() => selectPose(selectedPoseIndex + 1)}
          disabled={selectedPoseIndex === processedPoses.length - 1}
          class="nav-btn next"
        >
          Siguiente →
        </button>
      </div>
    {/if}
  {/if}
</div>

<style>
  .pose-viewer {
    background: var(--surface-color);
    border-radius: 12px;
    padding: 1.5rem;
    border: 1px solid var(--border-color);
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .viewer-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid var(--border-color);
  }

  .viewer-header h3 {
    margin: 0;
    color: var(--text-primary);
  }

  .result-summary {
    display: flex;
    gap: 1rem;
    font-size: 0.9rem;
  }

  .total-poses {
    color: var(--text-secondary);
  }

  .best-energy {
    color: var(--success-color);
    font-weight: 600;
  }

  .no-results, .no-poses {
    text-align: center;
    padding: 3rem 1rem;
    color: var(--text-secondary);
  }

  .controls {
    display: flex;
    flex-wrap: wrap;
    gap: 1rem;
    margin-bottom: 1rem;
    padding: 1rem;
    background: var(--background-secondary);
    border-radius: 8px;
    border: 1px solid var(--border-color);
  }

  .control-group {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.9rem;
  }

  .control-group label {
    font-weight: 500;
    color: var(--text-primary);
  }

  .control-group select {
    padding: 0.25rem 0.5rem;
    border: 1px solid var(--border-color);
    border-radius: 4px;
    background: var(--surface-color);
    color: var(--text-primary);
  }

  .energy-filter {
    flex: 1;
    min-width: 200px;
  }

  .energy-filter input[type="range"] {
    margin: 0 0.5rem;
  }

  .poses-list {
    flex: 1;
    overflow-y: auto;
    max-height: 500px;
  }

  .pose-item {
    background: var(--background-primary);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    margin-bottom: 0.75rem;
    padding: 1rem;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .pose-item:hover {
    border-color: var(--primary-color);
    box-shadow: 0 2px 8px rgba(59, 130, 246, 0.1);
  }

  .pose-item.selected {
    border-color: var(--primary-color);
    background: var(--primary-background);
    box-shadow: 0 4px 12px rgba(59, 130, 246, 0.15);
  }

  .pose-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.75rem;
  }

  .pose-rank {
    font-weight: 700;
    color: var(--primary-color);
    font-size: 1.1rem;
  }

  .pose-id {
    font-family: 'Courier New', monospace;
    color: var(--text-secondary);
    font-size: 0.85rem;
  }

  .select-btn {
    padding: 0.25rem 0.75rem;
    background: var(--primary-color);
    color: white;
    border: none;
    border-radius: 4px;
    font-size: 0.8rem;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .select-btn:hover:not(:disabled) {
    background: var(--primary-hover);
    transform: translateY(-1px);
  }

  .select-btn:disabled {
    background: var(--text-disabled);
    cursor: not-allowed;
  }

  .pose-metrics {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
    gap: 0.75rem;
    margin-bottom: 0.75rem;
  }

  .metric {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .metric-label {
    font-size: 0.8rem;
    color: var(--text-secondary);
    font-weight: 500;
  }

  .metric-value {
    font-weight: 600;
    font-size: 0.95rem;
  }

  .metric-value.energy {
    font-weight: 700;
  }

  .pose-interactions {
    font-size: 0.85rem;
    color: var(--text-secondary);
    margin-bottom: 0.5rem;
  }

  .energy-breakdown,
  .scoring-breakdown {
    margin-top: 0.75rem;
    padding-top: 0.75rem;
    border-top: 1px solid var(--border-color);
  }

  .scoring-breakdown {
    background: linear-gradient(135deg, #f8fafc 0%, #f1f5f9 100%);
    padding: 0.75rem;
    border-radius: 6px;
    border: 1px solid #e2e8f0;
  }

  .breakdown-header {
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 0.5rem;
  }

  .breakdown-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
    gap: 0.5rem;
    font-size: 0.8rem;
  }

  .breakdown-item {
    display: flex;
    justify-content: space-between;
    padding: 0.25rem 0;
  }

  .energy-value {
    font-weight: 600;
    font-family: 'Courier New', monospace;
  }

  .score-value {
    font-weight: 600;
    font-family: 'Courier New', monospace;
    color: var(--primary-color);
  }

  .rank-value {
    font-weight: 700;
    font-family: 'Courier New', monospace;
    color: var(--success-color);
    font-size: 0.9rem;
  }

  .rank-item {
    background: rgba(16, 185, 129, 0.1);
    border-radius: 4px;
    padding: 0.25rem 0.5rem !important;
  }

  .pose-navigation {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 1rem;
    margin-top: 1rem;
    padding: 1rem;
    border-top: 1px solid var(--border-color);
  }

  .nav-btn {
    padding: 0.5rem 1rem;
    background: var(--primary-color);
    color: white;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.2s ease;
    font-weight: 500;
  }

  .nav-btn:hover:not(:disabled) {
    background: var(--primary-hover);
    transform: translateY(-1px);
  }

  .nav-btn:disabled {
    background: var(--text-disabled);
    cursor: not-allowed;
    transform: none;
  }

  .pose-counter {
    font-weight: 600;
    color: var(--text-primary);
    padding: 0 1rem;
  }

  /* Responsive */
  @media (max-width: 768px) {
    .controls {
      flex-direction: column;
      align-items: stretch;
    }

    .control-group {
      justify-content: space-between;
    }

    .pose-metrics {
      grid-template-columns: repeat(2, 1fr);
    }

    .breakdown-grid {
      grid-template-columns: 1fr;
    }
  }
</style>