//! Exact SES signed-distance field on a uniform grid.
//!
//! Each node stores `D = dist(x, E)`, the distance to the region `E` where the probe centre
//! can be, quantized to `u16` over `[0, probe + band]` (resolution below 1e-4 Å). Two codes
//! are reserved: [`EXT`] for nodes outside every expanded sphere (`D = 0`, `g = probe`) and
//! [`DEEP`] for nodes farther than `probe + band` from `E` (`g` clamped to `-band`).
//!
//! The boundary of `E` is made of exposed sphere patches, arcs where two expanded spheres
//! meet and vertices where three meet. The nearest point of `E` to an interior node is on
//! one of them, and every point of them belongs to the closure of `E`, so splatting each
//! feature onto the nodes around it and keeping the minimum (atomic min, in parallel) gives
//! the exact distance:
//!
//! 1. Nodes inside any expanded sphere are marked as interior.
//! 2. **Arcs and vertices.** For every pair of intersecting spheres, the parts of their
//!    circle buried in common neighbours are removed analytically; nodes near an exposed
//!    arc get the distance to the circle, and the arc endpoints (vertices) are splatted as
//!    points.
//! 3. **Patches.** A node inside sphere `i` whose radial projection onto the sphere is not
//!    buried in a neighbour gets `D = R_i - |x - a_i|`.
//!
//! **Cavities.** To drop surfaces of internal cavities, only features on the boundary of the
//! outer component of `E` are splatted. Arcs are grouped into connected boundary pieces
//! through their shared vertices (like MSMS's reduced surface); a piece is outer if any of
//! its arcs touches grid nodes of `E` reachable from the grid boundary. Patches follow the
//! arcs of their sphere. Classifying whole pieces, rather than nodes, keeps narrow pockets
//! that contain no grid node, and avoids merging a cavity with the outside when their probe
//! spheres overlap through a thin wall.

use std::f64::consts::TAU;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU16, Ordering::Relaxed};

use rayon::prelude::*;

use super::SesParams;
use crate::error::{Error, Result};
use crate::geom::grid::Grid;
use crate::geom::vec3::*;
use crate::io::pdb::Atom;
use crate::mesh::Mesh;
use crate::progress::{Progress, check};

/// Node outside every expanded sphere.
pub(crate) const EXT: u16 = u16::MAX;
/// Node deeper than the band.
pub(crate) const DEEP: u16 = u16::MAX - 1;
/// Hard limit on grid size (2 bytes per node), to fail early instead of exhausting memory.
const MAX_NODES: usize = 1 << 30;
/// Vertex keys pack three sphere indices in 21 bits each.
const MAX_SPHERES: usize = 1 << 21;
/// Small slack so points exactly on a neighbour sphere count as exposed.
const EPS: f64 = 1e-9;

/// SES signed distance sampled on a grid: negative inside, positive outside, exact (up to
/// 1e-4 Å) at the nodes within `[-band, probe]` and clamped outside that range. Between
/// nodes it is interpolated trilinearly. Cheap to query from many threads (e.g. docking
/// clash tests).
pub struct SesGrid {
    pub(crate) origin: V3,
    pub(crate) spacing: f64,
    pub(crate) dims: [usize; 3],
    pub(crate) probe: f64,
    pub(crate) band: f64,
    /// Quantization: `D = q / scale`.
    pub(crate) scale: f64,
    pub(crate) data: Vec<u16>,
}

impl SesGrid {
    pub fn from_atoms(atoms: &[Atom], params: &SesParams, progress: &dyn Progress) -> Result<Self> {
        let (centers, radii) = params.spheres(atoms);
        if centers.is_empty() {
            return Err(Error::Invalid("no atoms left for the surface".into()));
        }
        Self::from_spheres(&centers, &radii, params, progress)
    }

    /// Field of arbitrary spheres (`radii` are van der Waals radii, without the probe).
    pub fn from_spheres(
        centers: &[V3],
        radii: &[f64],
        params: &SesParams,
        progress: &dyn Progress,
    ) -> Result<Self> {
        params.validate()?;
        if centers.is_empty() || centers.len() != radii.len() || centers.len() >= MAX_SPHERES {
            return Err(Error::Invalid(format!(
                "need one radius per sphere and 1 to {} spheres",
                MAX_SPHERES - 1
            )));
        }
        if centers.iter().flatten().any(|c| !c.is_finite())
            || radii.iter().any(|r| !(r.is_finite() && *r > 0.0))
        {
            return Err(Error::Invalid(
                "sphere centres must be finite and radii positive".into(),
            ));
        }
        progress.report("ses_setup", 0.0);
        let probe = params.probe;
        let h = params.grid_spacing;
        let band = params.band();
        let big: Vec<f64> = radii.iter().map(|r| r + probe).collect();
        let rmax = big.iter().copied().fold(0.0, f64::max);

        // Nodes beyond `rmax` of every centre are outside all expanded spheres, so with this
        // padding the whole grid boundary is solvent (and connected).
        let pad = rmax + 3.0 * h;
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for c in centers {
            for k in 0..3 {
                lo[k] = lo[k].min(c[k]);
                hi[k] = hi[k].max(c[k]);
            }
        }
        let origin = lo.map(|v| v - pad);
        let dims: [usize; 3] =
            std::array::from_fn(|k| ((hi[k] - lo[k] + 2.0 * pad) / h).ceil() as usize + 1);
        let n_nodes = match dims.iter().try_fold(1usize, |a, &d| a.checked_mul(d)) {
            Some(n) if n <= MAX_NODES => n,
            _ => {
                return Err(Error::Invalid(format!(
                    "SES grid of {}x{}x{} nodes is too large; increase grid_spacing",
                    dims[0], dims[1], dims[2]
                )));
            }
        };
        let cap = probe + band;
        let lat = Lattice {
            origin,
            h,
            dims,
            scale: DEEP as f64 / cap,
            cap,
        };
        let s = Spheres {
            c: centers,
            big: &big,
            nb: Neighbors::new(centers, &big),
        };
        check(progress)?;

        progress.report("ses_interior", 0.05);
        let data: Vec<AtomicU16> = (0..n_nodes)
            .into_par_iter()
            .map(|_| AtomicU16::new(EXT))
            .collect();
        (0..centers.len())
            .into_par_iter()
            .filter(|&i| s.nb.active[i])
            .for_each(|i| {
                lat.for_ball(centers[i], big[i], |idx, _| data[idx].store(DEEP, Relaxed));
            });
        check(progress)?;
        let outer = params.fill_cavities.then(|| {
            progress.report("ses_outer", 0.15);
            flood(dims, |idx| data[idx].load(Relaxed) == EXT)
        });
        check(progress)?;

        progress.report("ses_arcs", 0.25);
        let arcs = exposed_arcs(&s, progress);
        check(progress)?;
        progress.report("ses_classify", 0.3);
        let keep = match &outer {
            Some(o) => {
                let keep = classify(&s, &lat, &arcs, o);
                fill_unreached(&lat, &data, &s, &arcs, &keep, o);
                keep
            }
            None => Keep::all(arcs.len(), centers.len()),
        };
        progress.report("ses_splat_arcs", 0.35);
        splat_arcs(&lat, &data, &s, &arcs, &keep, progress);
        check(progress)?;
        progress.report("ses_patches", 0.45);
        splat_patches(&lat, &data, &s, &arcs, &keep, progress);
        check(progress)?;

        // Same layout, so this reuses the allocation.
        let data: Vec<u16> = data.into_iter().map(AtomicU16::into_inner).collect();
        Ok(SesGrid {
            origin,
            spacing: h,
            dims,
            probe,
            band,
            scale: lat.scale,
            data,
        })
    }

