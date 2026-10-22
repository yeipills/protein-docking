//! Rotation-friendly context shapes: critical points, local frames and bit-packed rings.
//!
//! Around each critical point the neighbourhood is sampled on a cylindrical grid in the local
//! frame (normal = z): `ring_radii × heights` rings of [`K`] samples each, sample `a` at
//! angle `2πa/K`. A rotation about the normal by `2π/K` is then a cyclic shift of every ring,
//! i.e. one `rotate_left` of a `u32`. Each sample is classified by its signed distance into
//! four bit masks per ring (the paper's bitset trick):
//!
//! - `inside`: `d < 0`,
//! - `core`: `d < -core` (deep inside; overlap there is a clash),
//! - `shell`: `0 <= d < shell` (the exterior band a partner should fill),
//! - `surf`: `|d| < surf` (the surface layer; two surfaces in contact share it).
//!
//! The ligand is sampled in the *docked* geometry: flipped (normal reversed) and mirrored so
//! that its ring `r`, bit `a` lands on the receptor's ring `r`, bit `a + k` under rotation `k`
//! (see [`pose`]). Matching is then pure AND + popcount (`matching` module).

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use super::surface::{Surface, SurfacePoint};
use crate::error::{Error, Result};
use crate::geom::grid::Grid;
use crate::geom::transform::{M3, Rigid, axis_angle_matrix, mat_mul, mat_vec, transpose};
use crate::geom::vec3::*;
use crate::progress::{Progress, check};

/// Samples per ring = rotations tried about the normal (angular step 11.25°).
pub const K: usize = 32;

/// Geometry of the context-shape descriptors.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DescriptorParams {
    /// Minimum distance between critical points, in Å.
    pub spacing: f64,
    /// Radius of the patch whose normals are averaged into the critical point normal, in Å.
    /// Larger values give a normal that reflects the overall patch orientation rather than
    /// atomic bumps, which is what a docking contact needs.
    pub normal_radius: f64,
    /// Ring radii (distance from the normal axis), in Å.
    pub ring_radii: Vec<f64>,
    /// Ring heights along the receptor normal, in Å (negative = inside the receptor).
    pub heights: Vec<f64>,
    /// Width of the exterior band rewarded when filled by the partner, in Å.
    pub shell: f64,
    /// Depth below the surface beyond which an overlap counts as a clash, in Å. Shallower
    /// overlaps are tolerated (side-chain flexibility, surface noise).
    pub core: f64,
    /// Half-width of the surface layer, in Å.
    pub surf: f64,
    /// Distance between the matched points along the receptor normal in a pose, in Å.
    pub gap: f64,
    /// Tilts of the ligand frames away from the patch normal, in degrees. Each ligand point
    /// gets its base frame plus `tilt_azimuths` frames per tilt angle. Patch normals of two
    /// surfaces in contact typically disagree by 10–20°, and with a single frame per point
    /// that error, times the ligand radius, dominates the pose error.
    pub tilt_angles: Vec<f64>,
    /// Directions of tilt per tilt angle, evenly spread around the normal.
    pub tilt_azimuths: usize,
}

impl Default for DescriptorParams {
    fn default() -> Self {
        Self {
            spacing: 2.5,
            normal_radius: 5.0,
            ring_radii: vec![1.5, 3.0, 4.5, 6.0, 7.5, 9.0],
            heights: vec![-3.75, -2.25, -0.75, 0.75, 2.25, 3.75],
            shell: 3.0,
            core: 1.5,
            surf: 1.0,
            gap: 0.0,
            tilt_angles: vec![12.0],
            tilt_azimuths: 6,
        }
    }
}

