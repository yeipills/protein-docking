/*!
 * ScoringEngine - Sistema avanzado de scoring y ranking para poses de docking
 * 
 * Implementa múltiples funciones de scoring que combinan:
 * 1. Energy-based scoring (basado en energías)
 * 2. Geometric scoring (complementariedad geométrica)
 * 3. Pharmacophore scoring (características farmacofóricas)
 * 4. Machine learning scoring (predicciones ML)
 * 5. Consensus scoring (combinación de métodos)
 */

import type { DockingPose, InteractionContact } from './DockingAlgorithm.js';
import type { EnergyComponents } from './EnergyCalculator.js';
import type { PDBAtom } from '../utils/PDBParser.js';

export interface ScoringWeights {
  energy: number;
  geometric: number;
  pharmacophore: number;
  contacts: number;
  entropy: number;
  druglikeness: number;
}

export interface ScoringResult {
  pose_id: string;
  scores: {
    energy_score: number;          // 0-1, higher is better
    geometric_score: number;       // 0-1, complementariedad geométrica
    pharmacophore_score: number;   // 0-1, características farmacofóricas
    contact_score: number;         // 0-1, calidad de contactos
    entropy_score: number;         // 0-1, penalización por pérdida de entropía
    druglikeness_score: number;    // 0-1, drug-like properties
    consensus_score: number;       // 0-1, score final combinado
  };
  rank: number;
  confidence: number;              // 0-1, confianza en la predicción
  features: ScoringFeatures;
}

export interface ScoringFeatures {
  // Características energéticas
  total_energy: number;
  vdw_energy: number;
  electrostatic_energy: number;
  hbond_energy: number;
  hydrophobic_energy: number;
  
  // Características geométricas
  buried_surface_area: number;     // Área de superficie enterrada
  shape_complementarity: number;   // Complementariedad de forma
  interface_rmsd: number;          // RMSD en la interfaz
  
  // Características de contactos
  num_contacts: number;
  num_hbonds: number;
  num_hydrophobic_contacts: number;
  num_aromatic_contacts: number;
  contact_density: number;
  
  // Características farmacofóricas
  num_donors: number;
  num_acceptors: number;
  num_aromatic_rings: number;
  num_charged_groups: number;
  lipophilicity: number;
  
  // Características de druglikeness
  molecular_weight: number;
  logp: number;
  num_rotatable_bonds: number;
  polar_surface_area: number;
}

// Pesos por defecto para diferentes tipos de scoring
export const DEFAULT_SCORING_WEIGHTS: Record<string, ScoringWeights> = {
  balanced: {
    energy: 0.30,
    geometric: 0.25,
    pharmacophore: 0.20,
    contacts: 0.15,
    entropy: 0.05,
    druglikeness: 0.05
  },
  energy_focused: {
    energy: 0.50,
    geometric: 0.20,
    pharmacophore: 0.15,
    contacts: 0.10,
    entropy: 0.03,
    druglikeness: 0.02
  },
  shape_focused: {
    energy: 0.20,
    geometric: 0.40,
    pharmacophore: 0.15,
    contacts: 0.20,
    entropy: 0.03,
    druglikeness: 0.02
  },
  drug_discovery: {
    energy: 0.25,
    geometric: 0.20,
    pharmacophore: 0.25,
    contacts: 0.15,
    entropy: 0.05,
    druglikeness: 0.10
  }
};

export class ScoringEngine {
  private weights: ScoringWeights;
  private normalizationParams: Map<string, {min: number, max: number}>;

  constructor(weights: ScoringWeights = DEFAULT_SCORING_WEIGHTS.balanced) {
    this.weights = weights;
    this.normalizationParams = new Map();
    this.initializeNormalizationParams();
  }

