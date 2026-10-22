//! Checks of the built-in SES generator: analytic shapes and the MSMS surface of the
//! 1AHW_l_u sample (probe 1.5 Å, density 1 vertex/Å²).

use std::collections::HashMap;
use std::f64::consts::PI;
use std::path::PathBuf;

use cs_core::geom::sdf::SignedDistance;
use cs_core::geom::vec3::*;
use cs_core::io::msms::MsmsSurface;
use cs_core::io::pdb::Structure;
use cs_core::mesh::Mesh;
use cs_core::progress::NoProgress;
use cs_core::surface::{self, Radii, SesGrid, SesParams};

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

fn params(probe: f64, h: f64) -> SesParams {
    SesParams {
        probe,
        grid_spacing: h,
        ..Default::default()
    }
}

/// Every edge is shared by exactly two faces, traversed in opposite directions.
fn assert_closed_and_oriented(m: &Mesh) {
    let mut directed: HashMap<(u32, u32), u32> = HashMap::new();
    for f in &m.faces {
        assert!(f[0] != f[1] && f[1] != f[2] && f[0] != f[2]);
        for k in 0..3 {
            *directed.entry((f[k], f[(k + 1) % 3])).or_default() += 1;
        }
    }
    for (&(a, b), &n) in &directed {
        assert_eq!(n, 1, "directed edge {a}->{b} used {n} times");
        assert_eq!(directed.get(&(b, a)), Some(&1), "edge {a}-{b} is open");
    }
    assert!(m.volume() > 0.0, "faces must point outwards");
    for n in &m.normals {
        assert!((norm(*n) - 1.0).abs() < 1e-9);
    }
}

#[test]
fn single_atom_is_a_sphere() {
    let r = 1.7;
    let m = surface::ses_spheres(&[[1.0, 2.0, 3.0]], &[r], &params(1.4, 0.2), &NoProgress).unwrap();
    assert_closed_and_oriented(&m);
    let area = 4.0 * PI * r * r;
    let volume = 4.0 / 3.0 * PI * r * r * r;
    assert!((m.area() / area - 1.0).abs() < 0.01, "area {}", m.area());
    assert!(
        (m.volume() / volume - 1.0).abs() < 0.01,
        "volume {}",
        m.volume()
    );
    for (v, n) in m.vertices.iter().zip(&m.normals) {
        let radial = sub(*v, [1.0, 2.0, 3.0]);
        assert!((norm(radial) - r).abs() < 0.01);
        assert!(dot(normalize(radial), *n) > 0.999);
    }
}

/// Two overlapping atoms: the probe cannot enter the groove between them, so the SES has a
/// concave (reentrant) torus there, at distance `rho - probe` from the axis, where `rho` is
/// the radius of the circle on which the probe touches both atoms.
#[test]
fn two_atoms_have_a_reentrant_groove() {
    let (r, d, probe) = (1.7, 2.6, 1.4);
    let centers = [[0.0, 0.0, 0.0], [d, 0.0, 0.0]];
    let p = params(probe, 0.2);
    let grid = SesGrid::from_spheres(&centers, &[r, r], &p, &NoProgress).unwrap();
    let mesh = grid.mesh(&NoProgress).unwrap();
    assert_closed_and_oriented(&mesh);

    let big = r + probe;
    let rho = (big * big - d * d / 4.0).sqrt();
    let groove = rho - probe;
    let seam = (r * r - d * d / 4.0).sqrt(); // where the two van der Waals spheres meet
    assert!(
        groove > seam + 0.3,
        "test geometry must have a visible groove"
    );
    let sdf = SignedDistance::new(mesh.vertices.clone(), &mesh.faces);
    for t in 0..12 {
        let a = t as f64 / 12.0 * 2.0 * PI;
        let dir = [0.0, a.cos(), a.sin()];
        let at = |s: f64| add([d / 2.0, 0.0, 0.0], scale(dir, s));
        // The seam is filled in: inside the SES, by (groove - seam).
        assert!((grid.signed_distance(at(seam)) + (groove - seam)).abs() < 0.02);
        assert!(grid.signed_distance(at(groove)).abs() < 0.02);
        assert!(sdf.signed_distance(at(groove)).abs() < 0.02);
        assert!((sdf.signed_distance(at(groove + 0.5)) - 0.5).abs() < 0.03);
    }
    // Exact volume by integrating rings 2πρ over the (x, ρ) half-plane: inside a van der
    // Waals sphere, or under the probe circle where it touches both spheres.
    let half_width = d / 2.0 * probe / big;
    let n = 1500;
    let (x0, x1, r1) = (-r, d + r, r);
    let (dx, dr) = ((x1 - x0) / n as f64, r1 / n as f64);
    let mut exact = 0.0;
    let mut union = 0.0;
    for a in 0..n {
        let x = x0 + (a as f64 + 0.5) * dx;
        let u = x - d / 2.0;
        let groove_top = if u.abs() <= half_width {
            rho - (probe * probe - u * u).sqrt()
        } else {
            f64::NEG_INFINITY
        };
        for b in 0..n {
            let q = (b as f64 + 0.5) * dr;
            let vdw = x * x + q * q <= r * r || (x - d) * (x - d) + q * q <= r * r;
            let ring = 2.0 * PI * q * dx * dr;
            if vdw {
                union += ring;
            }
            if vdw || q <= groove_top {
                exact += ring;
            }
        }
    }
    let v = mesh.volume();
    assert!(exact > union + 0.5, "groove too small to test");
    assert!((v / exact - 1.0).abs() < 0.01, "volume {v} vs {exact}");
}

