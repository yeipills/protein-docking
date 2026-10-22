//! CAPRI/DockQ quality of docked poses against a native (bound) complex.

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use super::align::{ChainPair, Correspondence, correspondences};
use super::grid::Grid;
use super::residues::{BACKBONE, Residue, protein_residues};
use crate::error::{Error, Result};
use crate::geom::transform::{Rigid, kabsch, rmsd};
use crate::geom::vec3::{V3, add, dist2, scale};
use crate::io::pdb::Structure;

/// Residue contact: any heavy-atom pair closer than this (Å). Used for fnat.
pub const CONTACT_CUTOFF: f64 = 5.0;
/// Interface residue: any heavy atom closer than this (Å) to the partner. Used for iRMSD.
pub const INTERFACE_CUTOFF: f64 = 10.0;
/// Upper bound on chain-mapping combinations scored per pose (see [`correspondences`]).
const MAX_MAPPINGS: usize = 16;

/// CAPRI quality class, ordered from worst to best.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Capri {
    Incorrect,
    Acceptable,
    Medium,
    High,
}

impl Capri {
    /// The three classes that count as a success, from least to most demanding.
    pub const HITS: [Capri; 3] = [Capri::Acceptable, Capri::Medium, Capri::High];

    /// CAPRI class from fnat, iRMSD and LRMSD (Méndez et al., Proteins 2003; Lensink et
    /// al., Proteins 2007): the best class whose fnat threshold is met together with either
    /// of its RMSD thresholds.
    ///
    /// | class | fnat | LRMSD | | iRMSD |
    /// |---|---|---|---|---|
    /// | high | ≥ 0.5 | ≤ 1 Å | or | ≤ 1 Å |
    /// | medium | ≥ 0.3 | ≤ 5 Å | or | ≤ 2 Å |
    /// | acceptable | ≥ 0.1 | ≤ 10 Å | or | ≤ 4 Å |
    pub fn classify(fnat: f64, irmsd: f64, lrmsd: f64) -> Capri {
        if fnat >= 0.5 && (lrmsd <= 1.0 || irmsd <= 1.0) {
            Capri::High
        } else if fnat >= 0.3 && (lrmsd <= 5.0 || irmsd <= 2.0) {
            Capri::Medium
        } else if fnat >= 0.1 && (lrmsd <= 10.0 || irmsd <= 4.0) {
            Capri::Acceptable
        } else {
            Capri::Incorrect
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Capri::Incorrect => "incorrect",
            Capri::Acceptable => "acceptable",
            Capri::Medium => "medium",
            Capri::High => "high",
        }
    }
}

/// DockQ score (Basu & Wallner, PLoS One 2016): the mean of fnat and the scaled iRMSD and
/// LRMSD, in `[0, 1]`. Thresholds 0.23 / 0.49 / 0.80 roughly match the CAPRI classes.
pub fn dockq(fnat: f64, irmsd: f64, lrmsd: f64) -> f64 {
    let s = |r: f64, d: f64| 1.0 / (1.0 + (r / d) * (r / d));
    (fnat + s(irmsd, 1.5) + s(lrmsd, 8.5)) / 3.0
}

/// Quality of one pose.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Metrics {
    /// Fraction of native residue contacts reproduced by the pose.
    pub fnat: f64,
    /// Fraction of the pose's residue contacts that are not native.
    pub fnonnat: f64,
    /// Backbone RMSD (Å) of the native interface residues after optimal superposition.
    pub irmsd: f64,
    /// Backbone RMSD (Å) of the smaller partner after superposing the larger one.
    pub lrmsd: f64,
    pub dockq: f64,
    pub capri: Capri,
    /// Native contacts reproduced, out of `nat_total`.
    pub nat_correct: usize,
    /// Native contacts in the bound complex (including residues absent from the model).
    pub nat_total: usize,
    /// Residue contacts in the pose (between residues present in both structures).
    pub model_contacts: usize,
}

/// Which partner is superposed for LRMSD (CAPRI: the larger one).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Receptor,
    Ligand,
}

