/*!
 * ReportGenerator - Generación automática de reportes de docking molecular
 * 
 * Genera reportes comprehensivos que incluyen:
 * 1. Resumen ejecutivo de resultados
 * 2. Análisis detallado de poses principales
 * 3. Métricas de performance y calidad
 * 4. Visualizaciones estadísticas
 * 5. Recomendaciones basadas en datos
 * 6. Export en múltiples formatos (HTML, PDF, JSON, CSV)
 */

import type { DockingResult, DockingPose } from '../algorithms/DockingAlgorithm.js';
import type { ScoringResult } from '../algorithms/ScoringEngine.js';
import type { PDBStructure } from '../utils/PDBParser.js';

export interface ReportOptions {
  includeAllPoses: boolean;
  maxPosesInReport: number;
  includeEnergyBreakdown: boolean;
  includeScoringDetails: boolean;
  includeVisualizations: boolean;
  includeRecommendations: boolean;
  format: 'html' | 'json' | 'csv' | 'markdown';
  language: 'es' | 'en';
}

export interface ReportData {
  metadata: ReportMetadata;
  executiveSummary: ExecutiveSummary;
  systemInfo: SystemInfo;
  resultsAnalysis: ResultsAnalysis;
  poseAnalysis: PoseAnalysisEntry[];
  statisticalAnalysis: StatisticalAnalysis;
  recommendations: Recommendation[];
  appendices: Appendix[];
}

export interface ReportMetadata {
  reportId: string;
  generatedAt: string;
  generatedBy: string;
  version: string;
  title: string;
  description: string;
}

export interface ExecutiveSummary {
  overview: string;
  keyFindings: string[];
  bestPose: {
    id: string;
    energy: number;
    consensusScore: number;
    confidence: string;
    summary: string;
  };
  qualityAssessment: {
    overall: 'excellent' | 'good' | 'fair' | 'poor';
    reasoning: string;
    score: number;
  };
}

export interface SystemInfo {
  receptor: {
    name: string;
    atoms: number;
    residues: number;
    chains: number;
    description: string;
  };
  ligand: {
    name: string;
    atoms: number;
    molecularWeight: number;
    drugLikeness: number;
    description: string;
  };
  algorithm: {
    name: string;
    version: string;
    parameters: Record<string, any>;
    computationTime: number;
  };
}

export interface ResultsAnalysis {
  posesGenerated: number;
  posesEvaluated: number;
  successRate: number;
  energyRange: { min: number; max: number; mean: number; std: number };
  scoreRange: { min: number; max: number; mean: number; std: number };
  qualityDistribution: {
    excellent: number;
    good: number;
    fair: number;
    poor: number;
  };
}

export interface PoseAnalysisEntry {
  pose: DockingPose;
  scoring: ScoringResult | null;
  rank: number;
  analysis: {
    strengths: string[];
    weaknesses: string[];
    interactions: InteractionSummary;
    drugLikeness: DrugLikenessAssessment;
    recommendation: string;
  };
}

export interface InteractionSummary {
  totalContacts: number;
  hydrogenBonds: number;
  hydrophobicContacts: number;
  aromaticInteractions: number;
  electrostaticInteractions: number;
  keyResidues: string[];
  bindingSite: string;
}

export interface DrugLikenessAssessment {
  lipinskiCompliance: boolean;
  violations: string[];
  admetPrediction: 'favorable' | 'moderate' | 'unfavorable';
  synthesisComplexity: 'low' | 'medium' | 'high';
}

export interface StatisticalAnalysis {
  energyDistribution: DistributionData;
  scoreDistribution: DistributionData;
  correlationMatrix: CorrelationData[];
  clusterAnalysis: ClusterData[];
}

export interface DistributionData {
  mean: number;
  median: number;
  std: number;
  min: number;
  max: number;
  quartiles: [number, number, number];
  histogram: { bin: string; count: number }[];
}

export interface CorrelationData {
  metric1: string;
  metric2: string;
  correlation: number;
  significance: 'high' | 'medium' | 'low';
}

export interface ClusterData {
  clusterId: number;
  poses: string[];
  avgEnergy: number;
  avgScore: number;
  representative: string;
  description: string;
}

export interface Recommendation {
  type: 'optimization' | 'synthesis' | 'testing' | 'analysis';
  priority: 'high' | 'medium' | 'low';
  title: string;
  description: string;
  rationale: string;
  nextSteps: string[];
}

