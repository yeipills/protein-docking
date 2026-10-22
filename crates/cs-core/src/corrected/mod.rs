//! Corrected pipeline: the same idea as the original scripts, with the geometry done right.
//!
//! 1. Surface points are the centroids of the selected MSMS faces (1-based ids handled).
//! 2. Critical points are thinned greedily so no two are closer than `min_spacing`
//!    (all neighbours are considered, not only the nearest 100).
//! 3. Each critical point gets `rays` evenly spread directions (Fibonacci lattice) of length
//!    `radius`, sampled at `samples` cell centres.
//! 4. Every sample gets its exact signed distance to the SES mesh (negative inside), and the
//!    layers are distance bands around the surface: `in4 < in3 < in2 < in1 < 0 <= out1 < ...`.

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::geom::fibonacci_sphere;
use crate::geom::grid::Grid;
use crate::geom::sdf::{SignedDistance, orient_outward};
use crate::geom::vec3::*;
use crate::io::msms::MsmsSurface;
use crate::model::{ContextShapes, Layer, Mode};
use crate::progress::{Progress, check};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Params {
    /// MSMS face types whose centroids are not used as surface points.
    pub skip_face_types: Vec<i32>,
    /// Minimum distance between critical points, in Å.
    pub min_spacing: f64,
    /// Ray length (context-shape radius), in Å.
    pub radius: f64,
    /// Rays per critical point.
    pub rays: usize,
    /// Samples (cells) per ray.
    pub samples: usize,
    /// Signed-distance band edges, in Å, from the deepest interior to the farthest exterior:
    /// `[in4, in3, in2, in1, surface, out1, out2, out3, out4]`.
    pub layer_edges: [f64; 9],
    /// Half-width of the SES band, in Å. Defaults to half the sample spacing.
    pub ses_half_width: Option<f64>,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            skip_face_types: vec![1],
            min_spacing: 10.0,
            radius: 3.0,
            rays: 100,
            samples: 32,
            layer_edges: [-1.0, -0.8, -0.4, -0.2, 0.0, 0.2, 0.4, 0.8, 1.0],
            ses_half_width: None,
        }
    }
}

impl Params {
    // Negated comparisons also reject NaN.
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    pub fn validate(&self) -> Result<()> {
        let bad = |m: &str| Err(Error::Invalid(m.to_string()));
        if !(self.min_spacing > 0.0) {
            return bad("min_spacing must be > 0");
        }
        if !(self.radius > 0.0) {
            return bad("radius must be > 0");
        }
        if !(1..=10_000).contains(&self.rays) {
            return bad("rays must be in 1..=10000");
        }
        if !(1..=1024).contains(&self.samples) {
            return bad("samples must be in 1..=1024");
        }
        if !self.layer_edges.windows(2).all(|w| w[0] < w[1]) {
            return bad("layer_edges must be strictly increasing");
        }
        if self.ses_half_width.is_some_and(|w| !(w >= 0.0)) {
            return bad("ses_half_width must be >= 0");
        }
        Ok(())
    }

    pub fn ses_half_width(&self) -> f64 {
        self.ses_half_width
            .unwrap_or(self.radius / self.samples as f64 / 2.0)
    }

    /// Layer bitmask for a signed distance.
    pub fn classify(&self, d: f64) -> u16 {
        let e = &self.layer_edges;
        let mut m = 0u16;
        if d.abs() <= self.ses_half_width() {
            m |= Layer::Ses.bit();
        }
        if d < e[4] {
            m |= Layer::Vol.bit();
        }
        let bands = [
            (Layer::In4, e[0], e[1]),
            (Layer::In3, e[1], e[2]),
            (Layer::In2, e[2], e[3]),
            (Layer::In1, e[3], e[4]),
            (Layer::Out1, e[4], e[5]),
            (Layer::Out2, e[5], e[6]),
            (Layer::Out3, e[6], e[7]),
        ];
        for (layer, lo, hi) in bands {
            if d >= lo && d < hi {
                m |= layer.bit();
            }
        }
        if d >= e[7] && d <= e[8] {
            m |= Layer::Out4.bit();
        }
        m
    }
}

