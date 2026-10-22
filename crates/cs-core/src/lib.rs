//! Context shapes of protein solvent-excluded surfaces.
//!
//! Input is an MSMS surface (`.vert` + `.face`). For each critical surface point a sphere of
//! rays is sampled and every sample is classified into ten layers around the surface
//! (`ses`, `in1..in4`, `out1..out4`, `vol`). Two pipelines produce the same [`ContextShapes`]:
//!
//! - [`corrected`]: exact signed distances, uniform rays, correct indexing (default).
//! - [`legacy`]: bit-for-bit reproduction of the original thesis scripts, for comparison.

pub mod corrected;
pub mod error;
pub mod export;
pub mod geom;
pub mod io;
pub mod legacy;
pub mod mesh;
pub mod model;
pub mod progress;

pub use error::{Error, Result};
pub use model::{ContextShapes, Layer, Mode};
