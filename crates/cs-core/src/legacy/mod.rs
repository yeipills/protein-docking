//! Legacy mode: a faithful, much faster port of the original thesis scripts
//! (`Script02_calculo_centroides`, `Script03_rayos_contexto`, `Script04_evaluacion_capas`,
//! `Script05_preparacion_capas_unity`).
//!
//! The goal is identical numbers, so the known defects are kept on purpose:
//! - MSMS vertex ids are 1-based but are used as 0-based indices (centroids are shifted).
//! - Ray angles use `phi, theta in linspace(0, 2*pi, 10)`, so many rays coincide.
//! - "Hits" come from `intersects_any` with the next sample's *absolute position* passed as
//!   the ray direction, and the first hit of every ray is dropped when reading it back.
//! - Layer boundaries are offset radially from the coordinate origin, not from the surface.
//! - The `out4` boundary is kept when it falls *outside* the sphere (inverted test).
//! - The CS summary file lists centres in sorted order, not in shape order.
//!
//! Every floating-point expression follows the evaluation order of the Python/NumPy original.

pub mod trimesh_ray;

use std::fmt::Write as _;

use rayon::prelude::*;

use crate::error::{Error, Result};
use crate::geom::grid::Grid;
use crate::geom::vec3::V3;
use crate::io::msms::MsmsSurface;
use crate::io::pyfmt::write_py_repr;
use crate::io::stl::Stl;
use crate::model::{ContextShapes, Layer, Mode};
use crate::progress::{Progress, check};
use trimesh_ray::TrimeshRay;

const RADIUS: f64 = 3.0;
const DELTA: usize = 10;
const RAYS: usize = DELTA * DELTA;
const MAX_DIST: f64 = 10.0;
const KNN: usize = 100;
/// Points per ray in Script03 (`n = 17`).
const POINTS: usize = 17;
const SEGMENTS: usize = POINTS - 1;
/// Hit flags left after Script04 drops the first one.
const CELLS: usize = SEGMENTS - 1;
const SENTINEL: V3 = [1000.0, 1000.0, 1000.0];

pub struct LegacyResult {
    pub shapes: ContextShapes,
    /// All triangle centroids (Script02 output, `centroidesPCS.txt`).
    pub centroids: Vec<V3>,
    /// Label of each context shape as written by Script03 (the counter of its last ray).
    pub cs_labels: Vec<u64>,
    /// Label of each ray in `rayos_contexto.txt`.
    pub ray_labels: Vec<u64>,
    pub ray_points: Vec<[V3; POINTS]>,
    pub hits: Vec<[bool; SEGMENTS]>,
}

pub fn run(surface: &MsmsSurface, stl: &Stl, progress: &dyn Progress) -> Result<LegacyResult> {
    progress.report("centroids", 0.0);
    let centroids = centroids(surface)?;
    check(progress)?;

    progress.report("critical_points", 0.05);
    let centers = filter_critical_points(&centroids);
    check(progress)?;

    progress.report("rays", 0.1);
    let (cs_labels, ray_labels, ray_ends) = compute_cr(&centers);
    let mesh = TrimeshRay::new(stl);
    let n_rays = ray_ends.len();
    let done = std::sync::atomic::AtomicUsize::new(0);
    let traced: Vec<([V3; POINTS], [bool; SEGMENTS])> = (0..n_rays)
        .into_par_iter()
        .map(|r| {
            if progress.is_cancelled() {
                return Err(Error::Cancelled);
            }
            let center = centers[r / RAYS];
            let points = linspace(center, ray_ends[r]);
            let mut hits = [false; SEGMENTS];
            for k in 0..SEGMENTS {
                hits[k] = mesh.intersects_any(points[k], points[k + 1]);
            }
            let d = done.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if d % 512 == 0 {
                progress.report("rays", 0.1 + 0.8 * d as f32 / n_rays as f32);
            }
            Ok((points, hits))
        })
        .collect::<Result<_>>()?;
    let (ray_points, hits): (Vec<_>, Vec<_>) = traced.into_iter().unzip();

    progress.report("layers", 0.9);
    let layers: Vec<u16> = ray_points
        .par_iter()
        .zip(&hits)
        .flat_map_iter(|(p, h)| evaluate_ray(p, h))
        .collect();
    check(progress)?;

    let shapes = ContextShapes {
        mode: Mode::Legacy,
        radius: RADIUS,
        surface_points: centroids.len(),
        centers,
        rays_per_center: RAYS,
        ray_ends,
        samples_per_ray: CELLS,
        sample_t: (0..CELLS).map(|j| j as f64 / SEGMENTS as f64).collect(),
        layers,
        distances: None,
    };
    progress.report("done", 1.0);
    Ok(LegacyResult {
        shapes,
        centroids,
        cs_labels,
        ray_labels,
        ray_points,
        hits,
    })
}

