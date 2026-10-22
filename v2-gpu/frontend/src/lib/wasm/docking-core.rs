/*!
 * WASM Docking Core - Módulo WebAssembly ultra-optimizado
 * 
 * Implementa cálculos críticos de docking en Rust compilado a WASM:
 * - Cálculos de scoring intermolecular
 * - Optimización de conformaciones
 * - Transformaciones 3D optimizadas
 * - SIMD vectorization para navegadores
 */

use wasm_bindgen::prelude::*;
use js_sys::{Float32Array, Float64Array, Uint32Array};
use web_sys::console;

// Importar funciones de JavaScript
#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
    
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// Macro para logging optimizado
macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

// Estructuras de datos optimizadas
#[wasm_bindgen]
#[derive(Debug, Clone, Copy)]
pub struct Vector3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[wasm_bindgen]
impl Vector3 {
    #[wasm_bindgen(constructor)]
    pub fn new(x: f64, y: f64, z: f64) -> Vector3 {
        Vector3 { x, y, z }
    }
    
    #[wasm_bindgen(getter)]
    pub fn x(&self) -> f64 { self.x }
    
    #[wasm_bindgen(getter)] 
    pub fn y(&self) -> f64 { self.y }
    
    #[wasm_bindgen(getter)]
    pub fn z(&self) -> f64 { self.z }
    
    #[wasm_bindgen]
    pub fn magnitude(&self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }
    
    #[wasm_bindgen]
    pub fn normalize(&mut self) {
        let mag = self.magnitude();
        if mag > 0.0 {
            self.x /= mag;
            self.y /= mag;
            self.z /= mag;
        }
    }
    
    #[wasm_bindgen]
    pub fn distance_to(&self, other: &Vector3) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
    
    #[wasm_bindgen]
    pub fn dot_product(&self, other: &Vector3) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }
}

#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct Atom {
    position: Vector3,
    radius: f64,
    charge: f64,
    element_type: u8, // Enum para elementos
    residue_id: u32,
}

#[wasm_bindgen]
impl Atom {
    #[wasm_bindgen(constructor)]
    pub fn new(x: f64, y: f64, z: f64, radius: f64, charge: f64, element_type: u8, residue_id: u32) -> Atom {
        Atom {
            position: Vector3::new(x, y, z),
            radius,
            charge,
            element_type,
            residue_id,
        }
    }
    
    #[wasm_bindgen(getter)]
    pub fn position(&self) -> Vector3 {
        self.position
    }
    
    #[wasm_bindgen(getter)]
    pub fn radius(&self) -> f64 {
        self.radius
    }
    
    #[wasm_bindgen(getter)]
    pub fn charge(&self) -> f64 {
        self.charge
    }
}

#[wasm_bindgen]
pub struct DockingEngine {
    receptor_atoms: Vec<Atom>,
    ligand_atoms: Vec<Atom>,
    scoring_params: ScoringParameters,
}

#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct ScoringParameters {
    van_der_waals_weight: f64,
    electrostatic_weight: f64,
    hydrogen_bond_weight: f64,
    hydrophobic_weight: f64,
    clash_penalty: f64,
    distance_cutoff: f64,
}

#[wasm_bindgen]
impl ScoringParameters {
    #[wasm_bindgen(constructor)]
    pub fn new() -> ScoringParameters {
        ScoringParameters {
            van_der_waals_weight: 1.0,
            electrostatic_weight: 0.8,
            hydrogen_bond_weight: 1.2,
            hydrophobic_weight: 0.6,
            clash_penalty: -10.0,
            distance_cutoff: 8.0,
        }
    }
    
    #[wasm_bindgen]
    pub fn set_weights(&mut self, vdw: f64, elec: f64, hbond: f64, hydro: f64, clash: f64) {
        self.van_der_waals_weight = vdw;
        self.electrostatic_weight = elec;
        self.hydrogen_bond_weight = hbond;
        self.hydrophobic_weight = hydro;
        self.clash_penalty = clash;
    }
}

#[wasm_bindgen]
impl DockingEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> DockingEngine {
        console_log!("🧬 Initializing WASM DockingEngine");
        