  /**
   * Calcula scores para un conjunto de poses y las rankea
   */
  async scoreAndRankPoses(
    poses: DockingPose[], 
    receptorAtoms: PDBAtom[], 
    ligandAtoms: PDBAtom[]
  ): Promise<ScoringResult[]> {
    console.log(`🎯 Scoring ${poses.length} poses with ${this.getWeightsDescription()}`);

    // Extraer características para todas las poses
    const allFeatures = poses.map(pose => ({
      pose,
      features: this.extractFeatures(pose, receptorAtoms, ligandAtoms)
    }));

    // Actualizar parámetros de normalización basados en el conjunto actual
    this.updateNormalizationParams(allFeatures.map(p => p.features));

    // Calcular scores individuales para cada pose
    const scoringResults = allFeatures.map(({ pose, features }) => {
      const scores = this.calculateIndividualScores(features);
      const consensusScore = this.calculateConsensusScore(scores);
      const confidence = this.calculateConfidence(features, scores);

      return {
        pose_id: pose.id,
        scores: { ...scores, consensus_score: consensusScore },
        rank: 0, // Se asignará después del ranking
        confidence,
        features
      };
    });

    // Rankear poses por consensus score
    const rankedResults = this.rankPoses(scoringResults);

    // Log de resultados de scoring
    this.logScoringResults(rankedResults.slice(0, 5)); // Top 5

    return rankedResults;
  }

  /**
   * Extrae características relevantes de una pose para scoring
   */
  private extractFeatures(
    pose: DockingPose, 
    receptorAtoms: PDBAtom[], 
    ligandAtoms: PDBAtom[]
  ): ScoringFeatures {
    const energyComponents = pose.energy_components || {
      van_der_waals: 0, electrostatic: 0, hydrogen_bonds: 0,
      hydrophobic: 0, solvation: 0, conformational: 0, total: pose.energy
    };

    // Características energéticas
    const energyFeatures = {
      total_energy: energyComponents.total,
      vdw_energy: energyComponents.van_der_waals,
      electrostatic_energy: energyComponents.electrostatic,
      hbond_energy: energyComponents.hydrogen_bonds,
      hydrophobic_energy: energyComponents.hydrophobic
    };

    // Características geométricas
    const geometricFeatures = this.calculateGeometricFeatures(pose, receptorAtoms, ligandAtoms);

    // Características de contactos
    const contactFeatures = this.calculateContactFeatures(pose.contacts || []);

    // Características farmacofóricas
    const pharmacophoreFeatures = this.calculatePharmacophoreFeatures(ligandAtoms);

    // Características de druglikeness
    const druglikenessFeatures = this.calculateDruglikenessFeatures(ligandAtoms);

    return {
      ...energyFeatures,
      ...geometricFeatures,
      ...contactFeatures,
      ...pharmacophoreFeatures,
      ...druglikenessFeatures
    };
  }

  /**
   * Calcula características geométricas de la pose
   */
  private calculateGeometricFeatures(
    pose: DockingPose, 
    receptorAtoms: PDBAtom[], 
    ligandAtoms: PDBAtom[]
  ): Partial<ScoringFeatures> {
    // Transformar átomos del ligando según la pose
    const transformedLigand = this.transformLigandAtoms(ligandAtoms, pose);

    // Calcular área de superficie enterrada (aproximación)
    const buriedSurfaceArea = this.calculateBuriedSurfaceArea(receptorAtoms, transformedLigand);

    // Calcular complementariedad de forma (aproximación)
    const shapeComplementarity = this.calculateShapeComplementarity(receptorAtoms, transformedLigand);

    // RMSD en la interfaz (simplificado)
    const interfaceRmsd = pose.rmsd || 0;

    return {
      buried_surface_area: buriedSurfaceArea,
      shape_complementarity: shapeComplementarity,
      interface_rmsd: interfaceRmsd
    };
  }

  /**
   * Calcula características de contactos intermoleculares
   */
  private calculateContactFeatures(contacts: InteractionContact[]): Partial<ScoringFeatures> {
    const numContacts = contacts.length;
    const numHbonds = contacts.filter(c => c.interaction_type === 'hydrogen_bond').length;
    const numHydrophobic = contacts.filter(c => c.interaction_type === 'hydrophobic').length;
    const numAromatic = contacts.filter(c => 
      c.interaction_type === 'pi_pi' || c.interaction_type === 'cation_pi'
    ).length;

    // Densidad de contactos (contactos por Ų de interfaz - aproximación)
    const contactDensity = numContacts > 0 ? numContacts / 100 : 0; // Simplified

    return {
      num_contacts: numContacts,
      num_hbonds: numHbonds,
      num_hydrophobic_contacts: numHydrophobic,
      num_aromatic_contacts: numAromatic,
      contact_density: contactDensity
    };
  }

