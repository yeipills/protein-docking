//! Bounding volume hierarchy over axis-aligned boxes (binned SAH build).
//!
//! Used for two queries: all primitives whose box overlaps a query box (legacy broad phase,
//! which must match an R-tree exactly), and nearest-primitive search for signed distances.

#![allow(clippy::needless_range_loop)] // per-axis loops over three arrays read best indexed

use super::vec3::V3;

#[derive(Debug, Clone, Copy)]
pub struct Aabb {
    pub min: V3,
    pub max: V3,
}

impl Aabb {
    pub const EMPTY: Aabb = Aabb {
        min: [f64::INFINITY; 3],
        max: [f64::NEG_INFINITY; 3],
    };

    pub fn from_points(pts: &[V3]) -> Aabb {
        let mut b = Aabb::EMPTY;
        for p in pts {
            b.grow_point(*p);
        }
        b
    }

    #[inline]
    pub fn grow_point(&mut self, p: V3) {
        for k in 0..3 {
            self.min[k] = self.min[k].min(p[k]);
            self.max[k] = self.max[k].max(p[k]);
        }
    }

    #[inline]
    pub fn grow(&mut self, o: &Aabb) {
        for k in 0..3 {
            self.min[k] = self.min[k].min(o.min[k]);
            self.max[k] = self.max[k].max(o.max[k]);
        }
    }

    /// Inclusive overlap test (touching boxes overlap), like libspatialindex.
    #[inline]
    pub fn overlaps(&self, o: &Aabb) -> bool {
        (0..3).all(|k| self.min[k] <= o.max[k] && self.max[k] >= o.min[k])
    }

    /// Squared distance from a point to the box (0 inside).
    #[inline]
    pub fn dist2(&self, p: V3) -> f64 {
        let mut d = 0.0;
        for k in 0..3 {
            let v = if p[k] < self.min[k] {
                self.min[k] - p[k]
            } else if p[k] > self.max[k] {
                p[k] - self.max[k]
            } else {
                0.0
            };
            d += v * v;
        }
        d
    }

    fn area(&self) -> f64 {
        let e = [
            self.max[0] - self.min[0],
            self.max[1] - self.min[1],
            self.max[2] - self.min[2],
        ];
        if e[0] < 0.0 {
            return 0.0;
        }
        2.0 * (e[0] * e[1] + e[1] * e[2] + e[2] * e[0])
    }

    fn center(&self, k: usize) -> f64 {
        0.5 * (self.min[k] + self.max[k])
    }
}

#[derive(Debug, Clone, Copy)]
struct Node {
    bounds: Aabb,
    /// Leaf: first index into `prims`; inner: index of the left child (right = left + 1).
    start: u32,
    /// Number of primitives for a leaf, 0 for an inner node.
    count: u32,
}

#[derive(Debug, Clone)]
pub struct Bvh {
    nodes: Vec<Node>,
    prims: Vec<u32>,
    pub boxes: Vec<Aabb>,
}

const LEAF_SIZE: usize = 4;
/// Traversal stacks are fixed arrays; the build stops splitting before this depth.
const MAX_DEPTH: usize = 120;
const BINS: usize = 12;