    /// Marching-cubes mesh of the zero level set: closed, outward oriented, vertices welded,
    /// normals from the field gradient.
    pub fn mesh(&self, progress: &dyn Progress) -> Result<Mesh> {
        check(progress)?;
        progress.report("ses_mesh", 0.75);
        let m = super::mc::extract(self, progress)?;
        progress.report("done", 1.0);
        Ok(m)
    }

    pub fn origin(&self) -> V3 {
        self.origin
    }

    pub fn spacing(&self) -> f64 {
        self.spacing
    }

    /// Number of nodes along x, y, z.
    pub fn dims(&self) -> [usize; 3] {
        self.dims
    }

    pub fn probe(&self) -> f64 {
        self.probe
    }

    /// Depth inside the surface down to which values are exact.
    pub fn band(&self) -> f64 {
        self.band
    }

    /// Field value at node `(i, j, k)`.
    pub fn node(&self, i: usize, j: usize, k: usize) -> f64 {
        self.value(self.index(i, j, k))
    }

    /// Signed distance to the SES at `p` (trilinear between nodes), clamped to
    /// `[-band, probe]`; `probe` outside the grid.
    pub fn signed_distance(&self, p: V3) -> f64 {
        let mut base = [0usize; 3];
        let mut t = [0.0; 3];
        for k in 0..3 {
            let u = (p[k] - self.origin[k]) / self.spacing;
            // Written so that NaN also takes the solvent branch.
            if !(u >= 0.0 && u <= (self.dims[k] - 1) as f64) {
                return self.probe;
            }
            let b = (u.floor() as usize).min(self.dims[k] - 2);
            base[k] = b;
            t[k] = u - b as f64;
        }
        let [i, j, k] = base;
        let v = |di: usize, dj: usize, dk: usize| self.node(i + di, j + dj, k + dk);
        let lerp = |a: f64, b: f64, t: f64| a + (b - a) * t;
        let x00 = lerp(v(0, 0, 0), v(1, 0, 0), t[0]);
        let x10 = lerp(v(0, 1, 0), v(1, 1, 0), t[0]);
        let x01 = lerp(v(0, 0, 1), v(1, 0, 1), t[0]);
        let x11 = lerp(v(0, 1, 1), v(1, 1, 1), t[0]);
        lerp(lerp(x00, x10, t[1]), lerp(x01, x11, t[1]), t[2])
    }

    /// True if `p` is inside the solvent-excluded volume.
    pub fn contains(&self, p: V3) -> bool {
        self.signed_distance(p) < 0.0
    }

    #[inline]
    pub(crate) fn index(&self, i: usize, j: usize, k: usize) -> usize {
        i + self.dims[0] * (j + self.dims[1] * k)
    }

    #[inline]
    pub(crate) fn value(&self, idx: usize) -> f64 {
        self.decode(self.data[idx])
    }

    #[inline]
    pub(crate) fn decode(&self, q: u16) -> f64 {
        if q == EXT {
            self.probe
        } else {
            self.probe - q as f64 / self.scale
        }
    }

    #[inline]
    pub(crate) fn point(&self, i: usize, j: usize, k: usize) -> V3 {
        [
            self.origin[0] + i as f64 * self.spacing,
            self.origin[1] + j as f64 * self.spacing,
            self.origin[2] + k as f64 * self.spacing,
        ]
    }
}

/// Bit set of the nodes satisfying `open` that are 6-connected to node 0 (scanline flood
/// fill).
fn flood(dims: [usize; 3], open: impl Fn(usize) -> bool) -> Vec<u64> {
    let [nx, ny, nz] = dims;
    let mut seen = vec![0u64; (nx * ny * nz).div_ceil(64)];
    let is_seen = |s: &[u64], i: usize| s[i / 64] >> (i % 64) & 1 == 1;
    let mut stack = vec![(0usize, 0usize, 0usize)];
    while let Some((i, j, k)) = stack.pop() {
        let row = nx * (j + ny * k);
        if is_seen(&seen, row + i) || !open(row + i) {
            continue;
        }
        let (mut a, mut b) = (i, i);
        while a > 0 && open(row + a - 1) && !is_seen(&seen, row + a - 1) {
            a -= 1;
        }
        while b + 1 < nx && open(row + b + 1) && !is_seen(&seen, row + b + 1) {
            b += 1;
        }
        for x in row + a..=row + b {
            seen[x / 64] |= 1 << (x % 64);
        }
        let mut visit = |j2: usize, k2: usize| {
            let r2 = nx * (j2 + ny * k2);
            let mut x = a;
            while x <= b {
                if open(r2 + x) && !is_seen(&seen, r2 + x) {
                    stack.push((x, j2, k2));
                    while x <= b && open(r2 + x) {
                        x += 1;
                    }
                } else {
                    x += 1;
                }
            }
        };
        if j > 0 {
            visit(j - 1, k);
        }
        if j + 1 < ny {
            visit(j + 1, k);
        }
        if k > 0 {
            visit(j, k - 1);
        }
        if k + 1 < nz {
            visit(j, k + 1);
        }
    }
    seen
}

