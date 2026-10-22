//! Reader for MSMS `.vert` / `.face` solvent-excluded surface files.
//!
//! Both files start with three header lines. Vertex rows are
//! `x y z nx ny nz face sphere type`; face rows are `v1 v2 v3 type sphere`, with 1-based
//! vertex indices.

use std::path::Path;

use crate::error::{Error, Result};

const HEADER_LINES: usize = 3;

#[derive(Debug, Clone)]
pub struct MsmsSurface {
    pub vertices: Vec<[f64; 3]>,
    pub normals: Vec<[f64; 3]>,
    /// Vertex indices exactly as written in the file (1-based).
    pub faces_raw: Vec<[u32; 3]>,
    /// MSMS face type: 1 = contact (atom sphere), 2 = toroidal, 3 = reentrant (probe).
    pub face_types: Vec<i32>,
}

impl MsmsSurface {
    pub fn read(vert_path: &Path, face_path: &Path) -> Result<Self> {
        let vert = std::fs::read_to_string(vert_path).map_err(|e| Error::io(vert_path, e))?;
        let face = std::fs::read_to_string(face_path).map_err(|e| Error::io(face_path, e))?;
        Self::parse(&vert, &face)
    }

    pub fn parse(vert_text: &str, face_text: &str) -> Result<Self> {
        let mut vertices = Vec::new();
        let mut normals = Vec::new();
        for (lineno, line) in data_lines(vert_text) {
            let mut it = line.split_ascii_whitespace();
            let mut next = |what: &str| -> Result<f64> {
                let tok = it
                    .next()
                    .ok_or_else(|| Error::parse("vert", lineno, format!("missing {what}")))?;
                tok.parse::<f64>()
                    .map_err(|_| Error::parse("vert", lineno, format!("invalid {what}: {tok:?}")))
            };
            vertices.push([next("x")?, next("y")?, next("z")?]);
            normals.push([next("nx")?, next("ny")?, next("nz")?]);
        }

        let mut faces_raw = Vec::new();
        let mut face_types = Vec::new();
        for (lineno, line) in data_lines(face_text) {
            let toks: Vec<&str> = line.split_ascii_whitespace().take(4).collect();
            if toks.len() < 4 {
                return Err(Error::parse("face", lineno, "expected `v1 v2 v3 type`"));
            }
            let mut idx = [0u32; 3];
            for k in 0..3 {
                idx[k] = toks[k].parse().map_err(|_| {
                    Error::parse(
                        "face",
                        lineno,
                        format!("invalid vertex index {:?}", toks[k]),
                    )
                })?;
            }
            let ty: f64 = toks[3]
                .parse()
                .map_err(|_| Error::parse("face", lineno, format!("invalid type {:?}", toks[3])))?;
            faces_raw.push(idx);
            face_types.push(ty as i32);
        }

        if vertices.is_empty() {
            return Err(Error::Invalid("the .vert file has no vertices".into()));
        }
        if faces_raw.is_empty() {
            return Err(Error::Invalid("the .face file has no faces".into()));
        }
        Ok(Self {
            vertices,
            normals,
            faces_raw,
            face_types,
        })
    }

    /// Faces converted to 0-based indices, validated against the vertex count.
    pub fn faces(&self) -> Result<Vec<[u32; 3]>> {
        let n = self.vertices.len() as u32;
        self.faces_raw
            .iter()
            .enumerate()
            .map(|(i, f)| {
                if f.iter().any(|&v| v == 0 || v > n) {
                    Err(Error::Invalid(format!(
                        "face {} references vertex outside 1..={n}: {f:?}",
                        i + 1
                    )))
                } else {
                    Ok([f[0] - 1, f[1] - 1, f[2] - 1])
                }
            })
            .collect()
    }
}

/// Non-empty lines after the header, with 1-based line numbers for error messages.
fn data_lines(text: &str) -> impl Iterator<Item = (usize, &str)> {
    text.lines()
        .enumerate()
        .skip(HEADER_LINES)
        .filter(|(_, l)| !l.trim().is_empty())
        .map(|(i, l)| (i + 1, l))
}

#[cfg(test)]
mod tests {
    use super::*;

    const VERT: &str = "# header\n#vertex\n 3 1 1.0 1.5\n  0.0 0.0 0.0 0 0 1 0 1 1\n  1.0 0.0 0.0 0 0 1 0 1 1\n  0.0 1.0 0.0 0 0 1 0 1 1\n";
    const FACE: &str = "# header\n#faces\n 1 1 1.0 1.5\n 1 2 3 3 1\n";

    #[test]
    fn parses_and_converts_indices() {
        let s = MsmsSurface::parse(VERT, FACE).unwrap();
        assert_eq!(s.vertices.len(), 3);
        assert_eq!(s.faces_raw, vec![[1, 2, 3]]);
        assert_eq!(s.face_types, vec![3]);
        assert_eq!(s.faces().unwrap(), vec![[0, 1, 2]]);
    }

    #[test]
    fn rejects_out_of_range_faces() {
        let s = MsmsSurface::parse(VERT, "a\nb\nc\n 1 2 4 3 1\n").unwrap();
        assert!(s.faces().is_err());
    }
}
