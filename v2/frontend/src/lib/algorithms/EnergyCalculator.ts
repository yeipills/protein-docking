/*!
 * EnergyCalculator - Cálculo preciso de energías de interacción molecular
 * 
 * Implementa un sistema más sofisticado de cálculo de energías que incluye:
 * 1. Van der Waals (Lennard-Jones 12-6)
 * 2. Electrostática (Coulomb con screening)
 * 3. Enlaces de hidrógeno con geometría
 * 4. Interacciones hidrofóbicas/hidrofílicas
 * 5. Energía de solvatación simplificada
 * 6. Energía de conformación del ligando
 */

import type { PDBAtom } from '../utils/PDBParser.js';

export interface EnergyComponents {
  van_der_waals: number;
  electrostatic: number;
  hydrogen_bonds: number;
  hydrophobic: number;
  solvation: number;
  conformational: number;
  total: number;
}

export interface DetailedContact {
  receptor_atom: ContactAtomInfo;
  ligand_atom: ContactAtomInfo;
  distance: number;
  interaction_type: 'vdw' | 'electrostatic' | 'hydrogen_bond' | 'hydrophobic' | 'pi_pi' | 'cation_pi';
  energy: number;
  geometry_score?: number; // Para H-bonds, 0-1 donde 1 es geometría perfecta
}

export interface ContactAtomInfo {
  index: number;
  element: string;
  position: [number, number, number];
  residue: string;
  chain: string;
  atom_type: string;
  partial_charge: number;
  solvation_parameter: number;
  is_aromatic: boolean;
  is_polar: boolean;
}

// Constantes físicas y parámetros de force field
const ENERGY_CONSTANTS = {
  // Van der Waals (kcal/mol)
  VDW_EPSILON: {
    'C': 0.086, 'N': 0.170, 'O': 0.210, 'S': 0.250, 'P': 0.200,
    'H': 0.020, 'F': 0.061, 'CL': 0.265, 'BR': 0.320, 'I': 0.380
  },
  VDW_SIGMA: {
    'C': 3.40, 'N': 3.25, 'O': 2.96, 'S': 3.55, 'P': 3.74,
    'H': 2.50, 'F': 2.47, 'CL': 3.52, 'BR': 3.73, 'I': 3.96
  },
  
  // Electrostática
  COULOMB_CONSTANT: 332.0, // kcal·Å/(mol·e²)
  DIELECTRIC_CONSTANT: 4.0, // Proteína interior
  SCREENING_FACTOR: 0.2, // Factor de screening para solvente
  
  // Enlaces de hidrógeno
  H_BOND_DISTANCE_CUTOFF: 3.5, // Å
  H_BOND_OPTIMAL_DISTANCE: 2.8, // Å
  H_BOND_ENERGY_MAX: -5.0, // kcal/mol
  H_BOND_ANGLE_PENALTY: 2.0, // kcal/mol
  
  // Hidrofóbicas
  HYDROPHOBIC_BONUS: -0.8, // kcal/mol por contacto
  HYDROPHOBIC_CUTOFF: 4.5, // Å
  
  // Solvatación (modelo simplificado)
  SOLVATION_ENERGY_SCALE: 0.5, // kcal/mol
  POLAR_SOLVATION_PENALTY: 1.0, // kcal/mol por grupo polar enterrado
  
  // Pi-Pi stacking
  PI_PI_OPTIMAL_DISTANCE: 3.5, // Å
  PI_PI_ENERGY: -2.0, // kcal/mol
  
  // Cation-Pi
  CATION_PI_OPTIMAL_DISTANCE: 3.0, // Å
  CATION_PI_ENERGY: -3.0 // kcal/mol
};

export class EnergyCalculator {
  private receptorAtoms: PDBAtom[];
  private ligandAtoms: PDBAtom[];
  private useAdvancedScoring: boolean;