/// Grid geometry and quantization shared by the splatting passes.
struct Lattice {
    origin: V3,
    h: f64,
    dims: [usize; 3],
    /// Distances at or beyond `cap = probe + band` are stored as [`DEEP`].
    cap: f64,
    scale: f64,
}

impl Lattice {
    #[inline]
    fn quantize(&self, d: f64) -> u16 {
        if d >= self.cap {
            DEEP
        } else {
            (d * self.scale).round().min(DEEP as f64) as u16
        }
    }

    /// Inclusive node range along axis `k` covering `[lo, hi]`, or `None` if empty.
    fn range(&self, k: usize, lo: f64, hi: f64) -> Option<(usize, usize)> {
        let a = ((lo - self.origin[k]) / self.h).ceil().max(0.0);
        let b = ((hi - self.origin[k]) / self.h)
            .floor()
            .min((self.dims[k] - 1) as f64);
        (a <= b).then_some((a as usize, b as usize))
    }

    /// Calls `f(index, position)` for every node in the closed ball.
    fn for_ball(&self, c: V3, r: f64, mut f: impl FnMut(usize, V3)) {
        let [nx, ny, _] = self.dims;
        let Some((k0, k1)) = self.range(2, c[2] - r, c[2] + r) else {
            return;
        };
        for k in k0..=k1 {
            let z = self.origin[2] + k as f64 * self.h;
            let rz2 = r * r - (z - c[2]) * (z - c[2]);
            if rz2 < 0.0 {
                continue;
            }
            let rz = rz2.sqrt();
            let Some((j0, j1)) = self.range(1, c[1] - rz, c[1] + rz) else {
                continue;
            };
            for j in j0..=j1 {
                let y = self.origin[1] + j as f64 * self.h;
                let ry2 = rz2 - (y - c[1]) * (y - c[1]);
                if ry2 < 0.0 {
                    continue;
                }
                let ry = ry2.sqrt();
                let Some((i0, i1)) = self.range(0, c[0] - ry, c[0] + ry) else {
                    continue;
                };
                let row = nx * (j + ny * k);
                for i in i0..=i1 {
                    f(row + i, [self.origin[0] + i as f64 * self.h, y, z]);
                }
            }
        }
    }

    /// Calls `f(index, position)` for every node in the box.
    fn for_box(&self, lo: V3, hi: V3, mut f: impl FnMut(usize, V3)) {
        let [nx, ny, _] = self.dims;
        let (Some((i0, i1)), Some((j0, j1)), Some((k0, k1))) = (
            self.range(0, lo[0], hi[0]),
            self.range(1, lo[1], hi[1]),
            self.range(2, lo[2], hi[2]),
        ) else {
            return;
        };
        for k in k0..=k1 {
            let z = self.origin[2] + k as f64 * self.h;
            for j in j0..=j1 {
                let y = self.origin[1] + j as f64 * self.h;
                let row = nx * (j + ny * k);
                for i in i0..=i1 {
                    f(row + i, [self.origin[0] + i as f64 * self.h, y, z]);
                }
            }
        }
    }

    /// True if the boundary point `p` (outward direction `n`) sees a node of the outer
    /// solvent region in a straight line: a corner of the cell containing `p + h/2 n` whose
    /// segment to `p` enters none of the `blockers` (centre, squared radius). The line of
    /// sight keeps a cavity behind a thin wall from passing as outer.
    fn touches_outer(
        &self,
        outer: &[u64],
        p: V3,
        n: V3,
        blockers: impl Iterator<Item = (V3, f64)> + Clone,
    ) -> bool {
        let s = add(p, scale(n, 0.5 * self.h));
        let mut base = [0usize; 3];
        for k in 0..3 {
            let u = ((s[k] - self.origin[k]) / self.h).floor();
            if !(u >= 0.0 && u < (self.dims[k] - 1) as f64) {
                return true; // outside the grid is solvent
            }
            base[k] = u as usize;
        }
        let [nx, ny, _] = self.dims;
        (0..8).any(|c| {
            let ijk = [
                base[0] + (c & 1),
                base[1] + (c >> 1 & 1),
                base[2] + (c >> 2),
            ];
            let idx = ijk[0] + nx * (ijk[1] + ny * ijk[2]);
            if outer[idx / 64] >> (idx % 64) & 1 == 0 {
                return false;
            }
            let node: V3 = std::array::from_fn(|k| self.origin[k] + ijk[k] as f64 * self.h);
            let seg = sub(node, p);
            let len2 = norm2(seg).max(1e-30);
            blockers.clone().all(|(b, r2)| {
                let t = (dot(sub(b, p), seg) / len2).clamp(0.0, 1.0);
                dist2(add(p, scale(seg, t)), b) >= r2 * (1.0 - 1e-9)
            })
        })
    }
}

#[inline]
fn lower(cell: &AtomicU16, q: u16) {
    if q < cell.load(Relaxed) {
        cell.fetch_min(q, Relaxed);
    }
}

/// Expanded spheres and their neighbourhoods.
struct Spheres<'a> {
    c: &'a [V3],
    big: &'a [f64],
    nb: Neighbors,
}

/// Intersecting pairs of expanded spheres, ignoring spheres contained in another one
/// (they add nothing to the union). Lists are sorted by index.
struct Neighbors {
    active: Vec<bool>,
    start: Vec<usize>,
    list: Vec<u32>,
}

