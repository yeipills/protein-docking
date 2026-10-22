//! Runs a docking job and writes its output files. Shared by the CLI and the server.

use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Context;
use cs_core::dock::{self, AtomSurfaceParams, DockInput, DockParams, Pose};
use cs_core::eval::{Metrics, Native, bench};
use cs_core::geom::transform::{Rigid, Rng};
use cs_core::io::pdb::Structure;
use cs_core::progress::Progress;
use cs_core::surface::{self, SesParams};
use serde::{Deserialize, Serialize};

use crate::pipeline::{OutputFile, Report, SUMMARY_FILE, Timings, sanitize_name};

/// Surface the context shapes are computed on (see `docs/DOCKING.md`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DockSurface {
    /// Atom-based SES grid; the default, and what the engine was tuned on.
    #[default]
    Atoms,
    /// The built-in SES generator.
    Ses,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockSpec {
    pub name: String,
    #[serde(default)]
    pub surface: DockSurface,
    #[serde(default)]
    pub params: DockParams,
    /// Start the search from a seeded random placement of the ligand. Poses are always
    /// reported relative to the uploaded ligand coordinates.
    #[serde(default)]
    pub randomize_seed: Option<u64>,
    /// Number of top poses written as complex PDB files.
    #[serde(default = "default_complexes")]
    pub complexes: usize,
}

fn default_complexes() -> usize {
    10
}

pub struct DockFiles {
    pub receptor: PathBuf,
    pub ligand: PathBuf,
    /// Bound receptor and ligand (the known answer), to score the poses with CAPRI/DockQ.
    pub native: Option<(PathBuf, PathBuf)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedPose {
    #[serde(flatten)]
    pub pose: Pose,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<Metrics>,
}

pub const DOCK_VIEWER_FILE: &str = "dock_viewer.bin";
pub const POSES_FILE: &str = "poses.json";

/// Maps a sub-task's `[0, 1]` progress into `[lo, hi]` of the whole job.
struct Scaled<'a> {
    inner: &'a dyn Progress,
    lo: f32,
    hi: f32,
}

impl Progress for Scaled<'_> {
    fn report(&self, stage: &str, fraction: f32) {
        self.inner.report(
            stage,
            self.lo + (self.hi - self.lo) * fraction.clamp(0.0, 1.0),
        );
    }

    fn is_cancelled(&self) -> bool {
        self.inner.is_cancelled()
    }
}

pub fn execute(
    spec: &DockSpec,
    files: &DockFiles,
    out_dir: &Path,
    progress: &dyn Progress,
) -> anyhow::Result<Report> {
    spec.params.validate()?;
    let name = sanitize_name(&spec.name);
    std::fs::create_dir_all(out_dir).with_context(|| format!("creating {}", out_dir.display()))?;
    let mut timings = Timings::default();
    let mut written: Vec<OutputFile> = Vec::new();
    let mut write = |file: String, kind: &str, data: &[u8]| -> anyhow::Result<()> {
        let path = out_dir.join(&file);
        std::fs::write(&path, data).with_context(|| format!("writing {}", path.display()))?;
        written.push(OutputFile {
            name: file,
            bytes: data.len() as u64,
            kind: kind.into(),
        });
        Ok(())
    };

    let t = Instant::now();
    let receptor = Structure::read(&files.receptor).context("receptor")?;
    let ligand = Structure::read(&files.ligand).context("ligand")?;
    let native = match &files.native {
        Some((rb, lb)) => {
            let rb = Structure::read(rb).context("native receptor")?;
            let lb = Structure::read(lb).context("native ligand")?;
            Some(Native::new(&rb, &lb, &receptor, &ligand)?)
        }
        None => None,
    };
    timings.read = t.elapsed().as_millis() as u64;

    // Optional random start; `start` maps the uploaded ligand to the docked one.
    let start = match spec.randomize_seed {
        Some(seed) => bench::randomize(&receptor, &ligand, 10.0, &mut Rng::new(seed)),
        None => Rigid::IDENTITY,
    };
    let ligand_start = bench::transformed(&ligand, &start);

    let t = Instant::now();
    let surfaces = Scaled {
        inner: progress,
        lo: 0.0,
        hi: 0.1,
    };
    progress.report("surfaces", 0.0);
    let make = |s: &Structure| -> cs_core::Result<DockInput> {
        match spec.surface {
            DockSurface::Atoms => {
                DockInput::from_atoms(s.clone(), &AtomSurfaceParams::default(), &surfaces)
            }
            DockSurface::Ses => DockInput::from_ses(s.clone(), &SesParams::default(), &surfaces),
        }
    };
    let (r, l) = rayon::join(|| make(&receptor), || make(&ligand_start));
    let (r, l) = (r?, l?);
    let output = dock::run_with_stats(
        &r,
        &l,
        &spec.params,
        &Scaled {
            inner: progress,
            lo: 0.1,
            hi: 0.95,
        },
    )?;
    timings.compute = t.elapsed().as_millis() as u64;

    let t = Instant::now();
    progress.report("writing", 0.95);
    let poses: Vec<RankedPose> = output
        .poses
        .into_iter()
        .map(|mut pose| {
            pose.transform = pose.transform.compose(&start);
            let metrics = native.as_ref().map(|n| n.evaluate(&pose.transform));
            RankedPose { pose, metrics }
        })
        .collect();

    for p in poses.iter().take(spec.complexes) {
        let pdb = dock::complex_pdb(&receptor, &ligand, &p.pose.transform);
        write(
            format!("{name}_pose_{:03}.pdb", p.pose.rank),
            "complex",
            pdb.as_bytes(),
        )?;
    }
    write(
        POSES_FILE.into(),
        "poses",
        &serde_json::to_vec_pretty(&poses)?,
    )?;

    // Display meshes: a coarser SES is plenty for the viewer.
    let display = SesParams {
        grid_spacing: 0.8,
        ..SesParams::default()
    };
    let no = cs_core::progress::NoProgress;
    let (rm, lm) = rayon::join(
        || surface::ses(&receptor.atoms, &display, &no),
        || surface::ses(&ligand.atoms, &display, &no),
    );
    write(
        DOCK_VIEWER_FILE.into(),
        "viewer",
        &dock_viewer_blob(&rm?, &lm?),
    )?;

    let best = |at_least: cs_core::eval::Capri| {
        poses
            .iter()
            .find(|p| p.metrics.as_ref().is_some_and(|m| m.capri >= at_least))
            .map(|p| p.pose.rank)
    };
    let summary = serde_json::json!({
        "kind": "dock",
        "surface": spec.surface,
        "receptor_atoms": receptor.atoms.len(),
        "ligand_atoms": ligand.atoms.len(),
        "randomize_seed": spec.randomize_seed,
        "stats": output.stats,
        "poses": poses,
        "native": native.as_ref().map(|n| serde_json::json!({
            "pose": n.optimal_pose(),
            "first_acceptable": best(cs_core::eval::Capri::Acceptable),
            "first_medium": best(cs_core::eval::Capri::Medium),
            "first_high": best(cs_core::eval::Capri::High),
            "best_dockq": poses.iter().filter_map(|p| p.metrics.as_ref()).map(|m| m.dockq).fold(None, |a: Option<f64>, d| Some(a.map_or(d, |a| a.max(d)))),
        })),
    });
    timings.write = t.elapsed().as_millis() as u64;
    let report = Report {
        summary,
        timings_ms: timings,
        files: written,
    };
    std::fs::write(
        out_dir.join(SUMMARY_FILE),
        serde_json::to_vec_pretty(&report)?,
    )?;
    progress.report("done", 1.0);
    Ok(report)
}

