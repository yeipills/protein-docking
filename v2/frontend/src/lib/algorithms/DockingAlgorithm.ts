/*!
 * DockingAlgorithm - Algoritmo básico de docking molecular
 * 
 * Implementa un algoritmo simplificado pero funcional de docking que:
 * 1. Genera poses candidatas por sampling monte carlo
 * 2. Evalúa energías de interacción 
 * 3. Optimiza poses con gradient descent
 * 4. Rankea resultados por score
 * 
 * Nota: Esta es una implementación educativa/demo. Un sistema real
 * usaría algoritmos más sofisticados como AutoDock Vina, GOLD, etc.
 */

import type { PDBStructure, PDBAtom } from '../utils/PDBParser.js';
import { EnergyCalculator, type EnergyComponents } from './EnergyCalculator.js';
import { ScoringEngine, type ScoringResult, DEFAULT_SCORING_WEIGHTS } from './ScoringEngine.js';
import { log } from '../utils/Logger.js';

export interface DockingPose {
  id: string;
  translation: [number, number, number];    // x, y, z translation
  rotation: [number, number, number];       // euler angles (radians)
  energy: number;                           // kcal/mol total energy
  score: number;                            // normalized score 0-1
  rmsd: number;                             // RMSD from reference (if available)
  contacts: InteractionContact[];
  transformation_matrix: number[];          // 4x4 transformation matrix
  energy_components?: EnergyComponents;     // Detailed energy breakdown
}

export interface InteractionContact {
  receptor_atom: ContactAtom;
  ligand_atom: ContactAtom;
  distance: number;
  interaction_type: 'hydrogen_bond' | 'hydrophobic' | 'electrostatic' | 'van_der_waals';
  energy_contribution: number;
}

export interface ContactAtom {
  index: number;
  element: string;
  position: [number, number, number];
  residue: string;
  chain: string;
  charge?: number;
}

export interface DockingResult {
  receptor_info: {
    filename: string;
    atoms: number;
    residues: number;
    chains: number;
  };
  ligand_info: {
    filename: string;
    atoms: number;
    residues: number;
    chains: number;
  };
  poses: DockingPose[];
  computation_time_ms: number;
  parameters: DockingParameters;
  scoring_results?: ScoringResult[];    // Resultados de scoring avanzado
  best_pose?: DockingPose;             // Mejor pose según scoring
  algorithm: string;                   // Algoritmo de docking usado
  statistics: {
    poses_generated: number;
    poses_evaluated: number;
    best_energy: number;
    worst_energy: number;
    mean_energy: number;
  };
}

export interface DockingParameters {
  max_poses: number;
  energy_threshold: number;        // kcal/mol - poses above this are discarded
  rmsd_threshold: number;          // Angstroms - for clustering
  sampling_strategy: 'random' | 'grid' | 'monte_carlo' | 'genetic';
  optimization_rounds: number;
  parallel_workers: number;
  use_gpu_acceleration: boolean;
  search_space: {
    center: [number, number, number];
    size: [number, number, number];
  };
  advanced_scoring: boolean;
}

// Energy calculation constants
const ENERGY_CONSTANTS = {
  VDW_WELL_DEPTH: -0.2,           // kcal/mol
  VDW_CLASH_PENALTY: 10.0,        // kcal/mol per Angstrom of overlap
  ELECTROSTATIC_SCALE: 332.0,     // conversion factor
  HYDROGEN_BOND_ENERGY: -2.0,     // kcal/mol
  HYDROPHOBIC_BONUS: -0.5,        // kcal/mol
  DEFORMATION_PENALTY: 1.0,       // kcal/mol per degree of deformation
};

export class DockingAlgorithm {
  private receptor: PDBStructure;
  private ligand: PDBStructure;
  private parameters: DockingParameters;
  private receptor_atoms: PDBAtom[];
  private ligand_atoms: PDBAtom[];
  private energyCalculator: EnergyCalculator;
  private scoringEngine: ScoringEngine;

