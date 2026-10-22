//! Atom-level re-scoring, local rigid refinement and clustering of candidate poses.
//!
//! The descriptor match only looks at one ~10 Å patch. Candidates are re-scored on the whole
//! interface with the heavy atoms of both partners. With `d` the distance from an atom centre
//! to the partner's van der Waals surface (negative inside, and then as deep as the true
//! penetration, not just the distance to the nearest atom centre):
//!
//! - **contacts**: atoms with `clash_cut ≤ d ≤ contact_cut` (≈ heavy-atom pairs at 3–4.5 Å)
//!   count 1, with linear ramps of 0.5 Å below and 1 Å above; counted on both sides and
//!   averaged. Interface size and shape complementarity in one number.
//! - **clash**: `Σ max(0, clash_cut - d)²` over both sides, averaged. Setting `clash_cut`
//!   below the ideal contact distance leaves room for the side-chain rearrangements unbound
//!   docking has to tolerate.
//! - **potential**: over the ligand atoms in the interface, a coarse atom-class contact
//!   potential (apolar / polar / cationic / anionic, see [`PAIR_ENERGY`]) that rewards
//!   apolar burial and salt bridges and penalizes burying charges against apolar atoms and
//!   like charges. A desolvation and electrostatics proxy, not a fitted force field.
//!
//! Every term is a lookup in a precomputed grid (receptor fields for ligand atoms and vice
//! versa). Atoms are processed in groups of up to 8 consecutive atoms, and a group is skipped
//! after one look-up in a small, cache-resident grid of lower bounds when it is far from the
//! partner, so a pose costs ~10–30 µs whatever the protein size.

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use super::grid3::{Grid3, distance_transform};
use crate::error::{Error, Result};
use crate::geom::transform::{Rigid, axis_angle_matrix};
use crate::geom::vec3::*;
use crate::io::pdb::{Atom, Structure};

/// Distances and weights of the atom-level score.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ScoreParams {
    /// Upper end of the full-weight contact range, in Å (`d` as in the module docs).
    pub contact_cut: f64,
    /// Lower end of the contact range and onset of the clash penalty, in Å.
    pub clash_cut: f64,
    pub w_contact: f64,
    pub w_clash: f64,
    pub w_potential: f64,
}

impl Default for ScoreParams {
    fn default() -> Self {
        Self {
            contact_cut: 2.8,
            clash_cut: 1.0,
            w_contact: 1.0,
            w_clash: 5.0,
            w_potential: 1.0,
        }
    }
}

impl ScoreParams {
    pub fn validate(&self) -> Result<()> {
        let ok = self.contact_cut > 0.0
            && self.contact_cut + 1.0 < VDW_CAP
            && self.clash_cut < self.contact_cut
            && self.clash_cut > -VDW_CAP
            && [self.w_contact, self.w_clash, self.w_potential]
                .iter()
                .all(|w| w.is_finite());
        if !ok {
            return Err(Error::Invalid(
                "score: need 0 < contact_cut < 3.5, -4.5 < clash_cut < contact_cut and finite weights"
                    .into(),
            ));
        }
        Ok(())
    }
}

/// Score terms of a pose (see module docs). `shape` is the descriptor match score of the
/// seed match the pose came from.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct ScoreComponents {
    pub shape: f64,
    pub contacts: f64,
    pub clash: f64,
    pub potential: f64,
}

impl ScoreComponents {
    /// Weighted total (higher is better). `potential` is an energy, so it is subtracted.
    pub fn total(&self, p: &ScoreParams) -> f64 {
        p.w_contact * self.contacts - p.w_clash * self.clash - p.w_potential * self.potential
    }
}

/// Atom classes of the contact potential.
const APOLAR: usize = 0;
const POLAR: usize = 1;
const CATION: usize = 2;
const ANION: usize = 3;

