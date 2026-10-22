//! Marching cubes with per-face ambiguity resolution, which makes the output watertight
//! and 2-manifold by construction.
//!
//! Instead of a 256-case table (the classic one leaves cracks where two cubes resolve a
//! shared ambiguous face differently), each cube traces its contour on its six faces:
//! every face contributes directed segments between its sign-changing edges, keeping the
//! outside (positive) corners on the left when the face is seen from outside the cube. An
//! ambiguous face (signs `+ - + -`) is resolved with the asymptotic decider, computed from
//! the four shared corner values in an order-independent way, so both cubes agree. Each
//! crossing edge lies on two faces of the cube and starts one segment and ends another, so
//! the segments close into vertex-disjoint loops, which are triangulated in loop order.
//! Neighbouring cubes walk their shared face segment in opposite directions, so the mesh is
//! consistently oriented, with normals towards positive values.

use rayon::prelude::*;

use super::field::{DEEP, EXT, SesGrid};
use crate::error::Result;
use crate::geom::vec3::*;
use crate::mesh::Mesh;
use crate::progress::{Progress, check};

/// Corner `c` of a cube has offset `(c & 1, (c >> 1) & 1, (c >> 2) & 1)`.
const CORNER: [[usize; 3]; 8] = [
    [0, 0, 0],
    [1, 0, 0],
    [0, 1, 0],
    [1, 1, 0],
    [0, 0, 1],
    [1, 0, 1],
    [0, 1, 1],
    [1, 1, 1],
];

/// Face corners, counter-clockwise seen from outside the cube.
const FACES: [[usize; 4]; 6] = [
    [0, 2, 3, 1], // z = 0
    [4, 5, 7, 6], // z = 1
    [0, 1, 5, 4], // y = 0
    [2, 6, 7, 3], // y = 1
    [0, 4, 6, 2], // x = 0
    [1, 3, 7, 5], // x = 1
];

/// Cube edge joining corners `a` and `b` (they differ in one bit, the axis).
fn edge_of(a: usize, b: usize) -> usize {
    let (a, b) = (a.min(b), a.max(b));
    let axis = (b - a).trailing_zeros() as usize;
    // Four edges per axis, numbered by the two remaining corner bits of `a`.
    let rest = match axis {
        0 => a >> 1,
        1 => (a & 1) | ((a >> 2) << 1),
        _ => a & 3,
    };
    axis * 4 + rest
}

/// Lower corner of edge `e`, and its axis.
fn edge_corner(e: usize) -> (usize, usize) {
    let (axis, rest) = (e / 4, e % 4);
    let c = match axis {
        0 => rest << 1,
        1 => (rest & 1) | ((rest >> 1) << 2),
        _ => rest,
    };
    (c, axis)
}

/// Bit mask of the edges of each face.
const FACE_EDGES: [u16; 6] = {
    let mut m = [0u16; 6];
    let mut f = 0;
    while f < 6 {
        let mut t = 0;
        while t < 4 {
            let (a, b) = (FACES[f][t], FACES[f][(t + 1) % 4]);
            let (lo, hi) = if a < b { (a, b) } else { (b, a) };
            let axis = (hi - lo).trailing_zeros() as usize;
            let rest = match axis {
                0 => lo >> 1,
                1 => (lo & 1) | ((lo >> 2) << 1),
                _ => lo & 3,
            };
            m[f] |= 1 << (axis * 4 + rest);
            t += 1;
        }
        f += 1;
    }
    m
};

fn same_face(a: usize, b: usize) -> bool {
    FACE_EDGES.iter().any(|m| (m >> a) & (m >> b) & 1 == 1)
}

/// Global id of the grid edge starting at node `idx` along `axis`.
#[inline]
fn edge_id(idx: usize, axis: usize) -> u64 {
    idx as u64 * 3 + axis as u64
}

/// Ids of loop-centre vertices: flag bit, then cube index and loop number (at most 4).
const CENTER_FLAG: u64 = 1 << 63;

/// Vertices closer than this fraction of an edge to a node are pulled in, so vertices on
/// different edges never coincide (no zero-area triangles).
const T_MIN: f64 = 1e-3;

#[inline]
fn crossing(g0: f64, g1: f64) -> f64 {
    (g0 / (g0 - g1)).clamp(T_MIN, 1.0 - T_MIN)
}