/// Static description of the native complex and of the unbound→bound mapping.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NativeInfo {
    /// Chain pairing bound→unbound of the receptor (`a` = bound chain, `b` = unbound chain).
    pub receptor_chains: Vec<ChainPair>,
    pub ligand_chains: Vec<ChainPair>,
    /// Protein residues of the bound receptor / ligand.
    pub receptor_residues: usize,
    pub ligand_residues: usize,
    /// Bound residues matched to an unbound residue.
    pub receptor_matched: usize,
    pub ligand_matched: usize,
    /// Native residue contacts, and how many of them involve only matched residues (the
    /// best fnat any pose of these unbound structures can reach is their ratio).
    pub native_contacts: usize,
    pub matched_contacts: usize,
    /// Native interface residues (matched) on each side, used for iRMSD.
    pub receptor_interface: usize,
    pub ligand_interface: usize,
    /// Partner superposed for LRMSD.
    pub lrmsd_reference: Side,
    /// Chain-mapping alternatives scored per pose (> 1 with identical chain copies).
    pub mappings: usize,
}

/// One unbound↔bound mapping and everything precomputed from it.
struct Mapping {
    rec_chains: Vec<ChainPair>,
    lig_chains: Vec<ChainPair>,
    rec_matched: Vec<bool>,
    lig_matched: Vec<bool>,
    /// Native contacts between matched residues, as sorted keys of unbound residue indices.
    native_keys: Vec<u64>,
    /// iRMSD atoms: unbound receptor (fixed), unbound ligand (moved by the pose), and the
    /// native positions of both, in that order.
    irms_rec: Vec<V3>,
    irms_lig: Vec<V3>,
    irms_native: Vec<V3>,
    /// LRMSD: superposition of the reference partner (unbound → bound), and backbone atoms
    /// of the other partner (unbound coordinates and native ones).
    lrms_fit: Rigid,
    lrms_mobile: Vec<V3>,
    lrms_target: Vec<V3>,
}

/// A native complex prepared for fast evaluation of many poses of the unbound ligand.
///
/// The receptor stays fixed; a pose is the rigid transform applied to the unbound ligand
/// coordinates as given to [`Native::new`]. Everything that does not depend on the pose
/// (residue correspondence, native contacts, interface atoms, receptor superposition) is
/// computed once, so [`Native::evaluate`] only transforms the ligand residues near the
/// receptor, finds their contacts on a grid and runs one small superposition (about
/// 0.1 ms per pose and core for a typical complex).
pub struct Native {
    nat_total: usize,
    lrmsd_reference: Side,
    mappings: Vec<Mapping>,
    /// Receptor heavy atoms (of residues matched in some mapping) and their residue index.
    rec_grid: Grid,
    rec_atom_res: Vec<u32>,
    /// Centres of those receptor residues, for pruning (see [`LigandResidue::reach`]).
    rec_res_grid: Grid,
    /// Ligand heavy atoms of residues matched in some mapping, grouped by residue; the
    /// residues (`lig_res`) and their indices among the ligand's residues (`lig_res_index`).
    lig_atoms: Vec<V3>,
    lig_res: Vec<LigandResidue>,
    lig_res_index: Vec<u32>,
    optimal_pose: Rigid,
    info: NativeInfo,
}

/// Bounding sphere of a ligand residue, used to skip residues far from the receptor.
struct LigandResidue {
    atoms: std::ops::Range<usize>,
    center: V3,
    reach: f64,
}

/// Centre (mean of the heavy atoms) and radius of a residue.
fn sphere(s: &Structure, r: &Residue) -> (V3, f64) {
    let n = r.atoms.len().max(1) as f64;
    let c = r
        .atoms
        .iter()
        .fold([0.0; 3], |acc, &k| add(acc, s.atoms[k].pos));
    let c = scale(c, 1.0 / n);
    let radius = r
        .atoms
        .iter()
        .map(|&k| dist2(s.atoms[k].pos, c))
        .fold(0.0, f64::max)
        .sqrt();
    (c, radius)
}

fn key(r: usize, l: usize) -> u64 {
    ((r as u64) << 32) | l as u64
}

