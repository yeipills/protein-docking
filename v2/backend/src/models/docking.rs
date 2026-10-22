/*!
 * Docking models - Estructuras para resultados de docking
 */

use serde::{Deserialize, Serialize};
use nalgebra::{Vector3, Matrix3};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockingResult {
    pub id: String,
    pub score: f64,
    pub energy: f64,
    pub transformation: Transformation,
    pub contacts: Vec<Contact>,
    pub metrics: DockingMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockingParameters {
    pub algorithm: DockingAlgorithm,
    pub max_iterations: u32,
    pub convergence_threshold: f64,
    pub grid_spacing: f64,
    pub search_radius: f64,
    pub enable_gpu: bool,
    pub enable_ml: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockingConfiguration {
    pub receptor_id: String,
    pub ligand_id: String,
    pub parameters: DockingParameters,
    pub constraints: Vec<DockingConstraint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transformation {
    pub translation: Vector3<f64>,
    pub rotation: Matrix3<f64>,
    pub rmsd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub receptor_atom_id: u32,
    pub ligand_atom_id: u32,
    pub distance: f64,
    pub contact_type: ContactType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockingMetrics {
    pub computation_time_ms: u64,
    pub iterations: u32,
    pub memory_used_mb: f64,
    pub gpu_utilization: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DockingAlgorithm {
    GpuAccelerated,
    SimdOptimized,
    Traditional,
    MachineLearning,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContactType {
    VanDerWaals,
    Hydrogen,
    Electrostatic,
    Hydrophobic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockingConstraint {
    pub constraint_type: ConstraintType,
    pub atoms: Vec<u32>,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstraintType {
    DistanceRestraint,
    AngleRestraint,
    PositionRestraint,
    SurfaceConstraint,
}

impl Default for DockingParameters {
    fn default() -> Self {
        Self {
            algorithm: DockingAlgorithm::GpuAccelerated,
            max_iterations: 10000,
            convergence_threshold: 1e-6,
            grid_spacing: 0.375,
            search_radius: 25.0,
            enable_gpu: true,
            enable_ml: true,
        }
    }
}