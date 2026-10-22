//! Output formats: Unity text files (legacy-compatible), a JSON summary and a compact binary
//! blob for the web viewer.

use std::fmt::Write as _;

use rayon::prelude::*;
use serde::Serialize;

use crate::geom::vec3::V3;
use crate::io::pyfmt::write_py_repr;
use crate::model::{ContextShapes, Layer, Mode};

fn push_v3(s: &mut String, v: V3) {
    write_py_repr(s, v[0]);
    s.push(' ');
    write_py_repr(s, v[1]);
    s.push(' ');
    write_py_repr(s, v[2]);
}

/// The ten `<name>_cs_<layer>_unity.txt` files, in [`Layer::ALL`] order. One line per ray:
/// `cs ray v_0 .. v_{n-1} cx cy cz ex ey ez`.
pub fn unity_layers(shapes: &ContextShapes) -> Vec<(Layer, String)> {
    // The coordinates are the same in all ten files: format them once.
    let suffixes: Vec<String> = (0..shapes.ray_count())
        .into_par_iter()
        .with_min_len(256)
        .map(|ray| {
            let mut s = String::with_capacity(120);
            s.push(' ');
            push_v3(&mut s, shapes.center_of_ray(ray));
            s.push(' ');
            push_v3(&mut s, shapes.ray_ends[ray]);
            s.push('\n');
            s
        })
        .collect();
    Layer::ALL
        .par_iter()
        .map(|&layer| {
            let mut s = String::with_capacity(
                suffixes.iter().map(String::len).sum::<usize>()
                    + shapes.layers.len() * 2
                    + suffixes.len() * 12,
            );
            for (ray, suffix) in suffixes.iter().enumerate() {
                write!(s, "{} {}", ray / shapes.rays_per_center, ray).unwrap();
                for &m in shapes.ray_samples(ray) {
                    s.push_str(if m & layer.bit() != 0 { " 1" } else { " 0" });
                }
                s.push_str(suffix);
            }
            (layer, s)
        })
        .collect()
}

/// `<name>_resumen_cs_unity.txt`: `index cx cy cz` per context shape.
///
/// The original Script05 wrote `np.unique(centres)` — sorted, so the indices do not match the
/// shape numbers in the layer files. Legacy mode keeps that; corrected mode writes shape order.
pub fn unity_summary(shapes: &ContextShapes) -> String {
    let mut centers = shapes.centers.clone();
    if shapes.mode == Mode::Legacy {
        centers.sort_by(|a, b| {
            a[0].total_cmp(&b[0])
                .then(a[1].total_cmp(&b[1]))
                .then(a[2].total_cmp(&b[2]))
        });
        centers.dedup();
    }
    let mut s = String::with_capacity(centers.len() * 64);
    for (i, c) in centers.iter().enumerate() {
        write!(s, "{i} ").unwrap();
        push_v3(&mut s, *c);
        s.push('\n');
    }
    s
}

#[derive(Debug, Serialize)]
pub struct Summary {
    pub mode: Mode,
    pub surface_points: usize,
    pub context_shapes: usize,
    pub rays_per_shape: usize,
    pub samples_per_ray: usize,
    pub radius: f64,
    pub layer_occupancy: Vec<LayerStat>,
}

#[derive(Debug, Serialize)]
pub struct LayerStat {
    pub layer: Layer,
    pub fraction: f64,
}

pub fn summary(shapes: &ContextShapes) -> Summary {
    Summary {
        mode: shapes.mode,
        surface_points: shapes.surface_points,
        context_shapes: shapes.centers.len(),
        rays_per_shape: shapes.rays_per_center,
        samples_per_ray: shapes.samples_per_ray,
        radius: shapes.radius,
        layer_occupancy: shapes
            .layer_occupancy()
            .into_iter()
            .map(|(layer, fraction)| LayerStat { layer, fraction })
            .collect(),
    }
}

/// Binary blob consumed by the web viewer (all little-endian):
///
/// ```text
/// "CSV1"
/// u32 n_vertices, u32 n_faces, f32[3 * n_vertices], u32[3 * n_faces]
/// u32 n_centers, f32[3 * n_centers]
/// u32 rays_per_center, f32[3 * n_centers * rays_per_center]   (ray end points)
/// u32 samples_per_ray, f32[samples_per_ray]                   (sample position along ray, 0..1)
/// u16[n_rays * samples_per_ray]                               (layer bitmask, Layer::ALL order)
/// padding to 4 bytes
/// u32 has_distances, f32[n_rays * samples_per_ray] if set
/// ```
pub fn viewer_blob(vertices: &[V3], faces: &[[u32; 3]], shapes: &ContextShapes) -> Vec<u8> {
    let n_samples = shapes.layers.len();
    let mut b = Vec::with_capacity(
        64 + vertices.len() * 12 + faces.len() * 12 + shapes.ray_ends.len() * 12 + n_samples * 6,
    );
    let u32le = |b: &mut Vec<u8>, v: usize| b.extend_from_slice(&(v as u32).to_le_bytes());
    let f32le = |b: &mut Vec<u8>, v: f64| b.extend_from_slice(&(v as f32).to_le_bytes());

    b.extend_from_slice(b"CSV1");
    u32le(&mut b, vertices.len());
    u32le(&mut b, faces.len());
    for v in vertices {
        v.iter().for_each(|&c| f32le(&mut b, c));
    }
    for f in faces {
        f.iter().for_each(|&i| u32le(&mut b, i as usize));
    }
    u32le(&mut b, shapes.centers.len());
    for c in &shapes.centers {
        c.iter().for_each(|&x| f32le(&mut b, x));
    }
    u32le(&mut b, shapes.rays_per_center);
    for e in &shapes.ray_ends {
        e.iter().for_each(|&x| f32le(&mut b, x));
    }
    u32le(&mut b, shapes.samples_per_ray);
    for &t in &shapes.sample_t {
        f32le(&mut b, t);
    }
    for &m in &shapes.layers {
        b.extend_from_slice(&m.to_le_bytes());
    }
    while b.len() % 4 != 0 {
        b.push(0);
    }
    match &shapes.distances {
        Some(d) => {
            u32le(&mut b, 1);
            for &x in d {
                b.extend_from_slice(&x.to_le_bytes());
            }
        }
        None => u32le(&mut b, 0),
    }
    b
}
