//! Docking benchmark runner (development tool).
//!
//! ```text
//! cargo run --release -p cs-core --example dock_bench -- <structures dir> <CASE[,CASE...]> \
//!     [--bound] [--seed N] [--params JSON] [--json out.json]
//! ```
//!
//! For each case reads `CASE_r_u.pdb` / `CASE_l_u.pdb` (or `_b` with `--bound`) from the
//! directory, moves the ligand by a seeded random rigid transform, docks it with the atom
//! surface model and reports, against the bound complex (`CASE_r_b.pdb`, `CASE_l_b.pdb`), the
//! ligand Cα RMSD at each stage: best seed of the shape search, after atom re-scoring, and the
//! final clustered ranking (rank of the first pose under 10 Å and under 5 Å).

use std::path::{Path, PathBuf};
use std::time::Instant;

use cs_core::dock::native::NativeReference;
use cs_core::dock::{self, DockInput, DockParams};
use cs_core::geom::transform::{Rigid, Rng};
use cs_core::io::pdb::Structure;
use cs_core::progress::NoProgress;
use serde::Serialize;

#[derive(Serialize)]
struct CaseResult {
    case: String,
    bound: bool,
    receptor_atoms: usize,
    ligand_atoms: usize,
    receptor_points: usize,
    ligand_points: usize,
    best_possible_rmsd: f64,
    /// Best RMSD among all seeds, and the descriptor rank of the first seed under 5 / 10 Å.
    seeds_best_rmsd: f64,
    seeds_rank_10: Option<usize>,
    seeds_rank_5: Option<usize>,
    /// Rank after atom re-scoring (before refinement and clustering).
    rescored_rank_10: Option<usize>,
    rescored_rank_5: Option<usize>,
    /// Final ranking.
    rank_10: Option<usize>,
    rank_5: Option<usize>,
    top1_rmsd: f64,
    best_top10_rmsd: f64,
    seconds_surfaces: f64,
    seconds_docking: f64,
    stats: dock::DockStats,
}

fn first_under(rmsds: &[f64], cut: f64) -> Option<usize> {
    rmsds.iter().position(|r| *r < cut).map(|i| i + 1)
}

fn fmt_rank(r: Option<usize>) -> String {
    r.map(|x| x.to_string()).unwrap_or_else(|| "-".into())
}

