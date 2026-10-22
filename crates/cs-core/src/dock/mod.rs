//! Rigid-body protein–protein docking by complementary context-shape matching
//! (after Shentu, Al Hasan, Bystroff & Zaki, Proteins 2008).
//!
//! Pipeline ([`run`]):
//!
//! 1. **Critical points** every `descriptor.spacing` Å on both surfaces, each with a patch
//!    normal and a local frame ([`descriptor`]). Ligand points also get frames tilted off the
//!    normal, because the patch normals of two surfaces in contact disagree by 10–20°.
//! 2. **Context shapes**: the neighbourhood of each point sampled on rings about the normal
//!    and packed into bit masks (inside / deep core / exterior shell / surface layer), so a
//!    rotation about the normal is a cyclic bit shift.
//! 3. **Matching**: every receptor point × ligand frame × [`descriptor::K`] rotations is
//!    scored with AND + popcount (fill + surface coincidence − clash), keeping the best
//!    seeds per receptor point and overall ([`matching`]).
//! 4. **Poses**: each seed defines a rigid transform of the ligand putting the two points
//!    together with opposed normals ([`descriptor::pose`]); near-duplicates are dropped.
//! 5. **Atom scoring**: the best seeds (descriptor order) get a short rigid refinement with a
//!    soft atom score, the best of those a full refinement with the final score (contacts,
//!    clashes, contact potential, interface propensity), and the results are **clustered**
//!    ([`scoring`]).
//!
//! The engine is generic over the surface representation ([`Surface`]): a triangle mesh
//! ([`MeshSurface`]) or the built-in atom model ([`AtomSurface`]). Everything is
//! deterministic: there is no randomness, and parallel results are merged in a fixed order.

pub mod descriptor;
pub mod grid3;
pub mod matching;
pub mod scoring;
pub mod surface;

use std::time::Instant;

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

pub use descriptor::DescriptorParams;
pub use scoring::{RefineParams, ScoreComponents, ScoreParams};
pub use surface::{AtomSurface, AtomSurfaceParams, MeshSurface, SesSurface, Surface, SurfacePoint};

use crate::error::{Error, Result};
use crate::geom::transform::Rigid;
use crate::geom::vec3::*;
use crate::io::pdb::{Structure, write_atom};
use crate::mesh::Mesh;
use crate::progress::{Progress, check};

/// One docking partner: its atoms (used for re-scoring, clustering and output) and the
/// surface its context shapes are computed on. Both must be in the same coordinate frame.
pub struct DockInput {
    pub structure: Structure,
    pub surface: Box<dyn Surface>,
}

impl DockInput {
    pub fn new(structure: Structure, surface: Box<dyn Surface>) -> DockInput {
        DockInput { structure, surface }
    }

    /// Partner whose surface is the built-in SES of its heavy atoms.
    pub fn from_atoms(
        structure: Structure,
        params: &AtomSurfaceParams,
        progress: &dyn Progress,
    ) -> Result<DockInput> {
        let surface = AtomSurface::new(&structure.atoms, params, progress)?;
        Ok(DockInput::new(structure, Box::new(surface)))
    }

    /// Partner whose surface is the SES from the built-in generator ([`SesSurface`]).
    pub fn from_ses(
        structure: Structure,
        params: &crate::surface::SesParams,
        progress: &dyn Progress,
    ) -> Result<DockInput> {
        let surface = SesSurface::new(&structure.atoms, params, progress)?;
        Ok(DockInput::new(structure, Box::new(surface)))
    }

    /// Partner whose surface is a closed, outward-oriented triangle mesh (MSMS, SES generator).
    pub fn from_mesh(structure: Structure, mesh: Mesh) -> DockInput {
        DockInput::new(structure, Box::new(MeshSurface::new(mesh)))
    }
}

