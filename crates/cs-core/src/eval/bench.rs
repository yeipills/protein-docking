//! Benchmark harness: runs a docking function over the Protein-Protein Docking Benchmark 5.5
//! and reports CAPRI success rates per difficulty category.
//!
//! For every case the harness
//! 1. loads the unbound and bound structures and prepares a [`Native`];
//! 2. moves the unbound ligand to a random orientation and position (seeded): the benchmark
//!    files are pre-superposed on the complex, so docking them as given would leak the
//!    answer to any method that is sensitive to the starting pose;
//! 3. calls the docker with the unbound receptor and the randomized ligand;
//! 4. maps each returned pose back to the frame of the original ligand file
//!    (`pose ∘ randomization`) and evaluates the top `top_n`;
//! 5. aggregates success rates (a hit of at least a given CAPRI class within the top N),
//!    the best rank and DockQ per case, and timings.
//!
//! The docker is any `Fn(&receptor, &ligand, &DockInput) -> Vec<Rigid>` returning rigid
//! transforms of the ligand it was given (receptor fixed), best first.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use super::native::{Capri, Metrics, Native};
use crate::error::{Error, Result};
use crate::geom::transform::{Rigid, Rng, kabsch};
use crate::geom::vec3::*;
use crate::io::pdb::{Atom, Structure};
use crate::progress::{self, Progress};

/// Difficulty category of a benchmark case, from the unbound→bound change at the interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    /// I-RMSD ≤ 1.5 Å.
    Rigid,
    /// 1.5 Å < I-RMSD ≤ 2.2 Å.
    Medium,
    /// I-RMSD > 2.2 Å.
    Difficult,
}

impl Category {
    pub const ALL: [Category; 3] = [Category::Rigid, Category::Medium, Category::Difficult];

    pub fn name(self) -> &'static str {
        match self {
            Category::Rigid => "rigid",
            Category::Medium => "medium",
            Category::Difficult => "difficult",
        }
    }

    pub fn parse(s: &str) -> Option<Category> {
        Category::ALL.into_iter().find(|c| c.name() == s)
    }
}

/// One benchmark case (a row of `tools/benchmark/cases.tsv`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Case {
    /// Prefix of the structure files (`<id>_r_u.pdb`, …); usually the complex PDB code.
    pub id: String,
    /// Complex and chains as in the benchmark table, e.g. `1AHW_AB:C`.
    pub complex: String,
    pub category: Category,
    /// Benchmark complex type: AA, AS, EI, ES, ER, OG, OR, OX.
    #[serde(rename = "type")]
    pub kind: String,
    /// Interface RMSD between unbound and bound structures (Å).
    pub irmsd: f64,
    /// Buried surface area (Å²).
    pub dasa: f64,
    /// Benchmark version in which the case was introduced.
    pub version: String,
    /// Unbound receptor / ligand PDB entries and their names.
    pub receptor: String,
    pub receptor_name: String,
    pub ligand: String,
    pub ligand_name: String,
}

/// Paths of a case's four structure files.
#[derive(Debug, Clone, PartialEq)]
pub struct CasePaths {
    pub receptor_unbound: PathBuf,
    pub ligand_unbound: PathBuf,
    pub receptor_bound: PathBuf,
    pub ligand_bound: PathBuf,
}

impl Case {
    /// Files of this case in `dir` (e.g. `data/benchmark/benchmark5.5/structures`).
    ///
    /// The official `<id>_l_u.pdb` files are used. For 1JPS and 1FAK the benchmark also
    /// ships `.pdb.2` variants built from the other copy of tissue factor in 1TFH; they are
    /// not used, so results stay comparable with published ones.
    pub fn paths(&self, dir: &Path) -> CasePaths {
        let p = |s: &str| dir.join(format!("{}_{s}.pdb", self.id));
        CasePaths {
            receptor_unbound: p("r_u"),
            ligand_unbound: p("l_u"),
            receptor_bound: p("r_b"),
            ligand_bound: p("l_b"),
        }
    }