/// Cavity surfaces are dropped unless asked for.
#[test]
fn closed_cavity_is_filled() {
    // A shell of atoms around an empty core big enough for the probe.
    let dirs = cs_core::geom::fibonacci_sphere(150);
    let centers: Vec<V3> = dirs.iter().map(|d| scale(*d, 6.0)).collect();
    let radii = vec![1.8; centers.len()];
    let mut p = params(1.4, 0.4);
    let filled = surface::ses_spheres(&centers, &radii, &p, &NoProgress).unwrap();
    p.fill_cavities = false;
    let hollow = surface::ses_spheres(&centers, &radii, &p, &NoProgress).unwrap();
    assert_closed_and_oriented(&filled);
    assert_closed_and_oriented(&hollow);
    assert!(hollow.volume() < filled.volume() - 50.0);
    let grid = SesGrid::from_spheres(&centers, &radii, &p, &NoProgress).unwrap();
    assert!(
        grid.signed_distance([0.0; 3]) > 0.0,
        "core is solvent without filling"
    );
    p.fill_cavities = true;
    let grid = SesGrid::from_spheres(&centers, &radii, &p, &NoProgress).unwrap();
    assert!(grid.signed_distance([0.0; 3]) < 0.0, "core is filled");

    // With a mouth wide enough for the probe, the core is a pocket, not a cavity.
    let (open_c, open_r): (Vec<V3>, Vec<f64>) = centers
        .iter()
        .filter(|c| c[2] < 4.5)
        .map(|&c| (c, 1.8))
        .unzip();
    let grid = SesGrid::from_spheres(&open_c, &open_r, &p, &NoProgress).unwrap();
    assert!(grid.signed_distance([0.0; 3]) > 0.0, "pocket stays open");
    assert_closed_and_oriented(&grid.mesh(&NoProgress).unwrap());
}

/// Per-atom radii used by MSMS for the sample, recovered from its vertices: a vertex lies
/// on the surface of the atom it is assigned to or outside it, so the closest one gives the
/// radius. Atoms without vertices take the radius of the same atom name in the same residue
/// type, or the united-atom table.
fn msms_radii(s: &Structure, vert_text: &str) -> Vec<f64> {
    let mut r = vec![f64::INFINITY; s.atoms.len()];
    for line in vert_text.lines().skip(3) {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() < 9 {
            continue;
        }
        let p: V3 = std::array::from_fn(|k| f[k].parse().unwrap());
        let atom = f[7].parse::<usize>().unwrap() - 1;
        r[atom] = r[atom].min(norm(sub(p, s.atoms[atom].pos)));
    }
    let mut by_name: HashMap<(String, String), f64> = HashMap::new();
    for (a, &ri) in s.atoms.iter().zip(&r) {
        if ri.is_finite() {
            by_name.insert((a.res_name.clone(), a.name.clone()), ri);
        }
    }
    s.atoms
        .iter()
        .zip(r)
        .map(|(a, ri)| {
            if ri.is_finite() {
                ri
            } else {
                by_name
                    .get(&(a.res_name.clone(), a.name.clone()))
                    .copied()
                    .unwrap_or_else(|| surface::united_radius(a))
            }
        })
        .collect()
}

