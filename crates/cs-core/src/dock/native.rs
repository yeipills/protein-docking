//! Comparison of docked poses with a known native complex (development and tests).
//!
//! Unbound and bound structures of the same protein may differ in chain names, numbering and
//! missing residues, so residues are paired by a global sequence alignment of each chain
//! against the best-matching chain of the other structure. The ligand RMSD reported here is
//! the CAPRI-style L-RMSD on Cα atoms: receptors superposed, ligand not refitted.

use crate::error::{Error, Result};
use crate::geom::transform::{Rigid, kabsch};
use crate::geom::vec3::*;
use crate::io::pdb::Structure;

/// One-letter code of a residue name (`X` when unknown).
pub fn one_letter(res_name: &str) -> char {
    match res_name {
        "ALA" => 'A',
        "ARG" => 'R',
        "ASN" => 'N',
        "ASP" => 'D',
        "CYS" | "CYX" => 'C',
        "GLN" => 'Q',
        "GLU" => 'E',
        "GLY" => 'G',
        "HIS" | "HID" | "HIE" | "HIP" => 'H',
        "ILE" => 'I',
        "LEU" => 'L',
        "LYS" => 'K',
        "MET" | "MSE" => 'M',
        "PHE" => 'F',
        "PRO" => 'P',
        "SER" => 'S',
        "THR" => 'T',
        "TRP" => 'W',
        "TYR" => 'Y',
        "VAL" => 'V',
        _ => 'X',
    }
}

/// Cα atoms per chain, in file order, with the residue letter.
fn chains_ca(s: &Structure) -> Vec<Vec<(char, V3)>> {
    let mut chains: Vec<(char, Vec<(char, V3)>)> = Vec::new();
    for a in s.atoms.iter().filter(|a| a.name == "CA" && !a.hetatm) {
        let letter = one_letter(&a.res_name);
        match chains.iter_mut().find(|(c, _)| *c == a.chain) {
            Some((_, v)) => v.push((letter, a.pos)),
            None => chains.push((a.chain, vec![(letter, a.pos)])),
        }
    }
    chains.into_iter().map(|(_, v)| v).collect()
}

/// Needleman–Wunsch on residue letters (match +2, mismatch -1, gap -2). Returns the score
/// and the aligned index pairs with identical letters.
fn align(a: &[char], b: &[char]) -> (i32, Vec<(usize, usize)>) {
    const GAP: i32 = -2;
    let (n, m) = (a.len(), b.len());
    let mut s = vec![0i32; (n + 1) * (m + 1)];
    let at = |i: usize, j: usize| i * (m + 1) + j;
    for i in 0..=n {
        s[at(i, 0)] = GAP * i as i32;
    }
    for j in 0..=m {
        s[at(0, j)] = GAP * j as i32;
    }
    let sub = |i: usize, j: usize| if a[i] == b[j] && a[i] != 'X' { 2 } else { -1 };
    for i in 1..=n {
        for j in 1..=m {
            s[at(i, j)] = (s[at(i - 1, j - 1)] + sub(i - 1, j - 1))
                .max(s[at(i - 1, j)] + GAP)
                .max(s[at(i, j - 1)] + GAP);
        }
    }
    let mut pairs = Vec::new();
    let (mut i, mut j) = (n, m);
    while i > 0 && j > 0 {
        if s[at(i, j)] == s[at(i - 1, j - 1)] + sub(i - 1, j - 1) {
            if a[i - 1] == b[j - 1] {
                pairs.push((i - 1, j - 1));
            }
            i -= 1;
            j -= 1;
        } else if s[at(i, j)] == s[at(i - 1, j)] + GAP {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    pairs.reverse();
    (s[at(n, m)], pairs)
}

/// Paired Cα coordinates `(a, b)` of two structures of the same protein(s). Each chain of `a`
/// is aligned to the unused chain of `b` with the best alignment score; pairs need at least
/// half of the shorter chain to be identical to count.
pub fn matched_ca(a: &Structure, b: &Structure) -> Vec<(V3, V3)> {
    let (ca, cb) = (chains_ca(a), chains_ca(b));
    let mut used = vec![false; cb.len()];
    let mut out = Vec::new();
    for x in &ca {
        let xl: Vec<char> = x.iter().map(|r| r.0).collect();
        let mut best: Option<(i32, usize, Vec<(usize, usize)>)> = None;
        for (j, y) in cb.iter().enumerate() {
            if used[j] {
                continue;
            }
            let yl: Vec<char> = y.iter().map(|r| r.0).collect();
            let (score, pairs) = align(&xl, &yl);
            if 2 * pairs.len() < xl.len().min(yl.len()) {
                continue;
            }
            if best.as_ref().is_none_or(|b| score > b.0) {
                best = Some((score, j, pairs));
            }
        }
        if let Some((_, j, pairs)) = best {
            used[j] = true;
            out.extend(pairs.into_iter().map(|(p, q)| (x[p].1, cb[j][q].1)));
        }
    }
    out
}

/// Native ligand placement expressed in the docking input frames.
pub struct NativeReference {
    /// Ligand input Cα atoms that have a native counterpart.
    pub ligand_ca: Vec<V3>,
    /// Their native positions, in the receptor input frame.
    pub native_ca: Vec<V3>,
}

impl NativeReference {
    /// `receptor` / `ligand`: docking inputs (ligand in its randomized input frame);
    /// `receptor_native` / `ligand_native`: the bound complex.
    pub fn new(
        receptor: &Structure,
        ligand: &Structure,
        receptor_native: &Structure,
        ligand_native: &Structure,
    ) -> Result<NativeReference> {
        let rec_pairs = matched_ca(receptor_native, receptor);
        let lig_pairs = matched_ca(ligand, ligand_native);
        if rec_pairs.len() < 3 || lig_pairs.len() < 3 {
            return Err(Error::Invalid(
                "could not pair the docking inputs with the native structures".into(),
            ));
        }
        let (from, to): (Vec<V3>, Vec<V3>) = rec_pairs.into_iter().unzip();
        let (native_to_input, _) = kabsch(&from, &to);
        Ok(NativeReference {
            ligand_ca: lig_pairs.iter().map(|p| p.0).collect(),
            native_ca: lig_pairs
                .iter()
                .map(|p| native_to_input.apply(p.1))
                .collect(),
        })
    }

    /// Ligand Cα RMSD of the pose `t` (applied to the ligand input) from the native.
    pub fn ligand_rmsd(&self, t: &Rigid) -> f64 {
        let s: f64 = self
            .ligand_ca
            .iter()
            .zip(&self.native_ca)
            .map(|(a, b)| dist2(t.apply(*a), *b))
            .sum();
        (s / self.ligand_ca.len() as f64).sqrt()
    }

    /// Lowest RMSD any rigid placement of the ligand input can reach (the unbound/bound
    /// conformational difference): the floor for [`Self::ligand_rmsd`].
    pub fn best_possible_rmsd(&self) -> f64 {
        kabsch(&self.ligand_ca, &self.native_ca).1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alignment_skips_gaps() {
        let a: Vec<char> = "ACDEFGHIK".chars().collect();
        let b: Vec<char> = "ACDFGHIKLM".chars().collect(); // E missing, LM extra
        let (_, pairs) = align(&a, &b);
        assert_eq!(
            pairs,
            vec![
                (0, 0),
                (1, 1),
                (2, 2),
                (4, 3),
                (5, 4),
                (6, 5),
                (7, 6),
                (8, 7)
            ]
        );
    }
}