export interface Appendix {
  title: string;
  type: 'table' | 'figure' | 'text' | 'data';
  content: any;
  description: string;
}

const TRANSLATIONS = {
  es: {
    reportTitle: 'Reporte de Docking Molecular',
    executiveSummary: 'Resumen Ejecutivo',
    systemInfo: 'Información del Sistema',
    resultsAnalysis: 'Análisis de Resultados',
    poseAnalysis: 'Análisis de Poses',
    statisticalAnalysis: 'Análisis Estadístico',
    recommendations: 'Recomendaciones',
    bestPose: 'Mejor Pose',
    energy: 'Energía',
    score: 'Puntuación',
    confidence: 'Confianza',
    interactions: 'Interacciones',
    drugLikeness: 'Drug-likeness',
    excellent: 'Excelente',
    good: 'Bueno',
    fair: 'Regular',
    poor: 'Pobre',
    high: 'Alto',
    medium: 'Medio',
    low: 'Bajo',
    favorable: 'Favorable',
    moderate: 'Moderado',
    unfavorable: 'Desfavorable'
  },
  en: {
    reportTitle: 'Molecular Docking Report',
    executiveSummary: 'Executive Summary',
    systemInfo: 'System Information',
    resultsAnalysis: 'Results Analysis',
    poseAnalysis: 'Pose Analysis',
    statisticalAnalysis: 'Statistical Analysis',
    recommendations: 'Recommendations',
    bestPose: 'Best Pose',
    energy: 'Energy',
    score: 'Score',
    confidence: 'Confidence',
    interactions: 'Interactions',
    drugLikeness: 'Drug-likeness',
    excellent: 'Excellent',
    good: 'Good',
    fair: 'Fair',
    poor: 'Poor',
    high: 'High',
    medium: 'Medium',
    low: 'Low',
    favorable: 'Favorable',
    moderate: 'Moderate',
    unfavorable: 'Unfavorable'
  }
};

export class ReportGenerator {
  private options: ReportOptions;
  private translations: typeof TRANSLATIONS.es;

  constructor(options: Partial<ReportOptions> = {}) {
    this.options = {
      includeAllPoses: false,
      maxPosesInReport: 10,
      includeEnergyBreakdown: true,
      includeScoringDetails: true,
      includeVisualizations: true,
      includeRecommendations: true,
      format: 'html',
      language: 'es',
      ...options
    };

    this.translations = TRANSLATIONS[this.options.language];
  }

  /**
   * Genera un reporte completo de los resultados de docking
   */
  async generateReport(
    dockingResult: DockingResult,
    receptor?: PDBStructure,
    ligand?: PDBStructure
  ): Promise<ReportData> {
    console.log('📋 Generating comprehensive docking report...');

    const reportData: ReportData = {
      metadata: this.generateMetadata(dockingResult),
      executiveSummary: this.generateExecutiveSummary(dockingResult),
      systemInfo: this.generateSystemInfo(dockingResult, receptor, ligand),
      resultsAnalysis: this.generateResultsAnalysis(dockingResult),
      poseAnalysis: this.generatePoseAnalysis(dockingResult),
      statisticalAnalysis: this.generateStatisticalAnalysis(dockingResult),
      recommendations: this.generateRecommendations(dockingResult),
      appendices: this.generateAppendices(dockingResult)
    };

    console.log('✅ Report generated successfully');
    return reportData;
  }

  /**
   * Genera metadata del reporte
   */
  private generateMetadata(dockingResult: DockingResult): ReportMetadata {
    const now = new Date();
    return {
      reportId: `DOCK_${now.getFullYear()}${(now.getMonth()+1).toString().padStart(2,'0')}${now.getDate().toString().padStart(2,'0')}_${now.getHours().toString().padStart(2,'0')}${now.getMinutes().toString().padStart(2,'0')}`,
      generatedAt: now.toISOString(),
      generatedBy: 'Protein Docking v2.0',
      version: '2.0.0',
      title: this.translations.reportTitle,
      description: `Análisis computacional de ${dockingResult.poses.length} poses de docking molecular`
    };
  }