impl Bvh {
    pub fn build(boxes: Vec<Aabb>) -> Bvh {
        let mut prims: Vec<u32> = (0..boxes.len() as u32).collect();
        let mut nodes = Vec::with_capacity(boxes.len() * 2);
        if boxes.is_empty() {
            return Bvh {
                nodes,
                prims,
                boxes,
            };
        }
        nodes.push(Node {
            bounds: Aabb::EMPTY,
            start: 0,
            count: boxes.len() as u32,
        });
        let mut stack = vec![(0usize, 0usize)];
        while let Some((ni, depth)) = stack.pop() {
            let (start, count) = (nodes[ni].start as usize, nodes[ni].count as usize);
            let slice = &mut prims[start..start + count];
            let mut bounds = Aabb::EMPTY;
            let mut cbounds = Aabb::EMPTY;
            for &p in slice.iter() {
                let b = &boxes[p as usize];
                bounds.grow(b);
                cbounds.grow_point([b.center(0), b.center(1), b.center(2)]);
            }
            nodes[ni].bounds = bounds;
            if count <= LEAF_SIZE || depth >= MAX_DEPTH {
                continue;
            }
            let Some(mid) = split_sah(slice, &boxes, &cbounds) else {
                continue;
            };
            let left = nodes.len();
            nodes.push(Node {
                bounds: Aabb::EMPTY,
                start: start as u32,
                count: mid as u32,
            });
            nodes.push(Node {
                bounds: Aabb::EMPTY,
                start: (start + mid) as u32,
                count: (count - mid) as u32,
            });
            nodes[ni].start = left as u32;
            nodes[ni].count = 0;
            stack.push((left, depth + 1));
            stack.push((left + 1, depth + 1));
        }
        Bvh {
            nodes,
            prims,
            boxes,
        }
    }

    /// Calls `f` for every primitive whose box overlaps `q` (inclusive).
    pub fn for_each_overlap(&self, q: &Aabb, mut f: impl FnMut(u32) -> bool) {
        if self.nodes.is_empty() {
            return;
        }
        let mut stack = [0u32; 2 * MAX_DEPTH + 4];
        let mut sp = 1;
        while sp > 0 {
            sp -= 1;
            let node = &self.nodes[stack[sp] as usize];
            if !node.bounds.overlaps(q) {
                continue;
            }
            if node.count > 0 {
                let s = node.start as usize;
                for &p in &self.prims[s..s + node.count as usize] {
                    if self.boxes[p as usize].overlaps(q) && !f(p) {
                        return;
                    }
                }
            } else {
                stack[sp] = node.start;
                stack[sp + 1] = node.start + 1;
                sp += 2;
            }
        }
    }

    /// Nearest primitive to `p`. `dist2(prim, p, best)` must return the exact squared distance
    /// (or anything `>= best` to reject). Returns `(prim, dist2)`.
    pub fn nearest(&self, p: V3, dist2: impl FnMut(u32, f64) -> f64) -> Option<(u32, f64)> {
        self.nearest_with_hint(p, None, dist2)
    }

    /// Like [`Bvh::nearest`], seeding the search bound with `hint` (e.g. the answer for a
    /// nearby point), which prunes most of the tree for coherent queries.
    pub fn nearest_with_hint(
        &self,
        p: V3,
        hint: Option<u32>,
        mut dist2: impl FnMut(u32, f64) -> f64,
    ) -> Option<(u32, f64)> {
        if self.nodes.is_empty() {
            return None;
        }
        let mut best = f64::INFINITY;
        let mut best_prim = u32::MAX;
        if let Some(h) = hint {
            best = dist2(h, f64::INFINITY);
            best_prim = h;
        }
        let mut stack: [(u32, f64); 2 * MAX_DEPTH + 4] = [(0, 0.0); 2 * MAX_DEPTH + 4];
        let mut sp = 1;
        stack[0] = (0, self.nodes[0].bounds.dist2(p));
        while sp > 0 {
            sp -= 1;
            let (ni, nd) = stack[sp];
            if nd >= best {
                continue;
            }
            let node = &self.nodes[ni as usize];
            if node.count > 0 {
                let s = node.start as usize;
                for &prim in &self.prims[s..s + node.count as usize] {
                    if prim == best_prim || self.boxes[prim as usize].dist2(p) >= best {
                        continue;
                    }
                    let d = dist2(prim, best);
                    if d < best {
                        best = d;
                        best_prim = prim;
                    }
                }
            } else {
                let (l, r) = (node.start, node.start + 1);
                let dl = self.nodes[l as usize].bounds.dist2(p);
                let dr = self.nodes[r as usize].bounds.dist2(p);
                // Push the farther child first so the nearer one is visited next.
                let (first, second) = if dl <= dr {
                    ((r, dr), (l, dl))
                } else {
                    ((l, dl), (r, dr))
                };
                if first.1 < best {
                    stack[sp] = first;
                    sp += 1;
                }
                if second.1 < best {
                    stack[sp] = second;
                    sp += 1;
                }
            }
        }
        (best_prim != u32::MAX).then_some((best_prim, best))
    }
}

