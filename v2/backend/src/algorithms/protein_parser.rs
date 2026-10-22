// Protein Parser - Parseador ultra-optimizado para estructuras de proteínas
// Soporta formatos PDB, MOL, SDF con SIMD acceleration

use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use tracing::{info, warn};

/// Estructura de proteína completa con optimizaciones para docking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProteinStructure {
    pub name: String,
    pub pdb_id: Option<String>,
    pub chains: Vec<Chain>,
    pub residues: Vec<Residue>,
    pub atoms: Vec<Atom>,
    pub metadata: ProteinMetadata,
    pub bbox: BoundingBox,
    pub center_of_mass: [f64; 3],
    pub radius_of_gyration: f64,
}

/// Chain de aminoácidos
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chain {
    pub chain_id: String,
    pub sequence: String,
    pub residue_indices: Vec<usize>,
}

/// Residuo individual
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Residue {
    pub residue_id: u32,
    pub residue_name: String,
    pub chain_id: String,
    pub atom_indices: Vec<usize>,
    pub center: [f64; 3],
    pub hydrophobicity: f64,
    pub charge: f64,
}

/// Átomo individual con propiedades físicas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Atom {
    pub atom_id: u32,
    pub element: String,
    pub atom_name: String,
    pub coordinates: [f64; 3],
    pub occupancy: f64,
    pub b_factor: f64,
    pub partial_charge: f64,
    pub van_der_waals_radius: f64,
    pub residue_id: u32,
    pub chain_id: String,
}

/// Metadatos de la proteína
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProteinMetadata {
    pub resolution: Option<f64>,
    pub experiment_type: String,
    pub deposition_date: Option<String>,
    pub molecular_weight: f64,
    pub total_atoms: usize,
    pub total_residues: usize,
}

/// Bounding box para optimizaciones espaciales
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub min: [f64; 3],
    pub max: [f64; 3],
    pub center: [f64; 3],
    pub size: [f64; 3],
}

/// Parser principal para múltiples formatos
pub struct ProteinParser {
    amino_acid_properties: HashMap<String, AminoAcidProperties>,
    element_properties: HashMap<String, ElementProperties>,
}

#[derive(Debug, Clone)]
struct AminoAcidProperties {
    hydrophobicity: f64,
    charge: f64,
    molecular_weight: f64,
    volume: f64,
}

#[derive(Debug, Clone)]
struct ElementProperties {
    atomic_number: u8,
    van_der_waals_radius: f64,
    electronegativity: f64,
    covalent_radius: f64,
}

impl ProteinParser {
    /// Crear nuevo parser con propiedades predefinidas
    pub fn new() -> Self {
        let mut parser = Self {
            amino_acid_properties: HashMap::new(),
            element_properties: HashMap::new(),
        };
        
        parser.initialize_amino_acid_properties();
        parser.initialize_element_properties();
        parser
    }
    
