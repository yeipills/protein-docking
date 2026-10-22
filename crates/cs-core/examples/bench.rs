//! Runs the benchmark harness with a reference docker (oracle or random) and writes
//! `report.json` and `report.md`.
//!
//! ```text
//! tools/benchmark/fetch.sh
//! cargo run --release -p cs-core --example bench -- --docker oracle --out out/bench-oracle
//! cargo run --release -p cs-core --example bench -- --docker random --out out/bench-random
//! ```
//!
//! Options: `--cases FILE` (default `tools/benchmark/cases.tsv`), `--dir DIR` (default
//! `data/benchmark/benchmark5.5/structures`), `--only ID,ID,…`, `--category rigid|medium|difficult`,
//! `--seed N`, `--top N`, `--poses N` (random poses per case), `--sequential`.
//!
//! A real docker plugs into `cs_core::eval::bench::run` the same way: any
//! `Fn(&receptor, &ligand, &DockInput) -> Vec<Rigid>`.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use cs_core::eval::bench::{self, BenchConfig, Category, fake};
use cs_core::progress::Progress;

/// Prints progress to stderr in steps of 10 %.
struct Stderr(AtomicU32);

impl Progress for Stderr {
    fn report(&self, _stage: &str, fraction: f32) {
        let step = (fraction * 10.0) as u32;
        if self.0.fetch_max(step, Ordering::Relaxed) < step {
            eprintln!("{:3} %", step * 10);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut cases_path = root.join("tools/benchmark/cases.tsv");
    let mut dir = root.join("data/benchmark/benchmark5.5/structures");
    let mut out = PathBuf::from("out/bench");
    let mut docker = String::from("oracle");
    let mut only: Option<Vec<String>> = None;
    let mut category: Option<Category> = None;
    let mut poses = 100;
    let mut config = BenchConfig {
        parallel_cases: true,
        ..BenchConfig::default()
    };
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut val = || args.next().ok_or(format!("{a} needs a value"));
        match a.as_str() {
            "--docker" => docker = val()?,
            "--cases" => cases_path = val()?.into(),
            "--dir" => dir = val()?.into(),
            "--out" => out = val()?.into(),
            "--only" => only = Some(val()?.split(',').map(str::to_string).collect()),
            "--category" => category = Some(Category::parse(&val()?).ok_or("invalid category")?),
            "--seed" => config.seed = val()?.parse()?,
            "--top" => config.top_n = val()?.parse()?,
            "--poses" => poses = val()?.parse()?,
            "--sequential" => config.parallel_cases = false,
            _ => return Err(format!("unknown argument {a}").into()),
        }
    }
    let cases: Vec<_> = bench::load_cases(&cases_path)?
        .into_iter()
        .filter(|c| only.as_ref().is_none_or(|o| o.contains(&c.id)))
        .filter(|c| category.is_none_or(|k| c.category == k))
        .collect();
    eprintln!("{} cases, docker {docker}", cases.len());
    let report = match docker.as_str() {
        // Optimal pose with up to 5° / 1 Å of noise, then random decoys.
        "oracle" => bench::run(
            "oracle",
            &cases,
            &dir,
            &config,
            fake::oracle(dir.clone(), 5.0, 1.0, poses - 1),
            &Stderr(AtomicU32::new(0)),
        )?,
        "random" => bench::run(
            "random",
            &cases,
            &dir,
            &config,
            fake::random(poses),
            &Stderr(AtomicU32::new(0)),
        )?,
        _ => return Err("--docker must be oracle or random".into()),
    };
    std::fs::create_dir_all(&out)?;
    std::fs::write(out.join("report.json"), report.to_json())?;
    std::fs::write(out.join("report.md"), report.to_markdown())?;
    let md = report.to_markdown();
    // Print the summary table only.
    for line in md.lines().take_while(|l| !l.starts_with("## Cases")) {
        println!("{line}");
    }
    eprintln!("wrote {}", out.display());
    Ok(())
}
