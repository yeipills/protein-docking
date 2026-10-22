mod pipeline;

use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Context;
use clap::{Parser, Subcommand, ValueEnum};
use cs_core::Mode;
use cs_core::corrected::Params;
use cs_core::progress::Progress;

#[derive(Parser)]
#[command(name = "cshape", version, about = "Context shapes of protein surfaces")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compute context shapes from an MSMS surface and write the result files.
    Run {
        /// MSMS vertex file (.vert)
        #[arg(long)]
        vert: PathBuf,
        /// MSMS face file (.face)
        #[arg(long)]
        face: PathBuf,
        /// Surface STL (required in legacy mode)
        #[arg(long)]
        stl: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = CliMode::Corrected)]
        mode: CliMode,
        /// Protein name used in output file names (default: .vert file stem)
        #[arg(long)]
        name: Option<String>,
        /// Output directory
        #[arg(long, short, default_value = "out")]
        out: PathBuf,
        /// JSON file with corrected-mode parameters
        #[arg(long)]
        params: Option<PathBuf>,
        /// Legacy mode: also write the ten large Script04 intermediate files
        #[arg(long)]
        intermediate: bool,
    },
    /// Compare the text files two output directories have in common, line by line.
    Compare { a: PathBuf, b: PathBuf },
}

#[derive(Clone, Copy, ValueEnum)]
enum CliMode {
    Corrected,
    Legacy,
}

struct Stderr;

impl Progress for Stderr {
    fn report(&self, stage: &str, fraction: f32) {
        eprint!("\r{:>5.1}%  {stage:<16}", fraction * 100.0);
        if fraction >= 1.0 {
            eprintln!();
        }
    }
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Command::Run {
            vert,
            face,
            stl,
            mode,
            name,
            out,
            params,
            intermediate,
        } => {
            let params: Params = match params {
                Some(p) => serde_json::from_slice(
                    &std::fs::read(&p).with_context(|| p.display().to_string())?,
                )?,
                None => Params::default(),
            };
            let name = name.unwrap_or_else(|| {
                vert.file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default()
            });
            let spec = pipeline::Spec {
                name,
                mode: match mode {
                    CliMode::Corrected => Mode::Corrected,
                    CliMode::Legacy => Mode::Legacy,
                },
                params,
                legacy_intermediate: intermediate,
            };
            let t = Instant::now();
            let report =
                pipeline::execute(&spec, &pipeline::Inputs { vert, face, stl }, &out, &Stderr)?;
            println!("{}", serde_json::to_string_pretty(&report.summary)?);
            eprintln!(
                "read {} ms, compute {} ms, write {} ms, total {:.2} s -> {}",
                report.timings_ms.read,
                report.timings_ms.compute,
                report.timings_ms.write,
                t.elapsed().as_secs_f64(),
                out.display()
            );
            Ok(())
        }
        Command::Compare { a, b } => compare(&a, &b),
    }
}

fn compare(a: &Path, b: &Path) -> anyhow::Result<()> {
    let mut names: Vec<String> = std::fs::read_dir(a)?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".txt") && b.join(n).is_file())
        .collect();
    names.sort();
    if names.is_empty() {
        anyhow::bail!("no .txt files in common");
    }
    let mut all_equal = true;
    for n in names {
        let (ta, tb) = (
            std::fs::read_to_string(a.join(&n))?,
            std::fs::read_to_string(b.join(&n))?,
        );
        let (la, lb): (Vec<&str>, Vec<&str>) = (ta.lines().collect(), tb.lines().collect());
        let same = la.iter().zip(&lb).filter(|(x, y)| x == y).count();
        let total = la.len().max(lb.len());
        let mark = if same == total { "OK  " } else { "DIFF" };
        all_equal &= same == total;
        println!("{mark} {n}: {same}/{total} identical lines");
        if same != total {
            if let Some((i, (x, y))) = la.iter().zip(&lb).enumerate().find(|(_, (x, y))| x != y) {
                println!(
                    "     first difference at line {}:\n     < {}\n     > {}",
                    i + 1,
                    trunc(x),
                    trunc(y)
                );
            }
        }
    }
    if all_equal {
        Ok(())
    } else {
        std::process::exit(1)
    }
}

fn trunc(s: &str) -> String {
    if s.len() > 160 {
        format!("{}…", &s[..160])
    } else {
        s.to_string()
    }
}