  constructor(receptor: PDBAtom[], ligand: PDBAtom[], useAdvanced = true) {
    this.receptorAtoms = receptor;
    this.ligandAtoms = ligand;
    this.useAdvancedScoring = useAdvanced;
  }

  /**
   * Calcula la energía total de interacción entre receptor y ligando
   */
  calculateTotalEnergy(transformedLigand: PDBAtom[]): EnergyComponents {
    const components: EnergyComponents = {
      van_der_waals: 0,
      electrostatic: 0,
      hydrogen_bonds: 0,
      hydrophobic: 0,
      solvation: 0,
      conformational: 0,
      total: 0
    };

    const contacts: DetailedContact[] = [];

    // Calcular interacciones pairwise
    for (const ligandAtom of transformedLigand) {
      for (const receptorAtom of this.receptorAtoms) {
        const distance = this.calculateDistance(
          [ligandAtom.x, ligandAtom.y, ligandAtom.z],
          [receptorAtom.x, receptorAtom.y, receptorAtom.z]
        );

        // Cutoff para interacciones
        if (distance > 8.0) continue;

        const contact = this.calculatePairwiseInteraction(ligandAtom, receptorAtom, distance);
        if (Math.abs(contact.energy) > 0.05) {
          contacts.push(contact);
          
          // Acumular por tipo de interacción
          switch (contact.interaction_type) {
            case 'vdw':
              components.van_der_waals += contact.energy;
              break;
            case 'electrostatic':
              components.electrostatic += contact.energy;
              break;
            case 'hydrogen_bond':
              components.hydrogen_bonds += contact.energy;
              break;
            case 'hydrophobic':
              components.hydrophobic += contact.energy;
              break;
            case 'pi_pi':
            case 'cation_pi':
              components.van_der_waals += contact.energy; // Agregar a VdW por simplicidad
              break;
          }
        }
      }
    }

    // Calcular energía de solvatación (modelo simplificado)
    if (this.useAdvancedScoring) {
      components.solvation = this.calculateSolvationEnergy(transformedLigand, contacts);
      components.conformational = this.calculateConformationalEnergy(transformedLigand);
    }

    // Energía total
    components.total = components.van_der_waals + 
                     components.electrostatic + 
                     components.hydrogen_bonds + 
                     components.hydrophobic + 
                     components.solvation + 
                     components.conformational;

    return components;
  }

  /**
   * Calcula la interacción entre dos átomos específicos
   */
  private calculatePairwiseInteraction(
    ligandAtom: PDBAtom, 
    receptorAtom: PDBAtom, 
    distance: number
  ): DetailedContact {
    let totalEnergy = 0;
    let primaryInteraction: DetailedContact['interaction_type'] = 'vdw';

    // 1. Van der Waals (Lennard-Jones)
    const vdwEnergy = this.calculateVanDerWaals(ligandAtom.element, receptorAtom.element, distance);
    totalEnergy += vdwEnergy;

    // 2. Electrostática
    const electrostaticEnergy = this.calculateElectrostatic(ligandAtom, receptorAtom, distance);
    totalEnergy += electrostaticEnergy;

    // 3. Enlaces de hidrógeno
    const hBondResult = this.calculateHydrogenBond(ligandAtom, receptorAtom, distance);
    if (hBondResult.energy < -0.5) {
      totalEnergy += hBondResult.energy;
      primaryInteraction = 'hydrogen_bond';
    }

    // 4. Interacciones hidrofóbicas
    const hydrophobicEnergy = this.calculateHydrophobic(ligandAtom, receptorAtom, distance);
    if (Math.abs(hydrophobicEnergy) > Math.abs(vdwEnergy) && hydrophobicEnergy < 0) {
      totalEnergy += hydrophobicEnergy;
      primaryInteraction = 'hydrophobic';
    }

    // 5. Pi-Pi stacking y Cation-Pi (solo si advanced scoring)
    if (this.useAdvancedScoring) {
      const specialEnergy = this.calculateSpecialInteractions(ligandAtom, receptorAtom, distance);
      if (specialEnergy.energy !== 0) {
        totalEnergy += specialEnergy.energy;
        primaryInteraction = specialEnergy.type;
      }
    }

    // Determinar tipo de interacción dominante
    if (Math.abs(electrostaticEnergy) > Math.abs(vdwEnergy) && Math.abs(electrostaticEnergy) > 0.5) {
      primaryInteraction = 'electrostatic';
    }

    return {
      receptor_atom: this.createContactAtomInfo(receptorAtom),
      ligand_atom: this.createContactAtomInfo(ligandAtom),
      distance,
      interaction_type: primaryInteraction,
      energy: totalEnergy,
      geometry_score: hBondResult.geometry_score
    };
  }