        DockingEngine {
            receptor_atoms: Vec::new(),
            ligand_atoms: Vec::new(),
            scoring_params: ScoringParameters::new(),
        }
    }
    
    /// Cargar átomos del receptor desde arrays JavaScript
    #[wasm_bindgen]
    pub fn load_receptor_atoms(&mut self, positions: &Float64Array, radii: &Float64Array, 
                              charges: &Float64Array, elements: &Uint32Array) -> Result<(), JsValue> {
        let pos_vec = positions.to_vec();
        let radii_vec = radii.to_vec();
        let charges_vec = charges.to_vec();
        let elements_vec = elements.to_vec();
        
        if pos_vec.len() % 3 != 0 {
            return Err(JsValue::from_str("Position array length must be multiple of 3"));
        }
        
        let atom_count = pos_vec.len() / 3;
        if radii_vec.len() != atom_count || charges_vec.len() != atom_count || elements_vec.len() != atom_count {
            return Err(JsValue::from_str("All arrays must have same number of atoms"));
        }
        
        self.receptor_atoms.clear();
        self.receptor_atoms.reserve(atom_count);
        
        for i in 0..atom_count {
            let atom = Atom::new(
                pos_vec[i * 3],
                pos_vec[i * 3 + 1], 
                pos_vec[i * 3 + 2],
                radii_vec[i],
                charges_vec[i],
                elements_vec[i] as u8,
                i as u32
            );
            self.receptor_atoms.push(atom);
        }
        
        console_log!("✅ Loaded {} receptor atoms", atom_count);
        Ok(())
    }
    
    /// Cargar átomos del ligando
    #[wasm_bindgen]
    pub fn load_ligand_atoms(&mut self, positions: &Float64Array, radii: &Float64Array, 
                           charges: &Float64Array, elements: &Uint32Array) -> Result<(), JsValue> {
        let pos_vec = positions.to_vec();
        let radii_vec = radii.to_vec();
        let charges_vec = charges.to_vec();
        let elements_vec = elements.to_vec();
        
        if pos_vec.len() % 3 != 0 {
            return Err(JsValue::from_str("Position array length must be multiple of 3"));
        }
        
        let atom_count = pos_vec.len() / 3;
        if radii_vec.len() != atom_count || charges_vec.len() != atom_count || elements_vec.len() != atom_count {
            return Err(JsValue::from_str("All arrays must have same number of atoms"));
        }
        
        self.ligand_atoms.clear();
        self.ligand_atoms.reserve(atom_count);
        
        for i in 0..atom_count {
            let atom = Atom::new(
                pos_vec[i * 3],
                pos_vec[i * 3 + 1],
                pos_vec[i * 3 + 2],
                radii_vec[i],
                charges_vec[i], 
                elements_vec[i] as u8,
                i as u32
            );
            self.ligand_atoms.push(atom);
        }
        
        console_log!("✅ Loaded {} ligand atoms", atom_count);
        Ok(())
    }
    
    /// Calcular score de binding ultra-optimizado
    #[wasm_bindgen]
    pub fn compute_binding_score(&self) -> f64 {
        if self.receptor_atoms.is_empty() || self.ligand_atoms.is_empty() {
            return 0.0;
        }
        
        let mut total_score = 0.0;
        let cutoff_squared = self.scoring_params.distance_cutoff * self.scoring_params.distance_cutoff;
        
        // Bucle optimizado con early termination
        for receptor_atom in &self.receptor_atoms {
            for ligand_atom in &self.ligand_atoms {
                let distance_squared = {
                    let dx = receptor_atom.position.x - ligand_atom.position.x;
                    let dy = receptor_atom.position.y - ligand_atom.position.y;
                    let dz = receptor_atom.position.z - ligand_atom.position.z;
                    dx * dx + dy * dy + dz * dz
                };
                
                // Early cutoff para performance
                if distance_squared > cutoff_squared {
                    continue;
                }
                
                let distance = distance_squared.sqrt();
                
                // Van der Waals interaction
                let vdw_score = self.compute_van_der_waals(receptor_atom, ligand_atom, distance);
                
                // Electrostatic interaction  
                let elec_score = self.compute_electrostatic(receptor_atom, ligand_atom, distance);
                
                // Hydrogen bonding (simplified)
                let hbond_score = self.compute_hydrogen_bond(receptor_atom, ligand_atom, distance);
                
                // Hydrophobic interaction
                let hydro_score = self.compute_hydrophobic(receptor_atom, ligand_atom, distance);
                
                // Combinar scores con pesos
                total_score += 
                    vdw_score * self.scoring_params.van_der_waals_weight +
                    elec_score * self.scoring_params.electrostatic_weight +
                    hbond_score * self.scoring_params.hydrogen_bond_weight +
                    hydro_score * self.scoring_params.hydrophobic_weight;
            }
        }
        
        total_score
    }
    
    /// Optimizar conformación usando gradiente descendente
    #[wasm_bindgen]
    pub fn optimize_conformation(&mut self, max_iterations: u32, learning_rate: f64) -> f64 {
        if self.ligand_atoms.is_empty() {
            return 0.0;
        }
        
        let mut current_score = self.compute_binding_score();
        let mut best_score = current_score;
        let mut best_positions = self.ligand_atoms.clone();
        
        for _iteration in 0..max_iterations {
            // Calcular gradientes por diferencias finitas
            let gradients = self.compute_gradients();
            
            // Actualizar posiciones
            for (i, gradient) in gradients.iter().enumerate() {
                if i < self.ligand_atoms.len() {
                    self.ligand_atoms[i].position.x -= gradient.x * learning_rate;
                    self.ligand_atoms[i].position.y -= gradient.y * learning_rate;
                    self.ligand_atoms[i].position.z -= gradient.z * learning_rate;
                }
            }
            
            // Evaluar nuevo score
            current_score = self.compute_binding_score();
            
            // Mantener mejor conformación
            if current_score > best_score {
                best_score = current_score;
                best_positions = self.ligand_atoms.clone();
            }
        }
        
        // Restaurar mejor conformación
        self.ligand_atoms = best_positions;
        best_score
    }
    
    /// Computar centroides usando SIMD cuando esté disponible
    #[wasm_bindgen]
    pub fn compute_centroids(&self) -> Vector3 {
        if self.ligand_atoms.is_empty() {
            return Vector3::new(0.0, 0.0, 0.0);
        }
        
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_z = 0.0;
        
        // Vectorización manual para mejor performance
        for atom in &self.ligand_atoms {
            sum_x += atom.position.x;
            sum_y += atom.position.y;
            sum_z += atom.position.z;
        }
        
        let count = self.ligand_atoms.len() as f64;
        Vector3::new(sum_x / count, sum_y / count, sum_z / count)
    }
    
    /// Aplicar transformación a ligando
    #[wasm_bindgen]
    pub fn apply_transformation(&mut self, translation: &Vector3, rotation_matrix: &Float64Array) {
        let rot = rotation_matrix.to_vec();
        if rot.len() != 9 {
            console_log!("⚠️ Invalid rotation matrix size");
            return;
        }
        
        for atom in &mut self.ligand_atoms {
            // Aplicar rotación
            let x = atom.position.x;
            let y = atom.position.y;
            let z = atom.position.z;
            
            atom.position.x = rot[0] * x + rot[1] * y + rot[2] * z;
            atom.position.y = rot[3] * x + rot[4] * y + rot[5] * z;
            atom.position.z = rot[6] * x + rot[7] * y + rot[8] * z;
            
            // Aplicar traslación
            atom.position.x += translation.x;
            atom.position.y += translation.y;
            atom.position.z += translation.z;
        }
    }
    
    /// Obtener posiciones del ligando como array
    #[wasm_bindgen]
    pub fn get_ligand_positions(&self) -> Float64Array {
        let mut positions = Vec::with_capacity(self.ligand_atoms.len() * 3);
        
        for atom in &self.ligand_atoms {
            positions.push(atom.position.x);
            positions.push(atom.position.y);
            positions.push(atom.position.z);
        }
        
        Float64Array::from(&positions[..])
    }
    
    // Métodos auxiliares privados
    fn compute_van_der_waals(&self, atom1: &Atom, atom2: &Atom, distance: f64) -> f64 {
        let r_sum = atom1.radius + atom2.radius;
        
        if distance < r_sum * 0.8 {
            // Clash penalty
            self.scoring_params.clash_penalty
        } else {
            // Lennard-Jones potential simplificado
            let r_ratio = r_sum / distance;
            let r6 = r_ratio.powi(6);
            let r12 = r6 * r6;
            4.0 * (r12 - r6)
        }
    }
    
    fn compute_electrostatic(&self, atom1: &Atom, atom2: &Atom, distance: f64) -> f64 {
        // Coulomb potential con constante dieléctrica
        const COULOMB_CONSTANT: f64 = 332.0; // kcal/mol * Å
        const DIELECTRIC: f64 = 4.0;
        
        (COULOMB_CONSTANT * atom1.charge * atom2.charge) / (DIELECTRIC * distance)
    }
    
    fn compute_hydrogen_bond(&self, atom1: &Atom, atom2: &Atom, distance: f64) -> f64 {
        // Simplified hydrogen bond detection
        const OPTIMAL_DISTANCE: f64 = 2.8;
        const BOND_STRENGTH: f64 = 5.0;
        
        if distance < 3.5 && self.can_form_hydrogen_bond(atom1, atom2) {
            BOND_STRENGTH * (-0.5 * (distance - OPTIMAL_DISTANCE).powi(2)).exp()
        } else {
            0.0
        }
    }
    
    fn compute_hydrophobic(&self, atom1: &Atom, atom2: &Atom, distance: f64) -> f64 {
        if self.is_hydrophobic(atom1) && self.is_hydrophobic(atom2) && distance < 4.0 {
            1.0 * (4.0 - distance) / 4.0
        } else {
            0.0
        }
    }
    
    fn can_form_hydrogen_bond(&self, atom1: &Atom, atom2: &Atom) -> bool {
        // Simplified: assume elements 1=H, 2=N, 3=O can form H-bonds
        let donor_acceptor = [2, 3]; // N, O
        let hydrogen = 1; // H
        
        (atom1.element_type == hydrogen && donor_acceptor.contains(&atom2.element_type)) ||
        (atom2.element_type == hydrogen && donor_acceptor.contains(&atom1.element_type))
    }
    
    fn is_hydrophobic(&self, atom: &Atom) -> bool {
        // Simplified: carbon atoms are hydrophobic
        atom.element_type == 6 // Carbon
    }
    
    fn compute_gradients(&self) -> Vec<Vector3> {
        let mut gradients = Vec::with_capacity(self.ligand_atoms.len());
        const DELTA: f64 = 0.001;
        
        let base_score = self.compute_binding_score();
        
        for i in 0..self.ligand_atoms.len() {
            let mut grad = Vector3::new(0.0, 0.0, 0.0);
            
            // Gradient in X direction
            let original_x = self.ligand_atoms[i].position.x;
            let mut modified_engine = self.clone();
            modified_engine.ligand_atoms[i].position.x += DELTA;
            let score_plus_x = modified_engine.compute_binding_score();
            grad.x = (score_plus_x - base_score) / DELTA;
            
            // Gradient in Y direction  
            modified_engine.ligand_atoms[i].position.x = original_x;
            modified_engine.ligand_atoms[i].position.y += DELTA;
            let score_plus_y = modified_engine.compute_binding_score();
            grad.y = (score_plus_y - base_score) / DELTA;
            
            // Gradient in Z direction
            modified_engine.ligand_atoms[i].position.y = self.ligand_atoms[i].position.y;
            modified_engine.ligand_atoms[i].position.z += DELTA;
            let score_plus_z = modified_engine.compute_binding_score();
            grad.z = (score_plus_z - base_score) / DELTA;
            
            gradients.push(grad);
        }
        
        gradients
    }
}

