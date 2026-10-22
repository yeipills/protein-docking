//! Bit-faithful port of trimesh's pure-Python `RayMeshIntersector.intersects_any`
//! (`trimesh.ray.ray_triangle`, no Embree), as used by the original Script03.
//!
//! Every floating-point step mirrors the NumPy expression it replaces, including the
//! broad phase (ray AABB clipped to the mesh bounds, queried against triangle boxes with
//! inclusive overlap like the R-tree) and the tolerances (`1e-5`, `1e-13`, `-1e-6`).

use crate::geom::bvh::{Aabb, Bvh};
use crate::geom::vec3::V3;
use crate::io::stl::Stl;

pub struct TrimeshRay {
    triangles: Vec<[V3; 3]>,
    normals: Vec<V3>,
    bounds: Aabb,
    bvh: Bvh,
}

/// `trimesh.util.diagonal_dot` for one row: `np.dot(a * b, [1, 1, 1])`.
#[inline]
fn ddot(a: V3, b: V3) -> f64 {
    let p = [a[0] * b[0], a[1] * b[1], a[2] * b[2]];
    p[0] + p[1] + p[2]
}

#[inline]
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

impl TrimeshRay {
    pub fn new(stl: &Stl) -> Self {
        let triangles = stl.triangles.clone();
        let normals = face_normals(&triangles, &stl.normals);
        let boxes: Vec<Aabb> = triangles.iter().map(|t| Aabb::from_points(t)).collect();
        let mut bounds = Aabb::EMPTY;
        for b in &boxes {
            bounds.grow(b);
        }
        let bvh = Bvh::build(boxes);
        Self {
            triangles,
            normals,
            bounds,
            bvh,
        }
    }

    /// `intersects_any(ray_origins=[origin], ray_directions=[direction])[0]`.
    pub fn intersects_any(&self, origin: V3, direction: V3) -> bool {
        let query = self.ray_bounds(origin, direction);
        let mut hit = false;
        self.bvh.for_each_overlap(&query, |ti| {
            hit = self.hits_triangle(ti as usize, origin, direction);
            !hit
        });
        hit
    }

    /// `trimesh.ray.ray_triangle.ray_bounds` with `buffer_dist = 1e-5`.
    fn ray_bounds(&self, o: V3, d: V3) -> Aabb {
        const BUFFER: f64 = 1e-5;
        // np.abs(d).argmax(): first index of the maximum.
        let mut axis = 0;
        for k in 1..3 {
            if d[k].abs() > d[axis].abs() {
                axis = k;
            }
        }
        let bound = [self.bounds.min[axis], self.bounds.max[axis]];
        let mut t = [0.0f64; 2];
        if d[axis] != 0.0 {
            for i in 0..2 {
                t[i] = (bound[i] - o[axis]) / d[axis];
            }
        }
        for ti in &mut t {
            if *ti < BUFFER {
                *ti = BUFFER;
            }
        }
        let on_a = [d[0] * t[0] + o[0], d[1] * t[0] + o[1], d[2] * t[0] + o[2]];
        let on_b = [d[0] * t[1] + o[0], d[1] * t[1] + o[1], d[2] * t[1] + o[2]];
        let mut b = Aabb::EMPTY;
        for k in 0..3 {
            b.min[k] = on_a[k].min(on_b[k]) - BUFFER;
            b.max[k] = on_a[k].max(on_b[k]) + BUFFER;
        }
        b
    }

    /// Narrow phase of `ray_triangle_id` for one (ray, triangle) candidate pair.
    #[allow(clippy::neg_cmp_op_on_partial_ord)] // NaN must count as invalid, as in NumPy
    fn hits_triangle(&self, ti: usize, o: V3, d: V3) -> bool {
        let tri = &self.triangles[ti];
        let n = self.normals[ti];

        // intersections.planes_lines
        let origin_vector = sub(tri[0], o);
        let proj_ori = ddot(origin_vector, n);
        let proj_dir = ddot(d, n);
        if !(proj_dir.abs() > 1e-5) {
            return false;
        }
        let distance = proj_ori / proj_dir;
        let loc = [
            d[0] * distance + o[0],
            d[1] * distance + o[1],
            d[2] * distance + o[2],
        ];

        // triangles.points_to_barycentric, method="cramer"
        let e0 = sub(tri[1], tri[0]);
        let e1 = sub(tri[2], tri[0]);
        let w = sub(loc, tri[0]);
        let dot00 = ddot(e0, e0);
        let dot01 = ddot(e0, e1);
        let dot02 = ddot(e0, w);
        let dot11 = ddot(e1, e1);
        let dot12 = ddot(e1, w);
        let inv = 1.0 / (dot00 * dot11 - dot01 * dot01);
        let b2 = (dot00 * dot12 - dot01 * dot02) * inv;
        let b1 = (dot11 * dot02 - dot01 * dot12) * inv;
        let b0 = 1.0 - b1 - b2;
        const ZERO: f64 = 1e-13;
        let inside = [b0, b1, b2].iter().all(|&b| b > -ZERO && b < 1.0 + ZERO);
        if !inside {
            return false;
        }

        // forward test against the ray direction
        ddot(sub(loc, o), d) > -1e-6
    }
}

/// trimesh keeps the file's normals only if the first 20 match the computed ones
/// (`np.allclose`); otherwise it recomputes all of them, zeroing degenerate faces.
fn face_normals(triangles: &[[V3; 3]], stored: &[V3]) -> Vec<V3> {
    let computed = |t: &[V3; 3]| -> V3 {
        // triangles.cross: np.cross(diff[0], diff[1]) with diff = np.diff(t, axis=0)
        let a = sub(t[1], t[0]);
        let b = sub(t[2], t[1]);
        let c = [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ];
        // util.unitize with threshold TOL_ZERO
        let norm = ddot(c, c).sqrt();
        if norm > 1e-13 {
            let r = 1.0 / norm;
            [c[0] * r, c[1] * r, c[2] * r]
        } else {
            [0.0; 3]
        }
    };

    let ptp = {
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for n in stored {
            for &c in n {
                lo = lo.min(c);
                hi = hi.max(c);
            }
        }
        hi - lo
    };
    let usable = stored.len() == triangles.len() && ptp.is_finite() && ptp >= 1e-8 && {
        triangles.iter().zip(stored).take(20).all(|(t, s)| {
            let c = computed(t);
            (0..3).all(|k| (c[k] - s[k]).abs() <= 1e-8 + 1e-5 * s[k].abs())
        })
    };
    if usable {
        stored.to_vec()
    } else {
        triangles.iter().map(computed).collect()
    }
}
