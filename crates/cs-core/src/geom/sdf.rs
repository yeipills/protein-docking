//! Exact signed distance to a triangle mesh.
//!
//! The sign comes from angle-weighted pseudonormals (Bærentzen & Aanæs, 2005): the normal of
//! the closest feature (face, edge or vertex) decides inside/outside. Unlike ray parity, this
//! is robust to small holes and needs a single nearest-triangle query per point.

use std::collections::HashMap;

use super::bvh::{Aabb, Bvh};
use super::vec3::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Feature {
    Face,
    /// Edge `k` joins corners `k` and `(k + 1) % 3`.
    Edge(u8),
    Vertex(u8),
}

pub struct SignedDistance {
    vertices: Vec<V3>,
    faces: Vec<[u32; 3]>,
    face_normals: Vec<V3>,
    edge_normals: Vec<[V3; 3]>,
    vertex_normals: Vec<V3>,
    bvh: Bvh,
}

impl SignedDistance {
    /// Builds the structure. Faces must be consistently oriented with normals pointing out of
    /// the solid (see [`orient_outward`]). Degenerate faces are ignored.
    pub fn new(vertices: Vec<V3>, faces: &[[u32; 3]]) -> Self {
        let faces: Vec<[u32; 3]> = faces
            .iter()
            .copied()
            .filter(|f| {
                let [a, b, c] = f.map(|i| vertices[i as usize]);
                norm2(cross(sub(b, a), sub(c, a))) > 1e-24
            })
            .collect();

        let face_normals: Vec<V3> = faces
            .iter()
            .map(|f| {
                let [a, b, c] = f.map(|i| vertices[i as usize]);
                normalize(cross(sub(b, a), sub(c, a)))
            })
            .collect();

        let mut vertex_normals = vec![[0.0; 3]; vertices.len()];
        let mut edge_acc: HashMap<(u32, u32), V3> = HashMap::with_capacity(faces.len() * 2);
        for (f, n) in faces.iter().zip(&face_normals) {
            for k in 0..3 {
                let (i, j, l) = (f[k], f[(k + 1) % 3], f[(k + 2) % 3]);
                let p = vertices[i as usize];
                let e1 = normalize(sub(vertices[j as usize], p));
                let e2 = normalize(sub(vertices[l as usize], p));
                let angle = dot(e1, e2).clamp(-1.0, 1.0).acos();
                vertex_normals[i as usize] = add(vertex_normals[i as usize], scale(*n, angle));
                let key = (i.min(j), i.max(j));
                let e = edge_acc.entry(key).or_insert([0.0; 3]);
                *e = add(*e, *n);
            }
        }
        let vertex_normals = vertex_normals.into_iter().map(normalize).collect();
        let edge_normals = faces
            .iter()
            .map(|f| {
                std::array::from_fn(|k| {
                    let (i, j) = (f[k], f[(k + 1) % 3]);
                    normalize(edge_acc[&(i.min(j), i.max(j))])
                })
            })
            .collect();

        let boxes = faces
            .iter()
            .map(|f| Aabb::from_points(&f.map(|i| vertices[i as usize])))
            .collect();
        let bvh = Bvh::build(boxes);
        Self {
            vertices,
            faces,
            face_normals,
            edge_normals,
            vertex_normals,
            bvh,
        }
    }

    /// Signed distance from `p` to the surface: negative inside, positive outside.
    pub fn signed_distance(&self, p: V3) -> f64 {
        self.signed_distance_hinted(p, &mut None)
    }

    /// Signed distance, using and updating `hint` (the closest face of the previous query).
    /// Much faster for successive nearby points such as samples along a ray.
    pub fn signed_distance_hinted(&self, p: V3, hint: &mut Option<u32>) -> f64 {
        let mut best_feature = Feature::Face;
        let mut best_point = [0.0; 3];
        let found = self.bvh.nearest_with_hint(p, *hint, |fi, best| {
            let tri = self.faces[fi as usize].map(|i| self.vertices[i as usize]);
            let (q, feature) = closest_point_on_triangle(p, tri);
            let d = dist2(p, q);
            if d < best {
                best_feature = feature;
                best_point = q;
            }
            d
        });
        let Some((fi, d2)) = found else {
            return f64::INFINITY;
        };
        *hint = Some(fi);
        let fi = fi as usize;
        let pn = match best_feature {
            Feature::Face => self.face_normals[fi],
            Feature::Edge(k) => self.edge_normals[fi][k as usize],
            Feature::Vertex(k) => self.vertex_normals[self.faces[fi][k as usize] as usize],
        };
        let d = d2.sqrt();
        if dot(sub(p, best_point), pn) < 0.0 {
            -d
        } else {
            d
        }
    }
}

