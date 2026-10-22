//! Minimal PDB reader/writer (fixed-column ATOM/HETATM records).

use std::fmt::Write as _;
use std::path::Path;

use crate::error::{Error, Result};
use crate::geom::vec3::V3;

#[derive(Debug, Clone, PartialEq)]
pub struct Atom {
    pub serial: i32,
    /// Atom name, trimmed (e.g. "CA").
    pub name: String,
    pub alt_loc: char,
    pub res_name: String,
    pub chain: char,
    pub res_seq: i32,
    pub i_code: char,
    pub pos: V3,
    pub occupancy: f64,
    pub b_factor: f64,
    /// Element symbol, upper case (inferred from the name when the column is empty).
    pub element: String,
    pub hetatm: bool,
}

/// Identifies a residue: chain, number, insertion code.
pub type ResidueId = (char, i32, char);

impl Atom {
    pub fn residue_id(&self) -> ResidueId {
        (self.chain, self.res_seq, self.i_code)
    }

    pub fn is_hydrogen(&self) -> bool {
        self.element == "H" || self.element == "D"
    }

    pub fn is_backbone(&self) -> bool {
        matches!(self.name.as_str(), "N" | "CA" | "C" | "O")
    }

    /// Van der Waals radius in Å (Bondi 1964; 1.8 Å for unknown elements).
    pub fn vdw_radius(&self) -> f64 {
        match self.element.as_str() {
            "H" | "D" => 1.20,
            "C" => 1.70,
            "N" => 1.55,
            "O" => 1.52,
            "S" => 1.80,
            "P" => 1.80,
            "SE" => 1.90,
            "F" => 1.47,
            "CL" => 1.75,
            "BR" => 1.85,
            "I" => 1.98,
            "FE" | "ZN" | "MG" | "CA" | "MN" | "CU" | "NI" | "CO" => 1.40,
            "NA" => 2.27,
            "K" => 2.75,
            _ => 1.80,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Structure {
    pub atoms: Vec<Atom>,
}

impl Structure {
    pub fn read(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path).map_err(|e| Error::io(path, e))?;
        Self::parse(&text)
    }

    /// Parses ATOM/HETATM records of the first model. Waters are skipped; for alternate
    /// locations only the first one (blank or 'A') is kept.
    pub fn parse(text: &str) -> Result<Self> {
        let mut atoms = Vec::new();
        for (i, line) in text.lines().enumerate() {
            if line.starts_with("ENDMDL") {
                break;
            }
            let hetatm = line.starts_with("HETATM");
            if !(line.starts_with("ATOM  ") || hetatm) {
                continue;
            }
            let col = |a: usize, b: usize| line.get(a..b.min(line.len())).unwrap_or("").trim();
            let num = |a: usize, b: usize, what: &str| -> Result<f64> {
                col(a, b)
                    .parse::<f64>()
                    .map_err(|_| Error::parse("pdb", i + 1, format!("invalid {what}")))
            };
            let ch = |a: usize| line.as_bytes().get(a).map(|&b| b as char).unwrap_or(' ');
            let res_name = col(17, 20).to_string();
            if res_name == "HOH" || res_name == "WAT" {
                continue;
            }
            let alt_loc = ch(16);
            if alt_loc != ' ' && alt_loc != 'A' {
                continue;
            }
            let name = col(12, 16).to_string();
            let mut element = col(76, 78).to_ascii_uppercase();
            if element.is_empty() {
                element = infer_element(line.get(12..16).unwrap_or(""));
            }
            atoms.push(Atom {
                serial: col(6, 11).parse().unwrap_or(0),
                name,
                alt_loc,
                res_name,
                chain: ch(21),
                res_seq: col(22, 26)
                    .parse()
                    .map_err(|_| Error::parse("pdb", i + 1, "invalid residue number"))?,
                i_code: ch(26),
                pos: [num(30, 38, "x")?, num(38, 46, "y")?, num(46, 54, "z")?],
                occupancy: col(54, 60).parse().unwrap_or(1.0),
                b_factor: col(60, 66).parse().unwrap_or(0.0),
                element,
                hetatm,
            });
        }
        if atoms.is_empty() {
            return Err(Error::Invalid(
                "the PDB file has no ATOM/HETATM records".into(),
            ));
        }
        Ok(Structure { atoms })
    }

    /// Heavy (non-hydrogen) atoms.
    pub fn heavy_atoms(&self) -> impl Iterator<Item = &Atom> {
        self.atoms.iter().filter(|a| !a.is_hydrogen())
    }

    pub fn chains(&self) -> Vec<char> {
        let mut c: Vec<char> = Vec::new();
        for a in &self.atoms {
            if !c.contains(&a.chain) {
                c.push(a.chain);
            }
        }
        c
    }

    /// Writes ATOM/HETATM records (and END).
    pub fn to_pdb(&self) -> String {
        let mut s = String::with_capacity(self.atoms.len() * 81);
        for a in &self.atoms {
            write_atom(&mut s, a);
        }
        s.push_str("END\n");
        s
    }
}

pub fn write_atom(s: &mut String, a: &Atom) {
    // Names shorter than 4 characters start in column 14 unless the element has 2 letters.
    let name = if a.name.len() < 4 && a.element.len() < 2 {
        format!(" {:<3}", a.name)
    } else {
        format!("{:<4}", a.name)
    };
    writeln!(
        s,
        "{:<6}{:>5} {}{}{:>3} {}{:>4}{}   {:>8.3}{:>8.3}{:>8.3}{:>6.2}{:>6.2}          {:>2}",
        if a.hetatm { "HETATM" } else { "ATOM" },
        a.serial % 100_000,
        name,
        a.alt_loc,
        a.res_name,
        a.chain,
        a.res_seq,
        a.i_code,
        a.pos[0],
        a.pos[1],
        a.pos[2],
        a.occupancy,
        a.b_factor,
        a.element
    )
    .unwrap();
}

fn infer_element(name_field: &str) -> String {
    // Columns 13-14 hold the element right-justified; a digit or space in 13 means 1 letter.
    let b = name_field.as_bytes();
    let first = b.first().copied().unwrap_or(b' ');
    let letters: String = name_field
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect();
    if first == b' ' || first.is_ascii_digit() {
        letters
            .chars()
            .next()
            .map(|c| c.to_ascii_uppercase().to_string())
            .unwrap_or_default()
    } else {
        letters
            .chars()
            .take(2)
            .collect::<String>()
            .to_ascii_uppercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PDB: &str = "\
ATOM      1  N   ASP L   1      -8.153   9.161  46.908  1.00 26.71      L    N
ATOM      2  CA  ASP L   1      -8.353   8.730  48.318  1.00 28.16           C
ATOM      3 CA  ACA  A 100A      1.000   2.000   3.000  0.50 10.00
ATOM      4 CA  BCA  A 100A      1.100   2.000   3.000  0.50 10.00
HETATM    5  O   HOH W   1       0.000   0.000   0.000  1.00  0.00           O
";

    #[test]
    fn parses_records() {
        let s = Structure::parse(PDB).unwrap();
        assert_eq!(s.atoms.len(), 3);
        assert_eq!(s.atoms[0].name, "N");
        assert_eq!(s.atoms[1].element, "C");
        assert_eq!(s.atoms[0].residue_id(), ('L', 1, ' '));
        assert_eq!(
            s.atoms[2].element, "CA",
            "two-letter element from the name columns"
        );
        assert_eq!(s.atoms[2].i_code, 'A');
        assert_eq!(s.chains(), vec!['L', 'A']);
    }

    #[test]
    fn round_trips() {
        let s = Structure::parse(PDB).unwrap();
        let back = Structure::parse(&s.to_pdb()).unwrap();
        assert_eq!(s.atoms.len(), back.atoms.len());
        for (a, b) in s.atoms.iter().zip(&back.atoms) {
            assert_eq!(a.name, b.name);
            assert_eq!(a.residue_id(), b.residue_id());
            assert_eq!(a.element, b.element);
            for k in 0..3 {
                assert!((a.pos[k] - b.pos[k]).abs() < 1e-9);
            }
        }
    }
}