  constructor(receptor: PDBStructure, ligand: PDBStructure, params: DockingParameters) {
    this.receptor = receptor;
    this.ligand = ligand;
    this.parameters = params;
    this.receptor_atoms = receptor.atoms;
    this.ligand_atoms = ligand.atoms;
    
    // Initialize advanced energy calculator
    this.energyCalculator = new EnergyCalculator(
      this.receptor_atoms, 
      this.ligand_atoms, 
      params.advanced_scoring
    );
    
    // Initialize scoring engine with appropriate weights
    const scoringPreset = params.advanced_scoring ? 'balanced' : 'energy_focused';
    this.scoringEngine = new ScoringEngine(DEFAULT_SCORING_WEIGHTS[scoringPreset]);
    
    log.info('Inicializando algoritmo de docking', 'DockingAlgorithm', {
      receptorAtoms: receptor.stats.totalAtoms,
      ligandAtoms: ligand.stats.totalAtoms,
      searchSpaceSize: params.search_space.size,
      advancedScoring: params.advanced_scoring,
      maxPoses: params.max_poses,
      strategy: params.sampling_strategy
    });
  }

  async performDocking(): Promise<DockingResult> {
    log.startPerf('docking_calculation');
    log.info('Iniciando cálculo de docking', 'DockingAlgorithm');
    
    // Step 1: Initialize search space
    this.initializeSearchSpace();
    
    // Step 2: Generate candidate poses
    log.info(`Generando ${this.parameters.max_poses} poses candidatas`, 'DockingAlgorithm');
    const candidates = await this.generateCandidatePoses();
    
    // Step 3: Evaluate energies
    log.info(`Evaluando energías para ${candidates.length} poses`, 'DockingAlgorithm');
    const evaluatedPoses = await this.evaluatePoses(candidates);
    
    // Step 4: Optimize promising poses
    const posesToOptimize = Math.min(evaluatedPoses.length, 20);
    log.info(`Optimizando ${posesToOptimize} mejores poses`, 'DockingAlgorithm');
    const optimizedPoses = await this.optimizePoses(evaluatedPoses.slice(0, 20));
    
    // Step 5: Cluster and rank results
    log.info('Agrupando y rankeando resultados', 'DockingAlgorithm');
    const finalPoses = this.clusterAndRank(optimizedPoses);
    
    // Step 6: Advanced scoring and re-ranking
    log.info(`Aplicando scoring avanzado a ${finalPoses.length} poses`, 'DockingAlgorithm');
    let scoringResults: ScoringResult[] = [];
    let bestPose: DockingPose | undefined;
    
    if (finalPoses.length > 0) {
      scoringResults = await this.scoringEngine.scoreAndRankPoses(
        finalPoses, 
        this.receptor_atoms, 
        this.ligand_atoms
      );
      
      // Update pose ordering based on consensus scores
      const scoringMap = new Map(scoringResults.map(s => [s.pose_id, s]));
      finalPoses.sort((a, b) => {
        const scoreA = scoringMap.get(a.id)?.scores.consensus_score || 0;
        const scoreB = scoringMap.get(b.id)?.scores.consensus_score || 0;
        return scoreB - scoreA; // Higher score is better
      });
      
      // Update pose scores
      finalPoses.forEach(pose => {
        const scoringResult = scoringMap.get(pose.id);
        if (scoringResult) {
          pose.score = scoringResult.scores.consensus_score;
        }
      });
      
      bestPose = finalPoses[0];
    }
    
    const computationTime = log.endPerf('docking_calculation', 'DockingAlgorithm');
    
    const result: DockingResult = {
      receptor_info: {
        filename: 'receptor.pdb',
        atoms: this.receptor.stats.totalAtoms,
        residues: this.receptor.stats.totalResidues,
        chains: this.receptor.stats.totalChains
      },
      ligand_info: {
        filename: 'ligand.pdb',
        atoms: this.ligand.stats.totalAtoms,
        residues: this.ligand.stats.totalResidues,
        chains: this.ligand.stats.totalChains
      },
      poses: finalPoses,
      computation_time_ms: computationTime,
      parameters: this.parameters,
      scoring_results: scoringResults,
      best_pose: bestPose,
      algorithm: 'DockingAlgorithm v2.0',
      statistics: {
        poses_generated: candidates.length,
        poses_evaluated: evaluatedPoses.length,
        best_energy: finalPoses.length > 0 ? finalPoses[0].energy : 0,
        worst_energy: finalPoses.length > 0 ? finalPoses[finalPoses.length - 1].energy : 0,
        mean_energy: finalPoses.length > 0 ? 
          finalPoses.reduce((sum, p) => sum + p.energy, 0) / finalPoses.length : 0
      }
    };
    
    log.info('Docking completado exitosamente', 'DockingAlgorithm', {
      computationTime: Math.round(computationTime),
      finalPoses: finalPoses.length,
      bestEnergy: result.statistics.best_energy,
      bestPoseId: bestPose?.id,
      consensusScore: scoringResults.length > 0 ? scoringResults[0].scores.consensus_score : undefined
    });
    
    return result;
  }