struct Stats {
    mean: f64,
    p95: f64,
    max: f64,
}

/// |distance| from `points` to `surface`.
fn stats(points: &[V3], surface: &Mesh) -> Stats {
    let sdf = SignedDistance::new(surface.vertices.clone(), &surface.faces);
    let mut d: Vec<f64> = points
        .iter()
        .map(|&p| sdf.signed_distance(p).abs())
        .collect();
    d.sort_by(f64::total_cmp);
    Stats {
        mean: d.iter().sum::<f64>() / d.len() as f64,
        p95: d[(d.len() - 1) * 95 / 100],
        max: d[d.len() - 1],
    }
}

#[test]
fn matches_msms_on_1ahw() {
    let s = Structure::read(&data_dir().join("complexes/1AHW/1AHW_l_u.pdb")).unwrap();
    let dir = data_dir().join("samples/1AHW_l_u");
    let vert_text = std::fs::read_to_string(dir.join("1AHW_l_u.vert")).unwrap();
    let face_text = std::fs::read_to_string(dir.join("1AHW_l_u.face")).unwrap();
    let reference = Mesh::from_msms(&MsmsSurface::parse(&vert_text, &face_text).unwrap()).unwrap();

    let radii = msms_radii(&s, &vert_text);
    // The united-atom table reproduces every radius MSMS used.
    for (a, &r) in s.atoms.iter().zip(&radii) {
        assert!(
            (surface::united_radius(a) - r).abs() < 0.01,
            "{} {} {}: united {} vs MSMS {r:.3}",
            a.res_name,
            a.res_seq,
            a.name,
            surface::united_radius(a)
        );
    }
    let p = SesParams {
        radii: Radii::United,
        ..params(1.5, 0.5)
    };
    let mesh = surface::ses(&s.atoms, &p, &NoProgress).unwrap();
    assert_closed_and_oriented(&mesh);

    let (area, volume) = (mesh.area(), mesh.volume());
    let (ref_area, ref_volume) = (reference.area(), reference.volume());
    // MSMS vertices lie on the exact surface, so they measure our error directly; our
    // vertices also see the chords of MSMS's coarser triangles.
    let from_ref = stats(&reference.vertices, &mesh);
    let to_ref = stats(&mesh.vertices, &reference);
    eprintln!(
        "1AHW_l_u: area {area:.1} vs {ref_area:.1} Å², volume {volume:.1} vs {ref_volume:.1} Å³\n\
         MSMS vertices -> ours: mean {:.3}, p95 {:.3}, max {:.3} Å\n\
         our vertices -> MSMS: mean {:.3}, p95 {:.3}, max {:.3} Å",
        from_ref.mean, from_ref.p95, from_ref.max, to_ref.mean, to_ref.p95, to_ref.max
    );
    assert!((volume / ref_volume - 1.0).abs() < 0.01);
    assert!((area / ref_area - 1.0).abs() < 0.06);
    assert!(from_ref.mean < 0.03 && from_ref.p95 < 0.08);
    assert!(to_ref.mean < 0.15 && to_ref.p95 < 0.4);
    // No pocket kept or filled differently from MSMS (it has two tiny cavities).
    assert!(from_ref.max < 0.6 && to_ref.max < 1.0);
}

#[test]
fn rejects_bad_input() {
    let p = SesParams::default();
    assert!(surface::ses_spheres(&[], &[], &p, &NoProgress).is_err());
    assert!(surface::ses_spheres(&[[0.0; 3]], &[-1.0], &p, &NoProgress).is_err());
    assert!(surface::ses_spheres(&[[f64::NAN; 3]], &[1.0], &p, &NoProgress).is_err());
    let bad = SesParams {
        grid_spacing: 0.0,
        ..Default::default()
    };
    assert!(bad.validate().is_err());
    // Spheres far apart would need an enormous grid.
    let far = [[0.0; 3], [1e5, 1e5, 1e5]];
    assert!(surface::ses_spheres(&far, &[1.5, 1.5], &p, &NoProgress).is_err());
}
