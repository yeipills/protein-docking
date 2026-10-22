//! Protein residues of a [`Structure`] and their one-letter codes.
//!
//! Evaluation works on residues rather than atoms: contacts, interfaces and the
//! unbound↔bound correspondence are all residue-level. Only heavy atoms are kept, and
//! HETATM records are ignored except for modified amino acids (e.g. selenomethionine),
//! which crystallographers often write as HETATM inside a protein chain.

use crate::io::pdb::Structure;

/// One residue of a protein chain.
#[derive(Debug, Clone, PartialEq)]
pub struct Residue {
    pub chain: char,
    pub res_seq: i32,
    pub i_code: char,
    /// Three-letter residue name as written in the file.
    pub name: String,
    /// One-letter code used for sequence alignment (`X` when unknown).
    pub code: char,
    /// Indices of the residue's heavy atoms in `Structure::atoms`.
    pub atoms: Vec<usize>,
}

impl Residue {
    /// Index (in `Structure::atoms`) of the backbone atom `name` (`N`, `CA`, `C` or `O`).
    pub fn atom(&self, s: &Structure, name: &str) -> Option<usize> {
        self.atoms
            .iter()
            .copied()
            .find(|&i| s.atoms[i].name == name)
    }
}

/// Backbone atom names used for iRMSD and LRMSD (CAPRI convention).
pub const BACKBONE: [&str; 4] = ["N", "CA", "C", "O"];

/// One-letter code of a residue name. Modified residues map to their parent amino acid so
/// that they align with the unmodified residue of the other structure.
pub fn one_letter(name: &str) -> char {
    match name {
        "ALA" => 'A',
        "ARG" => 'R',
        "ASN" => 'N',
        "ASP" => 'D',
        "CYS" => 'C',
        "GLN" => 'Q',
        "GLU" => 'E',
        "GLY" => 'G',
        "HIS" => 'H',
        "ILE" => 'I',
        "LEU" => 'L',
        "LYS" => 'K',
        "MET" => 'M',
        "PHE" => 'F',
        "PRO" => 'P',
        "SER" => 'S',
        "THR" => 'T',
        "TRP" => 'W',
        "TYR" => 'Y',
        "VAL" => 'V',
        "SEC" => 'U',
        "PYL" => 'O',
        "ASX" => 'B',
        "GLX" => 'Z',
        _ => modified_parent(name).unwrap_or('X'),
    }
}

/// Parent amino acid of common modified residues found in the docking benchmarks.
fn modified_parent(name: &str) -> Option<char> {
    Some(match name {
        "MSE" => 'M',
        "CME" | "CSO" | "CSD" | "CAS" | "CSS" => 'C',
        "SEP" => 'S',
        "TPO" => 'T',
        "PTR" | "TYS" => 'Y',
        "HYP" => 'P',
        "MLY" | "M3L" | "KCX" | "LLP" => 'K',
        "PCA" => 'E',
        "HIC" => 'H',
        _ => return None,
    })
}

/// Protein residues in file order. ATOM records always count; HETATM records only for the
/// modified amino acids known to [`one_letter`]. Hydrogens are dropped, and residues are
/// split whenever chain, number, insertion code or name changes (as Biopython does).
pub fn protein_residues(s: &Structure) -> Vec<Residue> {
    let mut out: Vec<Residue> = Vec::new();
    for (i, a) in s.atoms.iter().enumerate() {
        if a.is_hydrogen() || (a.hetatm && modified_parent(&a.res_name).is_none()) {
            continue;
        }
        let same = out.last().is_some_and(|r| {
            r.chain == a.chain
                && r.res_seq == a.res_seq
                && r.i_code == a.i_code
                && r.name == a.res_name
        });
        if same {
            out.last_mut().unwrap().atoms.push(i);
        } else {
            out.push(Residue {
                chain: a.chain,
                res_seq: a.res_seq,
                i_code: a.i_code,
                name: a.res_name.clone(),
                code: one_letter(&a.res_name),
                atoms: vec![i],
            });
        }
    }
    out
}

/// Chains in order of first appearance, each with the indices of its residues.
pub fn chains(residues: &[Residue]) -> Vec<(char, Vec<usize>)> {
    let mut out: Vec<(char, Vec<usize>)> = Vec::new();
    for (i, r) in residues.iter().enumerate() {
        match out.iter_mut().find(|(c, _)| *c == r.chain) {
            Some((_, v)) => v.push(i),
            None => out.push((r.chain, vec![i])),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const PDB: &str = "\
ATOM      1  N   GLY A   1       0.000   0.000   0.000  1.00  0.00           N
ATOM      2  CA  GLY A   1       1.000   0.000   0.000  1.00  0.00           C
ATOM      3  H   GLY A   1       1.000   1.000   0.000  1.00  0.00           H
HETATM    4  N   MSE A   2       2.000   0.000   0.000  1.00  0.00           N
HETATM    5  CA  MSE A   2       3.000   0.000   0.000  1.00  0.00           C
HETATM    6  C1  NAG A 301       9.000   0.000   0.000  1.00  0.00           C
ATOM      7  CA  ALA B   1       5.000   0.000   0.000  1.00  0.00           C
ATOM      8  CA  ALA B   1A      6.000   0.000   0.000  1.00  0.00           C
";

    #[test]
    fn residues_skip_hydrogens_and_ligands() {
        let s = Structure::parse(PDB).unwrap();
        let r = protein_residues(&s);
        let codes: String = r.iter().map(|r| r.code).collect();
        assert_eq!(codes, "GMAA");
        assert_eq!(r[0].atoms, vec![0, 1]);
        assert_eq!(r[1].atom(&s, "CA"), Some(4));
        assert_eq!(r[3].i_code, 'A');
        let c = chains(&r);
        assert_eq!(c, vec![('A', vec![0, 1]), ('B', vec![2, 3])]);
    }
}