  private initializeSearchSpace(): void {
    // Calculate search space around receptor binding site
    if (!this.parameters.search_space.center.some(v => v !== 0)) {
      // Auto-detect binding site (simplified - use receptor center)
      this.parameters.search_space.center = [...this.receptor.center];
    }
    
    if (!this.parameters.search_space.size.some(v => v !== 0)) {
      // Default search space size
      this.parameters.search_space.size = [20, 20, 20];
    }
    
    console.log(`🎯 Search space centered at (${this.parameters.search_space.center.map(c => c.toFixed(1)).join(', ')})`);
  }

  private async generateCandidatePoses(): Promise<DockingPose[]> {
    const poses: DockingPose[] = [];
    const { max_poses, sampling_strategy } = this.parameters;
    
    for (let i = 0; i < max_poses; i++) {
      let pose: DockingPose;
      
      switch (sampling_strategy) {
        case 'grid':
          pose = this.generateGridPose(i, max_poses);
          break;
        case 'monte_carlo':
          pose = this.generateMonteCarloPose();
          break;
        case 'genetic':
          pose = this.generateGeneticPose(poses);
          break;
        case 'random':
        default:
          pose = this.generateRandomPose();
          break;
      }
      
      poses.push(pose);
      
      // Show progress for long calculations
      if (i % Math.max(1, Math.floor(max_poses / 10)) === 0) {
        console.log(`  Generated ${i + 1}/${max_poses} poses (${(((i + 1) / max_poses) * 100).toFixed(1)}%)`);
      }
    }
    
    return poses;
  }

  private generateRandomPose(): DockingPose {
    const { center, size } = this.parameters.search_space;
    
    // Random translation within search space
    const translation: [number, number, number] = [
      center[0] + (Math.random() - 0.5) * size[0],
      center[1] + (Math.random() - 0.5) * size[1],
      center[2] + (Math.random() - 0.5) * size[2]
    ];
    
    // Random rotation (euler angles)
    const rotation: [number, number, number] = [
      Math.random() * 2 * Math.PI,
      Math.random() * 2 * Math.PI,
      Math.random() * 2 * Math.PI
    ];
    
    return {
      id: `pose_${Date.now()}_${Math.random().toString(36).substr(2, 6)}`,
      translation,
      rotation,
      energy: 0,
      score: 0,
      rmsd: 0,
      contacts: [],
      transformation_matrix: this.createTransformationMatrix(translation, rotation)
    };
  }

