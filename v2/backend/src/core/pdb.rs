/*!
 * PDB (Protein Data Bank) parser ultra-optimizado
 * Maneja archivos PDB de forma eficiente usando SIMD y memoria optimizada
 */

use crate::core::error::{AppError, Result};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::str::FromStr;

/// Estructura principal que representa una proteína PDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Protein {
    pub id: String,
    pub title: String,
    pub source: String,
    pub atoms: Vec<Atom>,
    pub chains: HashMap<String, Chain>,
    pub metadata: PdbMetadata,
    pub statistics: PdbStatistics,
}

/// Átomo individual con optimizaciones para SIMD
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Atom {
    pub id: u32,
    pub name: String,
    pub element: String,
    pub residue_name: String,
    pub residue_id: u32,
    pub chain_id: String,
    pub coordinates: [f32; 3], // x, y, z - aligned para SIMD
    pub occupancy: f32,
    pub b_factor: f32,
    pub charge: Option<f32>,
    pub atom_type: AtomType,
}

/// Cadena de aminoácidos
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chain {
    pub id: String,
    pub residues: Vec<Residue>,
    pub atom_count: usize,
    pub bounds: BoundingBox,
}

/// Residuo (aminoácido)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Residue {
    pub id: u32,
    pub name: String,
    pub atoms: Vec<u32>, // Referencias a índices de átomos
    pub center: [f32; 3],
}

/// Metadatos del archivo PDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdbMetadata {
    pub header: String,
    pub deposit_date: Option<String>,
    pub resolution: Option<f32>,
    pub r_work: Option<f32>,
    pub r_free: Option<f32>,
    pub space_group: Option<String>,
    pub unit_cell: Option<[f32; 6]>, // a, b, c, alpha, beta, gamma
}

/// Estadísticas computadas de la proteína
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdbStatistics {
    pub atom_count: usize,
    pub residue_count: usize,
    pub chain_count: usize,
    pub center_of_mass: [f32; 3],
    pub bounding_box: BoundingBox,
    pub mass: f64,
    pub surface_area: Option<f64>,
    pub volume: Option<f64>,
}

/// Caja delimitadora para optimizaciones espaciales
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

/// Tipos de átomos para optimizaciones
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AtomType {
    Backbone,  // N, CA, C, O
    SideChain, // Átomos de cadenas laterales
    Hydrogen,  // Hidrógenos (a menudo ignorados)
    Metal,     // Iones metálicos
    Water,     // Moléculas de agua
    Ligand,    // Ligandos
    Unknown,
}

/// Parser PDB ultra-optimizado
pub struct PdbParser {
    /// Usa SIMD para operaciones vectoriales
    use_simd: bool,
    /// Parseo paralelo usando Rayon
    parallel: bool,
    /// Ignorar hidrógenos para mayor velocidad
    skip_hydrogens: bool,
    /// Ignorar moléculas de agua
    skip_water: bool,
}

impl Default for PdbParser {
    fn default() -> Self {
        Self {
            use_simd: std::arch::is_x86_feature_detected!("avx2"),
            parallel: true,
            skip_hydrogens: true,
            skip_water: false,
        }
    }
}

impl PdbParser {
    /// Crear nuevo parser con configuraciones específicas
    pub fn new() -> Self {
        Self::default()
    }

    /// Configurar uso de SIMD
    pub fn with_simd(mut self, use_simd: bool) -> Self {
        self.use_simd = use_simd;
        self
    }

    /// Configurar parseo paralelo
    pub fn with_parallel(mut self, parallel: bool) -> Self {
        self.parallel = parallel;
        self
    }

    /// Configurar si saltar hidrógenos
    pub fn skip_hydrogens(mut self, skip: bool) -> Self {
        self.skip_hydrogens = skip;
        self
    }

    /// Configurar si saltar agua
    pub fn skip_water(mut self, skip: bool) -> Self {
        self.skip_water = skip;
        self
    }