impl DescriptorParams {
    // Negated comparisons also reject NaN.
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    pub fn validate(&self) -> Result<()> {
        let bad = |m: &str| Err(Error::Invalid(m.to_string()));
        if !(self.spacing >= 0.5) {
            return bad("descriptor spacing must be >= 0.5 Å");
        }
        if !(self.normal_radius > 0.0) {
            return bad("normal_radius must be > 0");
        }
        if self.ring_radii.is_empty() || self.heights.is_empty() {
            return bad("ring_radii and heights must not be empty");
        }
        if self.ring_radii.len() * self.heights.len() > 512 {
            return bad("at most 512 rings (ring_radii × heights)");
        }
        if !self.ring_radii.iter().all(|r| *r >= 0.0 && r.is_finite())
            || !self.heights.iter().all(|h| h.is_finite())
        {
            return bad("ring radii must be >= 0 and heights finite");
        }
        if !self
            .tilt_angles
            .iter()
            .all(|t| t.is_finite() && t.abs() <= 90.0)
            || self.tilt_azimuths > 64
        {
            return bad("tilt angles must be within ±90° and tilt_azimuths <= 64");
        }
        if !(self.shell > 0.0) || !(self.core >= 0.0) || !(self.surf > 0.0) || !self.gap.is_finite()
        {
            return bad("shell and surf must be > 0, core >= 0 and gap finite");
        }
        Ok(())
    }

    pub fn n_rings(&self) -> usize {
        self.ring_radii.len() * self.heights.len()
    }
}

/// Which side of the match a descriptor set is built for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Receptor,
    Ligand,
}

/// Critical points of one surface with their frames and packed context shapes. For the
/// ligand role every critical point appears once per frame (base + tilted copies).
pub struct Descriptors {
    pub points: Vec<V3>,
    /// Local frame per point: `[x, y, n]`, orthonormal and right-handed, `n` outward.
    pub frames: Vec<[V3; 3]>,
    pub n_rings: usize,
    /// `points.len() × n_rings` masks each (see module docs).
    pub shell: Vec<u32>,
    pub inside: Vec<u32>,
    pub core: Vec<u32>,
    pub surf: Vec<u32>,
}

/// The four masks of every ring of one descriptor.
#[derive(Clone, Copy)]
pub struct Rings<'a> {
    pub shell: &'a [u32],
    pub inside: &'a [u32],
    pub core: &'a [u32],
    pub surf: &'a [u32],
}

impl Descriptors {
    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    #[inline]
    pub fn rings(&self, i: usize) -> Rings<'_> {
        let r = i * self.n_rings..(i + 1) * self.n_rings;
        Rings {
            shell: &self.shell[r.clone()],
            inside: &self.inside[r.clone()],
            core: &self.core[r.clone()],
            surf: &self.surf[r],
        }
    }
}

/// Builds critical points, frames and context shapes of `surface` for the given role.
pub fn build(
    surface: &dyn Surface,
    params: &DescriptorParams,
    role: Role,
    progress: &dyn Progress,
) -> Result<Descriptors> {
    params.validate()?;
    let dense = surface.surface_points();
    if dense.is_empty() {
        return Err(Error::Invalid("the surface has no points".into()));
    }
    let mut crit = critical_points(&dense, params.spacing, params.normal_radius);
    if role == Role::Ligand {
        crit = crit
            .into_iter()
            .flat_map(|(c, f)| tilted_frames(&f, params).into_iter().map(move |t| (c, t)))
            .collect();
    }
    check(progress)?;

    let offsets = sample_offsets(params, role);
    let n_rings = params.n_rings();
    let per_point: Vec<[u32; 4]> = crit
        .par_iter()
        .map_init(
            || (Vec::with_capacity(offsets.len()), Vec::new()),
            |(world, dist), (c, frame)| {
                world.clear();
                world.extend(offsets.iter().map(|l| local_to_world(*c, frame, *l)));
                surface.signed_distances(world, dist);
                (0..n_rings)
                    .map(|r| {
                        // [shell, inside, core, surf]
                        let mut m = [0u32; 4];
                        for a in 0..K {
                            let d = dist[r * K + a];
                            let bit = 1u32 << a;
                            if d < 0.0 {
                                m[1] |= bit;
                                if d < -params.core {
                                    m[2] |= bit;
                                }
                            } else if d < params.shell {
                                m[0] |= bit;
                            }
                            if d.abs() < params.surf {
                                m[3] |= bit;
                            }
                        }
                        m
                    })
                    .collect::<Vec<_>>()
            },
        )
        .flatten_iter()
        .collect();
    check(progress)?;

    let (points, frames) = crit.into_iter().unzip();
    Ok(Descriptors {
        points,
        frames,
        n_rings,
        shell: per_point.iter().map(|m| m[0]).collect(),
        inside: per_point.iter().map(|m| m[1]).collect(),
        core: per_point.iter().map(|m| m[2]).collect(),
        surf: per_point.iter().map(|m| m[3]).collect(),
    })
}