impl Neighbors {
    fn new(c: &[V3], big: &[f64]) -> Neighbors {
        let rmax = big.iter().copied().fold(0.0, f64::max);
        let grid = Grid::new(c, 2.0 * rmax);
        let raw: Vec<(bool, Vec<u32>)> = (0..c.len())
            .into_par_iter()
            .map_init(Vec::new, |buf, i| {
                grid.within(c, c[i], 2.0 * rmax, buf);
                let mut contained = false;
                let mut list = Vec::new();
                for &(k, d) in buf.iter() {
                    let ku = k as usize;
                    if ku == i || d >= big[i] + big[ku] {
                        continue;
                    }
                    // Of two identical spheres, the lower index survives.
                    if d + big[i] <= big[ku] + 1e-9 && (big[i] < big[ku] - 1e-9 || ku < i) {
                        contained = true;
                    }
                    list.push(k);
                }
                (!contained, list)
            })
            .collect();
        let active: Vec<bool> = raw.iter().map(|r| r.0).collect();
        let mut start = Vec::with_capacity(c.len() + 1);
        let mut list = Vec::new();
        start.push(0);
        for (_, l) in &raw {
            let first = list.len();
            list.extend(l.iter().copied().filter(|&k| active[k as usize]));
            list[first..].sort_unstable();
            start.push(list.len());
        }
        Neighbors {
            active,
            start,
            list,
        }
    }

    fn of(&self, i: usize) -> &[u32] {
        &self.list[self.start[i]..self.start[i + 1]]
    }
}

/// Circle where two expanded spheres meet, parametrized as
/// `center + rho * (cos φ e1 + sin φ e2)`.
struct Circle {
    center: V3,
    axis: V3,
    e1: V3,
    e2: V3,
    rho: f64,
    /// Axial coordinates of the two sphere centres (`z_a < 0 < z_b` or both on one side).
    z_a: f64,
    z_b: f64,
}

impl Circle {
    fn new(a: V3, ra: f64, b: V3, rb: f64) -> Option<Circle> {
        let ab = sub(b, a);
        let d = norm(ab);
        if d < 1e-12 {
            return None;
        }
        let axis = scale(ab, 1.0 / d);
        let t = (d * d + ra * ra - rb * rb) / (2.0 * d);
        let rho2 = ra * ra - t * t;
        if rho2 <= 1e-12 {
            return None;
        }
        let helper = if axis[0].abs() < 0.9 {
            [1.0, 0.0, 0.0]
        } else {
            [0.0, 1.0, 0.0]
        };
        let e1 = normalize(cross(axis, helper));
        let e2 = cross(axis, e1);
        Some(Circle {
            center: add(a, scale(axis, t)),
            axis,
            e1,
            e2,
            rho: rho2.sqrt(),
            z_a: -t,
            z_b: d - t,
        })
    }

    /// Squared distance from `p` to the arc `g` (angles relative to `phase`).
    fn arc_dist2(&self, phase: f64, g: &Gap, p: V3) -> f64 {
        let v = sub(p, self.center);
        let z = dot(v, self.axis);
        let (x1, x2) = (dot(v, self.e1), dot(v, self.e2));
        let phi = (x2.atan2(x1) - phase).rem_euclid(TAU);
        if g.to - g.from >= TAU || (phi >= g.from && phi <= g.to) {
            let rr = (x1 * x1 + x2 * x2).sqrt();
            (rr - self.rho) * (rr - self.rho) + z * z
        } else {
            let ends = [g.from, g.to].map(|t| dist2(p, self.at(phase + t)));
            ends[0].min(ends[1])
        }
    }

    /// True if a point at radial distance `r` and axial coordinate `z` lies in the wedge
    /// where the circle is the nearest part of the solvent region: between the rays from
    /// the circle towards the two sphere centres (in the plane through the axis).
    #[inline]
    fn in_wedge(&self, r: f64, z: f64) -> bool {
        let (vr, tol) = (r - self.rho, 1e-9);
        -self.rho * z - self.z_a * vr <= tol && vr * self.z_b + self.rho * z <= tol
    }

    fn of(s: &Spheres, i: usize, j: usize) -> Option<Circle> {
        Circle::new(s.c[i], s.big[i], s.c[j], s.big[j])
    }

    fn at(&self, phi: f64) -> V3 {
        add(
            self.center,
            add(
                scale(self.e1, self.rho * phi.cos()),
                scale(self.e2, self.rho * phi.sin()),
            ),
        )
    }

    /// Angular interval `(start, length)` of the circle inside sphere `(b, rb)`:
    /// `Some(None)` if none of it, `None` if all of it.
    fn buried_by(&self, b: V3, rb: f64) -> Option<Option<(f64, f64)>> {
        // |P(φ) - b|² = ρ² - 2ρ m cos(φ - ψ) + |w|², with w = b - center.
        let w = sub(b, self.center);
        let (w1, w2) = (dot(w, self.e1), dot(w, self.e2));
        let m = (w1 * w1 + w2 * w2).sqrt();
        let s = (self.rho * self.rho + norm2(w) - rb * rb) / (2.0 * self.rho);
        if s >= m {
            Some(None)
        } else if s <= -m {
            None
        } else {
            let half = (s / m).acos();
            let psi = w2.atan2(w1);
            Some(Some(((psi - half).rem_euclid(TAU), 2.0 * half)))
        }
    }
}

/// No sphere: the circle is fully exposed, without endpoints.
const NONE: u32 = u32::MAX;

/// Exposed arc `(from, to)`, in angles relative to the circle's phase, with the spheres
/// that end it ([`NONE`] for a full circle).
#[derive(Debug, Clone, Copy)]
struct Gap {
    from: f64,
    to: f64,
    k_from: u32,
    k_to: u32,
}