/// All docking settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DockParams {
    pub descriptor: DescriptorParams,
    /// Weights of the descriptor match terms.
    pub weights: matching::MatchWeights,
    /// Seeds kept per receptor critical point (best ligand points, best rotation each).
    pub per_point: usize,
    /// Seeds kept in total, best descriptor score first.
    pub max_candidates: usize,
    /// Seeds whose ligand placements agree within about this many Å are duplicates; only
    /// the best of each is kept.
    pub dedup_distance: f64,
    /// How many of the best (deduplicated) seeds get a short refinement with the soft score.
    pub quick_top: usize,
    /// Objective of the short refinement: a seed is only accurate to a few Å, so overlaps
    /// are tolerated more than in the final score.
    pub soft: ScoreParams,
    pub quick_refine: RefineParams,
    /// Final atom score (full refinement and ranking).
    pub score: ScoreParams,
    /// Spacing of the atom score grids, in Å.
    pub score_grid_spacing: f64,
    /// How many of the best poses after the short refinement get the full refinement.
    pub refine_top: usize,
    pub refine: RefineParams,
    /// Clustering radius: ligand Cα RMSD between poses, in Å.
    pub cluster_radius: f64,
    /// Number of poses (cluster representatives) returned.
    pub n_poses: usize,
}

impl Default for DockParams {
    fn default() -> Self {
        Self {
            descriptor: DescriptorParams::default(),
            weights: matching::MatchWeights::default(),
            per_point: 30,
            max_candidates: 60_000,
            dedup_distance: 2.0,
            quick_top: 2000,
            soft: ScoreParams {
                contact_cut: 3.3,
                clash_cut: 0.0,
                w_clash: 1.0,
                w_propensity: 0.0,
                ..ScoreParams::default()
            },
            quick_refine: RefineParams {
                step: 1.5,
                rotation_arc: 1.5,
                min_step: 0.4,
                max_evaluations: 40,
            },
            score: ScoreParams::default(),
            score_grid_spacing: 0.75,
            refine_top: 1000,
            refine: RefineParams {
                max_evaluations: 150,
                ..RefineParams::default()
            },
            cluster_radius: 4.0,
            n_poses: 100,
        }
    }
}

impl DockParams {
    // Negated comparisons also reject NaN.
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    pub fn validate(&self) -> Result<()> {
        self.descriptor.validate()?;
        self.soft.validate()?;
        self.score.validate()?;
        let w = &self.weights;
        if w.fill < 0
            || w.surface < 0
            || w.clash < 0
            || self.per_point == 0
            || self.max_candidates == 0
        {
            return Err(Error::Invalid(
                "match weights must be >= 0, per_point and max_candidates > 0".into(),
            ));
        }
        if !(self.cluster_radius >= 0.0) || self.n_poses == 0 {
            return Err(Error::Invalid(
                "cluster_radius must be >= 0 and n_poses > 0".into(),
            ));
        }
        Ok(())
    }
}

/// A docked placement of the ligand.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pose {
    /// 1-based rank.
    pub rank: usize,
    /// Applied to the ligand input coordinates, puts the ligand on the receptor.
    pub transform: Rigid,
    /// Final score (higher is better), `components.total(score params)`.
    pub score: f64,
    pub components: ScoreComponents,
    /// Poses within `cluster_radius` of this one (including itself) among the refined ones.
    pub cluster_size: usize,
}

/// Counts and wall times of one run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DockStats {
    pub receptor_points: usize,
    pub ligand_points: usize,
    /// Receptor × ligand critical point pairs scored (each over all rotations).
    pub pairs: u64,
    /// Seeds kept by the matching stage.
    pub candidates: usize,
    /// Seeds left after removing near-duplicates.
    pub seeds: usize,
    pub seconds_descriptors: f64,
    pub seconds_matching: f64,
    /// Seed poses, atom grids, both refinement stages.
    pub seconds_refinement: f64,
    pub seconds_total: f64,
}

/// Docking result with run statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockOutput {
    pub poses: Vec<Pose>,
    pub stats: DockStats,
}