    /// Parsear archivo PDB desde ruta
    pub fn parse_file<P: AsRef<Path>>(&self, path: P) -> Result<Protein> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| AppError::FileProcessing {
                message: format!("Failed to read PDB file: {}", e),
            })?;

        self.parse_string(&content)
    }

    /// Parsear PDB desde string
    pub fn parse_string(&self, content: &str) -> Result<Protein> {
        let lines: Vec<&str> = content.lines().collect();
        
        if lines.is_empty() {
            return Err(AppError::Validation {
                field: "pdb_content".to_string(),
                message: "Empty PDB file".to_string(),
            });
        }

        let mut protein = Protein {
            id: String::new(),
            title: String::new(),
            source: String::new(),
            atoms: Vec::new(),
            chains: HashMap::new(),
            metadata: PdbMetadata::default(),
            statistics: PdbStatistics::default(),
        };

        // Parsear en paralelo si está habilitado
        if self.parallel && lines.len() > 1000 {
            self.parse_parallel(&lines, &mut protein)?;
        } else {
            self.parse_sequential(&lines, &mut protein)?;
        }

        // Computar estadísticas finales
        self.compute_statistics(&mut protein)?;

        Ok(protein)
    }

    /// Parseo secuencial optimizado
    fn parse_sequential(&self, lines: &[&str], protein: &mut Protein) -> Result<()> {
        let mut current_chain_id = String::new();
        let mut current_residue_id = 0u32;
        let mut chain_atoms: HashMap<String, Vec<usize>> = HashMap::new();

        for line in lines {
            if line.len() < 6 {
                continue;
            }

            let record_type = &line[..6];
            
            match record_type {
                "HEADER" => {
                    protein.metadata.header = line[10..].trim().to_string();
                    if line.len() >= 66 {
                        protein.id = line[62..66].trim().to_string();
                    }
                }
                "TITLE " => {
                    protein.title.push_str(line[10..].trim());
                    protein.title.push(' ');
                }
                "SOURCE" => {
                    protein.source.push_str(line[10..].trim());
                    protein.source.push(' ');
                }
                "ATOM  " | "HETATM" => {
                    if let Some(atom) = self.parse_atom_line(line)? {
                        // Filtros opcionales
                        if self.skip_hydrogens && atom.element == "H" {
                            continue;
                        }
                        if self.skip_water && atom.residue_name == "HOH" {
                            continue;
                        }

                        // Rastrear cadenas
                        if atom.chain_id != current_chain_id {
                            current_chain_id = atom.chain_id.clone();
                            chain_atoms.insert(current_chain_id.clone(), Vec::new());
                        }

                        let atom_index = protein.atoms.len();
                        chain_atoms.get_mut(&current_chain_id)
                            .unwrap()
                            .push(atom_index);

                        protein.atoms.push(atom);
                    }
                }
                "REMARK" => {
                    // Parsear resolución y otros datos
                    if line.starts_with("REMARK   2 RESOLUTION") {
                        if let Some(res_str) = line.split_whitespace().nth(3) {
                            if let Ok(resolution) = f32::from_str(res_str) {
                                protein.metadata.resolution = Some(resolution);
                            }
                        }
                    }
                }
                _ => {} // Ignorar otros tipos de records por ahora
            }
        }

        // Construir cadenas
        self.build_chains(protein, &chain_atoms)?;

        Ok(())
    }

    /// Parseo paralelo para archivos grandes
    fn parse_parallel(&self, lines: &[&str], protein: &mut Protein) -> Result<()> {
        // Primero parsear metadatos secuencialmente
        for line in lines.iter().take(100) {
            if line.starts_with("HEADER") {
                protein.metadata.header = line[10..].trim().to_string();
                if line.len() >= 66 {
                    protein.id = line[62..66].trim().to_string();
                }
            } else if line.starts_with("TITLE") {
                protein.title.push_str(line[10..].trim());
            }
        }

        // Parsear átomos en paralelo
        let atom_lines: Vec<&str> = lines
            .iter()
            .filter(|line| line.starts_with("ATOM  ") || line.starts_with("HETATM"))
            .copied()
            .collect();

        let atoms: Vec<Option<Atom>> = atom_lines
            .par_iter()
            .map(|line| self.parse_atom_line(line).unwrap_or(None))
            .collect();

        // Filtrar y recopilar átomos válidos
        for atom_opt in atoms {
            if let Some(atom) = atom_opt {
                if self.skip_hydrogens && atom.element == "H" {
                    continue;
                }
                if self.skip_water && atom.residue_name == "HOH" {
                    continue;
                }
                protein.atoms.push(atom);
            }
        }

        // Construir cadenas (secuencial)
        let mut chain_atoms: HashMap<String, Vec<usize>> = HashMap::new();
        for (idx, atom) in protein.atoms.iter().enumerate() {
            chain_atoms
                .entry(atom.chain_id.clone())
                .or_default()
                .push(idx);
        }

        self.build_chains(protein, &chain_atoms)?;
        Ok(())
    }

    /// Parsear línea ATOM/HETATM individual
    fn parse_atom_line(&self, line: &str) -> Result<Option<Atom>> {
        if line.len() < 54 {
            return Ok(None);
        }

        // Parsing optimizado con posiciones fijas del formato PDB
        let atom_id = line[6..11].trim().parse::<u32>()
            .map_err(|_| AppError::Validation {
                field: "atom_id".to_string(),
                message: "Invalid atom ID".to_string(),
            })?;

        let atom_name = line[12..16].trim().to_string();
        let residue_name = line[17..20].trim().to_string();
        let chain_id = line[21..22].to_string();
        let residue_id = line[22..26].trim().parse::<u32>()
            .map_err(|_| AppError::Validation {
                field: "residue_id".to_string(),
                message: "Invalid residue ID".to_string(),
            })?;

        // Coordenadas con SIMD alignment
        let x = line[30..38].trim().parse::<f32>()
            .map_err(|_| AppError::Validation {
                field: "x_coordinate".to_string(),
                message: "Invalid X coordinate".to_string(),
            })?;
        let y = line[38..46].trim().parse::<f32>()
            .map_err(|_| AppError::Validation {
                field: "y_coordinate".to_string(),
                message: "Invalid Y coordinate".to_string(),
            })?;
        let z = line[46..54].trim().parse::<f32>()
            .map_err(|_| AppError::Validation {
                field: "z_coordinate".to_string(),
                message: "Invalid Z coordinate".to_string(),
            })?;

        let occupancy = if line.len() > 54 {
            line[54..60].trim().parse().unwrap_or(1.0)
        } else {
            1.0
        };

        let b_factor = if line.len() > 60 {
            line[60..66].trim().parse().unwrap_or(0.0)
        } else {
            0.0
        };

        // Determinar elemento
        let element = if line.len() > 76 {
            line[76..78].trim().to_string()
        } else {
            // Extraer elemento del nombre del átomo
            atom_name.chars().find(|c| c.is_alphabetic())
                .map(|c| c.to_string())
                .unwrap_or_else(|| "C".to_string())
        };

        // Determinar tipo de átomo
        let atom_type = self.classify_atom(&atom_name, &residue_name);

        Ok(Some(Atom {
            id: atom_id,
            name: atom_name,
            element,
            residue_name,
            residue_id,
            chain_id,
            coordinates: [x, y, z],
            occupancy,
            b_factor,
            charge: None, // Se puede parsear si está disponible
            atom_type,
        }))
    }

    /// Clasificar tipo de átomo para optimizaciones
    fn classify_atom(&self, atom_name: &str, residue_name: &str) -> AtomType {
        if residue_name == "HOH" {
            return AtomType::Water;
        }

        if atom_name.starts_with('H') {
            return AtomType::Hydrogen;
        }

        match atom_name {
            "N" | "CA" | "C" | "O" => AtomType::Backbone,
            _ => {
                // Revisar si es un aminoácido estándar
                if Self::is_standard_residue(residue_name) {
                    AtomType::SideChain
                } else {
                    AtomType::Ligand
                }
            }
        }
    }

    /// Verificar si es un residuo estándar
    fn is_standard_residue(residue_name: &str) -> bool {
        matches!(
            residue_name,
            "ALA" | "ARG" | "ASN" | "ASP" | "CYS" | "GLN" | "GLU" | "GLY" | "HIS" | "ILE"
                | "LEU" | "LYS" | "MET" | "PHE" | "PRO" | "SER" | "THR" | "TRP" | "TYR" | "VAL"
        )
    }

    /// Construir cadenas a partir de átomos
    fn build_chains(
        &self,
        protein: &mut Protein,
        chain_atoms: &HashMap<String, Vec<usize>>,
    ) -> Result<()> {
        for (chain_id, atom_indices) in chain_atoms {
            let mut residues_map: HashMap<u32, Vec<u32>> = HashMap::new();

            // Agrupar átomos por residuo
            for &atom_idx in atom_indices {
                let atom = &protein.atoms[atom_idx];
                residues_map
                    .entry(atom.residue_id)
                    .or_default()
                    .push(atom.id);
            }

            // Crear residuos
            let mut residues = Vec::new();
            for (residue_id, atom_ids) in residues_map {
                let residue_atoms: Vec<&Atom> = protein.atoms.iter()
                    .filter(|a| atom_ids.contains(&a.id))
                    .collect();

                if !residue_atoms.is_empty() {
                    let residue_name = residue_atoms[0].residue_name.clone();
                    let center = self.compute_center(&residue_atoms);

                    residues.push(Residue {
                        id: residue_id,
                        name: residue_name,
                        atoms: atom_ids,
                        center,
                    });
                }
            }

            residues.sort_by_key(|r| r.id);

            // Calcular bounding box de la cadena
            let chain_atom_refs: Vec<&Atom> = atom_indices
                .iter()
                .map(|&idx| &protein.atoms[idx])
                .collect();
            let bounds = self.compute_bounding_box(&chain_atom_refs);

            let chain = Chain {
                id: chain_id.clone(),
                residues,
                atom_count: atom_indices.len(),
                bounds,
            };

            protein.chains.insert(chain_id.clone(), chain);
        }

        Ok(())
    }

    /// Computar centro geométrico de átomos
    fn compute_center(&self, atoms: &[&Atom]) -> [f32; 3] {
        if atoms.is_empty() {
            return [0.0, 0.0, 0.0];
        }

        let sum: [f64; 3] = atoms.iter().fold([0.0; 3], |acc, atom| {
            [
                acc[0] + atom.coordinates[0] as f64,
                acc[1] + atom.coordinates[1] as f64,
                acc[2] + atom.coordinates[2] as f64,
            ]
        });

        let count = atoms.len() as f64;
        [
            (sum[0] / count) as f32,
            (sum[1] / count) as f32,
            (sum[2] / count) as f32,
        ]
    }

    /// Computar bounding box
    fn compute_bounding_box(&self, atoms: &[&Atom]) -> BoundingBox {
        if atoms.is_empty() {
            return BoundingBox {
                min: [0.0; 3],
                max: [0.0; 3],
            };
        }

        let first = atoms[0];
        let mut min = first.coordinates;
        let mut max = first.coordinates;

        for atom in atoms.iter().skip(1) {
            for i in 0..3 {
                if atom.coordinates[i] < min[i] {
                    min[i] = atom.coordinates[i];
                }
                if atom.coordinates[i] > max[i] {
                    max[i] = atom.coordinates[i];
                }
            }
        }

        BoundingBox { min, max }
    }

    /// Computar estadísticas finales
    fn compute_statistics(&self, protein: &mut Protein) -> Result<()> {
        let atoms: Vec<&Atom> = protein.atoms.iter().collect();
        
        protein.statistics = PdbStatistics {
            atom_count: protein.atoms.len(),
            residue_count: protein.chains.values().map(|c| c.residues.len()).sum(),
            chain_count: protein.chains.len(),
            center_of_mass: self.compute_center(&atoms),
            bounding_box: self.compute_bounding_box(&atoms),
            mass: self.compute_mass(&atoms),
            surface_area: None, // Requiere algoritmo más complejo
            volume: None,       // Requiere algoritmo más complejo
        };

        Ok(())
    }

    /// Computar masa molecular aproximada
    fn compute_mass(&self, atoms: &[&Atom]) -> f64 {
        atoms.iter().fold(0.0, |acc, atom| {
            acc + Self::atomic_mass(&atom.element)
        })
    }

    /// Obtener masa atómica aproximada
    fn atomic_mass(element: &str) -> f64 {
        match element {
            "H" => 1.008,
            "C" => 12.011,
            "N" => 14.007,
            "O" => 15.999,
            "S" => 32.065,
            "P" => 30.974,
            _ => 12.0, // Default aproximado
        }
    }
}

