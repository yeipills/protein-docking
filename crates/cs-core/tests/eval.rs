//! Docking evaluation and benchmark harness.
//!
//! `data/dockq_reference.tsv` holds ligand poses of nine benchmark complexes and the
//! values computed for them by the official DockQ v2 (`tools/benchmark/validate_dockq.py`).
//! 1AHW is in the repository; the other complexes are checked only when the benchmark has
//! been downloaded (`tools/benchmark/fetch.sh`, or `CS_BENCHMARK_DIR`).

use std::path::{Path, PathBuf};

use cs_core::eval::align::ChainPair;
use cs_core::eval::bench::{self, BenchConfig, Case, fake};
use cs_core::eval::{Capri, Metrics, Native};
use cs_core::geom::transform::{Rigid, Rng};
use cs_core::io::pdb::{Atom, Structure};
use cs_core::progress::NoProgress;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn repo_complex_dir() -> PathBuf {
    root().join("data/complexes/1AHW")
}

fn benchmark_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("CS_BENCHMARK_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root().join("data/benchmark/benchmark5.5/structures"));
    dir.join("1AHW_r_u.pdb").exists().then_some(dir)
}

struct Complex {
    rb: Structure,
    lb: Structure,
    ru: Structure,
    lu: Structure,
}

fn load(dir: &Path, id: &str) -> Complex {
    let read = |s: &str| Structure::read(&dir.join(format!("{id}_{s}.pdb"))).unwrap();
    Complex {
        rb: read("r_b"),
        lb: read("l_b"),
        ru: read("r_u"),
        lu: read("l_u"),
    }
}

impl Complex {
    fn native(&self) -> Native {
        Native::new(&self.rb, &self.lb, &self.ru, &self.lu).unwrap()
    }
}

fn moved(s: &Structure, t: &Rigid) -> Structure {
    bench::transformed(s, t)
}

struct Reference {
    case: String,
    pose: Rigid,
    /// fnat, fnonnat, iRMSD, LRMSD, DockQ from the official DockQ.
    values: [f64; 5],
}

fn references() -> Vec<Reference> {
    let text =
        std::fs::read_to_string(root().join("crates/cs-core/tests/data/dockq_reference.tsv"))
            .unwrap();
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("case\t"))
        .map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            let nums = |s: &str| -> Vec<f64> { s.split(',').map(|x| x.parse().unwrap()).collect() };
            let r = nums(f[2]);
            let t = nums(f[3]);
            Reference {
                case: f[0].to_string(),
                pose: Rigid {
                    rot: [[r[0], r[1], r[2]], [r[3], r[4], r[5]], [r[6], r[7], r[8]]],
                    trans: [t[0], t[1], t[2]],
                },
                values: [4, 5, 6, 7, 8].map(|i| f[i].parse().unwrap()),
            }
        })
        .collect()
}

fn values(m: &Metrics) -> [f64; 5] {
    [m.fnat, m.fnonnat, m.irmsd, m.lrmsd, m.dockq]
}

fn same_chains(x: &[ChainPair], y: &[ChainPair]) -> bool {
    x.iter()
        .map(|p| (p.a, p.b))
        .eq(y.iter().map(|p| (p.a, p.b)))
}

/// Compares our metrics with DockQ for every reference pose of `case`.
///
/// DockQ read the poses from PDB files, i.e. with coordinates rounded to 0.001 Å, and with
/// the first chain mapping. Scoring the same rounded coordinates must agree to within
/// DockQ's single-precision arithmetic. Scoring the exact pose may differ slightly: rounding
/// can move a residue pair across the 5 Å contact cut-off.
fn check_against_dockq(dir: &Path, case: &str) -> usize {
    let c = load(dir, case);
    let native = c.native();
    let (rec0, lig0) = native.chain_mappings().next().unwrap();
    let (rec0, lig0) = (rec0.to_vec(), lig0.to_vec());
    let mut n = 0;
    for r in references().iter().filter(|r| r.case == case) {
        let rounded = Structure {
            atoms: moved(&c.lu, &r.pose)
                .atoms
                .into_iter()
                .map(|a| Atom {
                    pos: a.pos.map(|x| format!("{x:.3}").parse().unwrap()),
                    ..a
                })
                .collect(),
        };
        let as_read = Native::new(&c.rb, &c.lb, &c.ru, &rounded).unwrap();
        let k = as_read
            .chain_mappings()
            .position(|(rc, lc)| same_chains(rc, &rec0) && same_chains(lc, &lig0))
            .unwrap();
        let ours = values(&as_read.evaluate_each(&Rigid::IDENTITY)[k]);
        // fnat, fnonnat (exact ratios), iRMSD, LRMSD (Å), DockQ.
        let tol = [1e-5, 1e-5, 1e-3, 1e-3, 1e-4];
        for i in 0..5 {
            assert!(
                (ours[i] - r.values[i]).abs() <= tol[i],
                "{case}: metric {i} ours {} vs DockQ {} (rounded coordinates)",
                ours[i],
                r.values[i]
            );
        }

        let exact = native.evaluate_each(&r.pose)[0];
        let one_contact = 1.0 / exact.nat_total as f64 + 1e-9;
        assert!(
            (exact.fnat - r.values[0]).abs() <= one_contact,
            "{case}: exact fnat"
        );
        assert!(
            (exact.irmsd - r.values[2]).abs() < 5e-3,
            "{case}: exact iRMSD"
        );
        assert!(
            (exact.lrmsd - r.values[3]).abs() < 5e-3,
            "{case}: exact LRMSD"
        );
        assert!(
            (exact.dockq - r.values[4]).abs() < 0.01,
            "{case}: exact DockQ"
        );
        // The reported value is the best over chain mappings.
        assert!(native.evaluate(&r.pose).dockq >= exact.dockq);
        n += 1;
    }
    n
}