/// Local sample offsets in ring-major order (`ring * K + angle`).
///
/// Receptor sample `(ρ, a, z)` sits at angle `+2πa/K`, height `z`. The ligand sample with the
/// same indices sits at angle `-2πa/K`, height `gap - z`: that is where, in the ligand's own
/// frame, the receptor sample `(ρ, a + k, z)` falls when the pose for rotation `k` is applied.
fn sample_offsets(params: &DescriptorParams, role: Role) -> Vec<V3> {
    let mut out = Vec::with_capacity(params.n_rings() * K);
    for &rho in &params.ring_radii {
        for &z in &params.heights {
            for a in 0..K {
                let phi = std::f64::consts::TAU * a as f64 / K as f64;
                out.push(match role {
                    Role::Receptor => [rho * phi.cos(), rho * phi.sin(), z],
                    Role::Ligand => [rho * phi.cos(), -rho * phi.sin(), params.gap - z],
                });
            }
        }
    }
    out
}

#[inline]
fn local_to_world(c: V3, f: &[V3; 3], l: V3) -> V3 {
    add(
        c,
        add(add(scale(f[0], l[0]), scale(f[1], l[1])), scale(f[2], l[2])),
    )
}

/// Greedy thinning in input order (no two kept points closer than `spacing`), each kept point
/// with a frame built from the average normal of the dense points within `normal_radius`.
fn critical_points(dense: &[SurfacePoint], spacing: f64, normal_radius: f64) -> Vec<(V3, [V3; 3])> {
    let pos: Vec<V3> = dense.iter().map(|p| p.pos).collect();
    let grid = Grid::new(&pos, spacing);
    let mut removed = vec![false; pos.len()];
    let mut near = Vec::new();
    let mut kept = Vec::new();
    for i in 0..pos.len() {
        if removed[i] {
            continue;
        }
        kept.push(i);
        grid.within(&pos, pos[i], spacing, &mut near);
        for &(j, _) in &near {
            removed[j as usize] = true;
        }
    }
    let ngrid = Grid::new(&pos, normal_radius);
    kept.par_iter()
        .map_init(Vec::new, |near, &i| {
            ngrid.within(&pos, pos[i], normal_radius, near);
            let mut n = [0.0; 3];
            for &(j, d) in near.iter() {
                // Smooth weight so the normal varies continuously along the surface.
                let w = 1.0 - d / normal_radius;
                n = add(n, scale(dense[j as usize].normal, w));
            }
            let n = if norm2(n) > 1e-12 {
                normalize(n)
            } else {
                dense[i].normal
            };
            (pos[i], frame_from_normal(n))
        })
        .collect()
}

/// The frame itself followed by its tilted copies (see [`DescriptorParams::tilt_angles`]).
fn tilted_frames(f: &[V3; 3], params: &DescriptorParams) -> Vec<[V3; 3]> {
    let mut out = vec![*f];
    let n_az = if params.tilt_angles.is_empty() {
        0
    } else {
        params.tilt_azimuths.max(1)
    };
    for &tilt in &params.tilt_angles {
        for j in 0..n_az {
            let phi = std::f64::consts::TAU * j as f64 / n_az as f64;
            let axis = add(scale(f[0], phi.cos()), scale(f[1], phi.sin()));
            let r = axis_angle_matrix(axis, tilt.to_radians());
            out.push(f.map(|v| mat_vec(&r, v)));
        }
    }
    out
}