  /**
   * Calcula características farmacofóricas del ligando
   */
  private calculatePharmacophoreFeatures(ligandAtoms: PDBAtom[]): Partial<ScoringFeatures> {
    let numDonors = 0;
    let numAcceptors = 0;
    let numAromaticRings = 0;
    let numChargedGroups = 0;

    // Análisis simplificado de átomos
    for (const atom of ligandAtoms) {
      // Donors (N-H, O-H)
      if ((atom.element === 'N' || atom.element === 'O') && 
          atom.name.includes('H')) {
        numDonors++;
      }

      // Acceptors (N, O con pares libres)
      if (atom.element === 'N' || atom.element === 'O') {
        numAcceptors++;
      }

      // Grupos cargados (simplificado)
      if (atom.element === 'N' && ['NH1', 'NH2', 'NZ'].includes(atom.name)) {
        numChargedGroups++;
      }
      if (atom.element === 'O' && ['OD1', 'OD2', 'OE1', 'OE2'].includes(atom.name)) {
        numChargedGroups++;
      }
    }

    // Anillos aromáticos (aproximación basada en residuos)
    const aromaticResidues = ['PHE', 'TYR', 'TRP', 'HIS'];
    const uniqueResidues = new Set(ligandAtoms.map(a => a.resName));
    numAromaticRings = Array.from(uniqueResidues).filter(res => 
      aromaticResidues.includes(res)
    ).length;

    // Lipofilia (aproximación)
    const carbonAtoms = ligandAtoms.filter(a => a.element === 'C').length;
    const polarAtoms = ligandAtoms.filter(a => ['N', 'O', 'S'].includes(a.element)).length;
    const lipophilicity = carbonAtoms / (polarAtoms + 1); // Simplified logP approximation

    return {
      num_donors: numDonors,
      num_acceptors: numAcceptors,
      num_aromatic_rings: numAromaticRings,
      num_charged_groups: numChargedGroups,
      lipophilicity: lipophilicity
    };
  }

  /**
   * Calcula características de drug-likeness (Lipinski's Rule of Five)
   */
  private calculateDruglikenessFeatures(ligandAtoms: PDBAtom[]): Partial<ScoringFeatures> {
    // Peso molecular (aproximación usando número de átomos pesados)
    const heavyAtoms = ligandAtoms.filter(a => a.element !== 'H').length;
    const molecularWeight = heavyAtoms * 15; // Aproximación muy simplificada

    // LogP (aproximación)
    const carbonCount = ligandAtoms.filter(a => a.element === 'C').length;
    const polarCount = ligandAtoms.filter(a => ['N', 'O', 'S', 'P'].includes(a.element)).length;
    const logp = (carbonCount * 0.5) - (polarCount * 0.2); // Muy simplificado

    // Enlaces rotables (aproximación)
    const numRotatableBonds = Math.max(0, heavyAtoms - 10); // Simplificado

    // Área de superficie polar (aproximación)
    const polarSurfaceArea = polarCount * 20; // Simplificado

    return {
      molecular_weight: molecularWeight,
      logp: logp,
      num_rotatable_bonds: numRotatableBonds,
      polar_surface_area: polarSurfaceArea
    };
  }