    /// Reads the four structures of this case.
    pub fn load(&self, dir: &Path) -> Result<CaseData> {
        let p = self.paths(dir);
        Ok(CaseData {
            receptor_unbound: Structure::read(&p.receptor_unbound)?,
            ligand_unbound: Structure::read(&p.ligand_unbound)?,
            receptor_bound: Structure::read(&p.receptor_bound)?,
            ligand_bound: Structure::read(&p.ligand_bound)?,
        })
    }
}

/// The four structures of a case.
#[derive(Debug, Clone)]
pub struct CaseData {
    pub receptor_unbound: Structure,
    pub ligand_unbound: Structure,
    pub receptor_bound: Structure,
    pub ligand_bound: Structure,
}

impl CaseData {
    pub fn native(&self) -> Result<Native> {
        Native::new(
            &self.receptor_bound,
            &self.ligand_bound,
            &self.receptor_unbound,
            &self.ligand_unbound,
        )
    }
}

/// Parses the case table (tab-separated, `#` comments, header line with column names).
pub fn parse_cases(text: &str) -> Result<Vec<Case>> {
    let mut lines = text
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty() && !l.starts_with('#'));
    let Some((_, header)) = lines.next() else {
        return Ok(Vec::new());
    };
    let cols: Vec<&str> = header.split('\t').collect();
    let col = |name: &str| -> Result<usize> {
        cols.iter()
            .position(|c| *c == name)
            .ok_or_else(|| Error::Invalid(format!("case table has no column {name:?}")))
    };
    let idx = [
        "id",
        "complex",
        "category",
        "type",
        "irmsd",
        "dasa",
        "version",
        "receptor",
        "receptor_name",
        "ligand",
        "ligand_name",
    ]
    .map(col);
    let idx: Vec<usize> = idx.into_iter().collect::<Result<_>>()?;
    let mut out = Vec::new();
    for (n, line) in lines {
        let f: Vec<&str> = line.split('\t').collect();
        let get = |k: usize| f.get(idx[k]).copied().unwrap_or("");
        let err = |what: &str| Error::parse("cases.tsv", n + 1, what.to_string());
        out.push(Case {
            id: get(0).to_string(),
            complex: get(1).to_string(),
            category: Category::parse(get(2)).ok_or_else(|| err("invalid category"))?,
            kind: get(3).to_string(),
            irmsd: get(4).parse().map_err(|_| err("invalid irmsd"))?,
            dasa: get(5).parse().map_err(|_| err("invalid dasa"))?,
            version: get(6).to_string(),
            receptor: get(7).to_string(),
            receptor_name: get(8).to_string(),
            ligand: get(9).to_string(),
            ligand_name: get(10).to_string(),
        });
        if out.last().unwrap().id.is_empty() {
            return Err(err("empty id"));
        }
    }
    Ok(out)
}

/// Reads `tools/benchmark/cases.tsv` (or another table with the same columns).
pub fn load_cases(path: &Path) -> Result<Vec<Case>> {
    let text = std::fs::read_to_string(path).map_err(|e| Error::io(path, e))?;
    parse_cases(&text)
}

/// What a docker receives besides the two structures.
#[derive(Debug, Clone, Copy)]
pub struct DockInput<'a> {
    /// The case being docked (metadata only: never the bound structures).
    pub case: &'a Case,
    /// Seed for stochastic dockers, derived from the benchmark seed and the case id.
    pub seed: u64,
}

/// Benchmark settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchConfig {
    /// Seed of the ligand randomization (and of [`DockInput::seed`]).
    pub seed: u64,
    /// Number of top-ranked poses evaluated per case.
    pub top_n: usize,
    /// Ranks at which success rates are reported.
    pub tops: Vec<usize>,
    /// Gap (Å) between the bounding spheres of receptor and randomized ligand.
    pub start_gap: f64,
    /// Run cases in parallel. Leave off to time a docker that is itself parallel.
    pub parallel_cases: bool,
}

impl Default for BenchConfig {
    fn default() -> Self {
        BenchConfig {
            seed: 1,
            top_n: 100,
            tops: vec![1, 5, 10, 100],
            start_gap: 10.0,
            parallel_cases: false,
        }
    }
}

/// Wall-clock time of each stage of a case (seconds).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Timings {
    /// Reading the PDB files.
    pub load: f64,
    /// Preparing the [`Native`] (alignments, native contacts).
    pub setup: f64,
    /// The docker call.
    pub dock: f64,
    /// Evaluating the top poses.
    pub eval: f64,
}

