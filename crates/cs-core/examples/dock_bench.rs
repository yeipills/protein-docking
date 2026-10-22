//! Docking engine on the Docking Benchmark 5.5.
//!
//! ```text
//! cargo run --release -p cs-core --example dock_bench -- [--dir DIR] [--only ID,ID,...]
//!     [--category rigid|medium|difficult] [--ses] [--params JSON|@file] [--seed N]
//!     [--diagnose [--bound]] [--out DIR]
//! ```
//!
//! Default mode runs the `cs_core::eval::bench` harness (randomized ligand, CAPRI/DockQ of the
//! top 100 poses, success rates per category) and writes `report.json` / `report.md` to
//! `--out` (default `out/dock-bench`). Cases run one after another: the docker is already
//! parallel, so the reported docking times are wall times on all cores.
//!
//! `--diagnose` instead reports, per case, where the first acceptable / medium pose appears
//! among the seeds (descriptor order, before deduplication) and in the final ranking: it
//! shows whether the search or the scoring loses the near-native solutions. With `--bound`
//! it redocks the bound structures instead.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

use cs_core::dock::{self, DockInput, DockParams};
use cs_core::eval::Capri;
use cs_core::eval::bench::{self, BenchConfig, Case, Category};
use cs_core::geom::transform::{Rigid, Rng};
use cs_core::io::pdb::Structure;
use cs_core::progress::{NoProgress, Progress};

struct Stderr(AtomicU32);

impl Progress for Stderr {
    fn report(&self, _stage: &str, fraction: f32) {
        let step = (fraction * 20.0) as u32;
        if self.0.fetch_max(step, Ordering::Relaxed) < step {
            eprint!("{:.0}% ", step as f32 * 5.0);
        }
    }
}

fn inputs(receptor: &Structure, ligand: &Structure, ses: bool) -> (DockInput, DockInput) {
    let make = |s: &Structure| {
        if ses {
            DockInput::from_ses(
                s.clone(),
                &cs_core::surface::SesParams::default(),
                &NoProgress,
            )
            .expect("SES surface")
        } else {
            DockInput::from_atoms(s.clone(), &dock::AtomSurfaceParams::default(), &NoProgress)
                .expect("atom surface")
        }
    };
    rayon::join(|| make(receptor), || make(ligand))
}

fn first(classes: &[Capri], at_least: Capri) -> String {
    classes
        .iter()
        .position(|c| *c >= at_least)
        .map(|i| (i + 1).to_string())
        .unwrap_or_else(|| "-".into())
}