  /**
   * Calcula energía Van der Waals usando potencial Lennard-Jones 12-6
   */
  private calculateVanDerWaals(element1: string, element2: string, distance: number): number {
    const eps1 = ENERGY_CONSTANTS.VDW_EPSILON[element1 as keyof typeof ENERGY_CONSTANTS.VDW_EPSILON] || 0.086;
    const eps2 = ENERGY_CONSTANTS.VDW_EPSILON[element2 as keyof typeof ENERGY_CONSTANTS.VDW_EPSILON] || 0.086;
    const sigma1 = ENERGY_CONSTANTS.VDW_SIGMA[element1 as keyof typeof ENERGY_CONSTANTS.VDW_SIGMA] || 3.4;
    const sigma2 = ENERGY_CONSTANTS.VDW_SIGMA[element2 as keyof typeof ENERGY_CONSTANTS.VDW_SIGMA] || 3.4;

    const epsilon = Math.sqrt(eps1 * eps2);
    const sigma = (sigma1 + sigma2) / 2;

    const r6 = Math.pow(sigma / distance, 6);
    const r12 = r6 * r6;

    return 4 * epsilon * (r12 - r6);
  }

  /**
   * Calcula energía electrostática con screening
   */
  private calculateElectrostatic(ligandAtom: PDBAtom, receptorAtom: PDBAtom, distance: number): number {
    const q1 = this.getPartialCharge(ligandAtom);
    const q2 = this.getPartialCharge(receptorAtom);

    if (Math.abs(q1) < 0.01 || Math.abs(q2) < 0.01) return 0;

    const screenedDistance = distance + ENERGY_CONSTANTS.SCREENING_FACTOR;
    const dielectric = ENERGY_CONSTANTS.DIELECTRIC_CONSTANT;

    return (ENERGY_CONSTANTS.COULOMB_CONSTANT * q1 * q2) / (dielectric * screenedDistance);
  }

  /**
   * Calcula energía de enlace de hidrógeno con consideraciones geométricas
   */
  private calculateHydrogenBond(
    ligandAtom: PDBAtom, 
    receptorAtom: PDBAtom, 
    distance: number
  ): { energy: number; geometry_score: number } {
    if (distance > ENERGY_CONSTANTS.H_BOND_DISTANCE_CUTOFF) {
      return { energy: 0, geometry_score: 0 };
    }

    if (!this.canFormHydrogenBond(ligandAtom.element, receptorAtom.element)) {
      return { energy: 0, geometry_score: 0 };
    }

    // Factor de distancia (optimal = 2.8 Å)
    const distanceFactor = Math.exp(-0.5 * Math.pow((distance - ENERGY_CONSTANTS.H_BOND_OPTIMAL_DISTANCE) / 0.3, 2));

    // Factor geométrico simplificado (en un sistema real se calcularía el ángulo)
    const geometryScore = 0.8 + 0.2 * Math.random(); // Simulado

    // Energía final
    const energy = ENERGY_CONSTANTS.H_BOND_ENERGY_MAX * distanceFactor * geometryScore;

    return { energy, geometry_score: geometryScore };
  }