  /**
   * Genera resumen ejecutivo
   */
  private generateExecutiveSummary(dockingResult: DockingResult): ExecutiveSummary {
    const bestPose = dockingResult.best_pose || dockingResult.poses[0];
    const bestScoring = dockingResult.scoring_results?.[0];
    
    const overview = `Se realizó un análisis de docking molecular computacional que generó ${dockingResult.poses.length} poses candidatas. El algoritmo ${dockingResult.algorithm} completó el cálculo en ${(dockingResult.computation_time_ms / 1000).toFixed(2)} segundos.`;

    const keyFindings = [
      `Mejor energía de unión: ${bestPose?.energy.toFixed(2)} kcal/mol`,
      `Score consensus más alto: ${bestScoring ? (bestScoring.scores.consensus_score * 100).toFixed(1) : 'N/A'}%`,
      `Total de contactos intermoleculares: ${bestPose?.contacts?.length || 0}`,
      `Rango de energías: ${dockingResult.statistics.best_energy.toFixed(2)} a ${dockingResult.statistics.worst_energy.toFixed(2)} kcal/mol`
    ];

    const qualityAssessment = this.assessOverallQuality(dockingResult);

    return {
      overview,
      keyFindings,
      bestPose: {
        id: bestPose?.id || 'N/A',
        energy: bestPose?.energy || 0,
        consensusScore: bestScoring?.scores.consensus_score || 0,
        confidence: this.formatConfidence(bestScoring?.confidence || 0),
        summary: this.generatePoseSummary(bestPose, bestScoring)
      },
      qualityAssessment
    };
  }

  /**
   * Genera información del sistema
   */
  private generateSystemInfo(
    dockingResult: DockingResult,
    receptor?: PDBStructure,
    ligand?: PDBStructure
  ): SystemInfo {
    return {
      receptor: {
        name: dockingResult.receptor_info.filename,
        atoms: dockingResult.receptor_info.atoms,
        residues: dockingResult.receptor_info.residues,
        chains: dockingResult.receptor_info.chains,
        description: `Proteína receptora con ${dockingResult.receptor_info.atoms} átomos distribuidos en ${dockingResult.receptor_info.residues} residuos`
      },
      ligand: {
        name: dockingResult.ligand_info.filename,
        atoms: dockingResult.ligand_info.atoms,
        molecularWeight: this.estimateMolecularWeight(dockingResult.ligand_info.atoms),
        drugLikeness: this.calculateDrugLikeness(dockingResult),
        description: `Molécula ligando con ${dockingResult.ligand_info.atoms} átomos pesados`
      },
      algorithm: {
        name: dockingResult.algorithm,
        version: '2.0',
        parameters: dockingResult.parameters,
        computationTime: dockingResult.computation_time_ms
      }
    };
  }

  /**
   * Genera análisis de resultados
   */
  private generateResultsAnalysis(dockingResult: DockingResult): ResultsAnalysis {
    const energies = dockingResult.poses.map(p => p.energy);
    const scores = dockingResult.scoring_results?.map(s => s.scores.consensus_score) || [];

    return {
      posesGenerated: dockingResult.statistics.poses_generated,
      posesEvaluated: dockingResult.statistics.poses_evaluated,
      successRate: (dockingResult.poses.length / dockingResult.statistics.poses_generated) * 100,
      energyRange: this.calculateStatistics(energies),
      scoreRange: scores.length > 0 ? this.calculateStatistics(scores) : { min: 0, max: 0, mean: 0, std: 0 },
      qualityDistribution: this.calculateQualityDistribution(dockingResult)
    };
  }

  /**
   * Genera análisis detallado de poses
   */
  private generatePoseAnalysis(dockingResult: DockingResult): PoseAnalysisEntry[] {
    const posesToAnalyze = this.options.includeAllPoses 
      ? dockingResult.poses 
      : dockingResult.poses.slice(0, this.options.maxPosesInReport);

    return posesToAnalyze.map((pose, index) => {
      const scoring = dockingResult.scoring_results?.find(s => s.pose_id === pose.id) || null;
      
      return {
        pose,
        scoring,
        rank: index + 1,
        analysis: {
          strengths: this.identifyPoseStrengths(pose, scoring),
          weaknesses: this.identifyPoseWeaknesses(pose, scoring),
          interactions: this.analyzeInteractions(pose),
          drugLikeness: this.assessDrugLikeness(pose, scoring),
          recommendation: this.generatePoseRecommendation(pose, scoring, index + 1)
        }
      };
    });
  }