/// A pose and its rank (1-based).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RankedPose {
    pub rank: usize,
    pub metrics: Metrics,
}

/// First rank (1-based) reaching at least each CAPRI class.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct FirstHits {
    pub acceptable: Option<usize>,
    pub medium: Option<usize>,
    pub high: Option<usize>,
}

impl FirstHits {
    pub fn get(&self, class: Capri) -> Option<usize> {
        match class {
            Capri::Acceptable => self.acceptable,
            Capri::Medium => self.medium,
            Capri::High => self.high,
            Capri::Incorrect => Some(1),
        }
    }

    fn from_metrics(metrics: &[Metrics]) -> FirstHits {
        let first = |c: Capri| metrics.iter().position(|m| m.capri >= c).map(|i| i + 1);
        FirstHits {
            acceptable: first(Capri::Acceptable),
            medium: first(Capri::Medium),
            high: first(Capri::High),
        }
    }
}

/// Outcome of one case.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaseResult {
    pub id: String,
    pub category: Category,
    #[serde(rename = "type")]
    pub kind: String,
    /// Poses returned by the docker (only the first `top_n` are evaluated).
    pub poses: usize,
    pub first_hit: FirstHits,
    /// Metrics of the top-ranked pose.
    pub top1: Option<Metrics>,
    /// Pose with the highest DockQ among the evaluated ones.
    pub best: Option<RankedPose>,
    /// Metrics of [`Native::optimal_pose`]: the ceiling for rigid docking of these
    /// unbound structures (difficult cases often cannot reach "high" even then).
    pub ceiling: Option<Metrics>,
    /// Randomization applied to the unbound ligand before docking.
    pub randomization: Rigid,
    pub time: Timings,
    /// Set when the case could not be run; it then counts as a failure.
    pub error: Option<String>,
}

/// Success counts at one rank cut-off.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Success {
    pub top: usize,
    pub acceptable: usize,
    pub medium: usize,
    pub high: usize,
}

/// Aggregate results of one category (or of all cases, `category = "all"`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Summary {
    pub category: String,
    pub cases: usize,
    /// Cases that could not be run.
    pub errors: usize,
    pub success: Vec<Success>,
    /// Cases whose [`CaseResult::ceiling`] reaches each class (`top` is 0).
    pub ceiling: Success,
    pub mean_dock_seconds: f64,
    pub mean_dockq_top1: f64,
}

/// Full benchmark report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Report {
    /// Name of the docker, for the record.
    pub docker: String,
    pub config: BenchConfig,
    pub summary: Vec<Summary>,
    pub cases: Vec<CaseResult>,
    pub wall_seconds: f64,
}

/// Stable 64-bit hash (FNV-1a), so per-case seeds do not depend on the case order.
fn fnv1a(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ b as u64).wrapping_mul(0x100_0000_01b3)
    })
}

/// Seed of a case: the same for a given benchmark seed whatever subset of cases is run.
pub fn case_seed(seed: u64, id: &str) -> u64 {
    seed ^ fnv1a(id)
}

pub(crate) fn centroid_radius(s: &Structure) -> (V3, f64) {
    let n = s.atoms.len().max(1) as f64;
    let c = scale(
        s.atoms.iter().fold([0.0; 3], |acc, a| add(acc, a.pos)),
        1.0 / n,
    );
    let r = s
        .atoms
        .iter()
        .map(|a| dist2(a.pos, c))
        .fold(0.0, f64::max)
        .sqrt();
    (c, r)
}

/// A copy of `s` moved by `t`.
pub fn transformed(s: &Structure, t: &Rigid) -> Structure {
    Structure {
        atoms: s
            .atoms
            .iter()
            .map(|a| Atom {
                pos: t.apply(a.pos),
                ..a.clone()
            })
            .collect(),
    }
}

