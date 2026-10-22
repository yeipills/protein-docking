/*!
 * Protein models - Estructuras de datos para proteínas
 */

use serde::{Deserialize, Serialize};
use nalgebra::Vector3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Protein {
    pub id: String,
    pub name: String,
    pub structure: ProteinStructure,
    pub metadata: ProteinMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProteinStructure {
    pub atoms: Vec<Atom>,
    pub residues: Vec<Residue>,
    pub chains: Vec<Chain>,
    pub bonds: Vec<Bond>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Atom {
    pub id: u32,
    pub element: String,
    pub position: Vector3<f64>,
    pub charge: f64,
    pub radius: f64,
    pub residue_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Residue {
    pub id: u32,
    pub name: String,
    pub residue_type: ResidueType,
    pub chain_id: String,
    pub sequence_number: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chain {
    pub id: String,
    pub name: String,
    pub chain_type: ChainType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bond {
    pub atom1_id: u32,
    pub atom2_id: u32,
    pub bond_type: BondType,
    pub length: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResidueType {
    Amino,
    Nucleic,
    Ligand,
    Water,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChainType {
    Protein,
    DNA,
    RNA,
    Ligand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BondType {
    Single,
    Double,
    Triple,
    Aromatic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProteinMetadata {
    pub pdb_id: Option<String>,
    pub organism: Option<String>,
    pub resolution: Option<f64>,
    pub method: Option<String>,
    pub date: Option<chrono::DateTime<chrono::Utc>>,
}

impl Protein {
    pub fn center_of_mass(&self) -> Vector3<f64> {
        if self.structure.atoms.is_empty() {
            return Vector3::zeros();
        }
        
        let sum = self.structure.atoms.iter()
            .fold(Vector3::zeros(), |acc, atom| acc + atom.position);
        
        sum / self.structure.atoms.len() as f64
    }
    
    pub fn bounding_box(&self) -> (Vector3<f64>, Vector3<f64>) {
        if self.structure.atoms.is_empty() {
            return (Vector3::zeros(), Vector3::zeros());
        }
        
        let first_pos = self.structure.atoms[0].position;
        let (mut min_pos, mut max_pos) = (first_pos, first_pos);
        
        for atom in &self.structure.atoms {
            for i in 0..3 {
                min_pos[i] = min_pos[i].min(atom.position[i]);
                max_pos[i] = max_pos[i].max(atom.position[i]);
            }
        }
        
        (min_pos, max_pos)
    }
}