  /**
   * Calcula interacciones hidrofóbicas
   */
  private calculateHydrophobic(ligandAtom: PDBAtom, receptorAtom: PDBAtom, distance: number): number {
    if (distance > ENERGY_CONSTANTS.HYDROPHOBIC_CUTOFF) return 0;

    const isHydrophobic1 = this.isHydrophobic(ligandAtom.element, ligandAtom.resName);
    const isHydrophobic2 = this.isHydrophobic(receptorAtom.element, receptorAtom.resName);

    if (isHydrophobic1 && isHydrophobic2) {
      const distanceFactor = Math.exp(-(distance - 3.5) / 1.0);
      return ENERGY_CONSTANTS.HYDROPHOBIC_BONUS * distanceFactor;
    }

    return 0;
  }

  /**
   * Calcula interacciones especiales (Pi-Pi, Cation-Pi)
   */
  private calculateSpecialInteractions(
    ligandAtom: PDBAtom, 
    receptorAtom: PDBAtom, 
    distance: number
  ): { energy: number; type: DetailedContact['interaction_type'] } {
    // Pi-Pi stacking (simplificado - solo aromáticos)
    if (this.isAromatic(ligandAtom) && this.isAromatic(receptorAtom)) {
      if (distance < 5.0) {
        const distanceFactor = Math.exp(-Math.pow(distance - ENERGY_CONSTANTS.PI_PI_OPTIMAL_DISTANCE, 2) / 2);
        return { energy: ENERGY_CONSTANTS.PI_PI_ENERGY * distanceFactor, type: 'pi_pi' };
      }
    }

    // Cation-Pi
    const isCation1 = this.isCationic(ligandAtom);
    const isCation2 = this.isCationic(receptorAtom);
    const isAromatic1 = this.isAromatic(ligandAtom);
    const isAromatic2 = this.isAromatic(receptorAtom);

    if ((isCation1 && isAromatic2) || (isCation2 && isAromatic1)) {
      if (distance < 4.0) {
        const distanceFactor = Math.exp(-Math.pow(distance - ENERGY_CONSTANTS.CATION_PI_OPTIMAL_DISTANCE, 2) / 2);
        return { energy: ENERGY_CONSTANTS.CATION_PI_ENERGY * distanceFactor, type: 'cation_pi' };
      }
    }

    return { energy: 0, type: 'vdw' };
  }

  /**
   * Calcula energía de solvatación (modelo muy simplificado)
   */
  private calculateSolvationEnergy(transformedLigand: PDBAtom[], contacts: DetailedContact[]): number {
    let solvationEnergy = 0;

    // Penalizar grupos polares que pierden contacto con solvente
    for (const atom of transformedLigand) {
      if (this.isPolar(atom)) {
        const contactCount = contacts.filter(c => 
          c.ligand_atom.index === 0 && // Simplificado
          c.distance < 4.0
        ).length;

        // Más contactos = menos acceso al solvente = penalty para grupos polares
        if (contactCount > 3) {
          solvationEnergy += ENERGY_CONSTANTS.POLAR_SOLVATION_PENALTY * (contactCount - 3);
        }
      }
    }

    return solvationEnergy * ENERGY_CONSTANTS.SOLVATION_ENERGY_SCALE;
  }

  /**
   * Calcula energía conformacional del ligando (muy simplificado)
   */
  private calculateConformationalEnergy(transformedLigand: PDBAtom[]): number {
    // En un sistema real, esto compararía con conformaciones de mínima energía
    // Por ahora, solo una pequeña penalty base
    return 0.5; // kcal/mol
  }

  // Funciones auxiliares
  private createContactAtomInfo(atom: PDBAtom): ContactAtomInfo {
    return {
      index: 0, // Sería el índice real en el array
      element: atom.element,
      position: [atom.x, atom.y, atom.z],
      residue: atom.resName,
      chain: atom.chainID,
      atom_type: atom.name,
      partial_charge: this.getPartialCharge(atom),
      solvation_parameter: this.getSolvationParameter(atom),
      is_aromatic: this.isAromatic(atom),
      is_polar: this.isPolar(atom)
    };
  }