/// Docks `ligand` onto `receptor` and returns ranked poses (see the module docs).
pub fn run(
    receptor: &DockInput,
    ligand: &DockInput,
    params: &DockParams,
    progress: &dyn Progress,
) -> Result<Vec<Pose>> {
    Ok(run_with_stats(receptor, ligand, params, progress)?.poses)
}

/// [`run`], also returning counts and timings.
pub fn run_with_stats(
    receptor: &DockInput,
    ligand: &DockInput,
    params: &DockParams,
    progress: &dyn Progress,
) -> Result<DockOutput> {
    params.validate()?;
    let t0 = Instant::now();
    let mut stats = DockStats::default();

    progress.report("descriptors", 0.0);
    let (rec, lig) = rayon::join(
        || {
            descriptor::build(
                receptor.surface.as_ref(),
                &params.descriptor,
                descriptor::Role::Receptor,
                progress,
            )
        },
        || {
            descriptor::build(
                ligand.surface.as_ref(),
                &params.descriptor,
                descriptor::Role::Ligand,
                progress,
            )
        },
    );
    let (rec, lig) = (rec?, lig?);
    stats.receptor_points = rec.len();
    stats.ligand_points = lig.len();
    stats.pairs = rec.len() as u64 * lig.len() as u64;
    stats.seconds_descriptors = t0.elapsed().as_secs_f64();
    check(progress)?;

    let t1 = Instant::now();
    progress.report("matching", 0.1);
    let matches = matching::enumerate(
        &rec,
        &lig,
        params.weights,
        params.per_point,
        params.max_candidates,
        progress,
        (0.1, 0.6),
    )?;
    stats.candidates = matches.len();
    stats.seconds_matching = t1.elapsed().as_secs_f64();
    check(progress)?;

    // Seeds in descriptor order, without near-duplicates.
    let t3 = Instant::now();
    let gap = params.descriptor.gap;
    let seeds: Vec<(Rigid, ScoreComponents, f64)> = matches
        .iter()
        .map(|m| {
            let (p, q) = (m.p as usize, m.q as usize);
            let t = descriptor::pose(
                rec.points[p],
                &rec.frames[p],
                lig.points[q],
                &lig.frames[q],
                m.k as usize,
                gap,
            );
            let c = ScoreComponents {
                shape: m.score as f64,
                ..ScoreComponents::default()
            };
            (t, c, c.shape)
        })
        .collect();
    let reference = cluster_reference(&ligand.structure);
    let mut scored = dedup(seeds, &reference, params.dedup_distance);
    stats.seeds = scored.len();
    let scorer = scoring::Scorer::new(
        &receptor.structure,
        &ligand.structure,
        params.score_grid_spacing,
    )?;
    check(progress)?;

    progress.report("refinement", 0.7);
    let final_total = |c: &ScoreComponents| c.total(&params.score);
    // Short refinement of the best seeds with the soft score, then the final score.
    scored.truncate(params.quick_top.max(params.refine_top).max(params.n_poses));
    let n_quick = params.quick_top.min(scored.len());
    let mut all: Vec<(Rigid, ScoreComponents, f64)> = scored
        .par_iter()
        .enumerate()
        .map(|(i, (t, c0, _))| {
            check(progress)?;
            let t = if i < n_quick {
                scorer.refine(t, &params.soft, &params.quick_refine).0
            } else {
                *t
            };
            let mut c = scorer.evaluate(&t, &params.score);
            c.shape = c0.shape;
            Ok((t, c, final_total(&c)))
        })
        .collect::<Result<_>>()?;
    all.sort_by(|a, b| b.2.total_cmp(&a.2));
    check(progress)?;

    // Full refinement of the best with the final score.
    progress.report("refinement", 0.8);
    all.truncate(params.refine_top.max(params.n_poses));
    let n_refine = params.refine_top.min(all.len());
    let refined: Vec<(Rigid, ScoreComponents, f64)> = all[..n_refine]
        .par_iter()
        .map(|(t, c0, _)| {
            check(progress)?;
            let (t, mut c) = scorer.refine(t, &params.score, &params.refine);
            c.shape = c0.shape;
            Ok((t, c, final_total(&c)))
        })
        .collect::<Result<_>>()?;
    all.splice(..n_refine, refined);
    all.sort_by(|a, b| b.2.total_cmp(&a.2));
    stats.seconds_refinement = t3.elapsed().as_secs_f64();

    progress.report("clustering", 0.95);
    let transforms: Vec<Rigid> = all.iter().map(|x| x.0).collect();
    let clusters = scoring::cluster(&transforms, &reference, params.cluster_radius);
    let poses = clusters
        .into_iter()
        .take(params.n_poses)
        .enumerate()
        .map(|(i, (idx, size))| Pose {
            rank: i + 1,
            transform: all[idx].0,
            score: all[idx].2,
            components: all[idx].1,
            cluster_size: size,
        })
        .collect();
    stats.seconds_total = t0.elapsed().as_secs_f64();
    progress.report("done", 1.0);
    Ok(DockOutput { poses, stats })
}