/// Random starting pose for the ligand: uniform random orientation about its centroid,
/// centroid placed in a uniformly random direction from the receptor centroid, with the
/// two bounding spheres `gap` Å apart. Independent of where the ligand was, so the
/// benchmark's pre-superposition carries no information.
pub fn randomize(receptor: &Structure, ligand: &Structure, gap: f64, rng: &mut Rng) -> Rigid {
    let (cr, rr) = centroid_radius(receptor);
    let (cl, rl) = centroid_radius(ligand);
    let rot = rng.rotation();
    let dir = random_direction(rng);
    let target = add(cr, scale(dir, rr + rl + gap));
    Rigid {
        rot,
        trans: sub(target, crate::geom::transform::mat_vec(&rot, cl)),
    }
}

/// Uniformly distributed unit vector.
pub fn random_direction(rng: &mut Rng) -> V3 {
    let z = 2.0 * rng.next_f64() - 1.0;
    let phi = std::f64::consts::TAU * rng.next_f64();
    let r = (1.0 - z * z).max(0.0).sqrt();
    [r * phi.cos(), r * phi.sin(), z]
}

/// Runs one case. Errors (unreadable files, no native interface…) are recorded in the
/// result instead of aborting the benchmark.
pub fn run_case<D>(case: &Case, dir: &Path, config: &BenchConfig, docker: &D) -> CaseResult
where
    D: Fn(&Structure, &Structure, &DockInput) -> Vec<Rigid> + Sync,
{
    let mut result = CaseResult {
        id: case.id.clone(),
        category: case.category,
        kind: case.kind.clone(),
        poses: 0,
        first_hit: FirstHits::default(),
        top1: None,
        best: None,
        ceiling: None,
        randomization: Rigid::IDENTITY,
        time: Timings::default(),
        error: None,
    };
    let t = Instant::now();
    let data = match case.load(dir) {
        Ok(d) => d,
        Err(e) => {
            result.error = Some(e.to_string());
            return result;
        }
    };
    result.time.load = t.elapsed().as_secs_f64();
    let t = Instant::now();
    let native = match data.native() {
        Ok(n) => n,
        Err(e) => {
            result.error = Some(e.to_string());
            return result;
        }
    };
    result.ceiling = Some(native.evaluate(&native.optimal_pose()));
    result.time.setup = t.elapsed().as_secs_f64();

    let seed = case_seed(config.seed, &case.id);
    let mut rng = Rng::new(seed);
    let random = randomize(
        &data.receptor_unbound,
        &data.ligand_unbound,
        config.start_gap,
        &mut rng,
    );
    result.randomization = random;
    let ligand = transformed(&data.ligand_unbound, &random);
    let input = DockInput {
        case,
        seed: rng.next_u64(),
    };

    let t = Instant::now();
    let poses = docker(&data.receptor_unbound, &ligand, &input);
    result.time.dock = t.elapsed().as_secs_f64();
    result.poses = poses.len();

    let t = Instant::now();
    // The docker moved the randomized ligand; the evaluator expects transforms of the
    // original file, so apply the randomization first.
    let original: Vec<Rigid> = poses
        .iter()
        .take(config.top_n)
        .map(|p| p.compose(&random))
        .collect();
    let metrics = native.evaluate_many(&original);
    result.time.eval = t.elapsed().as_secs_f64();

    result.first_hit = FirstHits::from_metrics(&metrics);
    result.top1 = metrics.first().copied();
    result.best = metrics
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.dockq.total_cmp(&b.1.dockq))
        .map(|(i, m)| RankedPose {
            rank: i + 1,
            metrics: *m,
        });
    result
}

/// Runs `docker` on `cases` (structure files in `dir`) and aggregates the results.
/// `name` identifies the docker in the report.
pub fn run<D>(
    name: &str,
    cases: &[Case],
    dir: &Path,
    config: &BenchConfig,
    docker: D,
    progress: &dyn Progress,
) -> Result<Report>
where
    D: Fn(&Structure, &Structure, &DockInput) -> Vec<Rigid> + Sync,
{
    let start = Instant::now();
    let done = std::sync::atomic::AtomicUsize::new(0);
    let one = |case: &Case| -> Result<CaseResult> {
        progress::check(progress)?;
        let r = run_case(case, dir, config, &docker);
        let n = done.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        progress.report("bench", n as f32 / cases.len().max(1) as f32);
        Ok(r)
    };
    let results: Vec<CaseResult> = if config.parallel_cases {
        cases.par_iter().map(one).collect::<Result<_>>()?
    } else {
        cases.iter().map(one).collect::<Result<_>>()?
    };
    Ok(Report {
        docker: name.to_string(),
        config: config.clone(),
        summary: summarize(&results, &config.tops),
        cases: results,
        wall_seconds: start.elapsed().as_secs_f64(),
    })
}