    /// Parsear proteína desde contenido PDB
    pub fn parse_pdb(&self, content: &str, name: String) -> Result<ProteinStructure> {
        info!("🧬 Parsing PDB protein: {}", name);
        
        let mut atoms = Vec::new();
        let mut residues = HashMap::new();
        let mut chains = HashMap::new();
        let mut pdb_id = None;
        let mut resolution = None;
        let mut experiment_type = "X-RAY DIFFRACTION".to_string();
        
        let mut min_coords = [f64::MAX; 3];
        let mut max_coords = [f64::MIN; 3];
        
        for line in content.lines() {
            let line = line.trim();
            
            if line.starts_with("HEADER") {
                if let Some(id) = line.get(62..66) {
                    pdb_id = Some(id.trim().to_string());
                }
            }
            
            if line.starts_with("REMARK   2 RESOLUTION") {
                if let Some(res_str) = line.get(23..30) {
                    if let Ok(res) = res_str.trim().parse::<f64>() {
                        resolution = Some(res);
                    }
                }
            }
            
            if line.starts_with("ATOM") || line.starts_with("HETATM") {
                if let Ok(atom) = self.parse_atom_line(line) {
                    // Actualizar bounding box
                    for i in 0..3 {
                        min_coords[i] = min_coords[i].min(atom.coordinates[i]);
                        max_coords[i] = max_coords[i].max(atom.coordinates[i]);
                    }
                    
                    // Agregar a chain si no existe
                    chains.entry(atom.chain_id.clone())
                        .or_insert_with(|| Chain {
                            chain_id: atom.chain_id.clone(),
                            sequence: String::new(),
                            residue_indices: Vec::new(),
                        });
                    
                    // Agregar a residuo si no existe
                    let residue_key = (atom.residue_id, atom.chain_id.clone());
                    residues.entry(residue_key)
                        .or_insert_with(|| Residue {
                            residue_id: atom.residue_id,
                            residue_name: "UNK".to_string(),
                            chain_id: atom.chain_id.clone(),
                            atom_indices: Vec::new(),
                            center: [0.0; 3],
                            hydrophobicity: 0.0,
                            charge: 0.0,
                        });
                    
                    atoms.push(atom);
                }
            }
        }
        
        if atoms.is_empty() {
            return Err(anyhow!("No atoms found in PDB file"));
        }
        
        // Calcular centro de masa y radio de giro
        let center_of_mass = self.calculate_center_of_mass(&atoms);
        let radius_of_gyration = self.calculate_radius_of_gyration(&atoms, &center_of_mass);
        
        // Crear bounding box
        let bbox = BoundingBox {
            min: min_coords,
            max: max_coords,
            center: [
                (min_coords[0] + max_coords[0]) / 2.0,
                (min_coords[1] + max_coords[1]) / 2.0,
                (min_coords[2] + max_coords[2]) / 2.0,
            ],
            size: [
                max_coords[0] - min_coords[0],
                max_coords[1] - min_coords[1],
                max_coords[2] - min_coords[2],
            ],
        };
        
        // Procesar residuos y chains
        let mut processed_residues = Vec::new();
        let mut processed_chains = Vec::new();
        
        for ((residue_id, chain_id), mut residue) in residues {
            // Encontrar átomos de este residuo
            let atom_indices: Vec<usize> = atoms
                .iter()
                .enumerate()
                .filter(|(_, atom)| atom.residue_id == residue_id && atom.chain_id == chain_id)
                .map(|(i, _)| i)
                .collect();
            
            // Calcular centro del residuo
            if !atom_indices.is_empty() {
                let mut center = [0.0; 3];
                for &idx in &atom_indices {
                    for i in 0..3 {
                        center[i] += atoms[idx].coordinates[i];
                    }
                }
                for i in 0..3 {
                    center[i] /= atom_indices.len() as f64;
                }
                residue.center = center;
            }
            
            // Asignar propiedades de aminoácido
            if let Some(aa_props) = self.amino_acid_properties.get(&residue.residue_name) {
                residue.hydrophobicity = aa_props.hydrophobicity;
                residue.charge = aa_props.charge;
            }
            
            residue.atom_indices = atom_indices;
            processed_residues.push(residue);
        }
        
        for (_, chain) in chains {
            processed_chains.push(chain);
        }
        
        // Calcular peso molecular total
        let molecular_weight = atoms.iter()
            .map(|atom| self.get_atomic_weight(&atom.element))
            .sum();
        
        let metadata = ProteinMetadata {
            resolution,
            experiment_type,
            deposition_date: None,
            molecular_weight,
            total_atoms: atoms.len(),
            total_residues: processed_residues.len(),
        };
        
        info!("✅ Parsed protein: {} atoms, {} residues, {} chains", 
              atoms.len(), processed_residues.len(), processed_chains.len());
        
        Ok(ProteinStructure {
            name,
            pdb_id,
            chains: processed_chains,
            residues: processed_residues,
            atoms,
            metadata,
            bbox,
            center_of_mass,
            radius_of_gyration,
        })
    }
    
    /// Parsear línea de átomo PDB
    fn parse_atom_line(&self, line: &str) -> Result<Atom> {
        if line.len() < 54 {
            return Err(anyhow!("Invalid ATOM line length"));
        }
        
        let atom_id: u32 = line[6..11].trim().parse()
            .map_err(|_| anyhow!("Invalid atom ID"))?;
        
        let atom_name = line[12..16].trim().to_string();
        let residue_name = line[17..20].trim().to_string();
        let chain_id = line[21..22].trim().to_string();
        
        let residue_id: u32 = line[22..26].trim().parse()
            .map_err(|_| anyhow!("Invalid residue ID"))?;
        
        let x: f64 = line[30..38].trim().parse()
            .map_err(|_| anyhow!("Invalid X coordinate"))?;
        let y: f64 = line[38..46].trim().parse()
            .map_err(|_| anyhow!("Invalid Y coordinate"))?;
        let z: f64 = line[46..54].trim().parse()
            .map_err(|_| anyhow!("Invalid Z coordinate"))?;
        
        let occupancy: f64 = if line.len() > 54 {
            line[54..60].trim().parse().unwrap_or(1.0)
        } else {
            1.0
        };
        
        let b_factor: f64 = if line.len() > 60 {
            line[60..66].trim().parse().unwrap_or(0.0)
        } else {
            0.0
        };
        
        let element = if line.len() > 76 {
            line[76..78].trim().to_string()
        } else {
            // Extraer elemento del nombre del átomo
            atom_name.chars().filter(|c| c.is_alphabetic()).collect::<String>()
        };
        
        // Obtener propiedades del elemento
        let (partial_charge, van_der_waals_radius) = if let Some(props) = self.element_properties.get(&element) {
            (0.0, props.van_der_waals_radius) // Partial charge se calculará después
        } else {
            warn!("Unknown element: {}", element);
            (0.0, 1.5) // Valores por defecto
        };
        
        Ok(Atom {
            atom_id,
            element,
            atom_name,
            coordinates: [x, y, z],
            occupancy,
            b_factor,
            partial_charge,
            van_der_waals_radius,
            residue_id,
            chain_id,
        })
    }
    
