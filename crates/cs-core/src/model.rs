//! Result model shared by both pipelines, the exporters and the web viewer.

use serde::{Deserialize, Serialize};

use crate::geom::vec3::V3;

/// The ten context-shape layers, in the order of the legacy output files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Layer {
    Ses,
    In1,
    In2,
    In3,
    In4,
    Out1,
    Out2,
    Out3,
    Out4,
    Vol,
}

impl Layer {
    pub const ALL: [Layer; 10] = [
        Layer::Ses,
        Layer::In1,
        Layer::In2,
        Layer::In3,
        Layer::In4,
        Layer::Out1,
        Layer::Out2,
        Layer::Out3,
        Layer::Out4,
        Layer::Vol,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Layer::Ses => "ses",
            Layer::In1 => "in1",
            Layer::In2 => "in2",
            Layer::In3 => "in3",
            Layer::In4 => "in4",
            Layer::Out1 => "out1",
            Layer::Out2 => "out2",
            Layer::Out3 => "out3",
            Layer::Out4 => "out4",
            Layer::Vol => "vol",
        }
    }

    #[inline]
    pub fn bit(self) -> u16 {
        1 << (self as u16)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Geometrically correct pipeline (default).
    Corrected,
    /// Bit-for-bit reproduction of the original scripts, including their defects.
    Legacy,
}

/// Context shapes for every critical point.
///
/// Samples are indexed `(center * rays_per_center + ray) * samples_per_ray + sample`.
#[derive(Debug, Clone)]
pub struct ContextShapes {
    pub mode: Mode,
    pub radius: f64,
    /// Surface points before thinning (triangle centroids).
    pub surface_points: usize,
    /// Critical points: the centre of each context shape.
    pub centers: Vec<V3>,
    pub rays_per_center: usize,
    /// Absolute end point of each ray, `centers.len() * rays_per_center` entries.
    pub ray_ends: Vec<V3>,
    pub samples_per_ray: usize,
    /// Position of each sample along its ray as a fraction of the radius.
    pub sample_t: Vec<f64>,
    /// Layer membership bitmask per sample (see [`Layer::bit`]).
    pub layers: Vec<u16>,
    /// Signed distance to the surface per sample (corrected mode only).
    pub distances: Option<Vec<f32>>,
}

impl ContextShapes {
    pub fn ray_count(&self) -> usize {
        self.centers.len() * self.rays_per_center
    }

    pub fn ray_samples(&self, ray: usize) -> &[u16] {
        let s = self.samples_per_ray;
        &self.layers[ray * s..(ray + 1) * s]
    }

    pub fn center_of_ray(&self, ray: usize) -> V3 {
        self.centers[ray / self.rays_per_center]
    }

    /// Fraction of samples that belong to each layer.
    pub fn layer_occupancy(&self) -> Vec<(Layer, f64)> {
        let total = self.layers.len().max(1) as f64;
        Layer::ALL
            .iter()
            .map(|&l| {
                let n = self.layers.iter().filter(|&&m| m & l.bit() != 0).count();
                (l, n as f64 / total)
            })
            .collect()
    }
}