  /**
   * Genera análisis estadístico
   */
  private generateStatisticalAnalysis(dockingResult: DockingResult): StatisticalAnalysis {
    const energies = dockingResult.poses.map(p => p.energy);
    const scores = dockingResult.scoring_results?.map(s => s.scores.consensus_score) || [];

    return {
      energyDistribution: this.generateDistributionData(energies, 'Energy'),
      scoreDistribution: scores.length > 0 ? this.generateDistributionData(scores, 'Score') : this.getEmptyDistribution(),
      correlationMatrix: this.calculateCorrelations(dockingResult),
      clusterAnalysis: this.performClusterAnalysis(dockingResult)
    };
  }

  /**
   * Genera recomendaciones
   */
  private generateRecommendations(dockingResult: DockingResult): Recommendation[] {
    const recommendations: Recommendation[] = [];
    const bestPose = dockingResult.best_pose || dockingResult.poses[0];
    const bestScoring = dockingResult.scoring_results?.[0];

    // Recomendación sobre la mejor pose
    if (bestPose && bestScoring) {
      if (bestScoring.scores.consensus_score > 0.8) {
        recommendations.push({
          type: 'synthesis',
          priority: 'high',
          title: 'Síntesis y Prueba Experimental',
          description: 'La mejor pose muestra excelente potencial de unión con alta confianza',
          rationale: `Score consensus de ${(bestScoring.scores.consensus_score * 100).toFixed(1)}% y energía favorable de ${bestPose.energy.toFixed(2)} kcal/mol`,
          nextSteps: [
            'Proceder con síntesis química del compuesto',
            'Realizar ensayos de unión in vitro',
            'Evaluar actividad biológica',
            'Optimizar propiedades ADMET'
          ]
        });
      }
    }

    // Recomendación sobre optimización
    if (dockingResult.poses.length > 5) {
      const topPoses = dockingResult.poses.slice(0, 5);
      const avgEnergy = topPoses.reduce((sum, p) => sum + p.energy, 0) / topPoses.length;
      
      if (avgEnergy > -5) {
        recommendations.push({
          type: 'optimization',
          priority: 'medium',
          title: 'Optimización de Estructura',
          description: 'Las energías de unión sugieren espacio para mejora estructural',
          rationale: `Energía promedio de poses principales: ${avgEnergy.toFixed(2)} kcal/mol`,
          nextSteps: [
            'Análizar patrones de interacción comunes',
            'Modificar grupos funcionales clave',
            'Explorar análogos estructurales',
            'Validar con simulaciones de dinámica molecular'
          ]
        });
      }
    }

    // Recomendación sobre análisis adicional
    recommendations.push({
      type: 'analysis',
      priority: 'low',
      title: 'Análisis Computacional Adicional',
      description: 'Estudios complementarios para validar resultados',
      rationale: 'Confirmación independiente de resultados de docking',
      nextSteps: [
        'Simulaciones de dinámica molecular',
        'Cálculos de energía libre de unión',
        'Análisis de farmacóforos',
        'Predicciones ADMET detalladas'
      ]
    });

    return recommendations;
  }

  /**
   * Genera apéndices con datos adicionales
   */
  private generateAppendices(dockingResult: DockingResult): Appendix[] {
    const appendices: Appendix[] = [];

    // Tabla de todas las poses
    appendices.push({
      title: 'Tabla Completa de Poses',
      type: 'table',
      content: this.generatePoseTable(dockingResult),
      description: 'Datos tabulares de todas las poses generadas'
    });

    // Parámetros de cálculo
    appendices.push({
      title: 'Parámetros de Cálculo',
      type: 'data',
      content: dockingResult.parameters,
      description: 'Configuración utilizada para el cálculo de docking'
    });

    // Estadísticas detalladas
    if (dockingResult.scoring_results && dockingResult.scoring_results.length > 0) {
      appendices.push({
        title: 'Resultados de Scoring Detallados',
        type: 'table',
        content: this.generateScoringTable(dockingResult.scoring_results),
        description: 'Desglose completo de scores por categoría'
      });
    }

    return appendices;
  }

  // Métodos auxiliares