/// Script02: centroids of every face whose type is not 1, indexing vertices with the raw
/// (1-based) MSMS ids.
fn centroids(surface: &MsmsSurface) -> Result<Vec<V3>> {
    let v = &surface.vertices;
    let mut out = Vec::new();
    for (f, &ty) in surface.faces_raw.iter().zip(&surface.face_types) {
        if ty == 1 {
            continue;
        }
        let get = |i: u32| {
            v.get(i as usize).copied().ok_or_else(|| {
                Error::Invalid(format!(
                    "legacy mode: vertex id {i} is out of range once used as a 0-based index \
                     (the original scripts fail on this file too); use the corrected mode"
                ))
            })
        };
        let (a, b, c) = (get(f[0])?, get(f[1])?, get(f[2])?);
        out.push([
            (a[0] + b[0] + c[0]) / 3.0,
            (a[1] + b[1] + c[1]) / 3.0,
            (a[2] + b[2] + c[2]) / 3.0,
        ]);
    }
    Ok(out)
}

/// Script03 thinning: walk the points in order; for each one still present, keep it and
/// remove its (at most 100) nearest neighbours closer than 10 Å, itself included.
fn filter_critical_points(points: &[V3]) -> Vec<V3> {
    let grid = Grid::new(points, MAX_DIST);
    let mut current: Vec<V3> = points.to_vec();
    let mut removed = vec![false; points.len()];
    let mut near = Vec::new();
    let mut out = Vec::new();
    for i in 0..points.len() {
        // `int(item[0]) != 1000` on the possibly-overwritten list element.
        if current[i][0].trunc() == 1000.0 {
            continue;
        }
        grid.within(points, current[i], MAX_DIST, &mut near);
        near.retain(|(j, _)| !removed[*j as usize]);
        near.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        near.truncate(KNN);
        out.push(current[near[0].0 as usize]);
        for &(j, _) in &near {
            removed[j as usize] = true;
            current[j as usize] = SENTINEL;
        }
    }
    out
}

/// Script03 `computeCR(3, 10, centers)`: ray end points and the running labels.
fn compute_cr(centers: &[V3]) -> (Vec<u64>, Vec<u64>, Vec<V3>) {
    let angles = linspace_scalar(0.0, 2.0 * std::f64::consts::PI, DELTA);
    let mut counter: u64 = 0;
    let mut cs_labels = Vec::with_capacity(centers.len());
    let mut ray_labels = Vec::with_capacity(centers.len() * RAYS);
    let mut ends = Vec::with_capacity(centers.len() * RAYS);
    for c in centers {
        for &i in &angles {
            for &j in &angles {
                let x = RADIUS * i.sin() * j.cos() + c[0];
                let y = RADIUS * i.sin() * j.sin() + c[1];
                let z = RADIUS * i.cos() + c[2];
                counter += 1;
                ray_labels.push(counter);
                ends.push([x, y, z]);
            }
        }
        cs_labels.push(counter);
        counter += 1;
    }
    (cs_labels, ray_labels, ends)
}

/// `np.linspace(start, stop, n)` for scalars.
fn linspace_scalar(start: f64, stop: f64, n: usize) -> Vec<f64> {
    let step = (stop - start) / (n - 1) as f64;
    let mut v: Vec<f64> = (0..n).map(|k| k as f64 * step + start).collect();
    v[n - 1] = stop;
    v
}