/// Exposed arcs of a circle given its buried intervals `(start, len, sphere)`. Angles are
/// relative to the returned phase (the start of the first interval).
fn gaps_of(buried: &mut Vec<(f64, f64, u32)>, gaps: &mut Vec<Gap>) -> f64 {
    gaps.clear();
    if buried.is_empty() {
        gaps.push(Gap {
            from: 0.0,
            to: TAU,
            k_from: NONE,
            k_to: NONE,
        });
        return 0.0;
    }
    // Rotate so φ = 0 is a buried start; intervals crossing 2π also get a copy shifted
    // back, which lands in the first merged block (it contains 0).
    let phase = buried[0].0;
    let k0 = buried[0].2;
    for t in 0..buried.len() {
        let (s, l, k) = buried[t];
        let s = (s - phase).rem_euclid(TAU);
        buried[t] = (s, l, k);
        if s + l > TAU {
            buried.push((s - TAU, l, k));
        }
    }
    buried.sort_unstable_by(|x, y| x.0.total_cmp(&y.0));
    let (mut end, mut k_end) = (f64::NEG_INFINITY, NONE);
    for &(s, l, k) in buried.iter() {
        if s > end && end > f64::NEG_INFINITY {
            gaps.push(Gap {
                from: end,
                to: s,
                k_from: k_end,
                k_to: k,
            });
        }
        if s + l > end {
            end = s + l;
            k_end = k;
        }
    }
    if end < TAU {
        gaps.push(Gap {
            from: end,
            to: TAU,
            k_from: k_end,
            k_to: k0,
        });
    }
    phase
}

/// One exposed arc on the circle of spheres `i < j`.
struct Arc {
    i: u32,
    j: u32,
    phase: f64,
    gap: Gap,
}

impl Arc {
    /// Key of the vertex `v` at one end: the three sphere indices, sorted, and on which side
    /// of their centres' plane `v` lies (three spheres meet at two mirror points).
    fn vertex_key(&self, s: &Spheres, k: u32, v: V3) -> (u64, bool) {
        let mut t = [self.i as u64, self.j as u64, k as u64];
        t.sort_unstable();
        let [a, b, c] = t.map(|x| s.c[x as usize]);
        let side = dot(sub(v, a), cross(sub(b, a), sub(c, a))) > 0.0;
        (t[0] << 42 | t[1] << 21 | t[2], side)
    }
}

fn exposed_arcs(s: &Spheres, progress: &dyn Progress) -> Vec<Arc> {
    (0..s.c.len())
        .into_par_iter()
        .filter(|&i| s.nb.active[i])
        .map_init(
            || (Vec::new(), Vec::new(), Vec::new()),
            |(common, buried, gaps), i| {
                let mut arcs = Vec::new();
                if progress.is_cancelled() {
                    return arcs;
                }
                let ni = s.nb.of(i);
                'pairs: for &j in ni.iter().filter(|&&j| j as usize > i) {
                    let Some(circle) = Circle::of(s, i, j as usize) else {
                        continue;
                    };
                    intersect_sorted(ni, s.nb.of(j as usize), common);
                    buried.clear();
                    for &k in common.iter() {
                        match circle.buried_by(s.c[k as usize], s.big[k as usize]) {
                            Some(Some((start, len))) => buried.push((start, len, k)),
                            Some(None) => {}
                            None => continue 'pairs,
                        }
                    }
                    let phase = gaps_of(buried, gaps);
                    arcs.extend(gaps.iter().map(|&gap| Arc {
                        i: i as u32,
                        j,
                        phase,
                        gap,
                    }));
                }
                arcs
            },
        )
        .flatten_iter()
        .collect()
}

/// Which features to splat.
struct Keep {
    arcs: Vec<bool>,
    /// Per sphere: [`KEPT`], [`DROPPED`], or both when it has kept and dropped arcs.
    spheres: Vec<u8>,
    /// Arcs on each sphere that has both kind of arcs (empty for the others).
    mixed: Vec<Vec<u32>>,
}

const KEPT: u8 = 1;
const DROPPED: u8 = 2;

impl Keep {
    fn all(n_arcs: usize, n_spheres: usize) -> Keep {
        Keep {
            arcs: vec![true; n_arcs],
            spheres: vec![KEPT; n_spheres],
            mixed: vec![Vec::new(); n_spheres],
        }
    }
}