/// Zero crossing on the edge from node `idx` to `idx + stride` (`at` is the node's
/// coordinate along that axis), as a fraction of the edge.
///
/// Linear interpolation puts vertices on the concave side of a curved surface by about
/// `h²/(8R)`; the cubic through the two nodes beyond the edge removes most of that, as long
/// as all four values are exact (not clamped).
fn edge_root(grid: &SesGrid, idx: usize, axis: usize, at: usize) -> f64 {
    let stride = grid.dims[..axis].iter().product::<usize>();
    let (b, c) = (grid.value(idx), grid.value(idx + stride));
    let linear = crossing(b, c);
    if at == 0 || at + 2 >= grid.dims[axis] {
        return linear;
    }
    let qs = [
        grid.data[idx - stride],
        grid.data[idx],
        grid.data[idx + stride],
        grid.data[idx + 2 * stride],
    ];
    if qs.iter().any(|&q| q == EXT || q == DEEP) {
        return linear;
    }
    let (a, d) = (grid.decode(qs[0]), grid.decode(qs[3]));
    // Lagrange cubic through t = -1, 0, 1, 2.
    let c1 = -a / 3.0 - b / 2.0 + c - d / 6.0;
    let c2 = a / 2.0 - b + c / 2.0;
    let c3 = (-a + 3.0 * b - 3.0 * c + d) / 6.0;
    let p = |t: f64| b + t * (c1 + t * (c2 + t * c3));
    let dp = |t: f64| c1 + t * (2.0 * c2 + t * 3.0 * c3);
    // Newton safeguarded by bisection on [0, 1], where p changes sign.
    let (mut lo, mut hi) = if b > 0.0 { (0.0, 1.0) } else { (1.0, 0.0) }; // p(lo) > 0
    let mut t = linear;
    for _ in 0..12 {
        let v = p(t);
        if v > 0.0 {
            lo = t;
        } else {
            hi = t;
        }
        let step = t - v / dp(t);
        t = if step.is_finite() && step > lo.min(hi) && step < lo.max(hi) {
            step
        } else {
            0.5 * (lo + hi)
        };
        if v.abs() < 1e-9 {
            break;
        }
    }
    t.clamp(T_MIN, 1.0 - T_MIN)
}

/// Vertex of a cube's triangulation: on a crossing edge, or at the centre of a loop that
/// could not be ear-clipped safely.
#[derive(Debug, Clone, Copy)]
enum Vert {
    Edge(usize),
    Center(usize),
}

/// Triangles (as vertex ids) and loop centres produced by one z-slab of cubes.
type Slab = (Vec<[u64; 3]>, Vec<(u64, V3)>);