/// Contact energy per atom pair (full up to 4.5 Å, fading to 0 at 6 Å) by class; negative is
/// favourable. Hand-set from the usual qualitative trends (hydrophobic burial, salt bridges,
/// desolvation of charges); one interface atom typically has 5–15 partners in range.
pub const PAIR_ENERGY: [[f64; 4]; 4] = [
    //  apolar  polar  cation  anion
    [-0.040, 0.010, 0.020, 0.020],   // apolar
    [0.010, -0.015, -0.020, -0.020], // polar
    [0.020, -0.020, 0.100, -0.150],  // cation
    [0.020, -0.020, -0.150, 0.100],  // anion
];
const POT_NEAR: f64 = 4.5;
const POT_FAR: f64 = 6.0;

/// Class of an atom for [`PAIR_ENERGY`].
pub fn atom_class(a: &Atom) -> usize {
    match (a.res_name.as_str(), a.name.as_str()) {
        ("LYS", "NZ") | ("ARG", "NE" | "NH1" | "NH2") => CATION,
        ("ASP", "OD1" | "OD2") | ("GLU", "OE1" | "OE2") | (_, "OXT") => ANION,
        _ => match a.element.as_str() {
            "N" | "O" => POLAR,
            _ => APOLAR,
        },
    }
}

/// Distance from the partner's vdW surface beyond which an atom contributes nothing (the vdW
/// grids are clamped here). Must exceed `contact_cut + 1`.
const VDW_CAP: f64 = 4.5;
/// Largest distance of a group member from the group's first atom, in Å.
const GROUP_RADIUS: f64 = 6.0;
const GROUP_SIZE: usize = 8;

/// Up to [`GROUP_SIZE`] consecutive atoms within [`GROUP_RADIUS`] of the first one.
#[derive(Debug, Clone, Copy)]
struct Group {
    center: V3,
    radius: f64,
    start: u32,
    end: u32,
}

fn groups(atoms: &[V3]) -> Vec<Group> {
    let mut out: Vec<Group> = Vec::new();
    for (i, p) in atoms.iter().enumerate() {
        if let Some(g) = out.last_mut() {
            let d = dist2(*p, g.center).sqrt();
            if (g.end - g.start) < GROUP_SIZE as u32 && d <= GROUP_RADIUS {
                g.end += 1;
                g.radius = g.radius.max(d);
                continue;
            }
        }
        out.push(Group {
            center: *p,
            radius: 0.0,
            start: i as u32,
            end: i as u32 + 1,
        });
    }
    out
}

/// Heavy atoms of one partner with everything needed to score them against the other.
struct Side {
    atoms: Vec<V3>,
    class: Vec<u8>,
    groups: Vec<Group>,
    /// Signed distance to this partner's vdW surface (what the *other* side's atoms see).
    field: VdwField,
}

impl Side {
    fn new(atoms: &[&Atom], grid_spacing: f64) -> Side {
        let pos: Vec<V3> = atoms.iter().map(|a| a.pos).collect();
        Side {
            groups: groups(&pos),
            atoms: pos,
            class: atoms.iter().map(|a| atom_class(a) as u8).collect(),
            field: VdwField::new(atoms, grid_spacing),
        }
    }
}

/// Precomputed fields of a receptor/ligand pair.
pub struct Scorer {
    rec: Side,
    lig: Side,
    /// Potential felt by a ligand atom of each class, from all receptor atoms.
    rec_pot: Vec<Grid3>,
    lig_center: V3,
    lig_radius: f64,
}

/// Groups of both partners that can contribute near a given pose.
struct Interface {
    lig: Vec<u32>,
    rec: Vec<u32>,
}