/// Stage-by-stage diagnostics of one case.
fn diagnose(case: &Case, dir: &Path, seed: u64, ses: bool, bound: bool, params: &DockParams) {
    let data = case.load(dir).expect("case files");
    // Bound redocking docks the bound partners themselves.
    let (receptor, ligand0) = if bound {
        (&data.receptor_bound, &data.ligand_bound)
    } else {
        (&data.receptor_unbound, &data.ligand_unbound)
    };
    let native =
        cs_core::eval::Native::new(&data.receptor_bound, &data.ligand_bound, receptor, ligand0)
            .expect("native");
    let mut rng = Rng::new(bench::case_seed(seed, &case.id));
    let random = bench::randomize(receptor, ligand0, 10.0, &mut rng);
    let ligand = bench::transformed(ligand0, &random);
    let t0 = Instant::now();
    let (rec_in, lig_in) = inputs(receptor, &ligand, ses);
    let t_surf = t0.elapsed().as_secs_f64();

    // Seeds in descriptor order (public pipeline pieces).
    let rd = dock::descriptor::build(
        rec_in.surface.as_ref(),
        &params.descriptor,
        dock::descriptor::Role::Receptor,
        &NoProgress,
    )
    .unwrap();
    let ld = dock::descriptor::build(
        lig_in.surface.as_ref(),
        &params.descriptor,
        dock::descriptor::Role::Ligand,
        &NoProgress,
    )
    .unwrap();
    let seeds = dock::matching::enumerate(
        &rd,
        &ld,
        params.weights,
        params.per_point,
        params.max_candidates,
        &NoProgress,
        (0.0, 1.0),
    )
    .unwrap();
    let poses: Vec<Rigid> = seeds
        .iter()
        .map(|m| {
            let (p, q) = (m.p as usize, m.q as usize);
            dock::descriptor::pose(
                rd.points[p],
                &rd.frames[p],
                ld.points[q],
                &ld.frames[q],
                m.k as usize,
                params.descriptor.gap,
            )
        })
        .collect();
    let to_orig: Vec<Rigid> = poses.iter().map(|t| t.compose(&random)).collect();
    let seed_classes: Vec<Capri> = native
        .evaluate_many(&to_orig)
        .iter()
        .map(|m| m.capri)
        .collect();
    let t1 = Instant::now();
    let out = dock::run_with_stats(&rec_in, &lig_in, params, &NoProgress).unwrap();
    let t_dock = t1.elapsed().as_secs_f64();
    let finals: Vec<Rigid> = out
        .poses
        .iter()
        .map(|p| p.transform.compose(&random))
        .collect();
    let fm = native.evaluate_many(&finals);
    let fc: Vec<Capri> = fm.iter().map(|m| m.capri).collect();
    let best10 = fm.iter().take(10).map(|m| m.dockq).fold(0.0, f64::max);
    let ceiling = native.evaluate(&native.optimal_pose());
    println!(
        "{:<5} {:<3} {:>5} {:>5} {:>5.2} | {:>6} {:>6} | {:>4} {:>4} {:>5.2} {:>5.2} | {:>4.1} {:>5.1}",
        case.id,
        case.kind,
        out.stats.receptor_points,
        out.stats.ligand_points,
        ceiling.dockq,
        first(&seed_classes, Capri::Acceptable),
        first(&seed_classes, Capri::Medium),
        first(&fc, Capri::Acceptable),
        first(&fc, Capri::Medium),
        fm.first().map(|m| m.dockq).unwrap_or(0.0),
        best10,
        t_surf,
        t_dock
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut cases_path = root.join("tools/benchmark/cases.tsv");
    let mut dir = std::env::var("CS_BENCHMARK_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| root.join("data/benchmark/benchmark5.5/structures"));
    let mut out = PathBuf::from("out/dock-bench");
    let mut only: Option<Vec<String>> = None;
    let mut category: Option<Category> = None;
    let mut ses = false;
    let mut diag = false;
    let mut bound = false;
    let mut params = DockParams::default();
    let mut config = BenchConfig::default();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut val = || args.next().ok_or(format!("{a} needs a value"));
        match a.as_str() {
            "--cases" => cases_path = val()?.into(),
            "--dir" => dir = val()?.into(),
            "--out" => out = val()?.into(),
            "--only" => only = Some(val()?.split(',').map(str::to_string).collect()),
            "--category" => category = Some(Category::parse(&val()?).ok_or("invalid category")?),
            "--seed" => config.seed = val()?.parse()?,
            "--ses" => ses = true,
            "--diagnose" => diag = true,
            "--bound" => bound = true,
            "--params" => {
                let v = val()?;
                // `@file.json` reads the JSON from a file.
                let text = match v.strip_prefix('@') {
                    Some(path) => std::fs::read_to_string(path)?,
                    None => v,
                };
                params = serde_json::from_str(&text)?;
            }
            _ => return Err(format!("unknown argument {a}").into()),
        }
    }
    let cases: Vec<Case> = bench::load_cases(&cases_path)?
        .into_iter()
        .filter(|c| only.as_ref().is_none_or(|o| o.contains(&c.id)))
        .filter(|c| category.is_none_or(|k| c.category == k))
        .collect();
    eprintln!(
        "{} cases, surface {}",
        cases.len(),
        if ses { "ses" } else { "atoms" }
    );

    if diag {
        println!(
            "{:<5} {:<3} {:>5} {:>5} {:>5} | {:>6} {:>6} | {:>4} {:>4} {:>5} {:>5} | {:>4} {:>5}",
            "case",
            "typ",
            "recP",
            "ligP",
            "ceil",
            "seedA",
            "seedM",
            "finA",
            "finM",
            "dq1",
            "dq10",
            "surf",
            "dock"
        );
        for case in &cases {
            diagnose(case, &dir, config.seed, ses, bound, &params);
        }
        return Ok(());
    }

    let docker = |receptor: &Structure, ligand: &Structure, _: &bench::DockInput| -> Vec<Rigid> {
        let (r, l) = inputs(receptor, ligand, ses);
        match dock::run(&r, &l, &params, &NoProgress) {
            Ok(poses) => poses.into_iter().map(|p| p.transform).collect(),
            Err(e) => {
                eprintln!("docking failed: {e}");
                Vec::new()
            }
        }
    };
    let name = if ses {
        "context-shapes (SES mesh)"
    } else {
        "context-shapes (atom surface)"
    };
    let report = bench::run(
        name,
        &cases,
        &dir,
        &config,
        docker,
        &Stderr(AtomicU32::new(0)),
    )?;
    eprintln!();
    std::fs::create_dir_all(&out)?;
    std::fs::write(out.join("report.json"), report.to_json())?;
    std::fs::write(out.join("report.md"), report.to_markdown())?;
    for line in report
        .to_markdown()
        .lines()
        .take_while(|l| !l.starts_with("## Cases"))
    {
        println!("{line}");
    }
    eprintln!("wrote {}", out.display());
    Ok(())
}