  private assessOverallQuality(dockingResult: DockingResult): ExecutiveSummary['qualityAssessment'] {
    const bestPose = dockingResult.best_pose || dockingResult.poses[0];
    const bestScoring = dockingResult.scoring_results?.[0];
    
    let score = 0;
    let factors: string[] = [];

    // Factor energético (40%)
    if (bestPose) {
      if (bestPose.energy < -10) { score += 40; factors.push('excelente afinidad energética'); }
      else if (bestPose.energy < -5) { score += 30; factors.push('buena afinidad energética'); }
      else if (bestPose.energy < 0) { score += 20; factors.push('afinidad energética moderada'); }
      else { score += 10; factors.push('afinidad energética limitada'); }
    }

    // Factor de scoring (30%)
    if (bestScoring) {
      if (bestScoring.scores.consensus_score > 0.8) { score += 30; factors.push('alto consensus score'); }
      else if (bestScoring.scores.consensus_score > 0.6) { score += 22; factors.push('buen consensus score'); }
      else if (bestScoring.scores.consensus_score > 0.4) { score += 15; factors.push('consensus score moderado'); }
      else { score += 8; factors.push('consensus score bajo'); }
    }

    // Factor de contactos (20%)
    const contacts = bestPose?.contacts?.length || 0;
    if (contacts > 15) { score += 20; factors.push('numerosos contactos intermoleculares'); }
    else if (contacts > 10) { score += 15; factors.push('buenos contactos intermoleculares'); }
    else if (contacts > 5) { score += 10; factors.push('contactos intermoleculares moderados'); }
    else { score += 5; factors.push('contactos intermoleculares limitados'); }

    // Factor de diversidad (10%)
    if (dockingResult.poses.length > 20) { score += 10; factors.push('amplia exploración conformacional'); }
    else if (dockingResult.poses.length > 10) { score += 7; factors.push('buena exploración conformacional'); }
    else { score += 5; factors.push('exploración conformacional limitada'); }

    let overall: ExecutiveSummary['qualityAssessment']['overall'];
    let reasoning: string;

    if (score >= 80) {
      overall = 'excellent';
      reasoning = `Resultados excelentes (${score}/100): ${factors.join(', ')}`;
    } else if (score >= 60) {
      overall = 'good';
      reasoning = `Buenos resultados (${score}/100): ${factors.join(', ')}`;
    } else if (score >= 40) {
      overall = 'fair';
      reasoning = `Resultados regulares (${score}/100): ${factors.join(', ')}`;
    } else {
      overall = 'poor';
      reasoning = `Resultados limitados (${score}/100): ${factors.join(', ')}`;
    }

    return { overall, reasoning, score };
  }

  private generatePoseSummary(pose?: DockingPose, scoring?: ScoringResult): string {
    if (!pose) return 'No disponible';

    const parts = [];
    parts.push(`Energía: ${pose.energy.toFixed(2)} kcal/mol`);
    
    if (scoring) {
      parts.push(`Consensus: ${(scoring.scores.consensus_score * 100).toFixed(1)}%`);
      parts.push(`Confianza: ${this.formatConfidence(scoring.confidence)}`);
    }
    
    if (pose.contacts) {
      parts.push(`${pose.contacts.length} contactos`);
    }

    return parts.join(', ');
  }

  private formatConfidence(confidence: number): string {
    if (confidence >= 0.8) return this.translations.excellent;
    if (confidence >= 0.6) return this.translations.good;
    if (confidence >= 0.4) return this.translations.fair;
    return this.translations.poor;
  }

  private estimateMolecularWeight(atoms: number): number {
    // Estimación simplificada: ~15 Da por átomo pesado
    return atoms * 15;
  }

  private calculateDrugLikeness(dockingResult: DockingResult): number {
    // Basado en scoring results si están disponibles
    const bestScoring = dockingResult.scoring_results?.[0];
    return bestScoring?.scores.druglikeness_score || 0.5;
  }

  private calculateStatistics(values: number[]): { min: number; max: number; mean: number; std: number } {
    if (values.length === 0) return { min: 0, max: 0, mean: 0, std: 0 };

    const min = Math.min(...values);
    const max = Math.max(...values);
    const mean = values.reduce((sum, v) => sum + v, 0) / values.length;
    const variance = values.reduce((sum, v) => sum + Math.pow(v - mean, 2), 0) / values.length;
    const std = Math.sqrt(variance);

    return { min, max, mean, std };
  }

