//! End-to-end tests of the docking engine.
//!
//! The synthetic and surface tests run in a few seconds in debug builds. Redocking real
//! proteins is slow without optimizations, so those tests are `#[ignore]`d; run them with
//! `cargo test --release -p cs-core --test dock -- --ignored`.

use std::path::PathBuf;

use cs_core::dock::{
    self, AtomSurface, AtomSurfaceParams, DescriptorParams, DockInput, DockParams, MeshSurface,
    Surface,
};
use cs_core::eval::{Capri, Native};
use cs_core::geom::transform::{Rigid, Rng};
use cs_core::geom::vec3::*;
use cs_core::io::msms::MsmsSurface;
use cs_core::io::pdb::{Atom, Structure};
use cs_core::mesh::Mesh;
use cs_core::progress::NoProgress;

fn data(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data")
        .join(rel)
}

fn atom(pos: V3, serial: i32) -> Atom {
    Atom {
        serial,
        name: "CB".into(),
        alt_loc: ' ',
        res_name: "LEU".into(),
        chain: 'A',
        res_seq: serial,
        i_code: ' ',
        pos,
        occupancy: 1.0,
        b_factor: 0.0,
        element: "C".into(),
        hetatm: false,
    }
}

/// A slab with a hemispherical pocket in its top face (receptor) and a disc with a
/// hemispherical knob underneath that fills the pocket (ligand), in their native pose.
fn pocket_and_knob() -> (Structure, Structure) {
    let step = 1.5;
    // Ligand: a disc (layers z = 3.0 and 4.5) with a knob hanging below it (atoms within
    // 2.6 Å of the origin). Receptor: a slab whose top layer is 3.6 Å below the disc (a
    // clean contact), with every atom within 6.2 Å of the origin removed (a pocket that
    // takes the knob with the same contact distance).
    let mut rec = Vec::new();
    for i in -8..=8 {
        for j in -8..=8 {
            for k in -6..=0 {
                let p = [i as f64 * step, j as f64 * step, k as f64 * step - 0.6];
                if norm(p) > 6.2 {
                    rec.push(atom(p, rec.len() as i32 + 1));
                }
            }
        }
    }
    let mut lig = Vec::new();
    for i in -6..=6 {
        for j in -6..=6 {
            for k in -2..=3 {
                let p = [i as f64 * step, j as f64 * step, k as f64 * step];
                let knob = p[2] < 3.0 && norm(p) <= 2.6;
                let disc = p[2] >= 3.0 && (p[0] * p[0] + p[1] * p[1]) <= 64.0;
                if knob || disc {
                    lig.push(atom(p, lig.len() as i32 + 1));
                }
            }
        }
    }
    (Structure { atoms: rec }, Structure { atoms: lig })
}

/// Small, fast settings for debug-mode tests.
fn quick_params() -> DockParams {
    DockParams {
        descriptor: DescriptorParams {
            spacing: 3.5,
            ring_radii: vec![2.0, 4.0, 6.0, 8.0],
            heights: vec![-3.0, -1.0, 1.0, 3.0],
            tilt_angles: vec![12.0],
            tilt_azimuths: 4,
            ..Default::default()
        },
        per_point: 10,
        max_candidates: 3000,
        quick_top: 60,
        refine_top: 15,
        n_poses: 10,
        ..Default::default()
    }
}

fn rmsd(a: &[V3], b: &[V3]) -> f64 {
    (a.iter().zip(b).map(|(x, y)| dist2(*x, *y)).sum::<f64>() / a.len() as f64).sqrt()
}

fn random_placement(seed: u64) -> Rigid {
    let mut rng = Rng::new(seed);
    Rigid {
        rot: rng.rotation(),
        trans: [30.0, -12.0, 18.0],
    }
}