/// Deterministic right-handed frame `[x, y, n]`. The choice of `x` is arbitrary: all [`K`]
/// rotations about `n` are tried during matching.
pub fn frame_from_normal(n: V3) -> [V3; 3] {
    let a = if n[0].abs() < 0.6 {
        [1.0, 0.0, 0.0]
    } else if n[1].abs() < 0.6 {
        [0.0, 1.0, 0.0]
    } else {
        [0.0, 0.0, 1.0]
    };
    let x = normalize(sub(a, scale(n, dot(a, n))));
    let y = cross(n, x);
    [x, y, n]
}

/// Columns `x, y, n` as a matrix (local -> world rotation).
fn frame_matrix(f: &[V3; 3]) -> M3 {
    [
        [f[0][0], f[1][0], f[2][0]],
        [f[0][1], f[1][1], f[2][1]],
        [f[0][2], f[1][2], f[2][2]],
    ]
}

/// Rigid transform of the ligand that puts ligand point `q` (frame `fq`) on receptor point
/// `p` (frame `fp`) shifted by `gap` along the receptor normal, with the ligand normal
/// opposite to the receptor normal and rotated by `2πk/K` about it.
///
/// In local coordinates: `receptor_local = Rz(2πk/K) · F · ligand_local + gap·ẑ` with
/// `F = diag(1, -1, -1)` (half turn about x).
pub fn pose(p: V3, fp: &[V3; 3], q: V3, fq: &[V3; 3], k: usize, gap: f64) -> Rigid {
    let theta = std::f64::consts::TAU * k as f64 / K as f64;
    let (s, c) = theta.sin_cos();
    let rz_f: M3 = [[c, s, 0.0], [s, -c, 0.0], [0.0, 0.0, -1.0]]; // Rz(θ) · diag(1,-1,-1)
    let rot = mat_mul(
        &mat_mul(&frame_matrix(fp), &rz_f),
        &transpose(&frame_matrix(fq)),
    );
    let target = add(p, scale(fp[2], gap));
    Rigid {
        rot,
        trans: sub(target, mat_vec(&rot, q)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::transform::Rng;

    #[test]
    fn frames_are_orthonormal() {
        let mut rng = Rng::new(5);
        for _ in 0..200 {
            let n = normalize([
                rng.next_f64() - 0.5,
                rng.next_f64() - 0.5,
                rng.next_f64() - 0.5,
            ]);
            let [x, y, z] = frame_from_normal(n);
            assert!((norm(x) - 1.0).abs() < 1e-12 && (norm(y) - 1.0).abs() < 1e-12);
            assert!(dot(x, z).abs() < 1e-12 && dot(x, y).abs() < 1e-12);
            assert!(dist2(cross(x, y), z) < 1e-20);
        }
    }

    /// The pose for rotation `k` maps ligand sample `(r, a)` onto receptor sample
    /// `(r, a + k)`: the invariant the bit-shift matching relies on.
    #[test]
    fn pose_aligns_samples_with_shifted_rings() {
        let params = DescriptorParams {
            gap: 0.7,
            ..Default::default()
        };
        let rec = sample_offsets(&params, Role::Receptor);
        let lig = sample_offsets(&params, Role::Ligand);
        let mut rng = Rng::new(11);
        let rand_n = |rng: &mut Rng| {
            normalize([
                rng.next_f64() - 0.5,
                rng.next_f64() - 0.5,
                rng.next_f64() - 0.5,
            ])
        };
        let (p, q) = ([1.0, 2.0, 3.0], [-4.0, 0.5, 2.0]);
        let fp = frame_from_normal(rand_n(&mut rng));
        let fq = frame_from_normal(rand_n(&mut rng));
        for k in [0, 1, 7, 31] {
            let t = pose(p, &fp, q, &fq, k, params.gap);
            assert!(dot(t.apply_vector(fq[2]), fp[2]) < -1.0 + 1e-12);
            for r in 0..params.n_rings() {
                for a in 0..K {
                    let lw = t.apply(local_to_world(q, &fq, lig[r * K + a]));
                    let rw = local_to_world(p, &fp, rec[r * K + (a + k) % K]);
                    assert!(dist2(lw, rw) < 1e-18, "k={k} r={r} a={a}");
                }
            }
        }
    }
}