/// Residue pairs `(i, j)` with any heavy-atom pair closer than `cutoff`, restricted to
/// residues for which `keep_a` / `keep_b` hold. Sorted, without duplicates.
fn residue_contacts(
    sa: &Structure,
    ra: &[Residue],
    keep_a: impl Fn(usize) -> bool,
    sb: &Structure,
    rb: &[Residue],
    keep_b: impl Fn(usize) -> bool,
    cutoff: f64,
) -> Vec<u64> {
    let mut pts = Vec::new();
    let mut owner = Vec::new();
    for (j, r) in rb.iter().enumerate().filter(|(j, _)| keep_b(*j)) {
        for &k in &r.atoms {
            pts.push(sb.atoms[k].pos);
            owner.push(j);
        }
    }
    let grid = Grid::new(pts, cutoff);
    let mut keys = Vec::new();
    for (i, r) in ra.iter().enumerate().filter(|(i, _)| keep_a(*i)) {
        for &k in &r.atoms {
            grid.for_each_within(sa.atoms[k].pos, cutoff * cutoff, |p| {
                keys.push(key(i, owner[p]))
            });
        }
    }
    keys.sort_unstable();
    keys.dedup();
    keys
}

/// Backbone atom pairs (unbound position, bound position) of the matched residues for which
/// `select(bound residue)` holds; only atoms present in both residues are used.
fn backbone_pairs(
    sb: &Structure,
    rb: &[Residue],
    su: &Structure,
    ru: &[Residue],
    pairs: &[(usize, usize)],
    select: impl Fn(usize) -> bool,
) -> (Vec<V3>, Vec<V3>) {
    let (mut u, mut b) = (Vec::new(), Vec::new());
    for &(ib, iu) in pairs.iter().filter(|(ib, _)| select(*ib)) {
        for name in BACKBONE {
            if let (Some(x), Some(y)) = (rb[ib].atom(sb, name), ru[iu].atom(su, name)) {
                b.push(sb.atoms[x].pos);
                u.push(su.atoms[y].pos);
            }
        }
    }
    (u, b)
}