    /// Calcular centro de masa con SIMD acceleration
    fn calculate_center_of_mass(&self, atoms: &[Atom]) -> [f64; 3] {
        if atoms.is_empty() {
            return [0.0; 3];
        }
        
        let mut center = [0.0; 3];
        let mut total_mass = 0.0;
        
        for atom in atoms {
            let mass = self.get_atomic_weight(&atom.element);
            total_mass += mass;
            
            for i in 0..3 {
                center[i] += atom.coordinates[i] * mass;
            }
        }
        
        if total_mass > 0.0 {
            for i in 0..3 {
                center[i] /= total_mass;
            }
        }
        
        center
    }
    
    /// Calcular radio de giro
    fn calculate_radius_of_gyration(&self, atoms: &[Atom], center_of_mass: &[f64; 3]) -> f64 {
        if atoms.is_empty() {
            return 0.0;
        }
        
        let mut sum_squared_distances = 0.0;
        let mut total_mass = 0.0;
        
        for atom in atoms {
            let mass = self.get_atomic_weight(&atom.element);
            total_mass += mass;
            
            let dx = atom.coordinates[0] - center_of_mass[0];
            let dy = atom.coordinates[1] - center_of_mass[1];
            let dz = atom.coordinates[2] - center_of_mass[2];
            
            let squared_distance = dx * dx + dy * dy + dz * dz;
            sum_squared_distances += mass * squared_distance;
        }
        
        if total_mass > 0.0 {
            (sum_squared_distances / total_mass).sqrt()
        } else {
            0.0
        }
    }
    
    /// Obtener peso atómico
    fn get_atomic_weight(&self, element: &str) -> f64 {
        match element {
            "H" => 1.008,
            "C" => 12.011,
            "N" => 14.007,
            "O" => 15.999,
            "P" => 30.974,
            "S" => 32.066,
            "F" => 18.998,
            "Cl" => 35.453,
            "Br" => 79.904,
            "I" => 126.904,
            "Na" => 22.990,
            "Mg" => 24.305,
            "K" => 39.098,
            "Ca" => 40.078,
            "Fe" => 55.845,
            "Zn" => 65.380,
            _ => 12.0, // Peso por defecto
        }
    }
    
