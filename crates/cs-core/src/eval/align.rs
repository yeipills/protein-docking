//! Residue correspondence between two structures of the same protein(s), e.g. the unbound
//! and bound forms of a docking partner.
//!
//! Chain IDs, numbering and missing residues usually differ between the two files, so
//! residues are matched by global sequence alignment of each chain pair, and chains are
//! paired by the assignment that aligns the most identical residues.

use super::residues::{Residue, chains};
use crate::geom::transform::kabsch;
use crate::geom::vec3::V3;
use crate::io::pdb::Structure;

/// Scores of the global alignment, in half points so that everything stays integer.
/// They are those of DockQ v2 (Biopython `PairwiseAligner`: match 5, mismatch 0, gap open
/// −4, gap extend −0.5, end gaps not free), so both tools map residues the same way.
const MATCH: i32 = 10;
const MISMATCH: i32 = 0;
const GAP_OPEN: i32 = -8;
const GAP_EXTEND: i32 = -1;

/// Result of a global alignment of `a` against `b`.
#[derive(Debug, Clone, PartialEq)]
pub struct Alignment {
    /// Alignment score (in the units of Biopython's scores, i.e. half of the internal ones).
    pub score: f64,
    /// Aligned columns `(i, j)` (matches and mismatches, no gaps), increasing in both.
    pub columns: Vec<(usize, usize)>,
}

impl Alignment {
    /// Columns whose residues are identical.
    pub fn identical<'a>(
        &'a self,
        a: &'a [char],
        b: &'a [char],
    ) -> impl Iterator<Item = (usize, usize)> + 'a {
        self.columns
            .iter()
            .copied()
            .filter(move |&(i, j)| a[i] == b[j])
    }
}

/// Needleman–Wunsch global alignment with affine gaps (Gotoh). A gap of length `L` costs
/// `open + (L − 1) · extend`, including gaps at the ends: missing termini are penalised
/// like any other gap, as in DockQ.
pub fn align(a: &[char], b: &[char]) -> Alignment {
    const NEG: i32 = i32::MIN / 4;
    let (n, m) = (a.len(), b.len());
    let w = m + 1;
    // Three states: M (a[i-1] aligned with b[j-1]), X (a[i-1] against a gap), Y (b[j-1]
    // against a gap). `tb_*` record which state each cell came from.
    let mut sm = vec![NEG; (n + 1) * w];
    let mut sx = vec![NEG; (n + 1) * w];
    let mut sy = vec![NEG; (n + 1) * w];
    let mut tb = vec![[0u8; 3]; (n + 1) * w];
    sm[0] = 0;
    for i in 1..=n {
        sx[i * w] = GAP_OPEN + (i as i32 - 1) * GAP_EXTEND;
        tb[i * w][1] = if i == 1 { 0 } else { 1 };
    }
    for j in 1..=m {
        sy[j] = GAP_OPEN + (j as i32 - 1) * GAP_EXTEND;
        tb[j][2] = if j == 1 { 0 } else { 2 };
    }
    let best3 = |m: i32, x: i32, y: i32| -> (i32, u8) {
        if m >= x && m >= y {
            (m, 0)
        } else if x >= y {
            (x, 1)
        } else {
            (y, 2)
        }
    };
    for i in 1..=n {
        for j in 1..=m {
            let k = i * w + j;
            let d = (i - 1) * w + j - 1;
            let s = if a[i - 1] == b[j - 1] {
                MATCH
            } else {
                MISMATCH
            };
            let (v, t) = best3(sm[d], sx[d], sy[d]);
            sm[k] = v + s;
            tb[k][0] = t;
            let u = (i - 1) * w + j;
            let (v, t) = best3(sm[u] + GAP_OPEN, sx[u] + GAP_EXTEND, sy[u] + GAP_OPEN);
            sx[k] = v;
            tb[k][1] = t;
            let l = i * w + j - 1;
            let (v, t) = best3(sm[l] + GAP_OPEN, sx[l] + GAP_OPEN, sy[l] + GAP_EXTEND);
            sy[k] = v;
            tb[k][2] = t;
        }
    }
    let end = n * w + m;
    let (score, mut state) = best3(sm[end], sx[end], sy[end]);
    let mut columns = Vec::new();
    let (mut i, mut j) = (n, m);
    while i > 0 || j > 0 {
        let k = i * w + j;
        let prev = tb[k][state as usize];
        match state {
            0 => {
                columns.push((i - 1, j - 1));
                i -= 1;
                j -= 1;
            }
            1 => i -= 1,
            _ => j -= 1,
        }
        state = prev;
    }
    columns.reverse();
    Alignment {
        score: score as f64 / 2.0,
        columns,
    }
}