pub fn run(
    surface: &MsmsSurface,
    params: &Params,
    progress: &dyn Progress,
) -> Result<ContextShapes> {
    params.validate()?;
    progress.report("mesh", 0.0);
    let mut faces = surface.faces()?;
    orient_outward(&surface.vertices, &surface.normals, &mut faces);
    let sdf = SignedDistance::new(surface.vertices.clone(), &faces);
    check(progress)?;

    progress.report("critical_points", 0.05);
    let points: Vec<V3> = faces
        .iter()
        .zip(&surface.face_types)
        .filter(|(_, t)| !params.skip_face_types.contains(t))
        .map(|(f, _)| {
            let [a, b, c] = f.map(|i| surface.vertices[i as usize]);
            scale(add(add(a, b), c), 1.0 / 3.0)
        })
        .collect();
    if points.is_empty() {
        return Err(Error::Invalid(
            "no surface points: every face type is excluded by skip_face_types".into(),
        ));
    }
    let centers = thin(&points, params.min_spacing);
    check(progress)?;

    progress.report("rays", 0.1);
    let dirs = fibonacci_sphere(params.rays);
    let sample_t: Vec<f64> = (0..params.samples)
        .map(|k| (k as f64 + 0.5) / params.samples as f64)
        .collect();
    let ray_ends: Vec<V3> = centers
        .iter()
        .flat_map(|c| dirs.iter().map(move |d| add(*c, scale(*d, params.radius))))
        .collect();

    let n_centers = centers.len();
    let done = std::sync::atomic::AtomicUsize::new(0);
    let per_center: Vec<(Vec<f32>, Vec<u16>)> = centers
        .par_iter()
        .map(|c| {
            check(progress)?;
            let mut dist = Vec::with_capacity(dirs.len() * sample_t.len());
            let mut layers = Vec::with_capacity(dirs.len() * sample_t.len());
            let mut hint = None;
            for d in &dirs {
                for t in &sample_t {
                    let p = add(*c, scale(*d, params.radius * t));
                    let sd = sdf.signed_distance_hinted(p, &mut hint);
                    dist.push(sd as f32);
                    layers.push(params.classify(sd));
                }
            }
            let n = done.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            if n % 16 == 0 {
                progress.report("rays", 0.1 + 0.9 * n as f32 / n_centers as f32);
            }
            Ok((dist, layers))
        })
        .collect::<Result<_>>()?;

    let mut distances = Vec::with_capacity(ray_ends.len() * params.samples);
    let mut layers = Vec::with_capacity(ray_ends.len() * params.samples);
    for (d, l) in per_center {
        distances.extend(d);
        layers.extend(l);
    }
    progress.report("done", 1.0);
    Ok(ContextShapes {
        mode: Mode::Corrected,
        radius: params.radius,
        surface_points: points.len(),
        centers,
        rays_per_center: dirs.len(),
        ray_ends,
        samples_per_ray: params.samples,
        sample_t,
        layers,
        distances: Some(distances),
    })
}

/// Greedy thinning in input order: keep a point, drop everything closer than `spacing`.
fn thin(points: &[V3], spacing: f64) -> Vec<V3> {
    let grid = Grid::new(points, spacing);
    let mut removed = vec![false; points.len()];
    let mut near = Vec::new();
    let mut out = Vec::new();
    for i in 0..points.len() {
        if removed[i] {
            continue;
        }
        out.push(points[i]);
        grid.within(points, points[i], spacing, &mut near);
        for &(j, _) in &near {
            removed[j as usize] = true;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification_bands() {
        let p = Params::default();
        let has = |d: f64, l: Layer| p.classify(d) & l.bit() != 0;
        assert!(has(-0.1, Layer::In1) && has(-0.1, Layer::Vol));
        assert!(has(-0.3, Layer::In2) && !has(-0.3, Layer::In1));
        assert!(has(-0.9, Layer::In4));
        assert!(has(0.1, Layer::Out1) && !has(0.1, Layer::Vol));
        assert!(has(1.0, Layer::Out4));
        assert!(!has(1.5, Layer::Out4));
        assert!(has(0.01, Layer::Ses) && !has(0.2, Layer::Ses));
    }

    #[test]
    fn thinning_respects_spacing() {
        let pts: Vec<V3> = (0..1000)
            .map(|i| {
                [
                    (i % 10) as f64 * 1.3,
                    ((i / 10) % 10) as f64 * 1.3,
                    (i / 100) as f64 * 1.3,
                ]
            })
            .collect();
        let kept = thin(&pts, 4.0);
        for (i, a) in kept.iter().enumerate() {
            for b in &kept[i + 1..] {
                assert!(dist2(*a, *b).sqrt() >= 4.0);
            }
        }
        // Every input point is within the spacing of some kept point.
        for p in &pts {
            assert!(kept.iter().any(|k| dist2(*k, *p).sqrt() < 4.0));
        }
    }
}