/// Flips every face if most faces point against the given per-vertex outward normals
/// (MSMS provides them). Returns true if the faces were flipped.
pub fn orient_outward(vertices: &[V3], vertex_normals: &[V3], faces: &mut [[u32; 3]]) -> bool {
    let mut agree = 0i64;
    for f in faces.iter() {
        let [a, b, c] = f.map(|i| vertices[i as usize]);
        let n = cross(sub(b, a), sub(c, a));
        let vn = f
            .iter()
            .fold([0.0; 3], |acc, &i| add(acc, vertex_normals[i as usize]));
        agree += if dot(n, vn) >= 0.0 { 1 } else { -1 };
    }
    if agree < 0 {
        for f in faces.iter_mut() {
            f.swap(1, 2);
        }
        true
    } else {
        false
    }
}

/// Squared distance from `p` to triangle `tri`.
pub fn point_triangle_dist2(p: V3, tri: [V3; 3]) -> f64 {
    dist2(p, closest_point_on_triangle(p, tri).0)
}

/// Closest point on triangle `abc` to `p` (Ericson, Real-Time Collision Detection, 5.1.5),
/// together with the feature it lies on.
fn closest_point_on_triangle(p: V3, [a, b, c]: [V3; 3]) -> (V3, Feature) {
    let ab = sub(b, a);
    let ac = sub(c, a);
    let ap = sub(p, a);
    let d1 = dot(ab, ap);
    let d2 = dot(ac, ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return (a, Feature::Vertex(0));
    }
    let bp = sub(p, b);
    let d3 = dot(ab, bp);
    let d4 = dot(ac, bp);
    if d3 >= 0.0 && d4 <= d3 {
        return (b, Feature::Vertex(1));
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return (add(a, scale(ab, v)), Feature::Edge(0));
    }
    let cp = sub(p, c);
    let d5 = dot(ab, cp);
    let d6 = dot(ac, cp);
    if d6 >= 0.0 && d5 <= d6 {
        return (c, Feature::Vertex(2));
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return (add(a, scale(ac, w)), Feature::Edge(2));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return (add(b, scale(sub(c, b), w)), Feature::Edge(1));
    }
    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    (add(a, add(scale(ab, v), scale(ac, w))), Feature::Face)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unit cube centred at the origin, outward-facing triangles.
    fn cube() -> (Vec<V3>, Vec<[u32; 3]>) {
        let v: Vec<V3> = (0..8)
            .map(|i| {
                [
                    if i & 1 == 0 { -0.5 } else { 0.5 },
                    if i & 2 == 0 { -0.5 } else { 0.5 },
                    if i & 4 == 0 { -0.5 } else { 0.5 },
                ]
            })
            .collect();
        let quads = [
            [0, 2, 3, 1], // z-
            [4, 5, 7, 6], // z+
            [0, 1, 5, 4], // y-
            [2, 6, 7, 3], // y+
            [0, 4, 6, 2], // x-
            [1, 3, 7, 5], // x+
        ];
        let mut f = Vec::new();
        for q in quads {
            f.push([q[0], q[1], q[2]]);
            f.push([q[0], q[2], q[3]]);
        }
        (v, f)
    }

    #[test]
    fn cube_distances() {
        let (v, f) = cube();
        let sdf = SignedDistance::new(v, &f);
        let close = |a: f64, b: f64| (a - b).abs() < 1e-12;
        assert!(close(sdf.signed_distance([0.0, 0.0, 0.0]), -0.5));
        assert!(close(sdf.signed_distance([0.0, 0.0, 1.5]), 1.0));
        assert!(close(sdf.signed_distance([0.4, 0.0, 0.0]), -0.1));
        // Off a corner: distance to the vertex, sign from the vertex pseudonormal.
        let d = sdf.signed_distance([1.5, 1.5, 1.5]);
        assert!(close(d, 3f64.sqrt()));
        // Off an edge.
        assert!(close(sdf.signed_distance([1.5, 1.5, 0.0]), 2f64.sqrt()));
        assert!(sdf.signed_distance([0.49, 0.49, 0.49]) < 0.0);
    }

    #[test]
    fn orientation_is_fixed() {
        let (v, mut f) = cube();
        for t in f.iter_mut() {
            t.swap(1, 2);
        }
        let normals: Vec<V3> = v.iter().map(|p| normalize(*p)).collect();
        assert!(orient_outward(&v, &normals, &mut f));
        let sdf = SignedDistance::new(v, &f);
        assert!(sdf.signed_distance([0.0, 0.0, 0.0]) < 0.0);
    }
}
