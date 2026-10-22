//! Surface representations the docking engine can work with.
//!
//! The engine only needs two things from a molecular surface: an inside/outside test with a
//! notion of depth (a signed distance, negative inside) and points on the surface with outward
//! normals. [`Surface`] captures exactly that, so a triangle mesh (MSMS or the built-in SES
//! generator, via [`MeshSurface`]) and the atom-only model in this file ([`AtomSurface`]) are
//! interchangeable.

use serde::{Deserialize, Serialize};

use super::grid3::{Grid3, distance_transform};
use crate::error::{Error, Result};
use crate::geom::sdf::SignedDistance;
use crate::geom::vec3::*;
use crate::io::pdb::Atom;
use crate::mesh::Mesh;
use crate::progress::{Progress, check};

/// A point on a surface with its unit outward normal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfacePoint {
    pub pos: V3,
    pub normal: V3,
}

/// What the docking engine needs from a molecular surface.
pub trait Surface: Send + Sync {
    /// Signed distance from `p` to the surface in Å, negative inside. It must be accurate
    /// within a few Å of the surface; farther away only the sign matters.
    fn signed_distance(&self, p: V3) -> f64;

    /// Signed distances of many points. Callers pass spatially coherent batches (all samples
    /// of one descriptor), which implementations may exploit.
    fn signed_distances(&self, points: &[V3], out: &mut Vec<f64>) {
        out.clear();
        out.extend(points.iter().map(|p| self.signed_distance(*p)));
    }

    /// Dense points on the surface (spacing well below the critical-point spacing) with unit
    /// outward normals.
    fn surface_points(&self) -> Vec<SurfacePoint>;
}

/// A closed, outward-oriented triangle mesh used as a docking surface.
pub struct MeshSurface {
    mesh: Mesh,
    sdf: SignedDistance,
}

impl MeshSurface {
    pub fn new(mesh: Mesh) -> MeshSurface {
        let sdf = SignedDistance::new(mesh.vertices.clone(), &mesh.faces);
        MeshSurface { mesh, sdf }
    }

    pub fn mesh(&self) -> &Mesh {
        &self.mesh
    }
}

impl Surface for MeshSurface {
    fn signed_distance(&self, p: V3) -> f64 {
        self.sdf.signed_distance(p)
    }

    fn signed_distances(&self, points: &[V3], out: &mut Vec<f64>) {
        // Consecutive samples are close together: reuse the last closest face as a bound.
        let mut hint = None;
        out.clear();
        out.extend(
            points
                .iter()
                .map(|p| self.sdf.signed_distance_hinted(*p, &mut hint)),
        );
    }

    fn surface_points(&self) -> Vec<SurfacePoint> {
        self.mesh
            .vertices
            .iter()
            .zip(&self.mesh.normals)
            .filter(|(_, n)| norm2(**n) > 0.0)
            .map(|(v, n)| SurfacePoint {
                pos: *v,
                normal: normalize(*n),
            })
            .collect()
    }
}

/// The SES from the built-in generator ([`crate::surface`]): its exact grid field where it
/// is defined (from `-band` inside to `probe` outside) and the distance to its mesh beyond.
///
/// The grid alone clamps distances outside `[-band, probe]`, too short for the descriptor
/// shells; the mesh alone gets the sign wrong at a few percent of deep interior points
/// (pseudonormals of slivers from marching cubes). Together they are exact near the surface,
/// robust in sign everywhere and cheap: most samples never touch the mesh.
pub struct SesSurface {
    grid: crate::surface::SesGrid,
    mesh: MeshSurface,
}

impl SesSurface {
    /// SES of `atoms`. The interior band is raised to at least 4 Å so that the descriptor
    /// classes (down to `core` = 1.5 Å and the deepest rings) come from the exact field.
    pub fn new(
        atoms: &[Atom],
        params: &crate::surface::SesParams,
        progress: &dyn Progress,
    ) -> Result<Self> {
        let params = crate::surface::SesParams {
            interior_band: params.interior_band.max(4.0),
            ..params.clone()
        };
        let grid = crate::surface::SesGrid::from_atoms(atoms, &params, progress)?;
        let mesh = MeshSurface::new(grid.mesh(progress)?);
        Ok(SesSurface { grid, mesh })
    }

