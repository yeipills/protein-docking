//! Uniform cell grid for fixed-radius neighbour queries between two atom sets.

use crate::geom::vec3::{V3, dist2};

/// Points bucketed into cubic cells of side `cell` (compressed-row layout). A query for
/// radius ≤ `cell` only has to visit the 27 cells around the query point.
pub(crate) struct Grid {
    origin: V3,
    cell: f64,
    dims: [usize; 3],
    starts: Vec<u32>,
    items: Vec<u32>,
    points: Vec<V3>,
}

impl Grid {
    pub fn new(points: Vec<V3>, cell: f64) -> Grid {
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for p in &points {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        if points.is_empty() {
            lo = [0.0; 3];
            hi = [0.0; 3];
        }
        let dims = [0, 1, 2].map(|k| ((hi[k] - lo[k]) / cell).floor() as usize + 1);
        let mut g = Grid {
            origin: lo,
            cell,
            dims,
            starts: vec![0; dims[0] * dims[1] * dims[2] + 1],
            items: vec![0; points.len()],
            points,
        };
        let cells: Vec<usize> = g
            .points
            .iter()
            .map(|p| g.flat(g.coords(*p).unwrap()))
            .collect();
        for &c in &cells {
            g.starts[c + 1] += 1;
        }
        for i in 1..g.starts.len() {
            g.starts[i] += g.starts[i - 1];
        }
        let mut fill = g.starts.clone();
        for (i, &c) in cells.iter().enumerate() {
            g.items[fill[c] as usize] = i as u32;
            fill[c] += 1;
        }
        g
    }

    /// Integer cell coordinates of `p`, or `None` if it lies outside the grid.
    fn coords(&self, p: V3) -> Option<[usize; 3]> {
        let mut c = [0; 3];
        for k in 0..3 {
            let f = ((p[k] - self.origin[k]) / self.cell).floor();
            if f < 0.0 || f >= self.dims[k] as f64 {
                return None;
            }
            c[k] = f as usize;
        }
        Some(c)
    }

    fn flat(&self, c: [usize; 3]) -> usize {
        (c[0] * self.dims[1] + c[1]) * self.dims[2] + c[2]
    }

    /// Whether some point lies within `sqrt(r2)` of `p` (`r2 ≤ cell²`).
    pub fn any_within(&self, p: V3, r2: f64) -> bool {
        self.find_within(p, r2, |_| true)
    }

    /// Calls `f(index)` for every point within `sqrt(r2)` of `p` (`r2 ≤ cell²`).
    pub fn for_each_within(&self, p: V3, r2: f64, mut f: impl FnMut(usize)) {
        self.find_within(p, r2, |i| {
            f(i);
            false
        });
    }

    /// Visits points within `sqrt(r2)` of `p` until `f` returns true; returns whether it did.
    fn find_within(&self, p: V3, r2: f64, mut f: impl FnMut(usize) -> bool) -> bool {
        let mut lo = [0usize; 3];
        let mut hi = [0usize; 3];
        for k in 0..3 {
            let f = ((p[k] - self.origin[k]) / self.cell).floor();
            let (a, b) = (f - 1.0, f + 1.0);
            if b < 0.0 || a >= self.dims[k] as f64 {
                return false;
            }
            lo[k] = a.max(0.0) as usize;
            hi[k] = (b as usize).min(self.dims[k] - 1);
        }
        for x in lo[0]..=hi[0] {
            for y in lo[1]..=hi[1] {
                let row = (x * self.dims[1] + y) * self.dims[2];
                let (s, e) = (self.starts[row + lo[2]], self.starts[row + hi[2] + 1]);
                for &i in &self.items[s as usize..e as usize] {
                    if dist2(self.points[i as usize], p) < r2 && f(i as usize) {
                        return true;
                    }
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::transform::Rng;

    #[test]
    fn matches_brute_force() {
        let mut rng = Rng::new(3);
        let mut pt = || [0, 1, 2].map(|_| rng.next_f64() * 30.0 - 5.0);
        let pts: Vec<V3> = (0..500).map(|_| pt()).collect();
        let queries: Vec<V3> = (0..200).map(|_| pt()).collect();
        let g = Grid::new(pts.clone(), 5.0);
        for q in queries {
            let mut got = Vec::new();
            g.for_each_within(q, 25.0, |i| got.push(i));
            got.sort();
            let want: Vec<usize> = (0..pts.len())
                .filter(|&i| dist2(pts[i], q) < 25.0)
                .collect();
            assert_eq!(g.any_within(q, 25.0), !want.is_empty());
            assert_eq!(got, want);
        }
    }
}