/// `np.linspace(origin, end, 17)` for 3-vectors.
fn linspace(o: V3, e: V3) -> [V3; POINTS] {
    let step = [
        (e[0] - o[0]) / SEGMENTS as f64,
        (e[1] - o[1]) / SEGMENTS as f64,
        (e[2] - o[2]) / SEGMENTS as f64,
    ];
    let mut p = [[0.0; 3]; POINTS];
    for (k, pk) in p.iter_mut().enumerate() {
        let kf = k as f64;
        *pk = [
            kf * step[0] + o[0],
            kf * step[1] + o[1],
            kf * step[2] + o[2],
        ];
    }
    p[SEGMENTS] = e;
    p
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Cell {
    Ses,
    In,
    Out,
}

fn modulo(p: V3) -> f64 {
    (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt()
}

/// Script04 `suma_capa`: move `p` by `dist` along the direction from the origin to `p`.
fn suma_capa(p: V3, dist: f64) -> V3 {
    let m = modulo(p);
    [
        p[0] + dist * (p[0] / m),
        p[1] + dist * (p[1] / m),
        p[2] + dist * (p[2] / m),
    ]
}

fn pto_en_esfera(center: V3, p: V3) -> bool {
    let d2 = (p[0] - center[0]) * (p[0] - center[0])
        + (p[1] - center[1]) * (p[1] - center[1])
        + (p[2] - center[2]) * (p[2] - center[2]);
    d2 - RADIUS * RADIUS < 0.001
}

fn min_distance(p: V3, list: &[V3]) -> f64 {
    list.iter()
        .map(|q| {
            ((q[0] - p[0]) * (q[0] - p[0])
                + (q[1] - p[1]) * (q[1] - p[1])
                + (q[2] - p[2]) * (q[2] - p[2]))
                .sqrt()
        })
        .fold(f64::INFINITY, f64::min)
}

/// Script04 `calculo_cs`.
fn calculo_cs(p: V3, inner: &[V3], outer: &[V3], cell: Cell, interior: bool) -> bool {
    let eligible = match cell {
        Cell::Ses => true,
        Cell::In => interior,
        Cell::Out => !interior,
    };
    if !eligible {
        return false;
    }
    match (inner.first(), outer.first()) {
        (None, None) => false,
        (None, Some(&o)) => modulo(p) <= modulo(o),
        (Some(&i), None) => modulo(p) >= modulo(i),
        (Some(&i), Some(&o)) => {
            let (mp, mi, mo) = (modulo(p), modulo(i), modulo(o));
            mi <= mo && mi <= mp && mo >= mp
        }
    }
}

/// Script04 for one ray: 15 cells, each with a layer bitmask.
fn evaluate_ray(p: &[V3; POINTS], hits: &[bool; SEGMENTS]) -> [u16; CELLS] {
    // Reading `cr_totales` back skips the first flag.
    let bol: [bool; CELLS] = std::array::from_fn(|j| hits[j + 1]);
    let cells: [Cell; CELLS] = std::array::from_fn(|j| {
        if j + 1 < CELLS && bol[j] != bol[j + 1] {
            Cell::Ses
        } else if bol[j] {
            Cell::In
        } else {
            Cell::Out
        }
    });
    let center = p[0];

    // First pass: boundary points generated from every SES cell, in cell order.
    #[derive(Default)]
    struct Bounds {
        ses: Vec<V3>,
        inn: [Vec<V3>; 4],
        out: [Vec<V3>; 4],
    }
    let mut b = Bounds::default();
    const OFFSETS: [f64; 4] = [0.2, 0.4, 0.8, 1.0];
    for j in 0..CELLS {
        if cells[j] != Cell::Ses {
            continue;
        }
        let s = p[j];
        for (k, off) in OFFSETS.iter().enumerate() {
            let q = suma_capa(s, -off);
            if pto_en_esfera(center, q) {
                b.inn[k].push(q);
            }
        }
        for (k, off) in OFFSETS.iter().enumerate() {
            let q = suma_capa(s, *off);
            let inside = pto_en_esfera(center, q);
            // `if d_out4 < 1` keeps out4 points that are *outside* the sphere.
            if if k == 3 { !inside } else { inside } {
                b.out[k].push(q);
            }
        }
        if pto_en_esfera(center, s) {
            b.ses.push(s);
        }
    }

    // Second pass: classify every cell.
    let mut out = [0u16; CELLS];
    for j in 0..CELLS {
        let (q, cell) = (p[j], cells[j]);
        let mut m = 0u16;
        let mut set = |layer: Layer, v: bool| {
            if v {
                m |= layer.bit();
            }
        };
        set(
            Layer::Ses,
            cell == Cell::Ses && !b.ses.is_empty() && min_distance(q, &b.ses) < 1.0,
        );
        set(Layer::Vol, cell == Cell::In);
        set(Layer::In1, calculo_cs(q, &b.inn[0], &b.ses, cell, true));
        set(Layer::In2, calculo_cs(q, &b.inn[1], &b.inn[0], cell, true));
        set(Layer::In3, calculo_cs(q, &b.inn[2], &b.inn[1], cell, true));
        set(Layer::In4, calculo_cs(q, &b.inn[3], &b.inn[2], cell, true));
        set(Layer::Out1, calculo_cs(q, &b.ses, &b.out[0], cell, false));
        set(
            Layer::Out2,
            calculo_cs(q, &b.out[0], &b.out[1], cell, false),
        );
        set(
            Layer::Out3,
            calculo_cs(q, &b.out[1], &b.out[2], cell, false),
        );
        set(
            Layer::Out4,
            calculo_cs(q, &b.out[2], &b.out[3], cell, false),
        );
        out[j] = m;
    }
    out
}

// ---------------------------------------------------------------------------------------
// Text outputs of the original scripts.

fn push_v3(s: &mut String, v: V3) {
    write_py_repr(s, v[0]);
    s.push(' ');
    write_py_repr(s, v[1]);
    s.push(' ');
    write_py_repr(s, v[2]);
}

/// `centroidesPCS.txt`
pub fn write_centroids(r: &LegacyResult) -> String {
    let mut s = String::with_capacity(r.centroids.len() * 60);
    for c in &r.centroids {
        push_v3(&mut s, *c);
        s.push('\n');
    }
    s
}

/// `<name>_rayos_contexto.txt`
pub fn write_rayos_contexto(r: &LegacyResult) -> String {
    let sh = &r.shapes;
    let chunks: Vec<String> = (0..sh.ray_count())
        .into_par_iter()
        .map(|ray| {
            let mut s = String::with_capacity(128);
            write!(s, "{} ", r.ray_labels[ray]).unwrap();
            push_v3(&mut s, sh.center_of_ray(ray));
            s.push(' ');
            push_v3(&mut s, sh.ray_ends[ray]);
            s.push('\n');
            s
        })
        .collect();
    chunks.concat()
}

/// `<name>_cr_totales.txt`
pub fn write_cr_totales(r: &LegacyResult) -> String {
    let sh = &r.shapes;
    let chunks: Vec<String> = (0..sh.ray_count())
        .into_par_iter()
        .map(|ray| {
            let mut s = String::with_capacity(1100);
            write!(s, "{} ", r.cs_labels[ray / RAYS]).unwrap();
            push_v3(&mut s, sh.center_of_ray(ray));
            for p in &r.ray_points[ray] {
                s.push(' ');
                push_v3(&mut s, *p);
            }
            for h in &r.hits[ray] {
                s.push_str(if *h { " True" } else { " False" });
            }
            s.push('\n');
            s
        })
        .collect();
    chunks.concat()
}

/// Script04 per-layer intermediate file (`<name>_<layer>.txt`): one line per cell.
pub fn write_cs_intermediate(r: &LegacyResult, layer: Layer) -> String {
    let sh = &r.shapes;
    let chunks: Vec<String> = (0..sh.ray_count())
        .into_par_iter()
        .map(|ray| {
            let mut end = String::with_capacity(64);
            push_v3(&mut end, sh.ray_ends[ray]);
            let mut s = String::with_capacity(CELLS * 80);
            for &m in sh.ray_samples(ray) {
                let v = u8::from(m & layer.bit() != 0);
                writeln!(s, "{} {} {} {}", r.cs_labels[ray / RAYS], ray, v, end).unwrap();
            }
            s
        })
        .collect();
    chunks.concat()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::approx_constant)] // literal NumPy output
    fn linspace_matches_numpy() {
        // np.linspace(0, 2*np.pi, 10).tolist()
        let want = [
            0.0,
            0.6981317007977318,
            1.3962634015954636,
            2.0943951023931953,
            2.792526803190927,
            3.490658503988659,
            4.1887902047863905,
            4.886921905584122,
            5.585053606381854,
            6.283185307179586,
        ];
        assert_eq!(linspace_scalar(0.0, 2.0 * std::f64::consts::PI, 10), want);
    }

    #[test]
    fn ses_cells_are_flag_changes() {
        let p: [V3; POINTS] = std::array::from_fn(|k| [10.0, 10.0, 10.0 + 3.0 * k as f64 / 16.0]);
        let mut hits = [false; SEGMENTS];
        for h in hits.iter_mut().skip(5) {
            *h = true;
        }
        let m = evaluate_ray(&p, &hits);
        // bol = hits[1..]; the change is between bol[3] and bol[4].
        assert!(m[3] & Layer::Ses.bit() != 0);
        assert!(m[4] & Layer::Vol.bit() != 0);
        assert!(m[2] & Layer::Vol.bit() == 0);
    }
}