    pub fn mesh(&self) -> &Mesh {
        self.mesh.mesh()
    }
}

impl Surface for SesSurface {
    fn signed_distance(&self, p: V3) -> f64 {
        let g = self.grid.signed_distance(p);
        let eps = 1e-3;
        if g >= self.grid.probe() - eps {
            self.mesh.signed_distance(p).abs().max(g)
        } else if g <= -self.grid.band() + eps {
            -self.mesh.signed_distance(p).abs().max(-g)
        } else {
            g
        }
    }

    fn surface_points(&self) -> Vec<SurfacePoint> {
        self.mesh.surface_points()
    }
}

/// Parameters of the atom-only surface model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AtomSurfaceParams {
    /// Solvent probe radius in Å (1.4 Å = water, the usual SES definition).
    pub probe: f64,
    /// Grid spacing in Å; the distance field is accurate to roughly half of it.
    pub spacing: f64,
    /// Empty space kept around the molecule, in Å.
    pub margin: f64,
    /// Turn internal cavities (exterior space not connected to the outside) into interior,
    /// as MSMS does by default: their walls are not docking surface.
    pub fill_cavities: bool,
}

impl Default for AtomSurfaceParams {
    fn default() -> Self {
        Self {
            probe: 1.4,
            spacing: 0.5,
            margin: 6.0,
            fill_cavities: true,
        }
    }
}

/// Solvent-excluded surface of a set of atoms, tabulated as a signed-distance grid.
///
/// Construction, with `E(x)` the distance from `x` to the region a probe centre can reach
/// (outside every atom sphere grown by the probe radius, i.e. outside the SAS):
///
/// 1. Mark grid nodes inside the SAS by splatting the grown spheres.
/// 2. `E` = Euclidean distance transform to the non-marked nodes.
/// 3. Points with `E > probe` are exactly those no probe ball covers: the SES interior `M`.
///    There the SES distance is exactly `probe - E` (distance to a union of equal balls is
///    the distance to their centres minus the radius).
/// 4. Outside `M`, the distance is the distance transform to `M`, bounded below by
///    `probe - E` (`E` is 1-Lipschitz).
///
/// The result is accurate to about `spacing / 2`, fills crevices like a real SES and costs a
/// few hundred milliseconds for a 5000-atom protein.
pub struct AtomSurface {
    grid: Grid3,
}

impl AtomSurface {
    /// SES of the heavy atoms of `atoms` with Bondi radii.
    pub fn new(
        atoms: &[Atom],
        params: &AtomSurfaceParams,
        progress: &dyn Progress,
    ) -> Result<Self> {
        let (centers, radii): (Vec<V3>, Vec<f64>) = atoms
            .iter()
            .filter(|a| !a.is_hydrogen())
            .map(|a| (a.pos, a.vdw_radius()))
            .unzip();
        Self::from_spheres(&centers, &radii, params, progress)
    }