  private generateGridPose(index: number, total: number): DockingPose {
    // Generate poses on a regular grid
    const gridDim = Math.ceil(Math.pow(total, 1/3));
    const i = index % gridDim;
    const j = Math.floor(index / gridDim) % gridDim;
    const k = Math.floor(index / (gridDim * gridDim));
    
    const { center, size } = this.parameters.search_space;
    
    const translation: [number, number, number] = [
      center[0] + (i / (gridDim - 1) - 0.5) * size[0],
      center[1] + (j / (gridDim - 1) - 0.5) * size[1],
      center[2] + (k / (gridDim - 1) - 0.5) * size[2]
    ];
    
    // Random rotation for each grid point
    const rotation: [number, number, number] = [
      Math.random() * 2 * Math.PI,
      Math.random() * 2 * Math.PI,
      Math.random() * 2 * Math.PI
    ];
    
    return {
      id: `grid_${i}_${j}_${k}`,
      translation,
      rotation,
      energy: 0,
      score: 0,
      rmsd: 0,
      contacts: [],
      transformation_matrix: this.createTransformationMatrix(translation, rotation)
    };
  }

  private generateMonteCarloPose(): DockingPose {
    // Monte Carlo sampling with bias toward promising regions
    const { center, size } = this.parameters.search_space;
    
    // Gaussian distribution around center (more realistic)
    const translation: [number, number, number] = [
      center[0] + this.randomGaussian() * size[0] * 0.3,
      center[1] + this.randomGaussian() * size[1] * 0.3,
      center[2] + this.randomGaussian() * size[2] * 0.3
    ];
    
    // Slightly biased rotation (favor certain orientations)
    const rotation: [number, number, number] = [
      this.randomGaussian() * Math.PI,
      this.randomGaussian() * Math.PI,
      this.randomGaussian() * Math.PI
    ];
    
    return {
      id: `mc_${Date.now()}_${Math.random().toString(36).substr(2, 6)}`,
      translation,
      rotation,
      energy: 0,
      score: 0,
      rmsd: 0,
      contacts: [],
      transformation_matrix: this.createTransformationMatrix(translation, rotation)
    };
  }

  private generateGeneticPose(existingPoses: DockingPose[]): DockingPose {
    if (existingPoses.length < 2) {
      return this.generateRandomPose();
    }
    
    // Select two parent poses (roulette wheel selection based on fitness)
    const parent1 = this.selectParent(existingPoses);
    const parent2 = this.selectParent(existingPoses);
    
    // Crossover
    const alpha = Math.random();
    const translation: [number, number, number] = [
      alpha * parent1.translation[0] + (1 - alpha) * parent2.translation[0],
      alpha * parent1.translation[1] + (1 - alpha) * parent2.translation[1],
      alpha * parent1.translation[2] + (1 - alpha) * parent2.translation[2]
    ];
    
    const rotation: [number, number, number] = [
      alpha * parent1.rotation[0] + (1 - alpha) * parent2.rotation[0],
      alpha * parent1.rotation[1] + (1 - alpha) * parent2.rotation[1],
      alpha * parent1.rotation[2] + (1 - alpha) * parent2.rotation[2]
    ];
    
    // Mutation
    const mutationRate = 0.1;
    if (Math.random() < mutationRate) {
      const mutationStrength = 0.2;
      translation[0] += (Math.random() - 0.5) * mutationStrength;
      translation[1] += (Math.random() - 0.5) * mutationStrength;
      translation[2] += (Math.random() - 0.5) * mutationStrength;
      
      rotation[0] += (Math.random() - 0.5) * mutationStrength;
      rotation[1] += (Math.random() - 0.5) * mutationStrength;
      rotation[2] += (Math.random() - 0.5) * mutationStrength;
    }
    
    return {
      id: `genetic_${Date.now()}_${Math.random().toString(36).substr(2, 6)}`,
      translation,
      rotation,
      energy: 0,
      score: 0,
      rmsd: 0,
      contacts: [],
      transformation_matrix: this.createTransformationMatrix(translation, rotation)
    };
  }

  private selectParent(poses: DockingPose[]): DockingPose {
    // Roulette wheel selection (higher fitness = lower energy)
    const maxEnergy = Math.max(...poses.map(p => p.energy));
    const fitness = poses.map(p => maxEnergy - p.energy + 1);
    const totalFitness = fitness.reduce((sum, f) => sum + f, 0);
    
    let random = Math.random() * totalFitness;
    for (let i = 0; i < poses.length; i++) {
      random -= fitness[i];
      if (random <= 0) {
        return poses[i];
      }
    }
    
    return poses[poses.length - 1];
  }