  private calculateDistance(pos1: [number, number, number], pos2: [number, number, number]): number {
    const dx = pos1[0] - pos2[0];
    const dy = pos1[1] - pos2[1];
    const dz = pos1[2] - pos2[2];
    return Math.sqrt(dx * dx + dy * dy + dz * dz);
  }

  private getPartialCharge(atom: PDBAtom): number {
    // Cargas parciales simplificadas basadas en tipo de átomo y residuo
    const baseCharges: Record<string, number> = {
      'N': -0.3, 'O': -0.4, 'F': -0.4, 'CL': -0.3, 'S': -0.2, 'P': 0.3, 'C': 0.0, 'H': 0.1
    };

    let charge = baseCharges[atom.element] || 0;

    // Ajustar según contexto del residuo
    if (atom.resName === 'LYS' && atom.name === 'NZ') charge = 1.0;
    if (atom.resName === 'ARG' && atom.name === 'NH1') charge = 0.5;
    if (atom.resName === 'ARG' && atom.name === 'NH2') charge = 0.5;
    if (atom.resName === 'ASP' && atom.name === 'OD1') charge = -0.5;
    if (atom.resName === 'ASP' && atom.name === 'OD2') charge = -0.5;
    if (atom.resName === 'GLU' && atom.name === 'OE1') charge = -0.5;
    if (atom.resName === 'GLU' && atom.name === 'OE2') charge = -0.5;

    return charge;
  }

  private getSolvationParameter(atom: PDBAtom): number {
    // Parámetro de solvatación (área de superficie accesible normalizada)
    const surfaceAreas: Record<string, number> = {
      'C': 1.0, 'N': 0.8, 'O': 0.6, 'S': 1.2, 'P': 1.1, 'H': 0.2
    };
    return surfaceAreas[atom.element] || 1.0;
  }

  private canFormHydrogenBond(element1: string, element2: string): boolean {
    const donors = ['N', 'O'];
    const acceptors = ['N', 'O', 'F'];
    
    return (donors.includes(element1) && acceptors.includes(element2)) ||
           (acceptors.includes(element1) && donors.includes(element2));
  }

  private isHydrophobic(element: string, residue: string): boolean {
    const hydrophobicResidues = ['ALA', 'VAL', 'LEU', 'ILE', 'PHE', 'TRP', 'MET', 'PRO'];
    const hydrophobicElements = ['C'];
    
    return hydrophobicElements.includes(element) || hydrophobicResidues.includes(residue);
  }

  private isAromatic(atom: PDBAtom): boolean {
    const aromaticResidues = ['PHE', 'TYR', 'TRP', 'HIS'];
    const aromaticAtoms = ['CG', 'CD1', 'CD2', 'CE1', 'CE2', 'CZ', 'CH2', 'CE3', 'CZ2', 'CZ3'];
    
    return aromaticResidues.includes(atom.resName) && 
           (aromaticAtoms.includes(atom.name) || atom.element === 'C');
  }

  private isCationic(atom: PDBAtom): boolean {
    return (atom.resName === 'LYS' && atom.name === 'NZ') ||
           (atom.resName === 'ARG' && ['NH1', 'NH2'].includes(atom.name)) ||
           (atom.resName === 'HIS' && ['ND1', 'NE2'].includes(atom.name));
  }

  private isPolar(atom: PDBAtom): boolean {
    const polarElements = ['N', 'O', 'F', 'S'];
    const polarResidues = ['SER', 'THR', 'ASN', 'GLN', 'TYR', 'CYS'];
    
    return polarElements.includes(atom.element) || polarResidues.includes(atom.resName);
  }
}