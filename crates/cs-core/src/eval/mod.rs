//! Standard evaluation of protein–protein docking poses and a benchmark harness.
//!
//! - [`Native`] scores poses of an unbound ligand against the bound complex with the CAPRI
//!   measures (fnat, fnonnat, iRMSD, LRMSD and the CAPRI class) and DockQ. Definitions
//!   follow DockQ v2 (Mirabello & Wallner 2024), against which they are validated (see
//!   `docs/EVALUATION.md` and `tests/eval.rs`).
//! - [`align`] maps residues between unbound and bound structures, whose chain IDs,
//!   numbering and missing residues usually differ.
//! - [`bench`] runs any docking function over the Protein-Protein Docking Benchmark 5.5 and
//!   aggregates success rates per difficulty category.

pub mod align;
pub mod bench;
mod grid;
pub mod native;
pub mod residues;

pub use native::{Capri, Metrics, Native, NativeInfo, Side, dockq};