/// Keeps the arcs whose boundary piece (arcs joined through shared vertices) touches the
/// outer solvent region somewhere.
fn classify(s: &Spheres, lat: &Lattice, arcs: &[Arc], outer: &[u64]) -> Keep {
    let mut ends: Vec<((u64, bool), u32)> = arcs
        .par_iter()
        .enumerate()
        .flat_map_iter(|(a, arc)| {
            let circle = Circle::of(s, arc.i as usize, arc.j as usize);
            [(arc.gap.from, arc.gap.k_from), (arc.gap.to, arc.gap.k_to)]
                .into_iter()
                .filter(|&(_, k)| k != NONE)
                .filter_map(move |(phi, k)| {
                    let v = circle.as_ref()?.at(arc.phase + phi);
                    Some((arc.vertex_key(s, k, v), a as u32))
                })
        })
        .collect();
    ends.par_sort_unstable();
    let mut uf = UnionFind::new(arcs.len());
    for w in ends.windows(2) {
        if w[0].0 == w[1].0 {
            uf.union(w[0].1, w[1].1);
        }
    }
    // A patch can also join boundary pieces that share no vertex (a band between two
    // circles, a channel wall): union the pieces around each patch of each sphere.
    let mut by_sphere = vec![Vec::new(); s.c.len()];
    for (a, arc) in arcs.iter().enumerate() {
        by_sphere[arc.i as usize].push(a as u32);
        by_sphere[arc.j as usize].push(a as u32);
    }
    let root: Vec<u32> = (0..arcs.len() as u32).map(|a| uf.find(a)).collect();
    let dirs = SphereSamples::new(1024);
    let joins: Vec<(u32, u32)> = (0..s.c.len())
        .into_par_iter()
        .flat_map_iter(|i| {
            let on = &by_sphere[i];
            let several = on.iter().any(|&a| root[a as usize] != root[on[0] as usize]);
            if several {
                patch_joins(s, arcs, i, on, &dirs)
            } else {
                Vec::new()
            }
        })
        .collect();
    for (a, b) in joins {
        uf.union(a, b);
    }
    let root: Vec<u32> = (0..arcs.len() as u32).map(|a| uf.find(a)).collect();
    let touches: Vec<AtomicBool> = (0..arcs.len()).map(|_| AtomicBool::new(false)).collect();
    arcs.par_iter().enumerate().for_each(|(a, arc)| {
        if touches[root[a] as usize].load(Relaxed) {
            return;
        }
        let Some(circle) = Circle::of(s, arc.i as usize, arc.j as usize) else {
            return;
        };
        // A few points along the arc, each probed just outside the crease.
        let samples = if arc.gap.to - arc.gap.from >= TAU {
            4
        } else {
            1
        };
        for t in 0..samples {
            let f = (t as f64 + 0.5) / samples as f64;
            let p = circle.at(arc.phase + arc.gap.from + f * (arc.gap.to - arc.gap.from));
            let (i, j) = (arc.i as usize, arc.j as usize);
            let blockers = [i, j]
                .into_iter()
                .chain(s.nb.of(i).iter().chain(s.nb.of(j)).map(|&k| k as usize))
                .map(|k| (s.c[k], s.big[k] * s.big[k]));
            if lat.touches_outer(outer, p, normalize(sub(p, circle.center)), blockers) {
                touches[root[a] as usize].store(true, Relaxed);
                return;
            }
        }
    });
    let keep_arcs: Vec<bool> = root
        .iter()
        .map(|&r| touches[r as usize].load(Relaxed))
        .collect();
    let spheres: Vec<AtomicU8> = (0..s.c.len()).map(|_| AtomicU8::new(0)).collect();
    arcs.par_iter().zip(&keep_arcs).for_each(|(arc, &kept)| {
        let bit = if kept { KEPT } else { DROPPED };
        spheres[arc.i as usize].fetch_or(bit, Relaxed);
        spheres[arc.j as usize].fetch_or(bit, Relaxed);
    });
    let mut spheres: Vec<u8> = spheres.into_iter().map(AtomicU8::into_inner).collect();
    // Spheres without exposed arcs are isolated (or fully buried): test them directly.
    spheres.par_iter_mut().enumerate().for_each(|(i, st)| {
        if *st != 0 || !s.nb.active[i] {
            return;
        }
        let blockers = std::iter::once(i)
            .chain(s.nb.of(i).iter().map(|&k| k as usize))
            .map(|k| (s.c[k], s.big[k] * s.big[k]));
        let seen = (0..6).any(|t| {
            let mut n = [0.0; 3];
            n[t / 2] = if t % 2 == 0 { 1.0 } else { -1.0 };
            let p = add(s.c[i], scale(n, s.big[i]));
            let exposed = s.nb.of(i).iter().all(|&k| {
                let k = k as usize;
                dist2(p, s.c[k]) >= s.big[k] * s.big[k]
            });
            exposed && lat.touches_outer(outer, p, n, blockers.clone())
        });
        *st = if seen { KEPT } else { DROPPED };
    });
    let mut mixed = vec![Vec::new(); s.c.len()];
    for (a, arc) in arcs.iter().enumerate() {
        for t in [arc.i, arc.j] {
            if spheres[t as usize] == KEPT | DROPPED {
                mixed[t as usize].push(a as u32);
            }
        }
    }
    Keep {
        arcs: keep_arcs,
        spheres,
        mixed,
    }
}

/// Turns into interior the solvent nodes not reached from the grid boundary that lie in a
/// cavity. Such a node may also sit in a pocket whose mouth is too narrow for the grid, so
/// it takes the class of the patch it faces: the nearest point of the union of spheres,
/// which is on the boundary of the node's own solvent component.
fn fill_unreached(
    lat: &Lattice,
    data: &[AtomicU16],
    s: &Spheres,
    arcs: &[Arc],
    keep: &Keep,
    outer: &[u64],
) {
    let rmax = s.big.iter().copied().fold(0.0, f64::max);
    let grid = Grid::new(s.c, 2.0 * rmax);
    let [nx, ny, _] = lat.dims;
    (0..data.len())
        .into_par_iter()
        .filter(|&idx| outer[idx / 64] >> (idx % 64) & 1 == 0 && data[idx].load(Relaxed) == EXT)
        .for_each_init(Vec::new, |buf, idx| {
            let ijk = [idx % nx, (idx / nx) % ny, idx / (nx * ny)];
            let x: V3 = std::array::from_fn(|k| lat.origin[k] + ijk[k] as f64 * lat.h);
            grid.within(s.c, x, 2.0 * rmax, buf);
            let Some(&(k, d)) = buf
                .iter()
                .filter(|(k, d)| s.nb.active[*k as usize] && *d > 1e-12)
                .min_by(|a, b| (a.1 - s.big[a.0 as usize]).total_cmp(&(b.1 - s.big[b.0 as usize])))
            else {
                return;
            };
            let k = k as usize;
            let y = add(s.c[k], scale(sub(x, s.c[k]), s.big[k] / d));
            let kept = match keep.spheres[k] {
                KEPT => true,
                DROPPED => false,
                _ => keep.mixed[k]
                    .iter()
                    .filter_map(|&a| {
                        let arc = &arcs[a as usize];
                        let c = Circle::of(s, arc.i as usize, arc.j as usize)?;
                        Some((c.arc_dist2(arc.phase, &arc.gap, y), keep.arcs[a as usize]))
                    })
                    .min_by(|a, b| a.0.total_cmp(&b.0))
                    .is_none_or(|(_, kept)| kept),
            };
            if !kept {
                data[idx].store(DEEP, Relaxed);
            }
        });
}

/// Nearly uniform unit directions (Fibonacci lattice) with their lattice neighbours.
struct SphereSamples {
    dirs: Vec<V3>,
    adj: Vec<Vec<u32>>,
}