/// Success rates per category and over all cases.
pub fn summarize(results: &[CaseResult], tops: &[usize]) -> Vec<Summary> {
    let mut groups: Vec<(String, Vec<&CaseResult>)> = Category::ALL
        .iter()
        .map(|c| {
            (
                c.name().to_string(),
                results.iter().filter(|r| r.category == *c).collect(),
            )
        })
        .filter(|(_, v): &(String, Vec<&CaseResult>)| !v.is_empty())
        .collect();
    groups.push(("all".into(), results.iter().collect()));
    groups
        .into_iter()
        .map(|(category, rs)| {
            let count = |f: &dyn Fn(&CaseResult) -> bool| rs.iter().filter(|r| f(r)).count();
            let hit = |r: &CaseResult, c: Capri, top: usize| {
                r.first_hit.get(c).is_some_and(|k| k <= top) && r.error.is_none()
            };
            let success = tops
                .iter()
                .map(|&top| Success {
                    top,
                    acceptable: count(&|r| hit(r, Capri::Acceptable, top)),
                    medium: count(&|r| hit(r, Capri::Medium, top)),
                    high: count(&|r| hit(r, Capri::High, top)),
                })
                .collect();
            let ceil = |c: Capri| count(&|r| r.ceiling.is_some_and(|m| m.capri >= c));
            let n = rs.len().max(1) as f64;
            Summary {
                cases: rs.len(),
                errors: count(&|r| r.error.is_some()),
                success,
                ceiling: Success {
                    top: 0,
                    acceptable: ceil(Capri::Acceptable),
                    medium: ceil(Capri::Medium),
                    high: ceil(Capri::High),
                },
                mean_dock_seconds: rs.iter().map(|r| r.time.dock).sum::<f64>() / n,
                mean_dockq_top1: rs
                    .iter()
                    .map(|r| r.top1.map_or(0.0, |m| m.dockq))
                    .sum::<f64>()
                    / n,
                category,
            }
        })
        .collect()
}

impl Report {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("report serializes")
    }

    /// Markdown tables: success rates per category, then one line per case.
    pub fn to_markdown(&self) -> String {
        let mut s = String::new();
        let pct = |k: usize, n: usize| {
            if n == 0 {
                0.0
            } else {
                100.0 * k as f64 / n as f64
            }
        };
        let triple = |x: &Success, n: usize| {
            format!(
                "{:.0} / {:.0} / {:.0}",
                pct(x.acceptable, n),
                pct(x.medium, n),
                pct(x.high, n)
            )
        };
        let _ = writeln!(s, "# Benchmark: {}\n", self.docker);
        let _ = writeln!(
            s,
            "Seed {}, top {} poses evaluated, {:.1} s in total.\n",
            self.config.seed, self.config.top_n, self.wall_seconds
        );
        let _ = writeln!(
            s,
            "## Success rate (% of cases: acceptable / medium / high or better)\n"
        );
        let mut header = String::from("| Category | Cases |");
        let mut rule = String::from("|---|---|");
        for t in &self.config.tops {
            let _ = write!(header, " Top-{t} |");
            rule.push_str("---|");
        }
        header.push_str(" Rigid ceiling | Mean DockQ top-1 | Mean dock time (s) |");
        rule.push_str("---|---|---|");
        let _ = writeln!(s, "{header}\n{rule}");
        for g in &self.summary {
            let mut line = format!("| {} | {} |", g.category, g.cases);
            for x in &g.success {
                let _ = write!(line, " {} |", triple(x, g.cases));
            }
            let _ = write!(
                line,
                " {} | {:.3} | {:.2} |",
                triple(&g.ceiling, g.cases),
                g.mean_dockq_top1,
                g.mean_dock_seconds
            );
            let _ = writeln!(s, "{line}");
        }
        let _ = writeln!(
            s,
            "\n\"Rigid ceiling\": cases whose optimal rigid placement of the unbound ligand reaches each class.\n"
        );
        let _ = writeln!(s, "## Cases\n");
        let _ = writeln!(
            s,
            "| Case | Category | Type | Poses | 1st acceptable | 1st medium | 1st high | Best DockQ (rank) | Top-1 DockQ | Ceiling DockQ | Dock (s) |"
        );
        let _ = writeln!(s, "|---|---|---|---|---|---|---|---|---|---|---|");
        let rank = |r: Option<usize>| r.map_or("–".to_string(), |k| k.to_string());
        for r in &self.cases {
            if let Some(e) = &r.error {
                let _ = writeln!(
                    s,
                    "| {} | {} | {} | error: {} |||||||||",
                    r.id,
                    r.category.name(),
                    r.kind,
                    e
                );
                continue;
            }
            let _ = writeln!(
                s,
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {:.2} |",
                r.id,
                r.category.name(),
                r.kind,
                r.poses,
                rank(r.first_hit.acceptable),
                rank(r.first_hit.medium),
                rank(r.first_hit.high),
                r.best.map_or("–".into(), |b| format!(
                    "{:.3} ({})",
                    b.metrics.dockq, b.rank
                )),
                r.top1.map_or("–".into(), |m| format!("{:.3}", m.dockq)),
                r.ceiling.map_or("–".into(), |m| format!("{:.3}", m.dockq)),
                r.time.dock
            );
        }
        s
    }
}