impl Scorer {
    /// `grid_spacing`: spacing of the van der Waals distance grids, in Å.
    pub fn new(receptor: &Structure, ligand: &Structure, grid_spacing: f64) -> Result<Scorer> {
        if !(0.1..=2.0).contains(&grid_spacing) {
            return Err(Error::Invalid(
                "score grid spacing must be in [0.1, 2] Å".into(),
            ));
        }
        let rec: Vec<&Atom> = receptor.heavy_atoms().collect();
        let lig: Vec<&Atom> = ligand.heavy_atoms().collect();
        if rec.is_empty() || lig.is_empty() {
            return Err(Error::Invalid(
                "receptor and ligand need heavy atoms".into(),
            ));
        }
        let ((rec_side, lig_side), rec_pot) = rayon::join(
            || {
                rayon::join(
                    || Side::new(&rec, grid_spacing),
                    || Side::new(&lig, grid_spacing),
                )
            },
            || {
                (0..4)
                    .into_par_iter()
                    .map(|t| potential_field(&rec, t))
                    .collect()
            },
        );
        let n = lig.len() as f64;
        let lig_center = scale(lig.iter().fold([0.0; 3], |s, a| add(s, a.pos)), 1.0 / n);
        let lig_radius = lig
            .iter()
            .map(|a| dist2(a.pos, lig_center))
            .fold(0.0, f64::max)
            .sqrt();
        Ok(Scorer {
            rec: rec_side,
            lig: lig_side,
            rec_pot,
            lig_center,
            lig_radius,
        })
    }

    /// Centroid of the ligand heavy atoms (input coordinates).
    pub fn ligand_center(&self) -> V3 {
        self.lig_center
    }

    /// Score terms of the ligand placed by `t` (`shape` left at 0).
    pub fn evaluate(&self, t: &Rigid, p: &ScoreParams) -> ScoreComponents {
        self.evaluate_on(t, p, None, true)
    }

    pub fn total(&self, t: &Rigid, p: &ScoreParams) -> f64 {
        self.evaluate(t, p).total(p)
    }

    /// `only`: restrict to these groups; `cull`: skip groups far from the partner (exact,
    /// switchable for tests).
    fn evaluate_on(
        &self,
        t: &Rigid,
        p: &ScoreParams,
        only: Option<&Interface>,
        cull: bool,
    ) -> ScoreComponents {
        let soft = p.clash_cut - 0.5;
        let contact = |d: f64| {
            if d < soft {
                0.0
            } else if d < p.clash_cut {
                (d - soft) / 0.5
            } else if d <= p.contact_cut {
                1.0
            } else if d < p.contact_cut + 1.0 {
                p.contact_cut + 1.0 - d
            } else {
                0.0
            }
        };
        let clash = |d: f64| (p.clash_cut - d).max(0.0).powi(2);
        let mut c = ScoreComponents::default();

        // Ligand atoms in the receptor fields.
        let mut lig_contacts = 0.0;
        let mut lig_group = |g: &Group, c: &mut ScoreComponents| {
            if cull && self.rec.field.is_far(t.apply(g.center), g.radius) {
                return;
            }
            for i in g.start as usize..g.end as usize {
                let x = t.apply(self.lig.atoms[i]);
                if cull && self.rec.field.is_far(x, 0.0) {
                    continue;
                }
                let d = self.rec.field.fine.sample(x);
                if d >= VDW_CAP {
                    continue;
                }
                lig_contacts += contact(d);
                c.clash += clash(d);
                c.potential += self.rec_pot[self.lig.class[i] as usize].sample_or_zero(x);
            }
        };
        match only {
            Some(s) => s
                .lig
                .iter()
                .for_each(|&g| lig_group(&self.lig.groups[g as usize], &mut c)),
            None => self.lig.groups.iter().for_each(|g| lig_group(g, &mut c)),
        }

        // Receptor atoms in the ligand field (ligand frame).
        let inv = t.inverse();
        let mut rec_contacts = 0.0;
        let mut rec_group = |g: &Group, c: &mut ScoreComponents| {
            if cull && self.lig.field.is_far(inv.apply(g.center), g.radius) {
                return;
            }
            for i in g.start as usize..g.end as usize {
                let y = inv.apply(self.rec.atoms[i]);
                if cull && self.lig.field.is_far(y, 0.0) {
                    continue;
                }
                let d = self.lig.field.fine.sample(y);
                if d >= VDW_CAP {
                    continue;
                }
                rec_contacts += contact(d);
                c.clash += clash(d);
            }
        };
        match only {
            Some(s) => s
                .rec
                .iter()
                .for_each(|&g| rec_group(&self.rec.groups[g as usize], &mut c)),
            None => self.rec.groups.iter().for_each(|g| rec_group(g, &mut c)),
        }
        c.contacts = 0.5 * (lig_contacts + rec_contacts);
        c.clash *= 0.5;
        c
    }