pub(crate) fn extract(grid: &SesGrid, progress: &dyn Progress) -> Result<Mesh> {
    let [nx, ny, nz] = grid.dims;
    let strides = [1, nx, nx * ny];
    let offsets: [usize; 8] =
        CORNER.map(|o| o[0] * strides[0] + o[1] * strides[1] + o[2] * strides[2]);
    let h = grid.spacing;
    let coords = |idx: usize| [idx % nx, (idx / nx) % ny, idx / (nx * ny)];

    let slabs: Vec<Slab> = (0..nz - 1)
        .into_par_iter()
        .map(|k| {
            let mut tris = Vec::new();
            let mut centers = Vec::new();
            if progress.is_cancelled() {
                return (tris, centers);
            }
            let (mut local, mut local_centers) = (Vec::new(), Vec::new());
            for j in 0..ny - 1 {
                for i in 0..nx - 1 {
                    let base = grid.index(i, j, k);
                    let g: [f64; 8] = offsets.map(|o| grid.value(base + o));
                    let inside = g.iter().filter(|&&v| v <= 0.0).count();
                    if inside == 0 || inside == 8 {
                        continue;
                    }
                    local.clear();
                    local_centers.clear();
                    cube(&g, h, &mut local, &mut local_centers);
                    let id = |v: Vert| match v {
                        Vert::Edge(e) => {
                            let (c, axis) = edge_corner(e);
                            edge_id(base + offsets[c], axis)
                        }
                        Vert::Center(n) => CENTER_FLAG | (base as u64 * 4 + n as u64),
                    };
                    tris.extend(local.iter().map(|t| t.map(id)));
                    let origin = grid.point(i, j, k);
                    for (n, &p) in local_centers.iter().enumerate() {
                        centers.push((id(Vert::Center(n)), add(origin, p)));
                    }
                }
            }
            (tris, centers)
        })
        .collect();
    check(progress)?;
    progress.report("ses_mesh", 0.9);
    let tris: Vec<[u64; 3]> = slabs.iter().flat_map(|s| s.0.iter().copied()).collect();
    // Already sorted by id: slabs, rows and cubes are visited in index order.
    let centers: Vec<(u64, V3)> = slabs.iter().flat_map(|s| s.1.iter().copied()).collect();
    drop(slabs);

    let mut ids: Vec<u64> = tris.iter().flatten().copied().collect();
    ids.par_sort_unstable();
    ids.dedup();
    let faces: Vec<[u32; 3]> = tris
        .par_iter()
        .map(|t| t.map(|id| ids.binary_search(&id).unwrap() as u32))
        .collect();

    // Central differences; the surface never comes near the grid boundary (it is solvent).
    let gradient = |idx: usize| -> V3 {
        let p = coords(idx);
        std::array::from_fn(|a| {
            let lo = if p[a] > 0 { idx - strides[a] } else { idx };
            let hi = if p[a] + 1 < grid.dims[a] {
                idx + strides[a]
            } else {
                idx
            };
            grid.value(hi) - grid.value(lo)
        })
    };
    let (vertices, normals): (Vec<V3>, Vec<V3>) = ids
        .par_iter()
        .map(|&id| {
            if id & CENTER_FLAG != 0 {
                // Loop centre: trilinear blend of the cube's corner gradients.
                let c = centers[centers.binary_search_by_key(&id, |e| e.0).unwrap()].1;
                let base = ((id & !CENTER_FLAG) / 4) as usize;
                let [i, j, k] = coords(base);
                let t = sub(c, grid.point(i, j, k)).map(|v| (v / h).clamp(0.0, 1.0));
                let mut n = [0.0; 3];
                for (corner, o) in CORNER.iter().zip(offsets) {
                    let w: f64 = (0..3)
                        .map(|a| if corner[a] == 1 { t[a] } else { 1.0 - t[a] })
                        .product();
                    n = add(n, scale(gradient(base + o), w));
                }
                return (c, normalize(n));
            }
            let (idx, axis) = ((id / 3) as usize, (id % 3) as usize);
            let idx1 = idx + strides[axis];
            let [i, j, k] = coords(idx);
            let t = edge_root(grid, idx, axis, [i, j, k][axis]);
            let mut p = grid.point(i, j, k);
            p[axis] += t * h;
            let n = add(scale(gradient(idx), 1.0 - t), scale(gradient(idx1), t));
            (p, normalize(n))
        })
        .unzip();
    Ok(Mesh {
        vertices,
        normals,
        faces,
    })
}

