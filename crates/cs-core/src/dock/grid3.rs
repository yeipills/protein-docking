//! Dense scalar fields on a regular 3D grid and the exact Euclidean distance transform.
//!
//! Docking evaluates fields (signed distances, contact potentials) hundreds of millions of
//! times, so they are tabulated once on a grid and sampled by trilinear interpolation.

use rayon::prelude::*;

use crate::geom::vec3::*;

/// Scalar field sampled at the nodes `origin + h * (i, j, k)`; x varies fastest.
#[derive(Debug, Clone)]
pub struct Grid3 {
    pub origin: V3,
    pub h: f64,
    pub dims: [usize; 3],
    pub data: Vec<f32>,
}

impl Grid3 {
    /// Grid covering the box `[lo, hi]` with spacing `h`, every node set to `fill`.
    pub fn covering(lo: V3, hi: V3, h: f64, fill: f32) -> Grid3 {
        assert!(h > 0.0, "grid spacing must be positive");
        let dims: [usize; 3] =
            std::array::from_fn(|a| (((hi[a] - lo[a]) / h).ceil().max(0.0) as usize + 2).max(2));
        Grid3 {
            origin: lo,
            h,
            dims,
            data: vec![fill; dims[0] * dims[1] * dims[2]],
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    #[inline]
    pub fn index(&self, i: usize, j: usize, k: usize) -> usize {
        (k * self.dims[1] + j) * self.dims[0] + i
    }

    #[inline]
    pub fn node(&self, i: usize, j: usize, k: usize) -> V3 {
        [
            self.origin[0] + self.h * i as f64,
            self.origin[1] + self.h * j as f64,
            self.origin[2] + self.h * k as f64,
        ]
    }

    /// Upper corner of the grid box.
    pub fn max_corner(&self) -> V3 {
        std::array::from_fn(|a| self.origin[a] + self.h * (self.dims[a] - 1) as f64)
    }

    /// Inclusive node index range whose nodes may lie within `r` of `c` (clamped to the grid).
    pub fn node_range(&self, c: V3, r: f64) -> Option<[(usize, usize); 3]> {
        let mut out = [(0, 0); 3];
        for a in 0..3 {
            let lo = ((c[a] - r - self.origin[a]) / self.h).ceil();
            let hi = ((c[a] + r - self.origin[a]) / self.h).floor();
            let max = (self.dims[a] - 1) as f64;
            if hi < 0.0 || lo > max || lo > hi {
                return None;
            }
            out[a] = (lo.max(0.0) as usize, hi.min(max) as usize);
        }
        Some(out)
    }

    /// Trilinear interpolation. Outside the box the value of the nearest boundary point is
    /// returned plus the distance to the box, which keeps signed-distance fields positive (and
    /// roughly right) far from the molecule.
    #[inline]
    pub fn sample(&self, p: V3) -> f64 {
        let mut f = [0.0f64; 3];
        let mut i0 = [0usize; 3];
        let mut outside2 = 0.0;
        for a in 0..3 {
            let max = (self.dims[a] - 1) as f64;
            let mut u = (p[a] - self.origin[a]) / self.h;
            if u < 0.0 {
                outside2 += (u * self.h).powi(2);
                u = 0.0;
            } else if u > max {
                outside2 += ((u - max) * self.h).powi(2);
                u = max;
            }
            let fl = u.floor().min(max - 1.0);
            i0[a] = fl as usize;
            f[a] = u - fl;
        }
        let (nx, nxy) = (self.dims[0], self.dims[0] * self.dims[1]);
        let base = i0[2] * nxy + i0[1] * nx + i0[0];
        let d = &self.data;
        let c = |o: usize| d[base + o] as f64;
        let x00 = c(0) + (c(1) - c(0)) * f[0];
        let x10 = c(nx) + (c(nx + 1) - c(nx)) * f[0];
        let x01 = c(nxy) + (c(nxy + 1) - c(nxy)) * f[0];
        let x11 = c(nxy + nx) + (c(nxy + nx + 1) - c(nxy + nx)) * f[0];
        let y0 = x00 + (x10 - x00) * f[1];
        let y1 = x01 + (x11 - x01) * f[1];
        let v = y0 + (y1 - y0) * f[2];
        if outside2 > 0.0 {
            v + outside2.sqrt()
        } else {
            v
        }
    }

    /// Trilinear sample, 0 outside the box (for fields that vanish far from the molecule).
    #[inline]
    pub fn sample_or_zero(&self, p: V3) -> f64 {
        let (lo, hi) = (self.origin, self.max_corner());
        if (0..3).any(|a| p[a] < lo[a] || p[a] > hi[a]) {
            0.0
        } else {
            self.sample(p)
        }
    }

    /// Same geometry without data (for index queries while the data is mutably borrowed).
    pub fn shape_only(&self) -> Grid3 {
        Grid3 {
            origin: self.origin,
            h: self.h,
            dims: self.dims,
            data: Vec::new(),
        }
    }

    /// Gradient of the trilinear field by central differences (step `h / 2`).
    pub fn gradient(&self, p: V3) -> V3 {
        let e = 0.5 * self.h;
        std::array::from_fn(|a| {
            let mut lo = p;
            let mut hi = p;
            lo[a] -= e;
            hi[a] += e;
            (self.sample(hi) - self.sample(lo)) / (2.0 * e)
        })
    }
}

/// Large finite stand-in for "no feature" in squared voxel units (keeps the arithmetic of the
/// lower-envelope construction free of `inf - inf`).
const FAR: f32 = 1e12;

/// Exact Euclidean distance transform (Felzenszwalb & Huttenlocher, 2012).
///
/// Returns, for every node, the distance in Å to the nearest node where `feature` is true
/// (0 on features). Without any feature every distance is huge (≥ 1e6 Å).
pub fn distance_transform(dims: [usize; 3], h: f64, feature: &[bool]) -> Vec<f32> {
    let [nx, ny, nz] = dims;
    assert_eq!(feature.len(), nx * ny * nz);
    let mut d: Vec<f32> = feature.iter().map(|&f| if f { 0.0 } else { FAR }).collect();

    // Pass along x: rows are contiguous.
    d.par_chunks_mut(nx).for_each(|row| {
        let mut s = Scratch::new(nx);
        s.f[..nx].copy_from_slice(row);
        s.run(nx);
        row.copy_from_slice(&s.out[..nx]);
    });
    // Pass along y: each z plane is a contiguous block.
    d.par_chunks_mut(nx * ny).for_each(|plane| {
        let mut s = Scratch::new(ny);
        for i in 0..nx {
            for j in 0..ny {
                s.f[j] = plane[j * nx + i];
            }
            s.run(ny);
            for j in 0..ny {
                plane[j * nx + i] = s.out[j];
            }
        }
    });
    // Pass along z: gather each (i, j) column per y row, then scatter back.
    let cols: Vec<Vec<f32>> = (0..ny)
        .into_par_iter()
        .map(|j| {
            let mut s = Scratch::new(nz);
            let mut out = vec![0.0f32; nx * nz];
            for i in 0..nx {
                for k in 0..nz {
                    s.f[k] = d[(k * ny + j) * nx + i];
                }
                s.run(nz);
                for k in 0..nz {
                    out[i * nz + k] = s.out[k];
                }
            }
            out
        })
        .collect();
    for (j, col) in cols.iter().enumerate() {
        for i in 0..nx {
            for k in 0..nz {
                d[(k * ny + j) * nx + i] = col[i * nz + k];
            }
        }
    }
    let h = h as f32;
    d.par_iter_mut().for_each(|v| *v = v.sqrt() * h);
    d
}

/// Buffers for the 1D squared-distance transform (lower envelope of parabolas).
struct Scratch {
    f: Vec<f32>,
    out: Vec<f32>,
    v: Vec<usize>,
    z: Vec<f32>,
}

impl Scratch {
    fn new(n: usize) -> Scratch {
        Scratch {
            f: vec![0.0; n],
            out: vec![0.0; n],
            v: vec![0; n],
            z: vec![0.0; n + 1],
        }
    }

    fn run(&mut self, n: usize) {
        let (f, v, z) = (&self.f, &mut self.v, &mut self.z);
        let mut k = 0usize;
        v[0] = 0;
        z[0] = f32::NEG_INFINITY;
        z[1] = f32::INFINITY;
        for q in 1..n {
            let fq = f[q] + (q * q) as f32;
            loop {
                let p = v[k];
                let s = (fq - (f[p] + (p * p) as f32)) / (2.0 * (q as f32 - p as f32));
                if s <= z[k] && k > 0 {
                    k -= 1;
                } else {
                    k += 1;
                    v[k] = q;
                    z[k] = s;
                    z[k + 1] = f32::INFINITY;
                    break;
                }
            }
        }
        k = 0;
        for q in 0..n {
            while z[k + 1] < q as f32 {
                k += 1;
            }
            let p = v[k];
            let dq = q as f32 - p as f32;
            self.out[q] = (dq * dq + f[p]).min(FAR);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trilinear_reproduces_linear_fields() {
        let mut g = Grid3::covering([-1.0, -2.0, 0.5], [3.0, 2.0, 4.0], 0.5, 0.0);
        for k in 0..g.dims[2] {
            for j in 0..g.dims[1] {
                for i in 0..g.dims[0] {
                    let p = g.node(i, j, k);
                    let idx = g.index(i, j, k);
                    g.data[idx] = (2.0 * p[0] - p[1] + 0.5 * p[2]) as f32;
                }
            }
        }
        for p in [[0.3, 0.1, 1.7], [-0.7, 1.7, 3.6], [2.2, -1.3, 0.8]] {
            let v = g.sample(p);
            assert!((v - (2.0 * p[0] - p[1] + 0.5 * p[2])).abs() < 1e-5, "{v}");
            let gr = g.gradient(p);
            assert!((gr[0] - 2.0).abs() < 1e-4 && (gr[1] + 1.0).abs() < 1e-4);
        }
        // Outside the box: boundary value plus the distance to the box.
        let far = g.sample([10.0, 0.0, 1.0]);
        let edge = g.sample([g.max_corner()[0], 0.0, 1.0]);
        assert!((far - edge - (10.0 - g.max_corner()[0])).abs() < 1e-6);
    }

    #[test]
    fn distance_transform_matches_brute_force() {
        let dims = [13, 9, 11];
        let n = dims[0] * dims[1] * dims[2];
        let mut rng = crate::geom::transform::Rng::new(3);
        let feature: Vec<bool> = (0..n).map(|_| rng.next_f64() < 0.02).collect();
        let d = distance_transform(dims, 0.5, &feature);
        let coords = |idx: usize| {
            let i = idx % dims[0];
            let j = (idx / dims[0]) % dims[1];
            let k = idx / (dims[0] * dims[1]);
            [i as f64, j as f64, k as f64]
        };
        for idx in 0..n {
            let p = coords(idx);
            let best = (0..n)
                .filter(|&f| feature[f])
                .map(|f| dist2(coords(f), p).sqrt() * 0.5)
                .fold(f64::INFINITY, f64::min);
            assert!((d[idx] as f64 - best).abs() < 1e-4, "{} vs {best}", d[idx]);
        }
    }
}