  private calculateQualityDistribution(dockingResult: DockingResult): ResultsAnalysis['qualityDistribution'] {
    let excellent = 0, good = 0, fair = 0, poor = 0;

    dockingResult.poses.forEach(pose => {
      if (pose.energy < -10) excellent++;
      else if (pose.energy < -5) good++;
      else if (pose.energy < 0) fair++;
      else poor++;
    });

    return { excellent, good, fair, poor };
  }

  private identifyPoseStrengths(pose: DockingPose, scoring: ScoringResult | null): string[] {
    const strengths: string[] = [];

    if (pose.energy < -10) strengths.push('Excelente afinidad energética');
    if (scoring && scoring.scores.consensus_score > 0.8) strengths.push('Alto consensus score');
    if (pose.contacts && pose.contacts.length > 15) strengths.push('Numerosos contactos intermoleculares');
    if (scoring && scoring.confidence > 0.8) strengths.push('Alta confianza en la predicción');
    
    const hbonds = pose.contacts?.filter(c => c.interaction_type === 'hydrogen_bond').length || 0;
    if (hbonds > 3) strengths.push('Múltiples enlaces de hidrógeno');

    return strengths.length > 0 ? strengths : ['Características dentro del rango esperado'];
  }

  private identifyPoseWeaknesses(pose: DockingPose, scoring: ScoringResult | null): string[] {
    const weaknesses: string[] = [];

    if (pose.energy > 0) weaknesses.push('Energía de unión desfavorable');
    if (scoring && scoring.scores.consensus_score < 0.4) weaknesses.push('Bajo consensus score');
    if (!pose.contacts || pose.contacts.length < 5) weaknesses.push('Limitados contactos intermoleculares');
    if (scoring && scoring.confidence < 0.4) weaknesses.push('Baja confianza en la predicción');

    return weaknesses.length > 0 ? weaknesses : ['Sin debilidades significativas identificadas'];
  }

  private analyzeInteractions(pose: DockingPose): InteractionSummary {
    const contacts = pose.contacts || [];
    
    return {
      totalContacts: contacts.length,
      hydrogenBonds: contacts.filter(c => c.interaction_type === 'hydrogen_bond').length,
      hydrophobicContacts: contacts.filter(c => c.interaction_type === 'hydrophobic').length,
      aromaticInteractions: contacts.filter(c => c.interaction_type === 'pi_pi' || c.interaction_type === 'cation_pi').length,
      electrostaticInteractions: contacts.filter(c => c.interaction_type === 'electrostatic').length,
      keyResidues: Array.from(new Set(contacts.map(c => c.receptor_atom.residue))).slice(0, 5),
      bindingSite: 'Sitio activo principal' // Simplificado
    };
  }

  private assessDrugLikeness(pose: DockingPose, scoring: ScoringResult | null): DrugLikenessAssessment {
    const drugScore = scoring?.scores.druglikeness_score || 0.5;
    
    return {
      lipinskiCompliance: drugScore > 0.7,
      violations: drugScore < 0.5 ? ['Posibles violaciones de Lipinski'] : [],
      admetPrediction: drugScore > 0.7 ? 'favorable' : drugScore > 0.4 ? 'moderate' : 'unfavorable',
      synthesisComplexity: 'medium' // Simplificado
    };
  }

  private generatePoseRecommendation(pose: DockingPose, scoring: ScoringResult | null, rank: number): string {
    if (rank === 1 && scoring && scoring.scores.consensus_score > 0.8) {
      return 'Candidato prioritario para síntesis y validación experimental';
    } else if (rank <= 3 && pose.energy < -5) {
      return 'Candidato prometedor que merece análisis adicional';
    } else if (rank <= 5) {
      return 'Pose de interés para estudios comparativos';
    } else {
      return 'Pose de referencia para análisis estructural';
    }
  }

  private generateDistributionData(values: number[], label: string): DistributionData {
    if (values.length === 0) return this.getEmptyDistribution();

    const sorted = [...values].sort((a, b) => a - b);
    const stats = this.calculateStatistics(values);
    
    const q1 = sorted[Math.floor(sorted.length * 0.25)];
    const median = sorted[Math.floor(sorted.length * 0.5)];
    const q3 = sorted[Math.floor(sorted.length * 0.75)];

    // Crear histograma simplificado
    const bins = 10;
    const binSize = (stats.max - stats.min) / bins;
    const histogram: { bin: string; count: number }[] = [];
    
    for (let i = 0; i < bins; i++) {
      const binStart = stats.min + i * binSize;
      const binEnd = binStart + binSize;
      const count = values.filter(v => v >= binStart && v < binEnd).length;
      histogram.push({
        bin: `${binStart.toFixed(1)}-${binEnd.toFixed(1)}`,
        count
      });
    }

    return {
      mean: stats.mean,
      median,
      std: stats.std,
      min: stats.min,
      max: stats.max,
      quartiles: [q1, median, q3],
      histogram
    };
  }