    /// Groups that can contribute for any pose moving no atom more than `margin` Å from
    /// where `t` puts it.
    fn interface(&self, t: &Rigid, margin: f64) -> Interface {
        let lig = (0..self.lig.groups.len() as u32)
            .filter(|&i| {
                let g = &self.lig.groups[i as usize];
                !self.rec.field.is_far(t.apply(g.center), g.radius + margin)
            })
            .collect();
        let inv = t.inverse();
        let rec = (0..self.rec.groups.len() as u32)
            .filter(|&i| {
                let g = &self.rec.groups[i as usize];
                !self
                    .lig
                    .field
                    .is_far(inv.apply(g.center), g.radius + margin)
            })
            .collect();
        Interface { lig, rec }
    }

    /// Local rigid refinement by pattern search: rotations about the ligand centre and
    /// translations along the axes, first improvement wins, steps halve when stuck.
    /// Deterministic and monotone (never returns a worse pose).
    ///
    /// Only interface groups are evaluated during the search; the set is rebuilt whenever the
    /// accumulated displacement could exceed its margin, so the scores are exact.
    pub fn refine(
        &self,
        start: &Rigid,
        p: &ScoreParams,
        params: &RefineParams,
    ) -> (Rigid, ScoreComponents) {
        const MARGIN: f64 = 2.0;
        let radius = self.lig_radius;
        let mut best = *start;
        let mut iface = self.interface(&best, MARGIN);
        let mut anchor = best;
        let mut best_s = self.evaluate_on(&best, p, Some(&iface), true).total(p);
        // Rotation steps are sized so that the farthest atoms move `rotation_arc` times the
        // translation step: a fixed angle would be far too coarse for a large ligand.
        let mut tr = params.step;
        let mut rot = params.rotation_arc * tr / radius.max(1.0);
        let axes = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let mut evals = 0;
        while evals < params.max_evaluations && tr >= params.min_step {
            // Can a candidate of this round leave the margin of the current interface?
            let moved = max_displacement(&anchor, &best, self.lig_center, radius);
            if moved + tr + 2.0 * (rot / 2.0).sin() * radius > MARGIN {
                iface = self.interface(&best, MARGIN);
                anchor = best;
            }
            let mut improved = false;
            let center = best.apply(self.lig_center);
            'moves: for axis in axes {
                for sign in [1.0, -1.0] {
                    for kind in 0..2 {
                        let mv = if kind == 0 {
                            Rigid::translation(scale(axis, sign * tr))
                        } else {
                            Rigid::about(center, axis_angle_matrix(axis, sign * rot))
                        };
                        let cand = mv.compose(&best);
                        let s = self.evaluate_on(&cand, p, Some(&iface), true).total(p);
                        evals += 1;
                        if s > best_s + 1e-9 {
                            best = cand;
                            best_s = s;
                            improved = true;
                            break 'moves;
                        }
                    }
                }
            }
            if !improved {
                rot *= 0.5;
                tr *= 0.5;
            }
        }
        (best, self.evaluate(&best, p))
    }
}

/// Upper bound of how far any point within `radius` of `center` (ligand input frame) moves
/// between placements `a` and `b`: shift of the centre plus the chord of the rotation angle.
fn max_displacement(a: &Rigid, b: &Rigid, center: V3, radius: f64) -> f64 {
    let shift = norm(sub(a.apply(center), b.apply(center)));
    let r = a.inverse().compose(b).rot;
    let cos = ((r[0][0] + r[1][1] + r[2][2] - 1.0) / 2.0).clamp(-1.0, 1.0);
    shift + 2.0 * (cos.acos() / 2.0).sin() * radius
}