impl Native {
    /// Prepares the evaluation of poses of `ligand` (unbound) docked onto `receptor`
    /// (unbound), against the native complex `native_receptor` + `native_ligand` (bound).
    ///
    /// The "unbound" structures can also be a model's receptor and ligand chains: then
    /// `evaluate(&Rigid::IDENTITY)` scores the model as it is.
    pub fn new(
        native_receptor: &Structure,
        native_ligand: &Structure,
        receptor: &Structure,
        ligand: &Structure,
    ) -> Result<Native> {
        let (rb, lb) = (
            protein_residues(native_receptor),
            protein_residues(native_ligand),
        );
        let (ru, lu) = (protein_residues(receptor), protein_residues(ligand));
        if rb.is_empty() || lb.is_empty() || ru.is_empty() || lu.is_empty() {
            return Err(Error::Invalid("a structure has no protein residues".into()));
        }
        let rec_corr = correspondences(native_receptor, &rb, receptor, &ru);
        let lig_corr = correspondences(native_ligand, &lb, ligand, &lu);
        if rec_corr.is_empty() || lig_corr.is_empty() {
            return Err(Error::Invalid(format!(
                "no sequence correspondence between the bound and unbound {}",
                if rec_corr.is_empty() {
                    "receptor"
                } else {
                    "ligand"
                }
            )));
        }

        // Native contacts over all bound residues: contacts of residues missing from the
        // unbound structures still count in the denominator of fnat (as in DockQ).
        let native = residue_contacts(
            native_receptor,
            &rb,
            |_| true,
            native_ligand,
            &lb,
            |_| true,
            CONTACT_CUTOFF,
        );
        if native.is_empty() {
            return Err(Error::Invalid(
                "the native receptor and ligand are not in contact".into(),
            ));
        }
        // CAPRI superposes the larger partner for LRMSD (ties: the receptor).
        let lrmsd_reference = if rb.len() >= lb.len() {
            Side::Receptor
        } else {
            Side::Ligand
        };

        let mut mappings = Vec::new();
        let mut combos: Vec<(&Correspondence, &Correspondence)> = Vec::new();
        'outer: for rc in &rec_corr {
            for lc in &lig_corr {
                if combos.len() == MAX_MAPPINGS {
                    break 'outer;
                }
                combos.push((rc, lc));
            }
        }
        let mut info = None;
        let mut optimal_pose = Rigid::IDENTITY;
        for (rc, lc) in &combos {
            let mut rec_b2u = vec![None; rb.len()];
            let mut lig_b2u = vec![None; lb.len()];
            for &(b, u) in &rc.pairs {
                rec_b2u[b] = Some(u);
            }
            for &(b, u) in &lc.pairs {
                lig_b2u[b] = Some(u);
            }
            let split = |k: u64| ((k >> 32) as usize, (k & 0xffff_ffff) as usize);
            let mut native_keys: Vec<u64> = native
                .iter()
                .filter_map(|&k| {
                    let (r, l) = split(k);
                    Some(key(rec_b2u[r]?, lig_b2u[l]?))
                })
                .collect();
            native_keys.sort_unstable();

            // Interface residues: matched bound residues within 10 Å of a matched partner residue.
            let iface = residue_contacts(
                native_receptor,
                &rb,
                |i| rec_b2u[i].is_some(),
                native_ligand,
                &lb,
                |j| lig_b2u[j].is_some(),
                INTERFACE_CUTOFF,
            );
            let mut rec_iface = vec![false; rb.len()];
            let mut lig_iface = vec![false; lb.len()];
            for &k in &iface {
                let (r, l) = split(k);
                rec_iface[r] = true;
                lig_iface[l] = true;
            }
            let (irms_rec, nat_rec) =
                backbone_pairs(native_receptor, &rb, receptor, &ru, &rc.pairs, |i| {
                    rec_iface[i]
                });
            let (irms_lig, nat_lig) =
                backbone_pairs(native_ligand, &lb, ligand, &lu, &lc.pairs, |i| lig_iface[i]);
            let mut irms_native = nat_rec;
            irms_native.extend(nat_lig);

            let (rec_u, rec_b) =
                backbone_pairs(native_receptor, &rb, receptor, &ru, &rc.pairs, |_| true);
            let (lig_u, lig_b) =
                backbone_pairs(native_ligand, &lb, ligand, &lu, &lc.pairs, |_| true);
            if rec_u.len() < 3 || lig_u.len() < 3 {
                return Err(Error::Invalid("too few matched backbone atoms".into()));
            }
            let rec_fit = kabsch(&rec_u, &rec_b).0;
            let lig_fit = kabsch(&lig_u, &lig_b).0;
            let (lrms_fit, lrms_mobile, lrms_target) = match lrmsd_reference {
                Side::Receptor => (rec_fit, lig_u, lig_b),
                Side::Ligand => (lig_fit, rec_u, rec_b),
            };

            if info.is_none() {
                // Reference placement of the unbound ligand: superpose each partner's
                // interface backbone on the native one and chain the two fits. Whole-chain
                // fits would be thrown off by motions away from the interface, such as the
                // elbow of a Fab, and misplace the ligand on the binding site.
                let local = |u: &[V3], b: &[V3], global: Rigid| {
                    if u.len() >= 3 { kabsch(u, b).0 } else { global }
                };
                let rec_local = local(&irms_rec, &irms_native[..irms_rec.len()], rec_fit);
                let lig_local = local(&irms_lig, &irms_native[irms_rec.len()..], lig_fit);
                optimal_pose = rec_local.inverse().compose(&lig_local);
                info = Some(NativeInfo {
                    receptor_chains: rc.chains.clone(),
                    ligand_chains: lc.chains.clone(),
                    receptor_residues: rb.len(),
                    ligand_residues: lb.len(),
                    receptor_matched: rc.pairs.len(),
                    ligand_matched: lc.pairs.len(),
                    native_contacts: native.len(),
                    matched_contacts: native_keys.len(),
                    receptor_interface: rec_iface.iter().filter(|&&x| x).count(),
                    ligand_interface: lig_iface.iter().filter(|&&x| x).count(),
                    lrmsd_reference,
                    mappings: combos.len(),
                });
            }
            let mut rec_matched = vec![false; ru.len()];
            let mut lig_matched = vec![false; lu.len()];
            for u in rec_b2u.iter().flatten() {
                rec_matched[*u] = true;
            }
            for u in lig_b2u.iter().flatten() {
                lig_matched[*u] = true;
            }
            mappings.push(Mapping {
                rec_chains: rc.chains.clone(),
                lig_chains: lc.chains.clone(),
                rec_matched,
                lig_matched,
                native_keys,
                irms_rec,
                irms_lig,
                irms_native,
                lrms_fit,
                lrms_mobile,
                lrms_target,
            });
        }

