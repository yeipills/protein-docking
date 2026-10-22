//! End-to-end checks on the 1AHW_l_u sample (MSMS surface of tissue factor, the ligand of
//! complex 1AHW in the Protein-Protein Docking Benchmark).
//!
//! `legacy_oracle.sha256` holds the hashes of the files written by the original Python
//! scripts (Script02-05, run unmodified apart from file paths) on the same input.

use std::collections::HashMap;
use std::path::PathBuf;

use cs_core::corrected::{self, Params};
use cs_core::geom::sdf::{orient_outward, point_triangle_dist2};
use cs_core::geom::vec3::*;
use cs_core::io::msms::MsmsSurface;
use cs_core::io::stl::Stl;
use cs_core::progress::NoProgress;
use cs_core::{Layer, export, legacy};
use sha2::{Digest, Sha256};

fn sample_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/samples/1AHW_l_u")
}

fn surface() -> MsmsSurface {
    let d = sample_dir();
    MsmsSurface::read(&d.join("1AHW_l_u.vert"), &d.join("1AHW_l_u.face")).unwrap()
}

fn sha256(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn legacy_matches_original_scripts_byte_for_byte() {
    let stl = Stl::read(&sample_dir().join("1AHW_l_u.stl")).unwrap();
    let r = legacy::run(&surface(), &stl, &NoProgress).unwrap();

    let mut files: HashMap<String, String> = HashMap::new();
    files.insert("centroidesPCS.txt".into(), legacy::write_centroids(&r));
    files.insert(
        "1AHW_l_u_rayos_contexto.txt".into(),
        legacy::write_rayos_contexto(&r),
    );
    files.insert(
        "1AHW_l_u_cr_totales.txt".into(),
        legacy::write_cr_totales(&r),
    );
    files.insert(
        "1AHW_l_u_resumen_cs_unity.txt".into(),
        export::unity_summary(&r.shapes),
    );
    for (layer, text) in export::unity_layers(&r.shapes) {
        files.insert(format!("1AHW_l_u_cs_{}_unity.txt", layer.name()), text);
    }
    for layer in Layer::ALL {
        files.insert(
            format!("1AHW_l_u_{}.txt", layer.name()),
            legacy::write_cs_intermediate(&r, layer),
        );
    }

    let expected = include_str!("legacy_oracle.sha256");
    let mut checked = 0;
    for line in expected.lines() {
        let (hash, name) = line.split_once("  ").unwrap();
        let got = files
            .get(name)
            .unwrap_or_else(|| panic!("{name} not produced"));
        assert_eq!(
            sha256(got),
            hash,
            "{name} differs from the original scripts"
        );
        checked += 1;
    }
    assert_eq!(checked, 24);
}

/// Generalised winding number by brute force (sum of solid angles), > 0.5 inside.
fn winding_number(p: V3, verts: &[V3], faces: &[[u32; 3]]) -> f64 {
    let mut total = 0.0;
    for f in faces {
        let [a, b, c] = f.map(|i| sub(verts[i as usize], p));
        let (la, lb, lc) = (norm(a), norm(b), norm(c));
        let num = dot(a, cross(b, c));
        let den = la * lb * lc + dot(a, b) * lc + dot(b, c) * la + dot(c, a) * lb;
        total += 2.0 * num.atan2(den);
    }
    total / (4.0 * std::f64::consts::PI)
}

#[test]
fn corrected_signed_distance_agrees_with_brute_force() {
    let s = surface();
    let mut faces = s.faces().unwrap();
    orient_outward(&s.vertices, &s.normals, &mut faces);
    let sdf = cs_core::geom::sdf::SignedDistance::new(s.vertices.clone(), &faces);

    // Deterministic points near the surface: vertices pushed along their normals.
    let mut disagreements = 0;
    let n = 300;
    for k in 0..n {
        let i = (k * 7919) % s.vertices.len();
        let off = [-0.9, -0.3, -0.05, 0.05, 0.3, 0.9][k % 6];
        let p = add(s.vertices[i], scale(s.normals[i], off));
        let d = sdf.signed_distance(p);

        let brute = faces
            .iter()
            .map(|f| point_triangle_dist2(p, f.map(|i| s.vertices[i as usize])))
            .fold(f64::INFINITY, f64::min)
            .sqrt();
        assert!(
            (d.abs() - brute).abs() < 1e-9,
            "distance mismatch at {p:?}: {d} vs {brute}"
        );

        let inside = winding_number(p, &s.vertices, &faces) > 0.5;
        if inside != (d < 0.0) {
            disagreements += 1;
        }
    }
    // MSMS meshes have a few slivers; allow a tiny fraction of sign disagreements.
    assert!(
        disagreements <= n / 100,
        "{disagreements} sign disagreements out of {n}"
    );
}

#[test]
fn corrected_pipeline_is_consistent() {
    let params = Params::default();
    let shapes = corrected::run(&surface(), &params, &NoProgress).unwrap();
    let dist = shapes.distances.as_ref().unwrap();
    assert!(!shapes.centers.is_empty());
    assert_eq!(dist.len(), shapes.layers.len());
    assert_eq!(shapes.layers.len(), shapes.ray_count() * params.samples);
    for (d, m) in dist.iter().zip(&shapes.layers) {
        assert_eq!(
            *m,
            params.classify(*d as f64),
            "layers must follow distances"
        );
        assert_eq!(m & Layer::Vol.bit() != 0, (*d as f64) < 0.0);
    }
    // Shapes are centred on the surface, so roughly half of every sphere is inside.
    let vol = shapes.layer_occupancy()[9].1;
    assert!((0.35..0.65).contains(&vol), "vol occupancy {vol}");
    // Critical points keep the minimum spacing.
    for (i, a) in shapes.centers.iter().enumerate() {
        for b in &shapes.centers[i + 1..] {
            assert!(norm(sub(*a, *b)) >= params.min_spacing);
        }
    }
}