/// Settings of [`Scorer::refine`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RefineParams {
    /// Initial translation step, Å.
    pub step: f64,
    /// Rotation steps move the ligand atoms farthest from its centre by this many
    /// translation steps.
    pub rotation_arc: f64,
    /// Stop when the translation step falls below this, Å.
    pub min_step: f64,
    /// Score evaluations per pose at most.
    pub max_evaluations: usize,
}

impl Default for RefineParams {
    fn default() -> Self {
        Self {
            step: 1.0,
            rotation_arc: 1.5,
            min_step: 0.12,
            max_evaluations: 250,
        }
    }
}

/// Signed distance to the van der Waals surface of a set of atoms, plus a coarse grid of
/// lower bounds to skip far atoms cheaply (the fine grid does not fit in cache).
struct VdwField {
    fine: Grid3,
    /// Minimum of the fine field over each block of `COARSE`³ cells.
    coarse: Grid3,
    inv_coarse_h: f64,
}

const COARSE: usize = 2;

impl VdwField {
    /// Outside: exact `min |x - c| - r`, clamped at [`VDW_CAP`]. Inside: minus the distance
    /// to the outside (distance transform), so deep overlaps get large penalties; the plain
    /// `min |x - c| - r` is never deeper than one atom radius.
    fn new(atoms: &[&Atom], h: f64) -> VdwField {
        let cap = VDW_CAP;
        // Wide margin: within GROUP_RADIUS + 2 Å + 2h of the box boundary the field is >= cap,
        // so a group whose centre is outside the box can be skipped.
        let (lo, hi) = bounds(atoms, cap + GROUP_RADIUS + 6.0);
        let mut fine = Grid3::covering(lo, hi, h, cap as f32);
        splat(
            &mut fine,
            atoms,
            |a| a.vdw_radius() + cap,
            |a, d| (d - a.vdw_radius()) as f32,
            f32::min,
        );
        let outside: Vec<bool> = fine.data.iter().map(|&v| v >= 0.0).collect();
        let inside: Vec<bool> = outside.iter().map(|&o| !o).collect();
        let (depth, to_inside) = rayon::join(
            || distance_transform(fine.dims, h, &outside),
            || distance_transform(fine.dims, h, &inside),
        );
        for (v, d) in fine.data.iter_mut().zip(&depth) {
            if *v < 0.0 {
                *v = -d.max(-*v);
            }
        }
        // Lower bound of the true distance at each node, also beyond the clamp: the nearest
        // inside node is at most ~h farther than the surface.
        let bound = |idx: usize| {
            let v = fine.data[idx];
            if v < cap as f32 {
                v
            } else {
                v.max(to_inside[idx] - h as f32)
            }
        };
        // Coarse block (I, J, K) covers fine nodes I*C ..= (I+1)*C: every node a trilinear
        // sample inside the block can touch.
        let cd: [usize; 3] = std::array::from_fn(|a| (fine.dims[a] - 1).div_ceil(COARSE));
        let mut coarse = Grid3 {
            origin: fine.origin,
            h: h * COARSE as f64,
            dims: cd,
            data: vec![f32::INFINITY; cd[0] * cd[1] * cd[2]],
        };
        let blocks = |n: usize, d: usize| {
            let hi = (n / COARSE).min(d - 1);
            let lo = if n % COARSE == 0 && n > 0 {
                n / COARSE - 1
            } else {
                hi
            };
            lo.min(hi)..=hi
        };
        for k in 0..fine.dims[2] {
            for j in 0..fine.dims[1] {
                for i in 0..fine.dims[0] {
                    let v = bound(fine.index(i, j, k));
                    for ck in blocks(k, cd[2]) {
                        for cj in blocks(j, cd[1]) {
                            for ci in blocks(i, cd[0]) {
                                let idx = coarse.index(ci, cj, ck);
                                coarse.data[idx] = coarse.data[idx].min(v);
                            }
                        }
                    }
                }
            }
        }
        let inv_coarse_h = 1.0 / coarse.h;
        VdwField {
            fine,
            coarse,
            inv_coarse_h,
        }
    }