/// Partitions `slice` with a binned SAH split; returns the split position or `None` to make a leaf.
fn split_sah(slice: &mut [u32], boxes: &[Aabb], cbounds: &Aabb) -> Option<usize> {
    let mut best: Option<(f64, usize, usize)> = None; // (cost, axis, bin)
    for axis in 0..3 {
        let lo = cbounds.min[axis];
        let extent = cbounds.max[axis] - lo;
        if extent <= 0.0 {
            continue;
        }
        let mut bin_box = [Aabb::EMPTY; BINS];
        let mut bin_cnt = [0usize; BINS];
        let scale = BINS as f64 / extent;
        for &p in slice.iter() {
            let b = &boxes[p as usize];
            let i = (((b.center(axis) - lo) * scale) as usize).min(BINS - 1);
            bin_box[i].grow(b);
            bin_cnt[i] += 1;
        }
        let mut right_area = [0.0; BINS];
        let mut right_cnt = [0usize; BINS];
        let (mut acc, mut cnt) = (Aabb::EMPTY, 0);
        for i in (1..BINS).rev() {
            acc.grow(&bin_box[i]);
            cnt += bin_cnt[i];
            right_area[i] = acc.area();
            right_cnt[i] = cnt;
        }
        let (mut acc, mut cnt) = (Aabb::EMPTY, 0);
        for i in 0..BINS - 1 {
            acc.grow(&bin_box[i]);
            cnt += bin_cnt[i];
            if cnt == 0 || right_cnt[i + 1] == 0 {
                continue;
            }
            let cost = acc.area() * cnt as f64 + right_area[i + 1] * right_cnt[i + 1] as f64;
            if best.is_none_or(|(c, _, _)| cost < c) {
                best = Some((cost, axis, i));
            }
        }
    }
    let (_, axis, bin) = best?;
    let lo = cbounds.min[axis];
    let scale = BINS as f64 / (cbounds.max[axis] - lo);
    let mut i = 0;
    for j in 0..slice.len() {
        let b = &boxes[slice[j] as usize];
        let k = (((b.center(axis) - lo) * scale) as usize).min(BINS - 1);
        if k <= bin {
            slice.swap(i, j);
            i += 1;
        }
    }
    (i > 0 && i < slice.len()).then_some(i)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boxes() -> Vec<Aabb> {
        let mut v = Vec::new();
        for i in 0..200 {
            let x = (i * 37 % 101) as f64;
            let y = (i * 53 % 97) as f64;
            let z = (i * 11 % 89) as f64;
            v.push(Aabb {
                min: [x, y, z],
                max: [x + 1.5, y + 2.0, z + 0.5],
            });
        }
        v
    }

    #[test]
    fn overlap_matches_brute_force() {
        let b = boxes();
        let bvh = Bvh::build(b.clone());
        let q = Aabb {
            min: [10.0, 10.0, 10.0],
            max: [50.0, 40.0, 60.0],
        };
        let mut got = Vec::new();
        bvh.for_each_overlap(&q, |p| {
            got.push(p);
            true
        });
        got.sort();
        let want: Vec<u32> = (0..b.len() as u32)
            .filter(|&i| b[i as usize].overlaps(&q))
            .collect();
        assert_eq!(got, want);
    }

    #[test]
    fn nearest_matches_brute_force() {
        let b = boxes();
        let bvh = Bvh::build(b.clone());
        for p in [[0.0, 0.0, 0.0], [55.5, 12.0, 70.0], [200.0, -5.0, 3.0]] {
            let (_, d) = bvh.nearest(p, |i, _| b[i as usize].dist2(p)).unwrap();
            let want = b.iter().map(|x| x.dist2(p)).fold(f64::INFINITY, f64::min);
            assert_eq!(d, want);
        }
    }
}