/// Binary blob for the docking viewer (little-endian):
///
/// ```text
/// "DCK1"
/// receptor: u32 n_vertices, u32 n_faces, f32[3 * n_vertices], u32[3 * n_faces]
/// ligand:   same layout, in the uploaded ligand's coordinates (poses move it)
/// ```
pub fn dock_viewer_blob(receptor: &cs_core::mesh::Mesh, ligand: &cs_core::mesh::Mesh) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(b"DCK1");
    for m in [receptor, ligand] {
        b.extend_from_slice(&(m.vertices.len() as u32).to_le_bytes());
        b.extend_from_slice(&(m.faces.len() as u32).to_le_bytes());
        for v in &m.vertices {
            for c in v {
                b.extend_from_slice(&(*c as f32).to_le_bytes());
            }
        }
        for f in &m.faces {
            for i in f {
                b.extend_from_slice(&i.to_le_bytes());
            }
        }
    }
    b
}

/// Settings of a Docking Benchmark run (`cshape bench`).
pub struct BenchOptions {
    pub structures: PathBuf,
    pub cases: PathBuf,
    pub only: Option<Vec<String>>,
    pub category: Option<bench::Category>,
    pub limit: Option<usize>,
    pub surface: DockSurface,
    pub params: DockParams,
    pub seed: u64,
}

/// Runs the docking engine through the CAPRI/DockQ benchmark harness.
pub fn run_benchmark(
    opts: &BenchOptions,
    progress: &dyn Progress,
) -> anyhow::Result<bench::Report> {
    opts.params.validate()?;
    let mut cases: Vec<bench::Case> = bench::load_cases(&opts.cases)
        .with_context(|| format!("reading {}", opts.cases.display()))?
        .into_iter()
        .filter(|c| opts.only.as_ref().is_none_or(|o| o.contains(&c.id)))
        .filter(|c| opts.category.is_none_or(|k| c.category == k))
        .collect();
    if let Some(n) = opts.limit {
        cases.truncate(n);
    }
    anyhow::ensure!(!cases.is_empty(), "no benchmark cases selected");
    let config = bench::BenchConfig {
        seed: opts.seed,
        ..bench::BenchConfig::default()
    };
    let no = cs_core::progress::NoProgress;
    let docker = |receptor: &Structure, ligand: &Structure, _: &bench::DockInput| -> Vec<Rigid> {
        let make = |s: &Structure| match opts.surface {
            DockSurface::Atoms => {
                DockInput::from_atoms(s.clone(), &AtomSurfaceParams::default(), &no)
            }
            DockSurface::Ses => DockInput::from_ses(s.clone(), &SesParams::default(), &no),
        };
        let (r, l) = rayon::join(|| make(receptor), || make(ligand));
        match (r, l) {
            (Ok(r), Ok(l)) => match dock::run(&r, &l, &opts.params, &no) {
                Ok(poses) => poses.into_iter().map(|p| p.transform).collect(),
                Err(e) => {
                    tracing::warn!("docking failed: {e}");
                    Vec::new()
                }
            },
            (Err(e), _) | (_, Err(e)) => {
                tracing::warn!("surface failed: {e}");
                Vec::new()
            }
        }
    };
    let name = match opts.surface {
        DockSurface::Atoms => "cshape context shapes (atom surface)",
        DockSurface::Ses => "cshape context shapes (SES)",
    };
    Ok(bench::run(
        name,
        &cases,
        &opts.structures,
        &config,
        docker,
        progress,
    )?)
}