    /// True when every point within `radius` of `p` is certainly at least [`VDW_CAP`] from
    /// the surface (the field is 1-Lipschitz; `2h` absorbs interpolation slack).
    #[inline]
    fn is_far(&self, p: V3, radius: f64) -> bool {
        let g = &self.coarse;
        let mut idx = [0usize; 3];
        for a in 0..3 {
            let u = (p[a] - g.origin[a]) * self.inv_coarse_h;
            if !(0.0..g.dims[a] as f64).contains(&u) {
                return radius <= GROUP_RADIUS + 2.0;
            }
            idx[a] = u as usize;
        }
        let lower = g.data[g.index(idx[0], idx[1], idx[2])] as f64;
        lower - radius - 2.0 * self.fine.h >= VDW_CAP
    }
}

/// Bounding box of atoms grown by `pad`.
fn bounds(atoms: &[&Atom], pad: f64) -> (V3, V3) {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for a in atoms {
        for k in 0..3 {
            lo[k] = lo[k].min(a.pos[k] - pad);
            hi[k] = hi[k].max(a.pos[k] + pad);
        }
    }
    (lo, hi)
}

/// Splats a per-atom kernel `f(atom, distance) -> value` (zero beyond `reach`) into `grid`,
/// combining with `op`. Parallel over z planes; atoms are pre-binned by plane.
fn splat<F, O>(grid: &mut Grid3, atoms: &[&Atom], reach: impl Fn(&Atom) -> f64, f: F, op: O)
where
    F: Fn(&Atom, f64) -> f32 + Sync,
    O: Fn(f32, f32) -> f32 + Sync,
{
    let [nx, ny, nz] = grid.dims;
    let mut per_plane: Vec<Vec<u32>> = vec![Vec::new(); nz];
    let reaches: Vec<f64> = atoms.iter().map(|a| reach(a)).collect();
    for (i, a) in atoms.iter().enumerate() {
        if let Some([_, _, (k0, k1)]) = grid.node_range(a.pos, reaches[i]) {
            for plane in &mut per_plane[k0..=k1] {
                plane.push(i as u32);
            }
        }
    }
    let (origin, h) = (grid.origin, grid.h);
    let shape = grid.shape_only();
    grid.data
        .par_chunks_mut(nx * ny)
        .enumerate()
        .for_each(|(k, plane)| {
            let z = origin[2] + h * k as f64;
            for &ai in &per_plane[k] {
                let a = atoms[ai as usize];
                let r = reaches[ai as usize];
                let Some([(i0, i1), (j0, j1), _]) = shape.node_range(a.pos, r) else {
                    continue;
                };
                let dz2 = (z - a.pos[2]).powi(2);
                for j in j0..=j1 {
                    let y = origin[1] + h * j as f64;
                    let dyz2 = dz2 + (y - a.pos[1]).powi(2);
                    if dyz2 > r * r {
                        continue;
                    }
                    for i in i0..=i1 {
                        let x = origin[0] + h * i as f64;
                        let d2 = dyz2 + (x - a.pos[0]).powi(2);
                        if d2 <= r * r {
                            let v = &mut plane[j * nx + i];
                            *v = op(*v, f(a, d2.sqrt()));
                        }
                    }
                }
            }
        });
}

/// Potential felt by a probe atom of class `class` from `atoms` (1 Å grid).
fn potential_field(atoms: &[&Atom], class: usize) -> Grid3 {
    let (lo, hi) = bounds(atoms, POT_FAR + 1.0);
    let mut g = Grid3::covering(lo, hi, 1.0, 0.0);
    splat(
        &mut g,
        atoms,
        |_| POT_FAR,
        |a, d| {
            let w = if d <= POT_NEAR {
                1.0
            } else {
                (POT_FAR - d) / (POT_FAR - POT_NEAR)
            };
            (PAIR_ENERGY[class][atom_class(a)] * w) as f32
        },
        |acc, v| acc + v,
    );
    g
}