  private async evaluatePoses(poses: DockingPose[]): Promise<DockingPose[]> {
    const evaluatedPoses: DockingPose[] = [];
    
    for (let i = 0; i < poses.length; i++) {
      const pose = poses[i];
      
      // Transform ligand to pose position
      const transformedLigand = this.transformLigand(pose);
      
      // Calculate interaction energy
      const energy = this.calculateInteractionEnergy(transformedLigand, pose);
      
      // Skip poses above energy threshold
      if (energy > this.parameters.energy_threshold) {
        continue;
      }
      
      pose.energy = energy;
      pose.score = this.energyToScore(energy);
      
      evaluatedPoses.push(pose);
      
      // Show progress
      if (i % Math.max(1, Math.floor(poses.length / 10)) === 0) {
        console.log(`  Evaluated ${i + 1}/${poses.length} poses (${(((i + 1) / poses.length) * 100).toFixed(1)}%)`);
      }
    }
    
    // Sort by energy (best first)
    evaluatedPoses.sort((a, b) => a.energy - b.energy);
    
    console.log(`  Kept ${evaluatedPoses.length}/${poses.length} poses below energy threshold`);
    return evaluatedPoses;
  }

  private transformLigand(pose: DockingPose): PDBAtom[] {
    const transformed: PDBAtom[] = [];
    
    for (const atom of this.ligand_atoms) {
      // Apply transformation matrix
      const coords = this.applyTransformation(
        [atom.x, atom.y, atom.z],
        pose.transformation_matrix
      );
      
      transformed.push({
        ...atom,
        x: coords[0],
        y: coords[1],
        z: coords[2]
      });
    }
    
    return transformed;
  }

  private calculateInteractionEnergy(transformedLigand: PDBAtom[], pose: DockingPose): number {
    // Use the advanced energy calculator
    const energyComponents = this.energyCalculator.calculateTotalEnergy(transformedLigand);
    
    // Store detailed energy breakdown in pose
    pose.energy_components = energyComponents;
    
    // Convert detailed contacts to legacy format for backwards compatibility
    // This would need proper implementation with the detailed contacts from EnergyCalculator
    pose.contacts = this.createLegacyContacts(transformedLigand);
    
    // Log detailed energy breakdown for debugging
    if (Math.abs(energyComponents.total) > 1.0) {
      console.log(`💫 Energy breakdown for pose ${pose.id}:`);
      console.log(`   Van der Waals: ${energyComponents.van_der_waals.toFixed(2)} kcal/mol`);
      console.log(`   Electrostatic: ${energyComponents.electrostatic.toFixed(2)} kcal/mol`);
      console.log(`   Hydrogen bonds: ${energyComponents.hydrogen_bonds.toFixed(2)} kcal/mol`);
      console.log(`   Hydrophobic: ${energyComponents.hydrophobic.toFixed(2)} kcal/mol`);
      if (this.parameters.advanced_scoring) {
        console.log(`   Solvation: ${energyComponents.solvation.toFixed(2)} kcal/mol`);
        console.log(`   Conformational: ${energyComponents.conformational.toFixed(2)} kcal/mol`);
      }
      console.log(`   Total: ${energyComponents.total.toFixed(2)} kcal/mol`);
    }
    
    return energyComponents.total;
  }