/// Triangles of one cube with corner values `g` and side `h`. Loop centres, when needed,
/// go to `centers` (positions relative to corner 0).
fn cube(g: &[f64; 8], h: f64, out: &mut Vec<[Vert; 3]>, centers: &mut Vec<V3>) {
    let pos = |c: usize| g[c] > 0.0;
    // next[e] = edge where the contour goes after crossing edge e.
    let mut next = [usize::MAX; 12];
    for face in FACES {
        let mut cuts = [(0usize, false); 4];
        let mut n = 0;
        for t in 0..4 {
            let (a, b) = (face[t], face[(t + 1) % 4]);
            if pos(a) != pos(b) {
                // `true` for a + to - edge (a segment starts there).
                cuts[n] = (edge_of(a, b), pos(a));
                n += 1;
            }
        }
        match n {
            0 => {}
            2 => {
                let (s, e) = if cuts[0].1 { (0, 1) } else { (1, 0) };
                next[cuts[s].0] = cuts[e].0;
            }
            _ => {
                // Asymptotic decider: the saddle is positive iff the product of the positive
                // diagonal exceeds that of the negative one (same result in both cubes).
                let mut p = face.iter().filter(|&&c| pos(c)).map(|&c| g[c]);
                let mut m = face.iter().filter(|&&c| !pos(c)).map(|&c| g[c]);
                let pp = p.next().unwrap_or(0.0) * p.next().unwrap_or(0.0);
                let mm = m.next().unwrap_or(0.0) * m.next().unwrap_or(0.0);
                let connect_pos = pp > mm;
                for t in 0..4 {
                    if cuts[t].1 {
                        // Positives joined: cut off the negative corner after this edge;
                        // otherwise cut off the positive corner before it.
                        let e = if connect_pos {
                            (t + 1) % 4
                        } else {
                            (t + 3) % 4
                        };
                        next[cuts[t].0] = cuts[e].0;
                    }
                }
            }
        }
    }

    let vertex = |e: usize| -> V3 {
        let (c, axis) = edge_corner(e);
        let t = crossing(g[c], g[c | (1 << axis)]);
        let mut p = CORNER[c].map(|v| v as f64 * h);
        p[axis] += t * h;
        p
    };
    let mut done = [false; 12];
    let mut ring: Vec<usize> = Vec::with_capacity(12);
    let mut pts: Vec<V3> = Vec::with_capacity(12);
    for start in 0..12 {
        if next[start] == usize::MAX || done[start] {
            continue;
        }
        ring.clear();
        let mut e = start;
        while !done[e] {
            done[e] = true;
            ring.push(e);
            e = next[e];
        }
        // Ear clipping by shortest diagonal keeps the loop order, hence the orientation. A
        // diagonal joining two vertices of one cube face could also be chosen by the cube
        // across that face (an edge with four triangles), so those are never used.
        pts.clear();
        pts.extend(ring.iter().map(|&e| vertex(e)));
        while ring.len() > 3 {
            let n = ring.len();
            let diagonal = |a: usize| ((a + n - 1) % n, (a + 1) % n);
            let best = (0..n)
                .filter(|&a| {
                    let (u, v) = diagonal(a);
                    !same_face(ring[u], ring[v])
                })
                .min_by(|&a, &b| {
                    let ((ua, va), (ub, vb)) = (diagonal(a), diagonal(b));
                    dist2(pts[ua], pts[va]).total_cmp(&dist2(pts[ub], pts[vb]))
                });
            let Some(best) = best else { break };
            let (u, v) = diagonal(best);
            out.push([
                Vert::Edge(ring[u]),
                Vert::Edge(ring[best]),
                Vert::Edge(ring[v]),
            ]);
            ring.remove(best);
            pts.remove(best);
        }
        if ring.len() == 3 {
            out.push([
                Vert::Edge(ring[0]),
                Vert::Edge(ring[1]),
                Vert::Edge(ring[2]),
            ]);
        } else {
            // No safe ear left: fan around the loop centre.
            let c = Vert::Center(centers.len());
            let w = 1.0 / pts.len() as f64;
            centers.push(pts.iter().fold([0.0; 3], |a, &p| add(a, scale(p, w))));
            for t in 0..ring.len() {
                let u = (t + 1) % ring.len();
                out.push([Vert::Edge(ring[t]), Vert::Edge(ring[u]), c]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_numbering_round_trips() {
        for e in 0..12 {
            let (c, axis) = edge_corner(e);
            assert_eq!(c & (1 << axis), 0);
            assert_eq!(edge_of(c, c | (1 << axis)), e);
        }
        assert!(FACE_EDGES.iter().all(|m| m.count_ones() == 4));
    }

    /// Every sign pattern gives loops through all crossing edges, and inside the cube each
    /// directed edge between two vertices is used at most once.
    #[test]
    fn all_cases_triangulate() {
        for mask in 1u32..255 {
            let g: [f64; 8] = std::array::from_fn(|c| {
                if mask >> c & 1 == 1 {
                    0.3 + 0.1 * c as f64
                } else {
                    -0.5 - 0.07 * c as f64
                }
            });
            let (mut out, mut centers) = (Vec::new(), Vec::new());
            cube(&g, 1.0, &mut out, &mut centers);
            let crossings = (0..12)
                .filter(|&e| {
                    let (c, a) = edge_corner(e);
                    (g[c] > 0.0) != (g[c | (1 << a)] > 0.0)
                })
                .count();
            let key = |v: Vert| match v {
                Vert::Edge(e) => e,
                Vert::Center(n) => 100 + n,
            };
            let used: std::collections::HashSet<usize> = out
                .iter()
                .flatten()
                .map(|&v| key(v))
                .filter(|&k| k < 100)
                .collect();
            assert_eq!(used.len(), crossings, "mask {mask}");
            let mut directed = std::collections::HashSet::new();
            for t in &out {
                for k in 0..3 {
                    assert!(
                        directed.insert((key(t[k]), key(t[(k + 1) % 3]))),
                        "mask {mask}"
                    );
                }
            }
        }
    }
}
