//! Solvent-excluded surface (SES) computed directly from atoms, so a PDB file is enough and
//! no external MSMS run is needed.
//!
//! The SES is the boundary of the region a spherical probe cannot reach: with expanded
//! radii `R_i = r_i + probe`, the probe centre can be anywhere outside every expanded sphere
//! (the solvent-accessible exterior `E`), and a point `x` is outside the SES iff it is closer
//! than `probe` to `E`. The field
//!
//! ```text
//! g(x) = probe - dist(x, E)        (> 0 outside the SES, < 0 inside, |∇g| = 1)
//! ```
//!
//! is evaluated **exactly** at the nodes of a uniform grid and contoured at zero with
//! marching cubes ([`SesGrid::mesh`]). `dist(x, E)` is the distance to the nearest point of
//! the boundary of `E`, which is either inside an exposed patch of one expanded sphere, on
//! an exposed arc where two spheres meet, or at a vertex where three meet. Every exposed
//! feature is splatted onto the nodes within `probe + band` of it and each node keeps the
//! minimum, so no Euclidean distance transform of a voxelized solid is needed and there is
//! no voxelization error: the only approximation left is the piecewise-linear contouring.
//! See `docs/SURFACE.md` for details and validation against MSMS.

mod field;
mod mc;

use serde::{Deserialize, Serialize};

pub use field::SesGrid;

use crate::error::{Error, Result};
use crate::geom::vec3::V3;
use crate::io::pdb::Atom;
use crate::mesh::Mesh;
use crate::progress::Progress;

/// Atomic radius set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Radii {
    /// Bondi (1964) van der Waals radii by element ([`Atom::vdw_radius`]).
    #[default]
    Bondi,
    /// United-atom radii (hydrogens folded into heavy atoms) by residue and atom name: the
    /// set used to compute the MSMS surfaces of the sample data. Use it to reproduce those
    /// surfaces from heavy atoms; other atoms fall back to Bondi.
    United,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SesParams {
    /// Probe radius in Å.
    pub probe: f64,
    /// Grid node spacing in Å. Triangles are about this size.
    pub grid_spacing: f64,
    pub radii: Radii,
    /// Keep hydrogen atoms (most structures have none; with them, use Bondi radii).
    pub include_hydrogens: bool,
    /// Keep HETATM records: modified residues (MSE), cofactors, ions. Waters are always
    /// dropped by the PDB reader.
    pub include_hetatm: bool,
    /// Drop surfaces of internal cavities the probe cannot reach from outside (as MSMS does
    /// by default), leaving one closed surface per molecule.
    pub fill_cavities: bool,
    /// Depth inside the surface, in Å, down to which [`SesGrid::signed_distance`] is exact;
    /// deeper values are clamped. The mesh only needs `2.5 * grid_spacing`, the default
    /// when this is smaller.
    pub interior_band: f64,
}

impl Default for SesParams {
    fn default() -> Self {
        Self {
            probe: 1.4,
            grid_spacing: 0.5,
            radii: Radii::Bondi,
            include_hydrogens: false,
            include_hetatm: true,
            fill_cavities: true,
            interior_band: 0.0,
        }
    }
}

impl SesParams {
    // Negated comparisons also reject NaN.
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    pub fn validate(&self) -> Result<()> {
        let bad = |m: &str| Err(Error::Invalid(m.to_string()));
        if !(self.probe > 0.0 && self.probe <= 10.0) {
            return bad("probe must be in (0, 10] Å");
        }
        if !(self.grid_spacing >= 0.05 && self.grid_spacing <= 2.0) {
            return bad("grid_spacing must be in [0.05, 2] Å");
        }
        if !(self.interior_band >= 0.0 && self.interior_band <= 20.0) {
            return bad("interior_band must be in [0, 20] Å");
        }
        Ok(())
    }

    /// Interior band actually used: never thinner than what marching cubes needs.
    pub fn band(&self) -> f64 {
        self.interior_band.max(2.5 * self.grid_spacing)
    }

    /// Radius of `atom` in the selected set.
    pub fn radius(&self, atom: &Atom) -> f64 {
        match self.radii {
            Radii::Bondi => atom.vdw_radius(),
            Radii::United => united_radius(atom),
        }
    }

    /// Centres and radii of the atoms that make up the surface.
    pub fn spheres(&self, atoms: &[Atom]) -> (Vec<V3>, Vec<f64>) {
        atoms
            .iter()
            .filter(|a| self.include_hydrogens || !a.is_hydrogen())
            .filter(|a| self.include_hetatm || !a.hetatm)
            .map(|a| (a.pos, self.radius(a)))
            .unzip()
    }
}

/// SES mesh of `atoms`: closed, outward oriented, with unit outward vertex normals.
pub fn ses(atoms: &[Atom], params: &SesParams, progress: &dyn Progress) -> Result<Mesh> {
    SesGrid::from_atoms(atoms, params, progress)?.mesh(progress)
}

/// SES mesh of arbitrary spheres (`radii` are van der Waals radii, without the probe).
pub fn ses_spheres(
    centers: &[V3],
    radii: &[f64],
    params: &SesParams,
    progress: &dyn Progress,
) -> Result<Mesh> {
    SesGrid::from_spheres(centers, radii, params, progress)?.mesh(progress)
}

/// United-atom radius (Å) of a standard amino-acid atom; Bondi for anything else.
///
/// Carbons: 1.74 without hydrogens (and leucine CG), 1.86 aromatic CH, 2.0 aliphatic.
/// Nitrogens: 1.7 with one hydrogen, 1.8 with two, 2.0 for the lysine amine. Oxygens: 1.4 carbonyl/carboxyl, 1.6
/// hydroxyl. Sulfur: 1.8. These reproduce the radii of the MSMS sample surfaces.
pub fn united_radius(atom: &Atom) -> f64 {
    let res = atom.res_name.as_str();
    let name = atom.name.as_str();
    match atom.element.as_str() {
        "C" => {
            let no_h = name == "C"
                || matches!(
                    (res, name),
                    // LEU CG has a hydrogen, but the reference set types it like this.
                    ("ASP" | "ASN" | "PHE" | "TYR" | "TRP" | "HIS" | "LEU", "CG")
                        | ("GLU" | "GLN", "CD")
                        | ("TYR" | "ARG", "CZ")
                        | ("TRP", "CD2" | "CE2")
                );
            let aromatic_ch = matches!(
                (res, name),
                ("PHE", "CD1" | "CD2" | "CE1" | "CE2" | "CZ")
                    | ("TYR", "CD1" | "CD2" | "CE1" | "CE2")
                    | ("TRP", "CD1" | "CE3" | "CZ2" | "CZ3" | "CH2")
                    | ("HIS", "CD2" | "CE1")
            );
            if no_h {
                1.74
            } else if aromatic_ch {
                1.86
            } else {
                2.0
            }
        }
        "N" => match (res, name) {
            ("LYS", "NZ") => 2.0,
            ("ASN", "ND2") | ("GLN", "NE2") | ("ARG", "NH1" | "NH2") => 1.8,
            _ => 1.7,
        },
        "O" => match name {
            "OG" | "OG1" | "OH" => 1.6,
            _ => 1.4,
        },
        "S" => 1.8,
        _ => atom.vdw_radius(),
    }
}