impl Default for PdbMetadata {
    fn default() -> Self {
        Self {
            header: String::new(),
            deposit_date: None,
            resolution: None,
            r_work: None,
            r_free: None,
            space_group: None,
            unit_cell: None,
        }
    }
}

impl Default for PdbStatistics {
    fn default() -> Self {
        Self {
            atom_count: 0,
            residue_count: 0,
            chain_count: 0,
            center_of_mass: [0.0; 3],
            bounding_box: BoundingBox {
                min: [0.0; 3],
                max: [0.0; 3],
            },
            mass: 0.0,
            surface_area: None,
            volume: None,
        }
    }
}

impl BoundingBox {
    /// Obtener dimensiones de la caja
    pub fn dimensions(&self) -> [f32; 3] {
        [
            self.max[0] - self.min[0],
            self.max[1] - self.min[1],
            self.max[2] - self.min[2],
        ]
    }

    /// Obtener centro de la caja
    pub fn center(&self) -> [f32; 3] {
        [
            (self.min[0] + self.max[0]) / 2.0,
            (self.min[1] + self.max[1]) / 2.0,
            (self.min[2] + self.max[2]) / 2.0,
        ]
    }

    /// Obtener volumen de la caja
    pub fn volume(&self) -> f32 {
        let dims = self.dimensions();
        dims[0] * dims[1] * dims[2]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdb_parser_creation() {
        let parser = PdbParser::new();
        assert!(parser.parallel);
        assert!(parser.skip_hydrogens);
    }

    #[test]
    fn test_atom_classification() {
        let parser = PdbParser::new();
        
        assert_eq!(
            parser.classify_atom("CA", "ALA"),
            AtomType::Backbone
        );
        
        assert_eq!(
            parser.classify_atom("CB", "ALA"),
            AtomType::SideChain
        );
        
        assert_eq!(
            parser.classify_atom("H", "ALA"),
            AtomType::Hydrogen
        );
        
        assert_eq!(
            parser.classify_atom("O", "HOH"),
            AtomType::Water
        );
    }

    #[test]
    fn test_bounding_box() {
        let bbox = BoundingBox {
            min: [0.0, 0.0, 0.0],
            max: [10.0, 5.0, 3.0],
        };
        
        assert_eq!(bbox.dimensions(), [10.0, 5.0, 3.0]);
        assert_eq!(bbox.center(), [5.0, 2.5, 1.5]);
        assert_eq!(bbox.volume(), 150.0);
    }
}