#[test]
fn knob_docks_into_pocket() {
    let (rec, lig_native) = pocket_and_knob();
    let random = random_placement(3);
    let lig = dock::transform_structure(&lig_native, &random);
    let surf = AtomSurfaceParams::default();
    let r = DockInput::from_atoms(rec, &surf, &NoProgress).unwrap();
    let l = DockInput::from_atoms(lig.clone(), &surf, &NoProgress).unwrap();
    let poses = dock::run(&r, &l, &quick_params(), &NoProgress).unwrap();
    assert!(!poses.is_empty());
    let native: Vec<V3> = lig_native.atoms.iter().map(|a| a.pos).collect();
    let err = |p: &dock::Pose| {
        let placed: Vec<V3> = lig.atoms.iter().map(|a| p.transform.apply(a.pos)).collect();
        rmsd(&placed, &native)
    };
    // The disc can spin about the pocket axis (the shape is nearly symmetric), so judge the
    // knob placement by the centre of the knob, plus a loose RMSD.
    let knob_centre = |p: &dock::Pose| norm(p.transform.apply(random.apply([0.0, 0.0, 0.0])));
    let best = &poses[0];
    // Lattice-shaped bodies can interdigitate: half a lattice step away from the drawn
    // native the atom score is higher (more contacts), so allow ~2 Å on the knob centre.
    assert!(
        knob_centre(best) < 3.0,
        "top pose puts the knob {:.2} Å from the pocket (rmsd {:.2}, {:?})",
        knob_centre(best),
        err(best),
        best.components
    );
    assert!(poses.windows(2).all(|w| w[0].score >= w[1].score));
    assert!(poses.iter().enumerate().all(|(i, p)| p.rank == i + 1));
}

#[test]
fn docking_is_deterministic() {
    let (rec, lig) = pocket_and_knob();
    let lig = dock::transform_structure(&lig, &random_placement(5));
    let surf = AtomSurfaceParams::default();
    let r = DockInput::from_atoms(rec, &surf, &NoProgress).unwrap();
    let l = DockInput::from_atoms(lig, &surf, &NoProgress).unwrap();
    let params = DockParams {
        n_poses: 5,
        ..quick_params()
    };
    let a = dock::run(&r, &l, &params, &NoProgress).unwrap();
    let b = dock::run(&r, &l, &params, &NoProgress).unwrap();
    assert_eq!(
        serde_json::to_string(&a).unwrap(),
        serde_json::to_string(&b).unwrap()
    );
}

#[test]
fn complex_pdb_contains_both_partners() {
    let (rec, lig) = pocket_and_knob();
    let t = Rigid::translation([1.0, 2.0, 3.0]);
    let text = dock::complex_pdb(&rec, &lig, &t);
    let back = Structure::parse(&text).unwrap();
    assert_eq!(back.atoms.len(), rec.atoms.len() + lig.atoms.len());
    let moved = &back.atoms[rec.atoms.len()];
    assert!(dist2(moved.pos, t.apply(lig.atoms[0].pos)) < 1e-5);
    assert_eq!(text.matches("TER").count(), 2);
}

#[test]
fn cancellation_stops_the_run() {
    struct Cancelled;
    impl cs_core::progress::Progress for Cancelled {
        fn report(&self, _: &str, _: f32) {}
        fn is_cancelled(&self) -> bool {
            true
        }
    }
    let (rec, lig) = pocket_and_knob();
    let surf = AtomSurfaceParams::default();
    let r = DockInput::from_atoms(rec, &surf, &NoProgress).unwrap();
    let l = DockInput::from_atoms(lig, &surf, &NoProgress).unwrap();
    let err = dock::run(&r, &l, &quick_params(), &Cancelled).unwrap_err();
    assert!(matches!(err, cs_core::Error::Cancelled));
}

fn msms_mesh() -> Mesh {
    let d = data("samples/1AHW_l_u");
    let s = MsmsSurface::read(&d.join("1AHW_l_u.vert"), &d.join("1AHW_l_u.face")).unwrap();
    Mesh::from_msms(&s).unwrap()
}