        // Only atoms of residues matched in some mapping can form counted contacts.
        let any =
            |m: &[Mapping], f: fn(&Mapping) -> &Vec<bool>, i: usize| m.iter().any(|x| f(x)[i]);
        let (mut rec_pts, mut rec_atom_res) = (Vec::new(), Vec::new());
        let (mut rec_centers, mut max_rec_radius) = (Vec::new(), 0.0f64);
        for (i, r) in ru
            .iter()
            .enumerate()
            .filter(|(i, _)| any(&mappings, |m| &m.rec_matched, *i))
        {
            let (c, radius) = sphere(receptor, r);
            rec_centers.push(c);
            max_rec_radius = max_rec_radius.max(radius);
            for &k in &r.atoms {
                rec_pts.push(receptor.atoms[k].pos);
                rec_atom_res.push(i as u32);
            }
        }
        let (mut lig_atoms, mut lig_res_index, mut lig_res) = (Vec::new(), Vec::new(), Vec::new());
        for (i, r) in lu
            .iter()
            .enumerate()
            .filter(|(i, _)| any(&mappings, |m| &m.lig_matched, *i))
        {
            let (center, radius) = sphere(ligand, r);
            let start = lig_atoms.len();
            for &k in &r.atoms {
                lig_atoms.push(ligand.atoms[k].pos);
            }
            lig_res_index.push(i as u32);
            lig_res.push(LigandResidue {
                atoms: start..lig_atoms.len(),
                center,
                // Any receptor residue in contact has its centre within this distance.
                reach: radius + max_rec_radius + CONTACT_CUTOFF,
            });
        }
        let max_reach = lig_res
            .iter()
            .map(|r| r.reach)
            .fold(CONTACT_CUTOFF, f64::max);
        Ok(Native {
            nat_total: native.len(),
            lrmsd_reference,
            mappings,
            rec_grid: Grid::new(rec_pts, CONTACT_CUTOFF),
            rec_atom_res,
            rec_res_grid: Grid::new(rec_centers, max_reach),
            lig_atoms,
            lig_res_index,
            lig_res,
            optimal_pose,
            info: info.expect("at least one mapping"),
        })
    }

    pub fn info(&self) -> &NativeInfo {
        &self.info
    }

    /// Reference rigid placement of the unbound ligand on the unbound receptor: the native
    /// binding mode, obtained by superposing the interface backbone of each partner on the
    /// native complex. Its metrics approximate the ceiling for rigid-body docking of these
    /// unbound structures (the pose minimising each metric differs slightly).
    pub fn optimal_pose(&self) -> Rigid {
        self.optimal_pose
    }

    /// Chain mappings (receptor, ligand; bound chain → unbound chain) scored per pose, in
    /// the order of [`Native::evaluate_each`]. The first one is described by [`NativeInfo`].
    pub fn chain_mappings(&self) -> impl Iterator<Item = (&[ChainPair], &[ChainPair])> {
        self.mappings
            .iter()
            .map(|m| (m.rec_chains.as_slice(), m.lig_chains.as_slice()))
    }

    /// Quality of the ligand placed by `pose` (applied to the unbound ligand coordinates).
    /// With several chain mappings (identical chain copies, e.g. a homodimeric receptor)
    /// the best DockQ is reported: binding either copy is equally correct.
    pub fn evaluate(&self, pose: &Rigid) -> Metrics {
        self.evaluate_each(pose)
            .into_iter()
            .max_by(|a, b| a.dockq.total_cmp(&b.dockq))
            .expect("at least one mapping")
    }

    /// Quality of `pose` under each chain mapping of [`Native::chain_mappings`].
    pub fn evaluate_each(&self, pose: &Rigid) -> Vec<Metrics> {
        let mut keys = Vec::new();
        let r2 = CONTACT_CUTOFF * CONTACT_CUTOFF;
        for (res, &l) in self.lig_res.iter().zip(&self.lig_res_index) {
            // Most ligand residues are far from the receptor: one query on residue centres
            // skips their atoms.
            let c = pose.apply(res.center);
            if !self.rec_res_grid.any_within(c, res.reach * res.reach) {
                continue;
            }
            for p in &self.lig_atoms[res.atoms.clone()] {
                self.rec_grid.for_each_within(pose.apply(*p), r2, |i| {
                    keys.push(key(self.rec_atom_res[i] as usize, l as usize))
                });
            }
        }
        keys.sort_unstable();
        keys.dedup();
        self.mappings
            .iter()
            .map(|m| self.score(m, pose, &keys))
            .collect()
    }

    /// [`Native::evaluate`] for many poses, in parallel.
    pub fn evaluate_many(&self, poses: &[Rigid]) -> Vec<Metrics> {
        poses.par_iter().map(|p| self.evaluate(p)).collect()
    }

    fn score(&self, m: &Mapping, pose: &Rigid, keys: &[u64]) -> Metrics {
        let (mut model, mut shared) = (0, 0);
        for &k in keys {
            let (r, l) = ((k >> 32) as usize, (k & 0xffff_ffff) as usize);
            if m.rec_matched[r] && m.lig_matched[l] {
                model += 1;
                if m.native_keys.binary_search(&k).is_ok() {
                    shared += 1;
                }
            }
        }
        let fnat = shared as f64 / self.nat_total as f64;
        let fnonnat = if model > 0 {
            (model - shared) as f64 / model as f64
        } else {
            0.0
        };

        let mut pts = Vec::with_capacity(m.irms_native.len());
        pts.extend_from_slice(&m.irms_rec);
        pts.extend(m.irms_lig.iter().map(|p| pose.apply(*p)));
        let irmsd = kabsch(&pts, &m.irms_native).1;

        let to_native = match self.lrmsd_reference {
            // The receptor does not move: its superposition is fixed, the ligand follows it.
            Side::Receptor => m.lrms_fit.compose(pose),
            // The pose moved the ligand: undo it, then superpose the ligand.
            Side::Ligand => m.lrms_fit.compose(&pose.inverse()),
        };
        let lrmsd = rmsd(&m.lrms_mobile, &m.lrms_target, &to_native);

        Metrics {
            fnat,
            fnonnat,
            irmsd,
            lrmsd,
            dockq: dockq(fnat, irmsd, lrmsd),
            capri: Capri::classify(fnat, irmsd, lrmsd),
            nat_correct: shared,
            nat_total: self.nat_total,
            model_contacts: model,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capri_thresholds() {
        assert_eq!(Capri::classify(0.6, 0.9, 3.0), Capri::High);
        assert_eq!(Capri::classify(0.6, 1.5, 0.8), Capri::High);
        assert_eq!(Capri::classify(0.6, 1.5, 3.0), Capri::Medium);
        assert_eq!(Capri::classify(0.4, 0.5, 0.5), Capri::Medium);
        assert_eq!(Capri::classify(0.2, 3.9, 20.0), Capri::Acceptable);
        assert_eq!(Capri::classify(0.2, 4.5, 9.0), Capri::Acceptable);
        assert_eq!(Capri::classify(0.09, 0.5, 0.5), Capri::Incorrect);
        assert_eq!(Capri::classify(0.5, 5.0, 11.0), Capri::Incorrect);
        assert!(Capri::High > Capri::Medium && Capri::Acceptable > Capri::Incorrect);
    }

    #[test]
    fn dockq_formula() {
        assert!((dockq(1.0, 0.0, 0.0) - 1.0).abs() < 1e-12);
        assert!((dockq(0.0, 1.5, 8.5) - 1.0 / 3.0).abs() < 1e-12);
    }
}