/// Pairing of one chain of the first structure with one chain of the second.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ChainPair {
    pub a: char,
    pub b: char,
    /// Identical residues aligned between the two chains.
    pub identical: usize,
}

/// A residue correspondence between structures `a` and `b`.
#[derive(Debug, Clone, PartialEq)]
pub struct Correspondence {
    pub chains: Vec<ChainPair>,
    /// Matched residues `(index in a, index in b)` (identical residues only).
    pub pairs: Vec<(usize, usize)>,
}

impl Correspondence {
    pub fn identical(&self) -> usize {
        self.pairs.len()
    }
}

/// Maximum number of alternative correspondences returned by [`correspondences`].
const MAX_ALTERNATIVES: usize = 8;

/// Residue correspondences between `a` (e.g. bound) and `b` (e.g. unbound).
///
/// Every chain of `a` is aligned with every chain of `b`; only identical aligned residues
/// are matched (as in DockQ), and a chain pair counts only if at least half of the shorter
/// chain is matched, so unrelated chains are never paired. Chains are then assigned
/// one-to-one to maximise the number of matched residues; chains of `b` may stay unused
/// (e.g. extra copies of a homo-oligomer in the unbound file).
///
/// When `b` holds identical copies of a chain, several assignments are equally good; all
/// assignments within 10 % of the best that pair the same chains of `a` are returned, the
/// best first, so callers can score each and keep the best (DockQ also searches chain
/// permutations). Ties are ordered by the in-place CA RMSD followed by the superposed one,
/// which picks the right copy when the files share a frame (as in the docking benchmark).
pub fn correspondences(
    sa: &Structure,
    ra: &[Residue],
    sb: &Structure,
    rb: &[Residue],
) -> Vec<Correspondence> {
    let ca = chains(ra);
    let cb = chains(rb);
    let seq = |res: &[Residue], idx: &[usize]| -> Vec<char> {
        idx.iter().map(|&i| res[i].code).collect()
    };
    // pairs[i][j]: matched residue pairs for chain i of a with chain j of b (empty if too few).
    let mut pairs: Vec<Vec<Vec<(usize, usize)>>> = Vec::with_capacity(ca.len());
    for (_, ia) in &ca {
        let sa_seq = seq(ra, ia);
        let mut row = Vec::with_capacity(cb.len());
        for (_, ib) in &cb {
            let sb_seq = seq(rb, ib);
            let aln = align(&sa_seq, &sb_seq);
            let m: Vec<(usize, usize)> = aln
                .columns
                .iter()
                .filter(|&&(i, j)| same_residue(&ra[ia[i]], &rb[ib[j]]))
                .map(|&(i, j)| (ia[i], ib[j]))
                .collect();
            let shorter = ia.len().min(ib.len());
            row.push(if m.len() * 2 >= shorter && !m.is_empty() {
                m
            } else {
                Vec::new()
            });
        }
        pairs.push(row);
    }

    // Enumerate one-to-one assignments (chain of a -> chain of b or none) by depth-first search.
    let mut all: Vec<Vec<Option<usize>>> = Vec::new();
    let mut current = Vec::with_capacity(ca.len());
    let mut used = vec![false; cb.len()];
    enumerate(&pairs, 0, &mut current, &mut used, &mut all);
    let total = |asg: &[Option<usize>]| -> usize {
        asg.iter()
            .enumerate()
            .filter_map(|(i, j)| j.map(|j| pairs[i][j].len()))
            .sum()
    };
    let Some(best) = all.iter().max_by_key(|a| total(a)) else {
        return Vec::new();
    };
    let best_total = total(best);
    if best_total == 0 {
        return Vec::new();
    }
    let pattern: Vec<bool> = best.iter().map(Option::is_some).collect();
    let mut keep: Vec<(Vec<Option<usize>>, usize, f64, f64)> = all
        .iter()
        .filter(|a| a.iter().map(Option::is_some).eq(pattern.iter().copied()))
        .filter(|a| total(a) * 10 >= best_total * 9)
        .map(|a| {
            let corr_pairs: Vec<(usize, usize)> = a
                .iter()
                .enumerate()
                .filter_map(|(i, j)| j.map(|j| pairs[i][j].clone()))
                .flatten()
                .collect();
            let (direct, fitted) = ca_rmsd(sa, ra, sb, rb, &corr_pairs);
            (a.clone(), total(a), direct, fitted)
        })
        .collect();
    keep.sort_by(|x, y| {
        y.1.cmp(&x.1)
            .then(x.2.total_cmp(&y.2))
            .then(x.3.total_cmp(&y.3))
    });
    keep.truncate(MAX_ALTERNATIVES);
    keep.into_iter()
        .map(|(asg, _, _, _)| {
            let mut c = Correspondence {
                chains: Vec::new(),
                pairs: Vec::new(),
            };
            for (i, j) in asg.iter().enumerate() {
                if let Some(j) = *j {
                    c.chains.push(ChainPair {
                        a: ca[i].0,
                        b: cb[j].0,
                        identical: pairs[i][j].len(),
                    });
                    c.pairs.extend_from_slice(&pairs[i][j]);
                }
            }
            c
        })
        .collect()
}

