//! Uniform hash grid for fixed-radius neighbour queries.

use std::collections::HashMap;

use super::vec3::{V3, dist2};

pub struct Grid {
    cell: f64,
    cells: HashMap<[i32; 3], Vec<u32>>,
}

impl Grid {
    pub fn new(points: &[V3], cell: f64) -> Grid {
        assert!(cell > 0.0, "grid cell size must be positive");
        let mut cells: HashMap<[i32; 3], Vec<u32>> = HashMap::new();
        for (i, p) in points.iter().enumerate() {
            cells.entry(key(*p, cell)).or_default().push(i as u32);
        }
        Grid { cell, cells }
    }

    /// Indices of points strictly closer than `radius` (`radius <= cell`), with their distance.
    pub fn within(&self, points: &[V3], q: V3, radius: f64, out: &mut Vec<(u32, f64)>) {
        debug_assert!(radius <= self.cell);
        out.clear();
        let k = key(q, self.cell);
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    if let Some(ids) = self.cells.get(&[k[0] + dx, k[1] + dy, k[2] + dz]) {
                        for &i in ids {
                            let d = dist2(points[i as usize], q).sqrt();
                            if d < radius {
                                out.push((i, d));
                            }
                        }
                    }
                }
            }
        }
    }
}

fn key(p: V3, cell: f64) -> [i32; 3] {
    p.map(|c| (c / cell).floor() as i32)
}