#[test]
fn matches_official_dockq_on_1ahw() {
    assert_eq!(check_against_dockq(&repo_complex_dir(), "1AHW"), 11);
}

#[test]
fn matches_official_dockq_on_benchmark_cases() {
    let Some(dir) = benchmark_dir() else {
        eprintln!("benchmark not downloaded (tools/benchmark/fetch.sh): skipping");
        return;
    };
    let mut cases: Vec<String> = references().into_iter().map(|r| r.case).collect();
    cases.dedup();
    for case in cases.iter().filter(|c| *c != "1AHW") {
        assert_eq!(check_against_dockq(&dir, case), 11, "{case}");
    }
}

#[test]
fn native_against_itself_is_perfect() {
    let c = load(&repo_complex_dir(), "1AHW");
    let native = Native::new(&c.rb, &c.lb, &c.rb, &c.lb).unwrap();
    let m = native.evaluate(&Rigid::IDENTITY);
    assert_eq!((m.fnat, m.fnonnat, m.capri), (1.0, 0.0, Capri::High));
    assert!(m.irmsd < 1e-9 && m.lrmsd < 1e-9 && (m.dockq - 1.0).abs() < 1e-9);
    let info = native.info();
    assert_eq!((info.receptor_matched, info.ligand_matched), (428, 200));
    assert_eq!(info.native_contacts, 73);
}

#[test]
fn unbound_1ahw_mapping_and_reference_pose() {
    let c = load(&repo_complex_dir(), "1AHW");
    let native = c.native();
    let info = native.info();
    // Bound chains A, B (Fab), C (tissue factor) vs unbound L, H and A.
    let pairs = |p: &[ChainPair]| p.iter().map(|p| (p.a, p.b)).collect::<Vec<_>>();
    assert_eq!(pairs(&info.receptor_chains), vec![('A', 'L'), ('B', 'H')]);
    assert_eq!(pairs(&info.ligand_chains), vec![('C', 'A')]);
    assert_eq!(
        info.ligand_matched, 199,
        "one bound residue is missing in the unbound file"
    );
    // The reference pose is (almost) the benchmark's pre-superposed placement.
    let best = native.evaluate(&native.optimal_pose());
    let given = native.evaluate(&Rigid::IDENTITY);
    assert_eq!(best.capri, Capri::High);
    assert!(best.irmsd <= given.irmsd + 1e-6);
}

#[test]
fn metrics_ignore_frames_chain_ids_and_numbering() {
    let c = load(&repo_complex_dir(), "1AHW");
    let mut rng = Rng::new(11);
    let pose = Rigid::about(
        [10.0, 5.0, 40.0],
        Rigid::axis_angle([0.3, 1.0, 0.2], 0.2).rot,
    );
    let reference = c.native().evaluate(&pose);

    // Move the whole model, rename its chains and renumber its residues.
    let g = Rigid {
        rot: rng.rotation(),
        trans: [30.0, -12.0, 7.0],
    };
    let relabel = |s: &Structure, chains: &str| Structure {
        atoms: s
            .atoms
            .iter()
            .map(|a| Atom {
                pos: g.apply(a.pos),
                chain: chains
                    .chars()
                    .nth(s.chains().iter().position(|&x| x == a.chain).unwrap())
                    .unwrap(),
                res_seq: a.res_seq + 500,
                ..a.clone()
            })
            .collect(),
    };
    let posed_ligand = moved(&c.lu, &pose);
    let native = Native::new(
        &c.rb,
        &c.lb,
        &relabel(&c.ru, "XY"),
        &relabel(&posed_ligand, "Z"),
    )
    .unwrap();
    let m = native.evaluate(&Rigid::IDENTITY);
    assert_eq!(
        (m.nat_correct, m.model_contacts),
        (reference.nat_correct, reference.model_contacts)
    );
    assert!((m.irmsd - reference.irmsd).abs() < 1e-9);
    assert!((m.lrmsd - reference.lrmsd).abs() < 1e-9);

    // A pose of a pre-moved ligand file is the same placement composed with the move.
    let pre = Rigid {
        rot: rng.rotation(),
        trans: [-5.0, 9.0, 1.0],
    };
    let native2 = Native::new(&c.rb, &c.lb, &c.ru, &moved(&c.lu, &pre)).unwrap();
    let m2 = native2.evaluate(&pose.compose(&pre.inverse()));
    assert!((m2.dockq - reference.dockq).abs() < 1e-9);
}