    /// SES of arbitrary spheres (useful for synthetic shapes).
    #[allow(clippy::neg_cmp_op_on_partial_ord)] // negated comparisons also reject NaN
    pub fn from_spheres(
        centers: &[V3],
        radii: &[f64],
        params: &AtomSurfaceParams,
        progress: &dyn Progress,
    ) -> Result<Self> {
        if centers.is_empty() || centers.len() != radii.len() {
            return Err(Error::Invalid(
                "the atom surface needs at least one sphere and one radius per sphere".into(),
            ));
        }
        if !(params.spacing > 0.05 && params.spacing <= 2.0) {
            return Err(Error::Invalid(
                "surface grid spacing must be in (0.05, 2] Å".into(),
            ));
        }
        if !(params.probe >= 0.0) || !(params.margin >= 0.0) {
            return Err(Error::Invalid("probe and margin must be >= 0".into()));
        }
        let rp = params.probe;
        let rmax = radii.iter().copied().fold(0.0, f64::max);
        let pad = rmax + rp + params.margin + 2.0 * params.spacing;
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for c in centers {
            for a in 0..3 {
                lo[a] = lo[a].min(c[a] - pad);
                hi[a] = hi[a].max(c[a] + pad);
            }
        }
        let mut grid = Grid3::covering(lo, hi, params.spacing, 0.0);
        if grid.len() > 200_000_000 {
            return Err(Error::Invalid(
                "molecule too large for the surface grid; increase the spacing".into(),
            ));
        }
        let n = grid.len();

        // 1. SAS interior (probe-grown spheres).
        let mut sas_inside = vec![false; n];
        for (c, r) in centers.iter().zip(radii) {
            let rr = r + rp;
            let Some([(i0, i1), (j0, j1), (k0, k1)]) = grid.node_range(*c, rr) else {
                continue;
            };
            for k in k0..=k1 {
                for j in j0..=j1 {
                    for i in i0..=i1 {
                        if dist2(grid.node(i, j, k), *c) < rr * rr {
                            sas_inside[grid.index(i, j, k)] = true;
                        }
                    }
                }
            }
        }
        check(progress)?;

        // 2. Distance to the probe-accessible region. In 3D the nearest exterior node lies
        //    almost exactly at the distance of the continuous boundary (many nodes sit just
        //    outside it and tangential offsets cost only second order), so no correction.
        let exterior: Vec<bool> = sas_inside.iter().map(|&b| !b).collect();
        let e = distance_transform(grid.dims, params.spacing, &exterior);
        check(progress)?;

        // 3-4. SES interior and signed distance.
        let rp32 = rp as f32;
        let ses_inside: Vec<bool> = e.iter().map(|&v| v > rp32).collect();
        let to_inside = distance_transform(grid.dims, params.spacing, &ses_inside);
        for idx in 0..n {
            grid.data[idx] = if ses_inside[idx] {
                rp32 - e[idx]
            } else {
                to_inside[idx].max(rp32 - e[idx]).max(0.0)
            };
        }
        check(progress)?;
        if params.fill_cavities {
            fill_cavities(&mut grid);
        }
        Ok(AtomSurface { grid })
    }

    /// The tabulated signed-distance field.
    pub fn grid(&self) -> &Grid3 {
        &self.grid
    }
}

/// Flood fill of the exterior from the grid boundary (6-connected); exterior nodes it does
/// not reach are cavities and become shallow interior.
fn fill_cavities(grid: &mut Grid3) {
    let [nx, ny, nz] = grid.dims;
    let mut seen = vec![false; grid.len()];
    let mut stack: Vec<usize> = Vec::new();
    for k in 0..nz {
        for j in 0..ny {
            for i in 0..nx {
                let border =
                    i == 0 || j == 0 || k == 0 || i == nx - 1 || j == ny - 1 || k == nz - 1;
                let idx = grid.index(i, j, k);
                if border && grid.data[idx] >= 0.0 && !seen[idx] {
                    seen[idx] = true;
                    stack.push(idx);
                }
            }
        }
    }
    while let Some(idx) = stack.pop() {
        let (i, j, k) = (idx % nx, (idx / nx) % ny, idx / (nx * ny));
        let mut visit = |n: usize| {
            if !seen[n] && grid.data[n] >= 0.0 {
                seen[n] = true;
                stack.push(n);
            }
        };
        if i > 0 {
            visit(idx - 1);
        }
        if i + 1 < nx {
            visit(idx + 1);
        }
        if j > 0 {
            visit(idx - nx);
        }
        if j + 1 < ny {
            visit(idx + nx);
        }
        if k > 0 {
            visit(idx - nx * ny);
        }
        if k + 1 < nz {
            visit(idx + nx * ny);
        }
    }
    let shallow = -(grid.h as f32);
    for (v, s) in grid.data.iter_mut().zip(&seen) {
        if *v >= 0.0 && !s {
            *v = shallow;
        }
    }
}