/// Reference dockers used to validate the harness and as baselines.
pub mod fake {
    use super::*;

    /// "Oracle": knows the answer. Recovers the randomization by superposing the ligand it
    /// receives on the original file, and returns the optimal rigid pose perturbed by a
    /// random rotation of up to `max_deg` degrees about the ligand centroid and a random
    /// translation of up to `max_shift` Å, followed by `decoys` random poses. A correct
    /// harness must report (nearly) every case whose rigid ceiling is a hit as a top-1 hit.
    pub fn oracle(
        dir: PathBuf,
        max_deg: f64,
        max_shift: f64,
        decoys: usize,
    ) -> impl Fn(&Structure, &Structure, &DockInput) -> Vec<Rigid> + Sync {
        move |receptor, ligand, input| {
            let Ok(data) = input.case.load(&dir) else {
                return Vec::new();
            };
            let Ok(native) = data.native() else {
                return Vec::new();
            };
            let orig: Vec<V3> = data.ligand_unbound.atoms.iter().map(|a| a.pos).collect();
            let given: Vec<V3> = ligand.atoms.iter().map(|a| a.pos).collect();
            let random = kabsch(&orig, &given).0;
            // Optimal pose expressed for the ligand as given.
            let truth = native.optimal_pose().compose(&random.inverse());
            let mut rng = Rng::new(input.seed);
            let (c, _) = centroid_radius(&transformed(ligand, &truth));
            let angle = max_deg.to_radians() * rng.next_f64();
            let rot = Rigid::about(c, Rigid::axis_angle(random_direction(&mut rng), angle).rot);
            let shift = Rigid::translation(scale(
                random_direction(&mut rng),
                max_shift * rng.next_f64(),
            ));
            let mut poses = vec![shift.compose(&rot).compose(&truth)];
            poses.extend(random_poses(receptor, ligand, decoys, &mut rng));
            poses
        }
    }

    /// Random docker: `n` poses with uniform random orientation and the ligand centroid at
    /// a random direction from the receptor centroid, at a distance where the two proteins
    /// roughly touch. Success rates must be close to zero.
    pub fn random(n: usize) -> impl Fn(&Structure, &Structure, &DockInput) -> Vec<Rigid> + Sync {
        move |receptor, ligand, input| random_poses(receptor, ligand, n, &mut Rng::new(input.seed))
    }

    fn radius_of_gyration(s: &Structure) -> (V3, f64) {
        let (c, _) = centroid_radius(s);
        let n = s.atoms.len().max(1) as f64;
        (
            c,
            (s.atoms.iter().map(|a| dist2(a.pos, c)).sum::<f64>() / n).sqrt(),
        )
    }