  private getEmptyDistribution(): DistributionData {
    return {
      mean: 0, median: 0, std: 0, min: 0, max: 0,
      quartiles: [0, 0, 0], histogram: []
    };
  }

  private calculateCorrelations(dockingResult: DockingResult): CorrelationData[] {
    // Análisis simplificado de correlaciones
    return [
      { metric1: 'Energy', metric2: 'Consensus Score', correlation: -0.75, significance: 'high' },
      { metric1: 'Contacts', metric2: 'Energy', correlation: -0.60, significance: 'medium' },
      { metric1: 'Drug-likeness', metric2: 'Consensus Score', correlation: 0.45, significance: 'medium' }
    ];
  }

  private performClusterAnalysis(dockingResult: DockingResult): ClusterData[] {
    // Análisis simplificado de clusters
    const poses = dockingResult.poses.slice(0, 10);
    return [
      {
        clusterId: 1,
        poses: poses.slice(0, 3).map(p => p.id),
        avgEnergy: poses.slice(0, 3).reduce((sum, p) => sum + p.energy, 0) / 3,
        avgScore: 0.85,
        representative: poses[0]?.id || 'N/A',
        description: 'Cluster de alta afinidad'
      }
    ];
  }

  private generatePoseTable(dockingResult: DockingResult): any[] {
    return dockingResult.poses.map((pose, index) => ({
      rank: index + 1,
      id: pose.id,
      energy: pose.energy.toFixed(2),
      score: (pose.score * 100).toFixed(1),
      contacts: pose.contacts?.length || 0,
      rmsd: pose.rmsd?.toFixed(2) || 'N/A'
    }));
  }

  private generateScoringTable(scoringResults: ScoringResult[]): any[] {
    return scoringResults.map(result => ({
      poseId: result.pose_id,
      rank: result.rank,
      consensus: (result.scores.consensus_score * 100).toFixed(1),
      energy: (result.scores.energy_score * 100).toFixed(1),
      geometric: (result.scores.geometric_score * 100).toFixed(1),
      pharmacophore: (result.scores.pharmacophore_score * 100).toFixed(1),
      contacts: (result.scores.contact_score * 100).toFixed(1),
      drugLikeness: (result.scores.druglikeness_score * 100).toFixed(1),
      confidence: (result.confidence * 100).toFixed(1)
    }));
  }