impl Surface for AtomSurface {
    fn signed_distance(&self, p: V3) -> f64 {
        self.grid.sample(p)
    }

    /// Grid nodes within half a voxel of the zero level, projected onto it along the
    /// gradient (Newton step), with the normalized gradient as normal.
    fn surface_points(&self) -> Vec<SurfacePoint> {
        let g = &self.grid;
        let band = 0.5 * g.h;
        let mut out = Vec::new();
        for k in 1..g.dims[2] - 1 {
            for j in 1..g.dims[1] - 1 {
                for i in 1..g.dims[0] - 1 {
                    let d = g.data[g.index(i, j, k)] as f64;
                    if d.abs() > band {
                        continue;
                    }
                    let p = g.node(i, j, k);
                    let grad = g.gradient(p);
                    let len = norm(grad);
                    if len < 0.3 {
                        continue; // medial ridge: no well-defined surface direction
                    }
                    let n = scale(grad, 1.0 / len);
                    let q = sub(p, scale(n, d / len));
                    let gq = normalize(g.gradient(q));
                    out.push(SurfacePoint {
                        pos: q,
                        normal: if norm2(gq) > 0.0 { gq } else { n },
                    });
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::NoProgress;

    #[test]
    fn single_sphere_is_exact() {
        let s = AtomSurface::from_spheres(
            &[[1.0, 2.0, 3.0]],
            &[2.0],
            &AtomSurfaceParams::default(),
            &NoProgress,
        )
        .unwrap();
        for (p, want) in [
            ([1.0, 2.0, 4.0], -1.0),
            ([4.0, 2.0, 3.0], 1.0),
            ([1.0, 2.0, 1.5], -0.5),
            ([1.0, 6.0, 3.0], 2.0),
        ] {
            let d = s.signed_distance(p);
            assert!((d - want).abs() < 0.3, "{p:?}: {d} vs {want}");
        }
        let pts = s.surface_points();
        assert!(pts.len() > 100);
        for sp in pts {
            let r = sub(sp.pos, [1.0, 2.0, 3.0]);
            assert!((norm(r) - 2.0).abs() < 0.3, "{}", norm(r));
            // Grid-scale anisotropy of the distance transform tilts normals by up to ~25°
            // on a sphere this small; patch normals average it out.
            assert!(
                dot(normalize(r), sp.normal) > 0.85,
                "{}",
                dot(normalize(r), sp.normal)
            );
        }
    }

    #[test]
    fn internal_cavity_is_filled() {
        // A hollow shell of spheres: the inside is a cavity no probe can reach from outside.
        let dirs = crate::geom::fibonacci_sphere(400);
        let centers: Vec<V3> = dirs.iter().map(|d| scale(*d, 8.0)).collect();
        let radii = vec![1.8; centers.len()];
        let mk = |fill| {
            let p = AtomSurfaceParams {
                fill_cavities: fill,
                ..Default::default()
            };
            AtomSurface::from_spheres(&centers, &radii, &p, &NoProgress).unwrap()
        };
        assert!(mk(false).signed_distance([0.0; 3]) > 0.0);
        assert!(mk(true).signed_distance([0.0; 3]) < 0.0);
        assert!(mk(true).signed_distance([0.0, 0.0, 12.0]) > 0.0);
    }

    #[test]
    fn crevice_between_spheres_is_filled() {
        // Two touching spheres: the SES bridges the neck, the vdW surface does not.
        let s = AtomSurface::from_spheres(
            &[[-1.6, 0.0, 0.0], [1.6, 0.0, 0.0]],
            &[1.7, 1.7],
            &AtomSurfaceParams::default(),
            &NoProgress,
        )
        .unwrap();
        // At the neck, 0.7 Å off the axis: just outside both spheres, ~0.55 Å inside the SES
        // (the probe touching both spheres stops 1.25 Å above the axis).
        assert!(s.signed_distance([0.0, 0.7, 0.0]) < -0.2);
        assert!(s.signed_distance([0.0, 3.0, 0.0]) > 0.5);
    }
}
