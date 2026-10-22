use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Context;
use clap::{Parser, Subcommand, ValueEnum};
use cs_core::Mode;
use cs_core::corrected::Params;
use cs_core::progress::Progress;
use cshape::{docking, pipeline, server};

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
        #[arg(long, required_unless_present = "pdb", requires = "face")]
        vert: Option<PathBuf>,
        /// MSMS face file (.face)
        #[arg(long, requires = "vert")]
        face: Option<PathBuf>,
        /// PDB file: the solvent-excluded surface is generated (instead of --vert/--face)
        #[arg(long, conflicts_with_all = ["vert", "face"])]
        pdb: Option<PathBuf>,
        /// JSON file with surface parameters for --pdb (probe, grid_spacing, radii, ...)
        #[arg(long)]
        surface: Option<PathBuf>,
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
    /// Dock a ligand onto a receptor (rigid body, context-shape matching).
    Dock {
        /// Receptor PDB
        #[arg(long)]
        receptor: PathBuf,
        /// Ligand PDB (poses are reported for these coordinates)
        #[arg(long)]
        ligand: PathBuf,
        /// Bound receptor of the known complex, to score poses with CAPRI/DockQ
        #[arg(long, requires = "native_ligand")]
        native_receptor: Option<PathBuf>,
        /// Bound ligand of the known complex
        #[arg(long, requires = "native_receptor")]
        native_ligand: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = CliSurface::Atoms)]
        surface: CliSurface,
        /// JSON file with docking parameters (see docs/DOCKING.md)
        #[arg(long)]
        params: Option<PathBuf>,
        /// Start from a seeded random ligand placement (for pre-superposed inputs)
        #[arg(long)]
        randomize_seed: Option<u64>,
        /// Number of top poses written as complex PDB files
        #[arg(long, default_value_t = 10)]
        complexes: usize,
        #[arg(long)]
        name: Option<String>,
        #[arg(long, short, default_value = "out/dock")]
        out: PathBuf,
    },
    /// Run the docking engine on the Docking Benchmark 5.5 (CAPRI/DockQ success rates).
    Bench {
        /// Directory with the benchmark structures (tools/benchmark/fetch.sh)
        #[arg(
            long,
            env = "CS_BENCHMARK_DIR",
            default_value = "data/benchmark/benchmark5.5/structures"
        )]
        structures: PathBuf,
        #[arg(long, default_value = "tools/benchmark/cases.tsv")]
        cases: PathBuf,
        /// Comma-separated case ids
        #[arg(long, value_delimiter = ',')]
        only: Option<Vec<String>>,
        /// rigid, medium or difficult
        #[arg(long)]
        category: Option<String>,
        /// Run at most this many cases
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long, value_enum, default_value_t = CliSurface::Atoms)]
        surface: CliSurface,
        #[arg(long)]
        params: Option<PathBuf>,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long, short, default_value = "out/bench")]
        out: PathBuf,
    },
    /// Compare the text files two output directories have in common, line by line.
    Compare { a: PathBuf, b: PathBuf },
    /// Exit 0 if a local server answers /api/health (used by the Docker health check).
    Health {
        #[arg(long, env = "CSHAPE_ADDR", default_value = "127.0.0.1:8080")]
        addr: std::net::SocketAddr,
    },
    /// Run the web server (API + web client).
    Serve {
        #[arg(long, env = "CSHAPE_ADDR", default_value = "0.0.0.0:8080")]
        addr: std::net::SocketAddr,
        /// Directory for the database and job files
        #[arg(long, env = "CSHAPE_DATA", default_value = "var")]
        data: PathBuf,
        /// Built web client to serve (web/dist)
        #[arg(long, env = "CSHAPE_WEB", default_value = "web/dist")]
        web: PathBuf,
        /// Directory with sample proteins
        #[arg(long, env = "CSHAPE_SAMPLES", default_value = "data/samples")]
        samples: PathBuf,
        /// Directory with bundled docking complexes (Docking Benchmark layout)
        #[arg(long, env = "CSHAPE_COMPLEXES", default_value = "data/complexes")]
        complexes: PathBuf,
        /// Access token required by the API; unset means no authentication
        #[arg(long, env = "CSHAPE_TOKEN", hide_env_values = true)]
        token: Option<String>,
        /// Jobs computed at the same time (each one already uses every core)
        #[arg(long, env = "CSHAPE_WORKERS", default_value_t = 1)]
        workers: usize,
        /// Maximum upload size in MiB
        #[arg(long, env = "CSHAPE_MAX_UPLOAD_MB", default_value_t = 1024)]
        max_upload_mb: usize,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum CliSurface {
    Atoms,
    Ses,
}

impl From<CliSurface> for docking::DockSurface {
    fn from(s: CliSurface) -> Self {
        match s {
            CliSurface::Atoms => docking::DockSurface::Atoms,
            CliSurface::Ses => docking::DockSurface::Ses,
        }
    }
}

fn read_json<T: serde::de::DeserializeOwned + Default>(path: Option<PathBuf>) -> anyhow::Result<T> {
    match path {
        Some(p) => Ok(serde_json::from_slice(
            &std::fs::read(&p).with_context(|| p.display().to_string())?,
        )?),
        None => Ok(T::default()),
    }
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
            pdb,
            surface,
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
            let surface_params = match surface {
                Some(p) => serde_json::from_slice(
                    &std::fs::read(&p).with_context(|| p.display().to_string())?,
                )?,
                None => Default::default(),
            };
            let (input, stem_from) = match (pdb, vert, face) {
                (Some(pdb), _, _) => (pipeline::SurfaceInput::Pdb(pdb.clone()), pdb),
                (None, Some(vert), Some(face)) => (
                    pipeline::SurfaceInput::Msms {
                        vert: vert.clone(),
                        face,
                    },
                    vert,
                ),
                _ => anyhow::bail!("give either --pdb or both --vert and --face"),
            };
            let name = name.unwrap_or_else(|| {
                stem_from
                    .file_stem()
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
                surface: surface_params,
            };
            let t = Instant::now();
            let inputs = pipeline::Inputs {
                surface: input,
                stl,
            };
            let report = pipeline::execute(&spec, &inputs, &out, &Stderr)?;
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
        Command::Dock {
            receptor,
            ligand,
            native_receptor,
            native_ligand,
            surface,
            params,
            randomize_seed,
            complexes,
            name,
            out,
        } => {
            let name = name.unwrap_or_else(|| {
                let stem = |p: &Path| {
                    p.file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_default()
                };
                format!("{}+{}", stem(&receptor), stem(&ligand))
            });
            let spec = docking::DockSpec {
                name,
                surface: surface.into(),
                params: read_json(params)?,
                randomize_seed,
                complexes,
            };
            let files = docking::DockFiles {
                receptor,
                ligand,
                native: native_receptor.zip(native_ligand),
            };
            let t = Instant::now();
            let report = docking::execute(&spec, &files, &out, &Stderr)?;
            let s = &report.summary;
            for p in s["poses"].as_array().into_iter().flatten().take(10) {
                let m = &p["metrics"];
                if m.is_null() {
                    println!(
                        "#{:<3} score {:>8.2}",
                        p["rank"],
                        p["score"].as_f64().unwrap_or(0.0)
                    );
                } else {
                    println!(
                        "#{:<3} score {:>8.2}  DockQ {:.3}  {}  LRMSD {:>6.2}  iRMSD {:>5.2}  fnat {:.2}",
                        p["rank"],
                        p["score"].as_f64().unwrap_or(0.0),
                        m["dockq"].as_f64().unwrap_or(0.0),
                        m["capri"].as_str().unwrap_or(""),
                        m["lrmsd"].as_f64().unwrap_or(0.0),
                        m["irmsd"].as_f64().unwrap_or(0.0),
                        m["fnat"].as_f64().unwrap_or(0.0),
                    );
                }
            }
            if !s["native"].is_null() {
                println!(
                    "first acceptable: {}  first medium: {}  best DockQ: {}",
                    s["native"]["first_acceptable"],
                    s["native"]["first_medium"],
                    s["native"]["best_dockq"]
                );
            }
            eprintln!(
                "total {:.1} s -> {}",
                t.elapsed().as_secs_f64(),
                out.display()
            );
            Ok(())
        }
        Command::Bench {
            structures,
            cases,
            only,
            category,
            limit,
            surface,
            params,
            seed,
            out,
        } => {
            let category = match category {
                Some(c) => Some(
                    cs_core::eval::bench::Category::parse(&c)
                        .ok_or_else(|| anyhow::anyhow!("unknown category {c:?}"))?,
                ),
                None => None,
            };
            let opts = docking::BenchOptions {
                structures,
                cases,
                only,
                category,
                limit,
                surface: surface.into(),
                params: read_json(params)?,
                seed,
            };
            let report = docking::run_benchmark(&opts, &Stderr)?;
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
            eprintln!("-> {}", out.display());
            Ok(())
        }
        Command::Compare { a, b } => compare(&a, &b),
        Command::Health { addr } => health(addr),
        Command::Serve {
            addr,
            data,
            web,
            samples,
            complexes,
            token,
            workers,
            max_upload_mb,
        } => {
            tracing_subscriber::fmt()
                .with_env_filter(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| "info,tower_http=warn".into()),
                )
                .init();
            let opts = server::ServeOptions {
                addr,
                data_dir: data,
                web_dir: Some(web),
                samples_dir: samples,
                complexes_dir: complexes,
                token,
                workers,
                max_upload_mb,
            };
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?
                .block_on(server::serve(opts))
        }
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

fn health(mut addr: std::net::SocketAddr) -> anyhow::Result<()> {
    use std::io::{Read, Write};
    if addr.ip().is_unspecified() {
        addr.set_ip(std::net::Ipv4Addr::LOCALHOST.into());
    }
    let timeout = std::time::Duration::from_secs(2);
    let mut stream = std::net::TcpStream::connect_timeout(&addr, timeout)?;
    stream.set_read_timeout(Some(timeout))?;
    write!(
        stream,
        "GET /api/health HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n"
    )?;
    let mut buf = String::new();
    stream.read_to_string(&mut buf)?;
    if buf.starts_with("HTTP/1.1 200") {
        Ok(())
    } else {
        anyhow::bail!("unhealthy: {}", buf.lines().next().unwrap_or("no response"))
    }
}