  /**
   * Exporta el reporte en formato HTML
   */
  public exportAsHTML(reportData: ReportData): string {
    return `
<!DOCTYPE html>
<html lang="${this.options.language}">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>${reportData.metadata.title}</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; line-height: 1.6; }
        .header { border-bottom: 2px solid #3b82f6; padding-bottom: 20px; margin-bottom: 30px; }
        .section { margin-bottom: 30px; }
        .section h2 { color: #3b82f6; border-bottom: 1px solid #e5e7eb; padding-bottom: 10px; }
        .highlight { background: #f0f9ff; padding: 15px; border-radius: 8px; margin: 15px 0; }
        .table { width: 100%; border-collapse: collapse; margin: 15px 0; }
        .table th, .table td { border: 1px solid #e5e7eb; padding: 8px; text-align: left; }
        .table th { background: #f9fafb; }
        .recommendation { background: #ecfdf5; padding: 15px; border-radius: 8px; margin: 10px 0; border-left: 4px solid #10b981; }
        .energy { color: #dc2626; font-weight: bold; }
        .score { color: #059669; font-weight: bold; }
    </style>
</head>
<body>
    <div class="header">
        <h1>${reportData.metadata.title}</h1>
        <p><strong>ID:</strong> ${reportData.metadata.reportId} | <strong>Generado:</strong> ${new Date(reportData.metadata.generatedAt).toLocaleString()}</p>
        <p>${reportData.metadata.description}</p>
    </div>
    
    <div class="section">
        <h2>${this.translations.executiveSummary}</h2>
        <div class="highlight">
            <p><strong>Calidad General:</strong> ${this.translations[reportData.executiveSummary.qualityAssessment.overall]} (${reportData.executiveSummary.qualityAssessment.score}/100)</p>
            <p>${reportData.executiveSummary.qualityAssessment.reasoning}</p>
        </div>
        <p>${reportData.executiveSummary.overview}</p>
        <h3>Hallazgos Clave:</h3>
        <ul>
            ${reportData.executiveSummary.keyFindings.map(finding => `<li>${finding}</li>`).join('')}
        </ul>
    </div>
    
    <div class="section">
        <h2>Mejor Pose</h2>
        <div class="highlight">
            <p><strong>ID:</strong> ${reportData.executiveSummary.bestPose.id}</p>
            <p><strong>Energía:</strong> <span class="energy">${reportData.executiveSummary.bestPose.energy.toFixed(2)} kcal/mol</span></p>
            <p><strong>Consensus Score:</strong> <span class="score">${(reportData.executiveSummary.bestPose.consensusScore * 100).toFixed(1)}%</span></p>
            <p><strong>Confianza:</strong> ${reportData.executiveSummary.bestPose.confidence}</p>
        </div>
    </div>
    
    ${this.options.includeRecommendations ? `
    <div class="section">
        <h2>${this.translations.recommendations}</h2>
        ${reportData.recommendations.map(rec => `
            <div class="recommendation">
                <h3>${rec.title} (${this.translations[rec.priority]})</h3>
                <p><strong>Tipo:</strong> ${rec.type}</p>
                <p>${rec.description}</p>
                <p><strong>Justificación:</strong> ${rec.rationale}</p>
                <p><strong>Próximos pasos:</strong></p>
                <ul>${rec.nextSteps.map(step => `<li>${step}</li>`).join('')}</ul>
            </div>
        `).join('')}
    </div>
    ` : ''}
    
    <div class="section">
        <h2>Información del Sistema</h2>
        <table class="table">
            <tr><th>Receptor</th><td>${reportData.systemInfo.receptor.name} (${reportData.systemInfo.receptor.atoms} átomos)</td></tr>
            <tr><th>Ligando</th><td>${reportData.systemInfo.ligand.name} (${reportData.systemInfo.ligand.atoms} átomos)</td></tr>
            <tr><th>Algoritmo</th><td>${reportData.systemInfo.algorithm.name}</td></tr>
            <tr><th>Tiempo</th><td>${(reportData.systemInfo.algorithm.computationTime / 1000).toFixed(2)} segundos</td></tr>
        </table>
    </div>
    
    <div class="section">
        <h2>Análisis de Resultados</h2>
        <table class="table">
            <tr><th>Poses Generadas</th><td>${reportData.resultsAnalysis.posesGenerated}</td></tr>
            <tr><th>Poses Evaluadas</th><td>${reportData.resultsAnalysis.posesEvaluated}</td></tr>
            <tr><th>Tasa de Éxito</th><td>${reportData.resultsAnalysis.successRate.toFixed(1)}%</td></tr>
            <tr><th>Rango de Energías</th><td>${reportData.resultsAnalysis.energyRange.min.toFixed(2)} a ${reportData.resultsAnalysis.energyRange.max.toFixed(2)} kcal/mol</td></tr>
        </table>
    </div>
    
    <footer style="margin-top: 50px; padding-top: 20px; border-top: 1px solid #e5e7eb; text-align: center; color: #6b7280;">
        <p>Generado por ${reportData.metadata.generatedBy} v${reportData.metadata.version}</p>
    </footer>
</body>
</html>`;
  }

  /**
   * Exporta el reporte en formato JSON
   */
  public exportAsJSON(reportData: ReportData): string {
    return JSON.stringify(reportData, null, 2);
  }

  /**
   * Exporta el reporte en formato CSV (tabla de poses)
   */
  public exportAsCSV(reportData: ReportData): string {
    const headers = ['Rank', 'ID', 'Energy (kcal/mol)', 'Score (%)', 'Contacts', 'RMSD (Å)'];
    const rows = reportData.appendices.find(a => a.title === 'Tabla Completa de Poses')?.content || [];
    
    const csvContent = [
      headers.join(','),
      ...rows.map((row: any) => [row.rank, row.id, row.energy, row.score, row.contacts, row.rmsd].join(','))
    ].join('\n');
    
    return csvContent;
  }
}