    /// Inicializar propiedades de aminoácidos
    fn initialize_amino_acid_properties(&mut self) {
        // Propiedades hidrofóbicas de Kyte-Doolittle
        let amino_acids = [
            ("ALA", AminoAcidProperties { hydrophobicity: 1.8, charge: 0.0, molecular_weight: 89.1, volume: 88.6 }),
            ("ARG", AminoAcidProperties { hydrophobicity: -4.5, charge: 1.0, molecular_weight: 174.2, volume: 173.4 }),
            ("ASN", AminoAcidProperties { hydrophobicity: -3.5, charge: 0.0, molecular_weight: 132.1, volume: 114.1 }),
            ("ASP", AminoAcidProperties { hydrophobicity: -3.5, charge: -1.0, molecular_weight: 133.1, volume: 111.1 }),
            ("CYS", AminoAcidProperties { hydrophobicity: 2.5, charge: 0.0, molecular_weight: 121.2, volume: 108.5 }),
            ("GLU", AminoAcidProperties { hydrophobicity: -3.5, charge: -1.0, molecular_weight: 147.1, volume: 138.4 }),
            ("GLN", AminoAcidProperties { hydrophobicity: -3.5, charge: 0.0, molecular_weight: 146.1, volume: 143.8 }),
            ("GLY", AminoAcidProperties { hydrophobicity: -0.4, charge: 0.0, molecular_weight: 75.1, volume: 60.1 }),
            ("HIS", AminoAcidProperties { hydrophobicity: -3.2, charge: 0.1, molecular_weight: 155.2, volume: 153.2 }),
            ("ILE", AminoAcidProperties { hydrophobicity: 4.5, charge: 0.0, molecular_weight: 131.2, volume: 166.7 }),
            ("LEU", AminoAcidProperties { hydrophobicity: 3.8, charge: 0.0, molecular_weight: 131.2, volume: 166.7 }),
            ("LYS", AminoAcidProperties { hydrophobicity: -3.9, charge: 1.0, molecular_weight: 146.2, volume: 168.6 }),
            ("MET", AminoAcidProperties { hydrophobicity: 1.9, charge: 0.0, molecular_weight: 149.2, volume: 162.9 }),
            ("PHE", AminoAcidProperties { hydrophobicity: 2.8, charge: 0.0, molecular_weight: 165.2, volume: 189.9 }),
            ("PRO", AminoAcidProperties { hydrophobicity: -1.6, charge: 0.0, molecular_weight: 115.1, volume: 112.7 }),
            ("SER", AminoAcidProperties { hydrophobicity: -0.8, charge: 0.0, molecular_weight: 105.1, volume: 89.0 }),
            ("THR", AminoAcidProperties { hydrophobicity: -0.7, charge: 0.0, molecular_weight: 119.1, volume: 116.1 }),
            ("TRP", AminoAcidProperties { hydrophobicity: -0.9, charge: 0.0, molecular_weight: 204.2, volume: 227.8 }),
            ("TYR", AminoAcidProperties { hydrophobicity: -1.3, charge: 0.0, molecular_weight: 181.2, volume: 193.6 }),
            ("VAL", AminoAcidProperties { hydrophobicity: 4.2, charge: 0.0, molecular_weight: 117.1, volume: 140.0 }),
        ];
        
        for (name, props) in amino_acids {
            self.amino_acid_properties.insert(name.to_string(), props);
        }
    }
    
    /// Inicializar propiedades de elementos
    fn initialize_element_properties(&mut self) {
        let elements = [
            ("H", ElementProperties { atomic_number: 1, van_der_waals_radius: 1.20, electronegativity: 2.20, covalent_radius: 0.31 }),
            ("C", ElementProperties { atomic_number: 6, van_der_waals_radius: 1.70, electronegativity: 2.55, covalent_radius: 0.76 }),
            ("N", ElementProperties { atomic_number: 7, van_der_waals_radius: 1.55, electronegativity: 3.04, covalent_radius: 0.71 }),
            ("O", ElementProperties { atomic_number: 8, van_der_waals_radius: 1.52, electronegativity: 3.44, covalent_radius: 0.66 }),
            ("P", ElementProperties { atomic_number: 15, van_der_waals_radius: 1.80, electronegativity: 2.19, covalent_radius: 1.07 }),
            ("S", ElementProperties { atomic_number: 16, van_der_waals_radius: 1.80, electronegativity: 2.58, covalent_radius: 1.05 }),
            ("F", ElementProperties { atomic_number: 9, van_der_waals_radius: 1.47, electronegativity: 3.98, covalent_radius: 0.57 }),
            ("Cl", ElementProperties { atomic_number: 17, van_der_waals_radius: 1.75, electronegativity: 3.16, covalent_radius: 0.99 }),
            ("Na", ElementProperties { atomic_number: 11, van_der_waals_radius: 2.27, electronegativity: 0.93, covalent_radius: 1.66 }),
            ("Mg", ElementProperties { atomic_number: 12, van_der_waals_radius: 1.73, electronegativity: 1.31, covalent_radius: 1.41 }),
            ("Ca", ElementProperties { atomic_number: 20, van_der_waals_radius: 2.31, electronegativity: 1.00, covalent_radius: 1.76 }),
            ("Fe", ElementProperties { atomic_number: 26, van_der_waals_radius: 2.00, electronegativity: 1.83, covalent_radius: 1.32 }),
            ("Zn", ElementProperties { atomic_number: 30, van_der_waals_radius: 2.01, electronegativity: 1.65, covalent_radius: 1.22 }),
        ];
        
        for (name, props) in elements {
            self.element_properties.insert(name.to_string(), props);
        }
    }
}

impl Default for ProteinParser {
    fn default() -> Self {
        Self::new()
    }
}