fn run_case(dir: &Path, case: &str, bound: bool, seed: u64, params: &DockParams) -> CaseResult {
    let tag = if bound { "b" } else { "u" };
    let read = |what: &str| {
        let p = dir.join(format!("{case}_{what}.pdb"));
        Structure::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
    };
    let receptor = read(&format!("r_{tag}"));
    let ligand0 = read(&format!("l_{tag}"));
    let (rec_native, lig_native) = (read("r_b"), read("l_b"));

    // Random placement: the benchmark files are pre-superposed on the complex.
    let mut rng = Rng::new(seed);
    let random = Rigid {
        rot: rng.rotation(),
        trans: [
            60.0 * rng.next_f64() - 30.0,
            60.0 * rng.next_f64() - 30.0,
            60.0 * rng.next_f64() - 30.0,
        ],
    };
    let ligand = dock::transform_structure(&ligand0, &random);
    let native = NativeReference::new(&receptor, &ligand, &rec_native, &lig_native).unwrap();

    let t0 = Instant::now();
    let surf = dock::AtomSurfaceParams::default();
    let (rec_in, lig_in) = rayon::join(
        || DockInput::from_atoms(receptor.clone(), &surf, &NoProgress).unwrap(),
        || DockInput::from_atoms(ligand.clone(), &surf, &NoProgress).unwrap(),
    );
    let seconds_surfaces = t0.elapsed().as_secs_f64();

    // Stage diagnostics (re-runs the search part of the pipeline with the public pieces).
    let rec_d = dock::descriptor::build(
        rec_in.surface.as_ref(),
        &params.descriptor,
        dock::descriptor::Role::Receptor,
        &NoProgress,
    )
    .unwrap();
    let lig_d = dock::descriptor::build(
        lig_in.surface.as_ref(),
        &params.descriptor,
        dock::descriptor::Role::Ligand,
        &NoProgress,
    )
    .unwrap();
    let seeds = dock::matching::enumerate(
        &rec_d,
        &lig_d,
        params.weights,
        params.per_point,
        params.max_candidates,
        &NoProgress,
        (0.0, 1.0),
    )
    .unwrap();
    let scorer = dock::scoring::Scorer::new(&receptor, &ligand, params.score_grid_spacing).unwrap();
    let mut seed_eval: Vec<(f64, f64)> = seeds
        .iter()
        .map(|m| {
            let (p, q) = (m.p as usize, m.q as usize);
            let t = dock::descriptor::pose(
                rec_d.points[p],
                &rec_d.frames[p],
                lig_d.points[q],
                &lig_d.frames[q],
                m.k as usize,
                params.descriptor.gap,
            );
            (native.ligand_rmsd(&t), scorer.total(&t, &params.screen))
        })
        .collect();
    let seed_rmsd: Vec<f64> = seed_eval.iter().map(|x| x.0).collect();
    seed_eval.sort_by(|a, b| b.1.total_cmp(&a.1));
    let rescored: Vec<f64> = seed_eval.iter().map(|x| x.0).collect();

    let t1 = Instant::now();
    let out = dock::run_with_stats(&rec_in, &lig_in, params, &NoProgress).unwrap();
    let seconds_docking = t1.elapsed().as_secs_f64();
    let rmsd: Vec<f64> = out
        .poses
        .iter()
        .map(|p| native.ligand_rmsd(&p.transform))
        .collect();

    CaseResult {
        case: case.to_string(),
        bound,
        receptor_atoms: receptor.heavy_atoms().count(),
        ligand_atoms: ligand.heavy_atoms().count(),
        receptor_points: out.stats.receptor_points,
        ligand_points: out.stats.ligand_points,
        best_possible_rmsd: native.best_possible_rmsd(),
        seeds_best_rmsd: seed_rmsd.iter().copied().fold(f64::INFINITY, f64::min),
        seeds_rank_10: first_under(&seed_rmsd, 10.0),
        seeds_rank_5: first_under(&seed_rmsd, 5.0),
        rescored_rank_10: first_under(&rescored, 10.0),
        rescored_rank_5: first_under(&rescored, 5.0),
        rank_10: first_under(&rmsd, 10.0),
        rank_5: first_under(&rmsd, 5.0),
        top1_rmsd: rmsd.first().copied().unwrap_or(f64::NAN),
        best_top10_rmsd: rmsd.iter().take(10).copied().fold(f64::INFINITY, f64::min),
        seconds_surfaces,
        seconds_docking,
        stats: out.stats,
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        eprintln!(
            "usage: dock_bench <structures dir> <CASE[,CASE...]> [--bound] [--seed N] [--params JSON] [--json FILE]"
        );
        std::process::exit(2);
    }
    let dir = PathBuf::from(&args[0]);
    let cases: Vec<&str> = args[1].split(',').filter(|s| !s.is_empty()).collect();
    let mut bound = false;
    let mut seed = 1u64;
    let mut params = DockParams::default();
    let mut json_out = None;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--bound" => bound = true,
            "--seed" => {
                i += 1;
                seed = args[i].parse().expect("--seed N");
            }
            "--params" => {
                i += 1;
                // `--params @file.json` reads the JSON from a file.
                let text = match args[i].strip_prefix('@') {
                    Some(path) => std::fs::read_to_string(path).expect("--params @file"),
                    None => args[i].clone(),
                };
                params = serde_json::from_str(&text).expect("--params JSON");
            }
            "--json" => {
                i += 1;
                json_out = Some(PathBuf::from(&args[i]));
            }
            other => panic!("unknown argument {other}"),
        }
        i += 1;
    }

    println!(
        "{:<6} {:>5} {:>5} {:>5} {:>5} {:>5} | {:>6} {:>5} {:>5} | {:>5} {:>5} | {:>5} {:>5} {:>6} {:>6} | {:>5} {:>5}",
        "case",
        "recA",
        "ligA",
        "recP",
        "ligP",
        "floor",
        "seedB",
        "s10",
        "s5",
        "r10",
        "r5",
        "f10",
        "f5",
        "top1",
        "best10",
        "surf",
        "dock"
    );
    let mut results = Vec::new();
    for case in cases {
        let r = run_case(&dir, case, bound, seed, &params);
        println!(
            "{:<6} {:>5} {:>5} {:>5} {:>5} {:>5.1} | {:>6.1} {:>5} {:>5} | {:>5} {:>5} | {:>5} {:>5} {:>6.1} {:>6.1} | {:>5.1} {:>5.1}",
            r.case,
            r.receptor_atoms,
            r.ligand_atoms,
            r.receptor_points,
            r.ligand_points,
            r.best_possible_rmsd,
            r.seeds_best_rmsd,
            fmt_rank(r.seeds_rank_10),
            fmt_rank(r.seeds_rank_5),
            fmt_rank(r.rescored_rank_10),
            fmt_rank(r.rescored_rank_5),
            fmt_rank(r.rank_10),
            fmt_rank(r.rank_5),
            r.top1_rmsd,
            r.best_top10_rmsd,
            r.seconds_surfaces,
            r.seconds_docking
        );
        results.push(r);
    }
    let n = results.len();
    let hits = |f: &dyn Fn(&CaseResult) -> Option<usize>, top: usize| {
        results
            .iter()
            .filter(|r| f(r).is_some_and(|x| x <= top))
            .count()
    };
    println!(
        "summary over {n} cases: <10 Å in top1/10/100: {}/{}/{}; <5 Å in top1/10/100: {}/{}/{}",
        hits(&|r| r.rank_10, 1),
        hits(&|r| r.rank_10, 10),
        hits(&|r| r.rank_10, 100),
        hits(&|r| r.rank_5, 1),
        hits(&|r| r.rank_5, 10),
        hits(&|r| r.rank_5, 100)
    );
    if let Some(p) = json_out {
        std::fs::write(&p, serde_json::to_string_pretty(&results).unwrap()).unwrap();
    }
}