/// Atoms used to compare poses: Cα (all heavy atoms when there are none), at most 128,
/// evenly spread along the chain.
fn cluster_reference(ligand: &Structure) -> Vec<V3> {
    let mut pts: Vec<_> = ligand
        .atoms
        .iter()
        .filter(|a| a.name == "CA")
        .map(|a| a.pos)
        .collect();
    if pts.len() < 3 {
        pts = ligand.heavy_atoms().map(|a| a.pos).collect();
    }
    let step = pts.len().div_ceil(128).max(1);
    pts.into_iter().step_by(step).collect()
}

/// Drops poses that place the reference atoms like an earlier (better) pose: same cell of a
/// grid of `distance` Å for the centroid and for two far reference atoms. Approximate (two
/// near-identical poses can straddle a cell boundary) but linear time, which matters for
/// tens of thousands of seeds; the final clustering is exact.
fn dedup<T>(poses: Vec<(Rigid, T, f64)>, reference: &[V3], distance: f64) -> Vec<(Rigid, T, f64)> {
    if distance <= 0.0 || reference.is_empty() {
        return poses;
    }
    let n = reference.len() as f64;
    let c = scale(reference.iter().fold([0.0; 3], |a, b| add(a, *b)), 1.0 / n);
    let far = |from: V3| {
        *reference
            .iter()
            .max_by(|a, b| dist2(**a, from).total_cmp(&dist2(**b, from)))
            .unwrap()
    };
    let a = far(c);
    let b = far(a);
    let mut seen = std::collections::HashSet::new();
    poses
        .into_iter()
        .filter(|(t, _, _)| {
            let key: [i32; 9] = std::array::from_fn(|i| {
                let p = t.apply([c, a, b][i / 3]);
                (p[i % 3] / distance).floor() as i32
            });
            seen.insert(key)
        })
        .collect()
}

/// A copy of `s` with every atom moved by `t`.
pub fn transform_structure(s: &Structure, t: &Rigid) -> Structure {
    let mut out = s.clone();
    for a in &mut out.atoms {
        a.pos = t.apply(a.pos);
    }
    out
}

/// PDB text of the complex: receptor atoms, `TER`, ligand atoms moved by `pose`, `TER`, `END`.
/// Atom serial numbers are renumbered; chain identifiers are kept as they are.
pub fn complex_pdb(receptor: &Structure, ligand: &Structure, pose: &Rigid) -> String {
    let mut s = String::with_capacity((receptor.atoms.len() + ligand.atoms.len() + 4) * 81);
    let mut serial = 1;
    for a in &receptor.atoms {
        let mut a = a.clone();
        a.serial = serial;
        serial += 1;
        write_atom(&mut s, &a);
    }
    s.push_str("TER\n");
    for a in &ligand.atoms {
        let mut a = a.clone();
        a.serial = serial;
        a.pos = pose.apply(a.pos);
        serial += 1;
        write_atom(&mut s, &a);
    }
    s.push_str("TER\nEND\n");
    s
}