/// Greedy clustering of poses already sorted best-first: a pose joins the first cluster
/// whose representative places the reference atoms within `radius` Å RMSD, otherwise it
/// starts a new cluster. Returns `(representative index, members)` in order of creation.
pub fn cluster(poses: &[Rigid], reference: &[V3], radius: f64) -> Vec<(usize, usize)> {
    let r2 = radius * radius * reference.len() as f64;
    let mut reps: Vec<(usize, Vec<V3>, usize)> = Vec::new();
    for (i, t) in poses.iter().enumerate() {
        let x: Vec<V3> = reference.iter().map(|p| t.apply(*p)).collect();
        let hit = reps.iter_mut().find(|(_, y, _)| {
            let mut s = 0.0;
            for (a, b) in x.iter().zip(y.iter()) {
                s += dist2(*a, *b);
                if s >= r2 {
                    return false;
                }
            }
            true
        });
        match hit {
            Some(rep) => rep.2 += 1,
            None => reps.push((i, x, 1)),
        }
    }
    reps.into_iter().map(|(i, _, n)| (i, n)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::transform::Rng;

    fn atom(pos: V3, res: &str, name: &str, element: &str) -> Atom {
        Atom {
            serial: 1,
            name: name.into(),
            alt_loc: ' ',
            res_name: res.into(),
            chain: 'A',
            res_seq: 1,
            i_code: ' ',
            pos,
            occupancy: 1.0,
            b_factor: 0.0,
            element: element.into(),
            hetatm: false,
        }
    }

    #[test]
    fn classes() {
        assert_eq!(atom_class(&atom([0.0; 3], "LYS", "NZ", "N")), CATION);
        assert_eq!(atom_class(&atom([0.0; 3], "GLU", "OE2", "O")), ANION);
        assert_eq!(atom_class(&atom([0.0; 3], "SER", "OG", "O")), POLAR);
        assert_eq!(atom_class(&atom([0.0; 3], "LEU", "CD1", "C")), APOLAR);
    }

    #[test]
    fn contact_and_clash_terms() {
        let one = || Structure {
            atoms: vec![atom([0.0; 3], "LEU", "CD1", "C")],
        };
        let s = Scorer::new(&one(), &one(), 0.5).unwrap();
        let p = ScoreParams::default();
        // 3.8 Å apart: a clean apolar contact on both sides.
        let c = s.evaluate(&Rigid::translation([3.8, 0.0, 0.0]), &p);
        assert!((c.contacts - 1.0).abs() < 1e-6, "{c:?}");
        assert!(c.clash.abs() < 1e-9);
        assert!((c.potential - PAIR_ENERGY[0][0]).abs() < 1e-3);
        // 2.0 Å apart: d ≈ 0.3 Å, clash (1.0 - 0.3)² on each side, averaged.
        let c = s.evaluate(&Rigid::translation([2.0, 0.0, 0.0]), &p);
        assert!((c.clash - 0.49).abs() < 0.06, "{c:?}");
        assert!(c.contacts < 0.1);
        // Far apart: nothing.
        let c = s.evaluate(&Rigid::translation([20.0, 0.0, 0.0]), &p);
        assert_eq!(c, ScoreComponents::default());
    }

    /// Deep overlaps are penalized by their true depth, not by the distance to the nearest
    /// atom centre.
    #[test]
    fn deep_overlap_is_costly() {
        let mut atoms = Vec::new();
        for i in -3..=3 {
            for j in -3..=3 {
                for k in -3..=3 {
                    let p = [i as f64 * 1.5, j as f64 * 1.5, k as f64 * 1.5];
                    atoms.push(atom(p, "LEU", "CD1", "C"));
                }
            }
        }
        let block = Structure { atoms };
        let probe = Structure {
            atoms: vec![atom([0.0; 3], "LEU", "CD1", "C")],
        };
        let s = Scorer::new(&block, &probe, 0.5).unwrap();
        let p = ScoreParams::default();
        let centre = s.evaluate(&Rigid::IDENTITY, &p).clash;
        let rim = s.evaluate(&Rigid::translation([6.0, 0.0, 0.0]), &p).clash;
        assert!(centre > 20.0 && centre > 4.0 * rim, "{centre} {rim}");
    }

    /// Group culling and interface subsets give exactly the brute-force score.
    #[test]
    fn culling_is_exact() {
        let mut rng = Rng::new(4);
        let mut chain = |n: usize, off: f64| {
            let mut p = [off, 0.0, 0.0];
            (0..n)
                .map(|_| {
                    p = add(
                        p,
                        scale(
                            normalize([
                                rng.next_f64() - 0.5,
                                rng.next_f64() - 0.5,
                                rng.next_f64() - 0.5,
                            ]),
                            1.5,
                        ),
                    );
                    atom(p, "ALA", "CB", "C")
                })
                .collect::<Vec<_>>()
        };
        let rec = Structure {
            atoms: chain(300, 0.0),
        };
        let lig = Structure {
            atoms: chain(120, 0.0),
        };
        let s = Scorer::new(&rec, &lig, 0.5).unwrap();
        let p = ScoreParams::default();
        let mut rng = Rng::new(8);
        for _ in 0..50 {
            let t = Rigid {
                rot: rng.rotation(),
                trans: [
                    rng.next_f64() * 20.0 - 10.0,
                    rng.next_f64() * 20.0 - 10.0,
                    rng.next_f64() * 20.0 - 10.0,
                ],
            };
            let fast = s.evaluate(&t, &p);
            let brute = s.evaluate_on(&t, &p, None, false);
            assert!((fast.total(&p) - brute.total(&p)).abs() < 1e-9);
            let iface = s.interface(&t, 2.0);
            let sub = s.evaluate_on(&t, &p, Some(&iface), true);
            assert!((fast.total(&p) - sub.total(&p)).abs() < 1e-9);
        }
    }

    #[test]
    fn refinement_never_worsens() {
        let rec = Structure {
            atoms: (0..40)
                .map(|i| {
                    atom(
                        [
                            (i % 5) as f64 * 1.6,
                            (i / 5 % 4) as f64 * 1.6,
                            (i / 20) as f64 * 1.6,
                        ],
                        "LEU",
                        "CD1",
                        "C",
                    )
                })
                .collect(),
        };
        let lig = Structure {
            atoms: (0..10)
                .map(|i| atom([i as f64 * 1.5, 0.0, 0.0], "LEU", "CD1", "C"))
                .collect(),
        };
        let s = Scorer::new(&rec, &lig, 0.5).unwrap();
        let p = ScoreParams::default();
        let start = Rigid::translation([-2.0, 2.0, 6.0]);
        let (t, c) = s.refine(&start, &p, &RefineParams::default());
        assert!(c.total(&p) >= s.total(&start, &p) - 1e-9);
        assert!((c.total(&p) - s.total(&t, &p)).abs() < 1e-9);
    }

    #[test]
    fn clustering_groups_nearby_poses() {
        let reference = vec![[0.0, 0.0, 0.0], [5.0, 0.0, 0.0], [0.0, 5.0, 0.0]];
        let poses = vec![
            Rigid::IDENTITY,
            Rigid::translation([1.0, 0.0, 0.0]),
            Rigid::translation([10.0, 0.0, 0.0]),
            Rigid::translation([0.0, 2.0, 0.0]),
            Rigid::translation([10.5, 0.0, 0.0]),
        ];
        let c = cluster(&poses, &reference, 3.0);
        assert_eq!(c, vec![(0, 3), (2, 2)]);
    }
}
