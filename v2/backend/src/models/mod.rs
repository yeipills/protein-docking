/*!
 * Models - Estructuras de datos y tipos del dominio
 */

pub mod protein;
pub mod docking;
pub mod job;
pub mod file;

pub use protein::{Protein, ProteinStructure, Atom, Residue};
pub use docking::{DockingResult, DockingParameters, DockingConfiguration};
pub use job::{Job, JobStatus, JobType};
pub use file::{FileInfo, FileType, ProcessedFile};