  private createLegacyContacts(transformedLigand: PDBAtom[]): InteractionContact[] {
    const contacts: InteractionContact[] = [];
    
    // Create simplified contacts for backwards compatibility
    for (const ligandAtom of transformedLigand.slice(0, 10)) { // Limit for performance
      for (const receptorAtom of this.receptor_atoms.slice(0, 50)) { // Limit for performance
        const distance = this.calculateDistance(
          [ligandAtom.x, ligandAtom.y, ligandAtom.z],
          [receptorAtom.x, receptorAtom.y, receptorAtom.z]
        );
        
        if (distance < 4.0) { // Only close contacts
          contacts.push({
            receptor_atom: {
              index: 0,
              element: receptorAtom.element,
              position: [receptorAtom.x, receptorAtom.y, receptorAtom.z],
              residue: receptorAtom.resName,
              chain: receptorAtom.chainID
            },
            ligand_atom: {
              index: 0,
              element: ligandAtom.element,
              position: [ligandAtom.x, ligandAtom.y, ligandAtom.z],
              residue: ligandAtom.resName,
              chain: ligandAtom.chainID
            },
            distance,
            interaction_type: distance < 3.5 ? 'hydrogen_bond' : 'van_der_waals',
            energy_contribution: -0.5 // Simplified
          });
        }
      }
    }
    
    return contacts.slice(0, 20); // Return top 20 contacts
  }

  private calculatePairwiseEnergy(
    ligandAtom: PDBAtom, 
    receptorAtom: PDBAtom, 
    distance: number
  ): InteractionContact {
    let energy = 0;
    let interactionType: InteractionContact['interaction_type'] = 'van_der_waals';
    
    // Van der Waals interaction
    const vdwDistance = this.getVdWRadius(ligandAtom.element) + this.getVdWRadius(receptorAtom.element);
    
    if (distance < vdwDistance * 0.8) {
      // Clash penalty
      energy += ENERGY_CONSTANTS.VDW_CLASH_PENALTY * (vdwDistance * 0.8 - distance);
    } else if (distance < vdwDistance * 1.2) {
      // Attractive van der Waals
      energy += ENERGY_CONSTANTS.VDW_WELL_DEPTH * Math.exp(-(distance - vdwDistance) / 0.5);
    }
    
    // Hydrogen bonding
    if (this.canFormHydrogenBond(ligandAtom, receptorAtom) && distance < 3.5) {
      energy += ENERGY_CONSTANTS.HYDROGEN_BOND_ENERGY * (1 - distance / 3.5);
      interactionType = 'hydrogen_bond';
    }
    
    // Hydrophobic interactions
    if (this.isHydrophobic(ligandAtom.element) && this.isHydrophobic(receptorAtom.element) && distance < 4.0) {
      energy += ENERGY_CONSTANTS.HYDROPHOBIC_BONUS;
      interactionType = 'hydrophobic';
    }
    
    // Electrostatic interactions (simplified)
    const charge1 = this.getPartialCharge(ligandAtom.element);
    const charge2 = this.getPartialCharge(receptorAtom.element);
    if (Math.abs(charge1) > 0.1 && Math.abs(charge2) > 0.1) {
      energy += ENERGY_CONSTANTS.ELECTROSTATIC_SCALE * charge1 * charge2 / distance;
      interactionType = 'electrostatic';
    }
    
    return {
      receptor_atom: {
        index: 0, // Would need proper indexing
        element: receptorAtom.element,
        position: [receptorAtom.x, receptorAtom.y, receptorAtom.z],
        residue: receptorAtom.resName,
        chain: receptorAtom.chainID
      },
      ligand_atom: {
        index: 0, // Would need proper indexing
        element: ligandAtom.element,
        position: [ligandAtom.x, ligandAtom.y, ligandAtom.z],
        residue: ligandAtom.resName,
        chain: ligandAtom.chainID
      },
      distance,
      interaction_type: interactionType,
      energy_contribution: energy
    };
  }

  private async optimizePoses(poses: DockingPose[]): Promise<DockingPose[]> {
    const optimized: DockingPose[] = [];
    
    for (let i = 0; i < poses.length; i++) {
      const optimizedPose = await this.localOptimization(poses[i]);
      optimized.push(optimizedPose);
      
      if (i % 5 === 0) {
        console.log(`  Optimized ${i + 1}/${poses.length} poses`);
      }
    }
    
    return optimized.sort((a, b) => a.energy - b.energy);
  }