/// Identical residues: same one-letter code, and for unknown residues the same name.
fn same_residue(a: &Residue, b: &Residue) -> bool {
    a.code == b.code && (a.code != 'X' || a.name == b.name)
}

fn enumerate(
    pairs: &[Vec<Vec<(usize, usize)>>],
    i: usize,
    current: &mut Vec<Option<usize>>,
    used: &mut [bool],
    out: &mut Vec<Vec<Option<usize>>>,
) {
    // Guard against combinatorial blow-up on very large assemblies.
    if out.len() >= 100_000 {
        return;
    }
    if i == pairs.len() {
        out.push(current.clone());
        return;
    }
    for j in 0..used.len() {
        if !used[j] && !pairs[i][j].is_empty() {
            used[j] = true;
            current.push(Some(j));
            enumerate(pairs, i + 1, current, used, out);
            current.pop();
            used[j] = false;
        }
    }
    current.push(None);
    enumerate(pairs, i + 1, current, used, out);
    current.pop();
}

/// CA RMSD of matched residues without and with optimal superposition.
fn ca_rmsd(
    sa: &Structure,
    ra: &[Residue],
    sb: &Structure,
    rb: &[Residue],
    pairs: &[(usize, usize)],
) -> (f64, f64) {
    let (mut pa, mut pb): (Vec<V3>, Vec<V3>) = (Vec::new(), Vec::new());
    for &(i, j) in pairs {
        if let (Some(x), Some(y)) = (ra[i].atom(sa, "CA"), rb[j].atom(sb, "CA")) {
            pa.push(sa.atoms[x].pos);
            pb.push(sb.atoms[y].pos);
        }
    }
    if pa.is_empty() {
        return (f64::INFINITY, f64::INFINITY);
    }
    let direct = crate::geom::transform::rmsd(&pa, &pb, &crate::geom::transform::Rigid::IDENTITY);
    (direct, kabsch(&pa, &pb).1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    #[test]
    fn identical_sequences_align_diagonally() {
        let a = chars("ACDEFGHIK");
        let aln = align(&a, &a);
        assert_eq!(aln.columns, (0..9).map(|i| (i, i)).collect::<Vec<_>>());
        assert_eq!(aln.score, 45.0);
    }

    #[test]
    fn internal_gap_is_found() {
        // b lacks "GHI" (a disordered loop).
        let a = chars("MKTAYIAKQRQISFVKSHFSRQ");
        let b = chars("MKTAYIAKQRSFVKSHFSRQ");
        let aln = align(&a, &b);
        let ident: Vec<_> = aln.identical(&a, &b).collect();
        assert_eq!(ident.len(), b.len());
        // Score: 20 matches, one gap of length 2.
        assert_eq!(aln.score, 20.0 * 5.0 - 4.0 - 0.5);
    }

    #[test]
    fn terminal_gaps_and_mutation() {
        let a = chars("GSHMKVLAAGIVGLPNVGKS");
        let b = chars("KVLAWGIVGLPNVG");
        let aln = align(&a, &b);
        let ident: Vec<_> = aln.identical(&a, &b).collect();
        assert_eq!(ident.len(), b.len() - 1, "one point mutation");
        assert_eq!(ident[0], (4, 0));
    }

    #[test]
    fn empty_inputs() {
        assert!(align(&[], &chars("AC")).columns.is_empty());
        assert!(align(&chars("AC"), &[]).columns.is_empty());
    }
}