impl SphereSamples {
    fn new(n: usize) -> Self {
        let dirs = crate::geom::fibonacci_sphere(n);
        let reach = 1.6 * (4.0 * std::f64::consts::PI / n as f64).sqrt();
        let cos_reach = reach.cos();
        // z decreases by 2/n per index, so neighbours are within this many indices.
        let window = (reach * n as f64 / 2.0).ceil() as usize + 1;
        let adj = (0..n)
            .map(|t| {
                (t.saturating_sub(window)..(t + window + 1).min(n))
                    .filter(|&u| u != t && dot(dirs[t], dirs[u]) > cos_reach)
                    .map(|u| u as u32)
                    .collect()
            })
            .collect();
        SphereSamples { dirs, adj }
    }
}

/// Pairs of arcs of sphere `i` (`on`) that bound the same exposed patch, found by sampling
/// the sphere: exposed samples are grouped into patches through lattice neighbours, and
/// each sample belongs to the boundary of its patch through its nearest arc.
fn patch_joins(
    s: &Spheres,
    arcs: &[Arc],
    i: usize,
    on: &[u32],
    samples: &SphereSamples,
) -> Vec<(u32, u32)> {
    let (a, r) = (s.c[i], s.big[i]);
    let near: Vec<(V3, f64)> =
        s.nb.of(i)
            .iter()
            .map(|&k| (s.c[k as usize], s.big[k as usize] * s.big[k as usize] - EPS))
            .collect();
    let bounds: Vec<(Circle, f64, Gap, u32)> = on
        .iter()
        .filter_map(|&id| {
            let arc = &arcs[id as usize];
            Some((
                Circle::of(s, arc.i as usize, arc.j as usize)?,
                arc.phase,
                arc.gap,
                id,
            ))
        })
        .collect();
    let points: Vec<V3> = samples.dirs.iter().map(|d| add(a, scale(*d, r))).collect();
    let exposed: Vec<bool> = points
        .iter()
        .map(|&p| near.iter().all(|&(b, r2)| dist2(p, b) >= r2))
        .collect();
    let nearest = |p: V3| {
        bounds
            .iter()
            .map(|(c, phase, g, id)| (c.arc_dist2(*phase, g, p), *id))
            .min_by(|x, y| x.0.total_cmp(&y.0))
            .map(|x| x.1)
    };
    let mut seen = vec![false; points.len()];
    let mut joins = Vec::new();
    let mut stack = Vec::new();
    for start in 0..points.len() {
        if !exposed[start] || seen[start] {
            continue;
        }
        let first = nearest(points[start]);
        seen[start] = true;
        stack.push(start);
        while let Some(t) = stack.pop() {
            if let (Some(x), Some(y)) = (first, nearest(points[t])) {
                if x != y {
                    joins.push((x, y));
                }
            }
            for &u in &samples.adj[t] {
                let u = u as usize;
                if exposed[u] && !seen[u] {
                    seen[u] = true;
                    stack.push(u);
                }
            }
        }
    }
    joins.sort_unstable();
    joins.dedup();
    joins
}

struct UnionFind {
    parent: Vec<u32>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        UnionFind {
            parent: (0..n as u32).collect(),
        }
    }

    fn find(&mut self, mut a: u32) -> u32 {
        while self.parent[a as usize] != a {
            let up = self.parent[self.parent[a as usize] as usize];
            self.parent[a as usize] = up;
            a = up;
        }
        a
    }

    fn union(&mut self, a: u32, b: u32) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.parent[ra.max(rb) as usize] = ra.min(rb);
        }
    }
}

fn splat_arcs(
    lat: &Lattice,
    data: &[AtomicU16],
    s: &Spheres,
    arcs: &[Arc],
    keep: &Keep,
    progress: &dyn Progress,
) {
    arcs.par_iter()
        .zip(&keep.arcs)
        .filter(|(_, kept)| **kept)
        .for_each(|(arc, _)| {
            if progress.is_cancelled() {
                return;
            }
            let Some(circle) = Circle::of(s, arc.i as usize, arc.j as usize) else {
                return;
            };
            splat_arc(lat, data, &circle, arc.phase, &arc.gap);
            // Each vertex (i < j < k) is splatted once, from the circle of i and j, onto the
            // cone spanned by the directions to the three centres, where it is the nearest
            // point of the solvent region.
            for (phi, k) in [(arc.gap.from, arc.gap.k_from), (arc.gap.to, arc.gap.k_to)] {
                if k == NONE || k <= arc.j {
                    continue;
                }
                let v = circle.at(arc.phase + phi);
                let [m0, m1, m2] = [arc.i, arc.j, k].map(|t| sub(s.c[t as usize], v));
                let det = dot(m0, cross(m1, m2));
                let sign = if det < 0.0 { -1.0 } else { 1.0 };
                let rows = [cross(m1, m2), cross(m2, m0), cross(m0, m1)].map(|r| scale(r, sign));
                let flat = det.abs() < 1e-9;
                lat.for_ball(v, lat.cap, |idx, x| {
                    if data[idx].load(Relaxed) == EXT {
                        return;
                    }
                    let y = sub(x, v);
                    if flat || rows.iter().all(|r| dot(*r, y) >= -1e-9) {
                        lower(&data[idx], lat.quantize(norm(y)));
                    }
                });
            }
        });
}