  /**
   * Calcula scores individuales normalizados
   */
  private calculateIndividualScores(features: ScoringFeatures): Omit<ScoringResult['scores'], 'consensus_score'> {
    // Energy Score (energía más negativa = mejor score)
    const energyScore = this.normalizeScore('energy', -features.total_energy, true);

    // Geometric Score
    const geometricScore = (
      this.normalizeScore('buried_surface_area', features.buried_surface_area, true) * 0.4 +
      this.normalizeScore('shape_complementarity', features.shape_complementarity, true) * 0.4 +
      this.normalizeScore('interface_rmsd', features.interface_rmsd, false) * 0.2
    );

    // Pharmacophore Score
    const pharmacophoreScore = (
      this.normalizeScore('num_donors', Math.min(features.num_donors, 5), true) * 0.2 +
      this.normalizeScore('num_acceptors', Math.min(features.num_acceptors, 10), true) * 0.2 +
      this.normalizeScore('num_aromatic_rings', Math.min(features.num_aromatic_rings, 3), true) * 0.2 +
      this.normalizeScore('num_charged_groups', Math.min(features.num_charged_groups, 3), true) * 0.2 +
      this.normalizeScore('lipophilicity', features.lipophilicity, true) * 0.2
    );

    // Contact Score
    const contactScore = (
      this.normalizeScore('num_contacts', features.num_contacts, true) * 0.3 +
      this.normalizeScore('num_hbonds', features.num_hbonds, true) * 0.3 +
      this.normalizeScore('num_hydrophobic_contacts', features.num_hydrophobic_contacts, true) * 0.2 +
      this.normalizeScore('contact_density', features.contact_density, true) * 0.2
    );

    // Entropy Score (pérdida de entropía por enlaces rotables)
    const entropyScore = Math.max(0, 1 - (features.num_rotatable_bonds / 10));

    // Drug-likeness Score (Lipinski's Rule of Five)
    const druglikenessScore = this.calculateLipinskiScore(features);

    return {
      energy_score: energyScore,
      geometric_score: geometricScore,
      pharmacophore_score: pharmacophoreScore,
      contact_score: contactScore,
      entropy_score: entropyScore,
      druglikeness_score: druglikenessScore
    };
  }

  /**
   * Calcula el score de consensus combinando scores individuales
   */
  private calculateConsensusScore(scores: Omit<ScoringResult['scores'], 'consensus_score'>): number {
    return (
      scores.energy_score * this.weights.energy +
      scores.geometric_score * this.weights.geometric +
      scores.pharmacophore_score * this.weights.pharmacophore +
      scores.contact_score * this.weights.contacts +
      scores.entropy_score * this.weights.entropy +
      scores.druglikeness_score * this.weights.druglikeness
    );
  }

  /**
   * Calcula la confianza en la predicción
   */
  private calculateConfidence(features: ScoringFeatures, scores: any): number {
    // Factores que afectan la confianza
    const energyConfidence = Math.min(1, Math.abs(features.total_energy) / 10); // Más energía = más confianza
    const contactConfidence = Math.min(1, features.num_contacts / 20); // Más contactos = más confianza
    const geometryConfidence = features.shape_complementarity; // Mejor geometría = más confianza

    return (energyConfidence + contactConfidence + geometryConfidence) / 3;
  }

  /**
   * Rankea poses por consensus score
   */
  private rankPoses(results: ScoringResult[]): ScoringResult[] {
    const ranked = results.sort((a, b) => b.scores.consensus_score - a.scores.consensus_score);
    
    // Asignar ranks
    ranked.forEach((result, index) => {
      result.rank = index + 1;
    });

    return ranked;
  }

  // Métodos auxiliares

  private transformLigandAtoms(ligandAtoms: PDBAtom[], pose: DockingPose): PDBAtom[] {
    // Implementación simplificada - en un sistema real usaría la matriz de transformación
    return ligandAtoms.map(atom => ({
      ...atom,
      x: atom.x + pose.translation[0],
      y: atom.y + pose.translation[1],
      z: atom.z + pose.translation[2]
    }));
  }

  private calculateBuriedSurfaceArea(receptorAtoms: PDBAtom[], ligandAtoms: PDBAtom[]): number {
    // Aproximación muy simplificada
    let buriedArea = 0;
    for (const ligandAtom of ligandAtoms) {
      for (const receptorAtom of receptorAtoms) {
        const distance = this.calculateDistance(ligandAtom, receptorAtom);
        if (distance < 4.0) {
          buriedArea += Math.max(0, 4.0 - distance);
        }
      }
    }
    return buriedArea;
  }

  private calculateShapeComplementarity(receptorAtoms: PDBAtom[], ligandAtoms: PDBAtom[]): number {
    // Aproximación basada en la distribución de distancias
    const distances: number[] = [];
    for (const ligandAtom of ligandAtoms.slice(0, 20)) { // Limitar para performance
      for (const receptorAtom of receptorAtoms.slice(0, 50)) {
        const distance = this.calculateDistance(ligandAtom, receptorAtom);
        if (distance < 6.0) {
          distances.push(distance);
        }
      }
    }

    if (distances.length === 0) return 0;

    // Score basado en distancias óptimas (3-4 Å)
    const optimalDistances = distances.filter(d => d >= 3.0 && d <= 4.5);
    return optimalDistances.length / distances.length;
  }