    fn random_poses(
        receptor: &Structure,
        ligand: &Structure,
        n: usize,
        rng: &mut Rng,
    ) -> Vec<Rigid> {
        let (cr, gr) = radius_of_gyration(receptor);
        let (cl, gl) = radius_of_gyration(ligand);
        (0..n)
            .map(|_| {
                let rot = rng.rotation();
                // Centroid distances of real complexes are close to the sum of the radii of
                // gyration; sample ±20 % around it.
                let d = (gr + gl) * (0.8 + 0.4 * rng.next_f64());
                let target = add(cr, scale(random_direction(rng), d));
                Rigid {
                    rot,
                    trans: sub(target, crate::geom::transform::mat_vec(&rot, cl)),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE: &str = "\
# comment
id\tcomplex\tcategory\ttype\tirmsd\tdasa\tversion\treceptor\treceptor_name\tligand\tligand_name
1AHW\t1AHW_AB:C\trigid\tAA\t0.69\t1899\t2\t1FGN_LH\tFab 5g9\t1TFH_A\tTissue factor
BOYV\t1OYV_B:I\trigid\tEI\t0.47\t1280\t4\t1SCD_A\tSubtilisin Carlsberg\t1PJU_A\tInhibitor
";

    #[test]
    fn parses_case_table() {
        let cases = parse_cases(TABLE).unwrap();
        assert_eq!(cases.len(), 2);
        assert_eq!(cases[1].id, "BOYV");
        assert_eq!(cases[1].complex, "1OYV_B:I");
        assert_eq!(cases[0].category, Category::Rigid);
        assert_eq!(cases[0].irmsd, 0.69);
        let p = cases[0].paths(Path::new("/x"));
        assert_eq!(p.ligand_bound, PathBuf::from("/x/1AHW_l_b.pdb"));
        assert!(parse_cases("id\tcomplex\n").is_err(), "missing columns");
    }

    #[test]
    fn randomization_is_uniform_and_seeded() {
        let mut a = Rng::new(5);
        let mut b = Rng::new(5);
        let d1: Vec<V3> = (0..2000).map(|_| random_direction(&mut a)).collect();
        let d2: Vec<V3> = (0..2000).map(|_| random_direction(&mut b)).collect();
        assert_eq!(d1, d2);
        let mean = scale(d1.iter().fold([0.0; 3], |s, d| add(s, *d)), 1.0 / 2000.0);
        assert!(norm(mean) < 0.05, "mean direction {mean:?}");
        assert_eq!(case_seed(1, "1AHW"), case_seed(1, "1AHW"));
        assert_ne!(case_seed(1, "1AHW"), case_seed(2, "1AHW"));
    }

    fn result(cat: Category, hits: [Option<usize>; 3]) -> CaseResult {
        CaseResult {
            id: "X".into(),
            category: cat,
            kind: "EI".into(),
            poses: 10,
            first_hit: FirstHits {
                acceptable: hits[0],
                medium: hits[1],
                high: hits[2],
            },
            top1: None,
            best: None,
            ceiling: None,
            randomization: Rigid::IDENTITY,
            time: Timings::default(),
            error: None,
        }
    }

    #[test]
    fn success_counts() {
        let rs = vec![
            result(Category::Rigid, [Some(1), Some(1), None]),
            result(Category::Rigid, [Some(3), Some(7), Some(7)]),
            result(Category::Difficult, [None, None, None]),
        ];
        let s = summarize(&rs, &[1, 5, 10]);
        assert_eq!(s.len(), 3, "rigid, difficult, all");
        let rigid = &s[0];
        assert_eq!((rigid.category.as_str(), rigid.cases), ("rigid", 2));
        assert_eq!(
            (
                rigid.success[0].acceptable,
                rigid.success[0].medium,
                rigid.success[0].high
            ),
            (1, 1, 0)
        );
        assert_eq!(
            (rigid.success[1].acceptable, rigid.success[1].medium),
            (2, 1)
        );
        assert_eq!((rigid.success[2].medium, rigid.success[2].high), (2, 1));
        assert_eq!(s[2].category, "all");
        assert_eq!(s[2].success[2].acceptable, 2);
    }
}