#[test]
fn evaluates_many_poses_quickly() {
    let c = load(&repo_complex_dir(), "1AHW");
    let native = c.native();
    let mut rng = Rng::new(3);
    let poses: Vec<Rigid> = (0..200)
        .map(|_| {
            let rot = Rigid::about([0.0; 3], rng.rotation());
            Rigid::translation([rng.next_f64() * 20.0, 0.0, 0.0]).compose(&rot)
        })
        .collect();
    let all = native.evaluate_many(&poses);
    assert_eq!(all.len(), 200);
    for (p, m) in poses.iter().zip(&all).take(10) {
        assert_eq!(native.evaluate(p), *m);
    }
}

fn case_1ahw() -> Case {
    bench::load_cases(&root().join("tools/benchmark/cases.tsv"))
        .unwrap()
        .into_iter()
        .find(|c| c.id == "1AHW")
        .unwrap()
}

#[test]
fn case_table_covers_benchmark_5_5() {
    let cases = bench::load_cases(&root().join("tools/benchmark/cases.tsv")).unwrap();
    assert_eq!(cases.len(), 257);
    let count = |c: bench::Category| cases.iter().filter(|x| x.category == c).count();
    assert_eq!(
        (
            count(bench::Category::Rigid),
            count(bench::Category::Medium),
            count(bench::Category::Difficult)
        ),
        (162, 60, 35)
    );
    let mut ids: Vec<&str> = cases.iter().map(|c| c.id.as_str()).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 257, "file prefixes are unique");
    if let Some(dir) = benchmark_dir() {
        for c in &cases {
            let p = c.paths(&dir);
            for f in [
                p.receptor_unbound,
                p.ligand_unbound,
                p.receptor_bound,
                p.ligand_bound,
            ] {
                assert!(f.exists(), "{}", f.display());
            }
        }
    }
}

#[test]
fn harness_with_oracle_and_random_dockers() {
    let dir = repo_complex_dir();
    let cases = vec![case_1ahw()];
    let config = BenchConfig::default();

    let oracle = bench::run(
        "oracle",
        &cases,
        &dir,
        &config,
        fake::oracle(dir.clone(), 5.0, 1.0, 20),
        &NoProgress,
    )
    .unwrap();
    let r = &oracle.cases[0];
    assert!(r.error.is_none());
    assert_eq!(r.poses, 21);
    assert_eq!(r.first_hit.acceptable, Some(1), "{r:?}");
    assert_eq!(r.first_hit.medium, Some(1));
    assert!(r.top1.unwrap().dockq > 0.6);
    assert_eq!(r.ceiling.unwrap().capri, Capri::High);
    let all = oracle.summary.iter().find(|s| s.category == "all").unwrap();
    assert_eq!(
        (all.cases, all.success[0].top, all.success[0].acceptable),
        (1, 1, 1)
    );

    let random = bench::run(
        "random",
        &cases,
        &dir,
        &config,
        fake::random(100),
        &NoProgress,
    )
    .unwrap();
    let r = &random.cases[0];
    assert_eq!(r.poses, 100);
    assert_eq!(r.first_hit.acceptable, None, "{r:?}");
    assert!(r.best.unwrap().metrics.dockq < 0.23);

    // Returning the randomized ligand as is must not score: the harness hid the native pose.
    let lazy = |_: &Structure, _: &Structure, _: &bench::DockInput| vec![Rigid::IDENTITY];
    let r = bench::run_case(&cases[0], &dir, &config, &lazy);
    assert_eq!(r.top1.unwrap().capri, Capri::Incorrect);
    // …while undoing the randomization recovers the benchmark's pre-superposed pose.
    let undo = |_: &Structure, _: &Structure, input: &bench::DockInput| {
        let mut rng = Rng::new(bench::case_seed(config.seed, &input.case.id));
        let c = load(&dir, "1AHW");
        vec![bench::randomize(&c.ru, &c.lu, config.start_gap, &mut rng).inverse()]
    };
    let r = bench::run_case(&cases[0], &dir, &config, &undo);
    let given = load(&dir, "1AHW").native().evaluate(&Rigid::IDENTITY);
    assert!((r.top1.unwrap().dockq - given.dockq).abs() < 1e-9);

    let md = oracle.to_markdown();
    assert!(md.contains("| all | 1 | 100 / 100 /"), "{md}");
    let json: serde_json::Value = serde_json::from_str(&oracle.to_json()).unwrap();
    assert_eq!(json["cases"][0]["id"], "1AHW");
}

#[test]
fn harness_records_missing_files_as_errors() {
    let mut case = case_1ahw();
    case.id = "NONE".into();
    let report = bench::run(
        "random",
        &[case],
        &repo_complex_dir(),
        &BenchConfig::default(),
        fake::random(5),
        &NoProgress,
    )
    .unwrap();
    assert!(report.cases[0].error.is_some());
    assert_eq!(report.summary.last().unwrap().errors, 1);
}