fn splat_arc(lat: &Lattice, data: &[AtomicU16], circle: &Circle, phase: f64, g: &Gap) {
    // Bounding box of the arc from samples at most 0.25 rad apart (the chord sag is then
    // below 0.008 ρ), grown by the splat radius.
    let steps = ((g.to - g.from) / 0.25).ceil().max(1.0) as usize;
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for t in 0..=steps {
        let p = circle.at(phase + g.from + (g.to - g.from) * t as f64 / steps as f64);
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    let grow = lat.cap + 0.01 * circle.rho;
    let lo = lo.map(|v| v - grow);
    let hi = hi.map(|v| v + grow);
    let full = g.to - g.from >= TAU;
    lat.for_box(lo, hi, |idx, x| {
        let cur = data[idx].load(Relaxed);
        if cur == EXT {
            return;
        }
        let v = sub(x, circle.center);
        let z = dot(v, circle.axis);
        let radial = sub(v, scale(circle.axis, z));
        let rr = norm(radial);
        let d = ((rr - circle.rho) * (rr - circle.rho) + z * z).sqrt();
        if d >= lat.cap || !circle.in_wedge(rr, z) {
            return;
        }
        let q = lat.quantize(d);
        if q >= cur {
            return;
        }
        if !full && rr > 1e-12 {
            let phi =
                (dot(radial, circle.e2).atan2(dot(radial, circle.e1)) - phase).rem_euclid(TAU);
            if phi < g.from || phi > g.to {
                return;
            }
        }
        lower(&data[idx], q);
    });
}

fn splat_patches(
    lat: &Lattice,
    data: &[AtomicU16],
    s: &Spheres,
    arcs: &[Arc],
    keep: &Keep,
    progress: &dyn Progress,
) {
    (0..s.c.len())
        .into_par_iter()
        .filter(|&i| s.nb.active[i] && keep.spheres[i] != DROPPED)
        .for_each_init(Vec::new, |near: &mut Vec<(V3, f64, f64)>, i| {
            if progress.is_cancelled() {
                return;
            }
            // Patches of a sphere lining both the outside and a cavity take the class of
            // the nearest arc on the sphere: any path on the sphere from a point to an arc
            // of another patch crosses the boundary of its own patch first.
            let bounds: Vec<(Circle, f64, Gap, bool)> = keep.mixed[i]
                .iter()
                .filter_map(|&a| {
                    let arc = &arcs[a as usize];
                    let circle = Circle::of(s, arc.i as usize, arc.j as usize)?;
                    Some((circle, arc.phase, arc.gap, keep.arcs[a as usize]))
                })
                .collect();
            let (a, r) = (s.c[i], s.big[i]);
            // Closest neighbours first: they bury most directions.
            near.clear();
            near.extend(s.nb.of(i).iter().map(|&k| {
                let k = k as usize;
                (s.c[k], s.big[k] * s.big[k] - EPS, dist2(a, s.c[k]))
            }));
            near.sort_unstable_by(|x, y| x.2.total_cmp(&y.2));
            lat.for_ball(a, r - 1e-9, |idx, x| {
                let d = norm(sub(x, a));
                if r - d >= lat.cap || d < 1e-12 || data[idx].load(Relaxed) == 0 {
                    return;
                }
                let u = scale(sub(x, a), 1.0 / d);
                let proj = add(a, scale(u, r));
                if near.iter().any(|&(b, r2, _)| dist2(proj, b) < r2) {
                    return;
                }
                let kept = bounds
                    .iter()
                    .map(|(c, phase, g, kept)| (c.arc_dist2(*phase, g, proj), *kept))
                    .min_by(|x, y| x.0.total_cmp(&y.0))
                    .is_none_or(|(_, kept)| kept);
                if kept {
                    lower(&data[idx], lat.quantize(r - d));
                }
            });
        });
}

/// Elements of both sorted lists.
fn intersect_sorted(a: &[u32], b: &[u32], out: &mut Vec<u32>) {
    out.clear();
    let (mut x, mut y) = (0, 0);
    while x < a.len() && y < b.len() {
        match a[x].cmp(&b[y]) {
            std::cmp::Ordering::Less => x += 1,
            std::cmp::Ordering::Greater => y += 1,
            std::cmp::Ordering::Equal => {
                out.push(a[x]);
                x += 1;
                y += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::NoProgress;

    fn params(h: f64) -> SesParams {
        SesParams {
            grid_spacing: h,
            ..Default::default()
        }
    }

    #[test]
    fn single_sphere_field_is_exact() {
        let g =
            SesGrid::from_spheres(&[[0.3, -0.2, 0.1]], &[1.7], &params(0.4), &NoProgress).unwrap();
        let mut checked = 0;
        for k in 0..g.dims[2] {
            for j in 0..g.dims[1] {
                for i in 0..g.dims[0] {
                    let x = g.point(i, j, k);
                    let exact = norm(sub(x, [0.3, -0.2, 0.1])) - 1.7;
                    if exact > -g.band() + 1e-3 && exact < g.probe() - 1e-3 {
                        assert!((g.node(i, j, k) - exact).abs() < 1e-4);
                        checked += 1;
                    }
                }
            }
        }
        assert!(checked > 1000);
    }

    #[test]
    fn gaps_handle_wraparound() {
        let mut gaps = Vec::new();
        // [5.5, 5.5 + 1.2) wraps past 2π and covers [0, 0.417).
        let mut b = vec![(0.0, 0.3, 7), (5.5, 1.2, 8), (2.0, 1.0, 9)];
        let phase = gaps_of(&mut b, &mut gaps);
        assert_eq!(phase, 0.0);
        assert_eq!(gaps.len(), 2);
        assert!((gaps[0].from - (5.5 + 1.2 - TAU)).abs() < 1e-12 && gaps[0].k_from == 8);
        assert!((gaps[0].to - 2.0).abs() < 1e-12 && gaps[0].k_to == 9);
        assert!((gaps[1].from - 3.0).abs() < 1e-12 && gaps[1].k_from == 9);
        assert!((gaps[1].to - 5.5).abs() < 1e-12 && gaps[1].k_to == 8);
    }

    #[test]
    fn circle_burial_matches_brute_force() {
        let circle = Circle::new([0.0; 3], 3.0, [3.0, 0.5, 0.0], 3.2).unwrap();
        let (b, rb) = ([1.0, 2.5, 1.0], 2.4);
        let (s, l) = circle.buried_by(b, rb).unwrap().unwrap();
        for t in 0..720 {
            let phi = t as f64 / 720.0 * TAU;
            let inside = dist2(circle.at(phi), b) < rb * rb;
            let rel = (phi - s).rem_euclid(TAU);
            let near_edge = rel.min((rel - l).abs()).min(TAU - rel) < 1e-6;
            if !near_edge {
                assert_eq!(inside, rel < l, "phi = {phi}");
            }
        }
    }
}