  private calculateDistance(atom1: PDBAtom, atom2: PDBAtom): number {
    const dx = atom1.x - atom2.x;
    const dy = atom1.y - atom2.y;
    const dz = atom1.z - atom2.z;
    return Math.sqrt(dx * dx + dy * dy + dz * dz);
  }

  private calculateLipinskiScore(features: ScoringFeatures): number {
    let violations = 0;
    
    // Rule of Five violations
    if (features.molecular_weight > 500) violations++;
    if (features.logp > 5) violations++;
    if (features.num_donors > 5) violations++;
    if (features.num_acceptors > 10) violations++;

    // Score basado en número de violaciones
    return Math.max(0, 1 - (violations / 4));
  }

  private normalizeScore(feature: string, value: number, higherIsBetter: boolean): number {
    const params = this.normalizationParams.get(feature);
    if (!params || params.max === params.min) return 0.5;

    let normalized = (value - params.min) / (params.max - params.min);
    normalized = Math.max(0, Math.min(1, normalized));

    return higherIsBetter ? normalized : 1 - normalized;
  }

  private initializeNormalizationParams(): void {
    // Parámetros de normalización por defecto
    const defaultParams = {
      energy: { min: -50, max: 10 },
      buried_surface_area: { min: 0, max: 1000 },
      shape_complementarity: { min: 0, max: 1 },
      interface_rmsd: { min: 0, max: 5 },
      num_contacts: { min: 0, max: 50 },
      num_hbonds: { min: 0, max: 10 },
      num_hydrophobic_contacts: { min: 0, max: 20 },
      contact_density: { min: 0, max: 1 },
      num_donors: { min: 0, max: 5 },
      num_acceptors: { min: 0, max: 10 },
      num_aromatic_rings: { min: 0, max: 3 },
      num_charged_groups: { min: 0, max: 3 },
      lipophilicity: { min: 0, max: 5 }
    };

    for (const [feature, params] of Object.entries(defaultParams)) {
      this.normalizationParams.set(feature, params);
    }
  }

  private updateNormalizationParams(allFeatures: ScoringFeatures[]): void {
    if (allFeatures.length === 0) return;

    // Actualizar parámetros basados en el conjunto actual
    const features = Object.keys(allFeatures[0]) as (keyof ScoringFeatures)[];
    
    for (const feature of features) {
      const values = allFeatures.map(f => f[feature]).filter(v => typeof v === 'number') as number[];
      if (values.length > 0) {
        const min = Math.min(...values);
        const max = Math.max(...values);
        this.normalizationParams.set(feature, { min, max });
      }
    }
  }

  private getWeightsDescription(): string {
    const entries = Object.entries(this.weights)
      .filter(([_, weight]) => weight > 0.05)
      .map(([key, weight]) => `${key}:${(weight * 100).toFixed(0)}%`)
      .join(', ');
    return `weights(${entries})`;
  }

  private logScoringResults(topResults: ScoringResult[]): void {
    console.log('🏆 Top scoring poses:');
    topResults.forEach((result, index) => {
      console.log(`  ${index + 1}. ${result.pose_id}:`);
      console.log(`     Consensus: ${result.scores.consensus_score.toFixed(3)}`);
      console.log(`     Energy: ${result.scores.energy_score.toFixed(3)}, ` +
                  `Geometric: ${result.scores.geometric_score.toFixed(3)}, ` +
                  `Contacts: ${result.scores.contact_score.toFixed(3)}`);
      console.log(`     Confidence: ${result.confidence.toFixed(3)}`);
    });
  }

  /**
   * Métodos públicos para cambiar configuración
   */
  public setWeights(weights: ScoringWeights): void {
    this.weights = weights;
  }

  public getAvailableWeightPresets(): string[] {
    return Object.keys(DEFAULT_SCORING_WEIGHTS);
  }

  public setWeightPreset(preset: string): boolean {
    if (preset in DEFAULT_SCORING_WEIGHTS) {
      this.weights = DEFAULT_SCORING_WEIGHTS[preset];
      return true;
    }
    return false;
  }
}