  private async localOptimization(pose: DockingPose): Promise<DockingPose> {
    let bestPose = { ...pose };
    let bestEnergy = pose.energy;
    
    const stepSize = 0.1;
    const rotationStep = 0.05;
    
    // Simple gradient descent optimization
    for (let round = 0; round < this.parameters.optimization_rounds; round++) {
      let improved = false;
      
      // Try small perturbations in each direction
      const perturbations = [
        [stepSize, 0, 0, 0, 0, 0],
        [-stepSize, 0, 0, 0, 0, 0],
        [0, stepSize, 0, 0, 0, 0],
        [0, -stepSize, 0, 0, 0, 0],
        [0, 0, stepSize, 0, 0, 0],
        [0, 0, -stepSize, 0, 0, 0],
        [0, 0, 0, rotationStep, 0, 0],
        [0, 0, 0, -rotationStep, 0, 0],
        [0, 0, 0, 0, rotationStep, 0],
        [0, 0, 0, 0, -rotationStep, 0],
        [0, 0, 0, 0, 0, rotationStep],
        [0, 0, 0, 0, 0, -rotationStep]
      ];
      
      for (const perturbation of perturbations) {
        const testPose: DockingPose = {
          ...bestPose,
          id: `${bestPose.id}_opt_${round}`,
          translation: [
            bestPose.translation[0] + perturbation[0],
            bestPose.translation[1] + perturbation[1],
            bestPose.translation[2] + perturbation[2]
          ],
          rotation: [
            bestPose.rotation[0] + perturbation[3],
            bestPose.rotation[1] + perturbation[4],
            bestPose.rotation[2] + perturbation[5]
          ]
        };
        
        testPose.transformation_matrix = this.createTransformationMatrix(
          testPose.translation, 
          testPose.rotation
        );
        
        const transformedLigand = this.transformLigand(testPose);
        const energy = this.calculateInteractionEnergy(transformedLigand, testPose);
        
        if (energy < bestEnergy) {
          bestEnergy = energy;
          bestPose = testPose;
          bestPose.energy = energy;
          bestPose.score = this.energyToScore(energy);
          improved = true;
        }
      }
      
      // Early termination if no improvement
      if (!improved) {
        break;
      }
    }
    
    return bestPose;
  }

  private clusterAndRank(poses: DockingPose[]): DockingPose[] {
    // Simple clustering by RMSD
    const clusters: DockingPose[][] = [];
    const { rmsd_threshold } = this.parameters;
    
    for (const pose of poses) {
      let addedToCluster = false;
      
      for (const cluster of clusters) {
        const representative = cluster[0];
        const rmsd = this.calculateRMSD(pose, representative);
        
        if (rmsd < rmsd_threshold) {
          cluster.push(pose);
          addedToCluster = true;
          break;
        }
      }
      
      if (!addedToCluster) {
        clusters.push([pose]);
      }
    }
    
    // Take best pose from each cluster
    const finalPoses: DockingPose[] = [];
    for (const cluster of clusters) {
      cluster.sort((a, b) => a.energy - b.energy);
      const bestInCluster = cluster[0];
      bestInCluster.rmsd = 0; // Representative pose
      finalPoses.push(bestInCluster);
    }
    
    // Sort final poses by energy
    finalPoses.sort((a, b) => a.energy - b.energy);
    
    console.log(`  Clustered ${poses.length} poses into ${clusters.length} groups`);
    return finalPoses.slice(0, Math.min(finalPoses.length, 20)); // Return top 20
  }