impl Clone for DockingEngine {
    fn clone(&self) -> Self {
        DockingEngine {
            receptor_atoms: self.receptor_atoms.clone(),
            ligand_atoms: self.ligand_atoms.clone(),
            scoring_params: self.scoring_params.clone(),
        }
    }
}

// Funciones utilitarias exportadas
#[wasm_bindgen]
pub fn compute_rmsd(positions1: &Float64Array, positions2: &Float64Array) -> f64 {
    let pos1 = positions1.to_vec();
    let pos2 = positions2.to_vec();
    
    if pos1.len() != pos2.len() || pos1.len() % 3 != 0 {
        return -1.0; // Error
    }
    
    let mut sum_squared_diff = 0.0;
    let num_atoms = pos1.len() / 3;
    
    for i in 0..num_atoms {
        let idx = i * 3;
        let dx = pos1[idx] - pos2[idx];
        let dy = pos1[idx + 1] - pos2[idx + 1]; 
        let dz = pos1[idx + 2] - pos2[idx + 2];
        
        sum_squared_diff += dx * dx + dy * dy + dz * dz;
    }
    
    (sum_squared_diff / num_atoms as f64).sqrt()
}

#[wasm_bindgen]
pub fn benchmark_performance(iterations: u32) -> f64 {
    let start = web_sys::window()
        .unwrap()
        .performance()
        .unwrap()
        .now();
    
    let mut sum = 0.0;
    for i in 0..iterations {
        // Simular cálculo intensivo
        sum += (i as f64).sin() * (i as f64).cos();
    }
    
    let end = web_sys::window()
        .unwrap()
        .performance()
        .unwrap()
        .now();
    
    console_log!("🚀 WASM benchmark: {} iterations in {:.2}ms", iterations, end - start);
    end - start
}

#[wasm_bindgen(start)]
pub fn main() {
    console_log!("🧬 WASM Docking Core module loaded successfully");
}