/// The atom model, the MSMS mesh, the built-in SES mesh and the SES grid + mesh surface
/// describe the same surface:
/// points 1 Å off the MSMS surface along its normals get the same inside/outside verdict.
#[test]
fn surface_models_agree_on_1ahw_ligand() {
    let s = Structure::read(&data("complexes/1AHW/1AHW_l_u.pdb")).unwrap();
    let atoms = AtomSurface::new(&s.atoms, &AtomSurfaceParams::default(), &NoProgress).unwrap();
    let mesh = msms_mesh();
    let msms = MeshSurface::new(mesh.clone());
    let ses = MeshSurface::new(
        cs_core::surface::ses(
            &s.atoms,
            &cs_core::surface::SesParams::default(),
            &NoProgress,
        )
        .unwrap(),
    );
    let hybrid = dock::SesSurface::new(
        &s.atoms,
        &cs_core::surface::SesParams::default(),
        &NoProgress,
    )
    .unwrap();
    let mut agree = [0usize; 3];
    let mut n = 0;
    for (v, nrm) in mesh.vertices.iter().zip(&mesh.normals).step_by(37) {
        for off in [-1.0, 1.0] {
            let p = add(*v, scale(*nrm, off));
            let reference = msms.signed_distance(p) > 0.0;
            n += 1;
            for (i, model) in [&atoms as &dyn Surface, &ses, &hybrid].iter().enumerate() {
                if (model.signed_distance(p) > 0.0) == reference {
                    agree[i] += 1;
                }
            }
        }
    }
    // MSMS used united-atom radii, the others Bondi radii: a few points near crevices differ.
    for (i, a) in agree.iter().enumerate() {
        assert!(*a as f64 > 0.93 * n as f64, "model {i}: {a}/{n}");
    }
    // Surface points with unit outward normals.
    let pts = atoms.surface_points();
    assert!(pts.len() > 10_000);
    let mut near = 0;
    for p in pts.iter().step_by(101) {
        assert!((norm(p.normal) - 1.0).abs() < 1e-9);
        if msms.signed_distance(p.pos).abs() < 1.0 {
            near += 1;
        }
    }
    let total = pts.len().div_ceil(101);
    assert!(near as f64 > 0.95 * total as f64, "{near}/{total}");
}

/// Docks the ligand of 1AHW from the random start `seed` (same placement as
/// `examples/dock.rs --randomize seed`) and returns the rank of the first pose of at least
/// `class`.
fn redock_1ahw(bound: bool, ses: bool, class: Capri, seed: u64) -> Option<usize> {
    let tag = if bound { "b" } else { "u" };
    let read = |n: &str| Structure::read(&data(&format!("complexes/1AHW/1AHW_{n}.pdb"))).unwrap();
    let (rec, lig0) = (read(&format!("r_{tag}")), read(&format!("l_{tag}")));
    let native = Native::new(&read("r_b"), &read("l_b"), &rec, &lig0).unwrap();
    let mut rng = Rng::new(seed);
    let n = lig0.atoms.len() as f64;
    let c = lig0
        .atoms
        .iter()
        .fold([0.0; 3], |c, a| add(c, scale(a.pos, 1.0 / n)));
    let random = Rigid::about(c, rng.rotation()).compose(&Rigid::translation([25.0, -15.0, 30.0]));
    let lig = dock::transform_structure(&lig0, &random);
    let make = |s: &Structure| {
        if ses {
            DockInput::from_ses(
                s.clone(),
                &cs_core::surface::SesParams::default(),
                &NoProgress,
            )
            .unwrap()
        } else {
            DockInput::from_atoms(s.clone(), &AtomSurfaceParams::default(), &NoProgress).unwrap()
        }
    };
    let poses = dock::run(
        &make(&rec),
        &make(&lig),
        &DockParams::default(),
        &NoProgress,
    )
    .unwrap();
    let transforms: Vec<Rigid> = poses.iter().map(|p| p.transform.compose(&random)).collect();
    native
        .evaluate_many(&transforms)
        .iter()
        .position(|m| m.capri >= class)
        .map(|i| i + 1)
}

// The result depends on the starting orientation of the ligand (critical points and frames
// move with it), so each slow test docks from three random starts.

#[test]
#[ignore = "slow in debug builds; run with --release -- --ignored"]
fn bound_redocking_1ahw_finds_native() {
    let ranks: Vec<_> = (1..=3)
        .map(|s| redock_1ahw(true, false, Capri::Medium, s))
        .collect();
    let hits = ranks.iter().filter(|r| r.is_some_and(|r| r <= 10)).count();
    assert!(hits >= 2, "first medium pose per start: {ranks:?}");
}

#[test]
#[ignore = "slow in debug builds; run with --release -- --ignored"]
fn bound_redocking_1ahw_with_ses_surface() {
    let ranks: Vec<_> = (1..=3)
        .map(|s| redock_1ahw(true, true, Capri::Acceptable, s))
        .collect();
    assert!(
        ranks.iter().any(|r| r.is_some_and(|r| r <= 100)),
        "first acceptable pose per start: {ranks:?}"
    );
}

#[test]
#[ignore = "slow in debug builds; run with --release -- --ignored"]
fn unbound_docking_1ahw_finds_acceptable() {
    let ranks: Vec<_> = (1..=3)
        .map(|s| redock_1ahw(false, false, Capri::Acceptable, s))
        .collect();
    assert!(
        ranks.iter().any(|r| r.is_some_and(|r| r <= 100)),
        "first acceptable pose per start: {ranks:?}"
    );
}