  // Utility functions
  private createTransformationMatrix(translation: [number, number, number], rotation: [number, number, number]): number[] {
    // Create 4x4 transformation matrix from translation and euler angles
    const [tx, ty, tz] = translation;
    const [rx, ry, rz] = rotation;
    
    const cosX = Math.cos(rx), sinX = Math.sin(rx);
    const cosY = Math.cos(ry), sinY = Math.sin(ry);
    const cosZ = Math.cos(rz), sinZ = Math.sin(rz);
    
    return [
      cosY * cosZ, -cosY * sinZ, sinY, tx,
      sinX * sinY * cosZ + cosX * sinZ, -sinX * sinY * sinZ + cosX * cosZ, -sinX * cosY, ty,
      -cosX * sinY * cosZ + sinX * sinZ, cosX * sinY * sinZ + sinX * cosZ, cosX * cosY, tz,
      0, 0, 0, 1
    ];
  }

  private applyTransformation(coords: [number, number, number], matrix: number[]): [number, number, number] {
    const [x, y, z] = coords;
    return [
      matrix[0] * x + matrix[1] * y + matrix[2] * z + matrix[3],
      matrix[4] * x + matrix[5] * y + matrix[6] * z + matrix[7],
      matrix[8] * x + matrix[9] * y + matrix[10] * z + matrix[11]
    ];
  }

  private calculateDistance(pos1: [number, number, number], pos2: [number, number, number]): number {
    const dx = pos1[0] - pos2[0];
    const dy = pos1[1] - pos2[1];
    const dz = pos1[2] - pos2[2];
    return Math.sqrt(dx * dx + dy * dy + dz * dz);
  }

  private calculateRMSD(pose1: DockingPose, pose2: DockingPose): number {
    // Simplified RMSD calculation based on translation difference
    const dx = pose1.translation[0] - pose2.translation[0];
    const dy = pose1.translation[1] - pose2.translation[1];
    const dz = pose1.translation[2] - pose2.translation[2];
    return Math.sqrt(dx * dx + dy * dy + dz * dz);
  }

  private energyToScore(energy: number): number {
    // Convert energy to normalized score (0-1, higher is better)
    // This is a simplified conversion
    return Math.max(0, Math.min(1, 1 / (1 + Math.exp(energy + 5))));
  }

  private randomGaussian(): number {
    // Box-Muller transform for gaussian random numbers
    const u = Math.random();
    const v = Math.random();
    return Math.sqrt(-2 * Math.log(u)) * Math.cos(2 * Math.PI * v);
  }

  private getVdWRadius(element: string): number {
    const radii: Record<string, number> = {
      'H': 1.20, 'C': 1.70, 'N': 1.55, 'O': 1.52, 'F': 1.47,
      'P': 1.80, 'S': 1.80, 'CL': 1.75, 'BR': 1.85, 'I': 1.98
    };
    return radii[element.toUpperCase()] || 1.7;
  }

  private canFormHydrogenBond(atom1: PDBAtom, atom2: PDBAtom): boolean {
    const donors = ['N', 'O'];
    const acceptors = ['N', 'O', 'F'];
    
    return (donors.includes(atom1.element) && acceptors.includes(atom2.element)) ||
           (acceptors.includes(atom1.element) && donors.includes(atom2.element));
  }

  private isHydrophobic(element: string): boolean {
    return element === 'C' || element === 'S';
  }

  private getPartialCharge(element: string): number {
    const charges: Record<string, number> = {
      'N': -0.3, 'O': -0.4, 'F': -0.4, 'CL': -0.3, 'S': -0.2, 'P': 0.3
    };
    return charges[element.toUpperCase()] || 0;
  }
}

// Factory function for easy algorithm creation
export function createDockingAlgorithm(
  receptor: PDBStructure, 
  ligand: PDBStructure, 
  params: Partial<DockingParameters> = {}
): DockingAlgorithm {
  const defaultParams: DockingParameters = {
    max_poses: 100,
    energy_threshold: 0.0,
    rmsd_threshold: 2.0,
    sampling_strategy: 'monte_carlo',
    optimization_rounds: 10,
    parallel_workers: 4,
    use_gpu_acceleration: false,
    search_space: {
      center: [0, 0, 0],
      size: [20, 20, 20]
    },
    advanced_scoring: true
  };

  const mergedParams = { ...defaultParams, ...params };
  return new DockingAlgorithm(receptor, ligand, mergedParams);
}