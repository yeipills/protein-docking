//! Binary and ASCII STL reader.
//!
//! Coordinates keep the precision of the file: binary STL stores `f32`, which is widened to
//! `f64` exactly; ASCII values are parsed directly as `f64`. This matches how trimesh loads
//! them, which the legacy mode depends on.

use std::path::Path;

use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct Stl {
    pub triangles: Vec<[[f64; 3]; 3]>,
    /// Normals as stored in the file (may be zero or inconsistent).
    pub normals: Vec<[f64; 3]>,
}

impl Stl {
    pub fn read(path: &Path) -> Result<Self> {
        let data = std::fs::read(path).map_err(|e| Error::io(path, e))?;
        Self::parse(&data)
    }

    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() >= 84 {
            let n = u32::from_le_bytes(data[80..84].try_into().unwrap()) as usize;
            if data.len() == 84 + 50 * n {
                return Ok(parse_binary(&data[84..], n));
            }
        }
        if data.starts_with(b"solid") {
            let text = std::str::from_utf8(data)
                .map_err(|_| Error::Invalid("ASCII STL is not valid UTF-8".into()))?;
            return parse_ascii(text);
        }
        Err(Error::Invalid(
            "unrecognised STL: size does not match the binary header and it is not ASCII".into(),
        ))
    }
}

fn parse_binary(body: &[u8], n: usize) -> Stl {
    let mut triangles = Vec::with_capacity(n);
    let mut normals = Vec::with_capacity(n);
    let f = |b: &[u8], i: usize| f32::from_le_bytes(b[i * 4..i * 4 + 4].try_into().unwrap()) as f64;
    for rec in body.chunks_exact(50) {
        normals.push([f(rec, 0), f(rec, 1), f(rec, 2)]);
        triangles.push([
            [f(rec, 3), f(rec, 4), f(rec, 5)],
            [f(rec, 6), f(rec, 7), f(rec, 8)],
            [f(rec, 9), f(rec, 10), f(rec, 11)],
        ]);
    }
    Stl { triangles, normals }
}

fn parse_ascii(text: &str) -> Result<Stl> {
    let mut triangles = Vec::new();
    let mut normals = Vec::new();
    let mut verts: Vec<[f64; 3]> = Vec::with_capacity(3);
    let mut normal = [0.0; 3];
    for (i, line) in text.lines().enumerate() {
        let mut it = line.split_ascii_whitespace();
        let kw = it.next();
        let xyz = |it: &mut std::str::SplitAsciiWhitespace| -> Result<[f64; 3]> {
            let mut v = [0.0; 3];
            for c in &mut v {
                *c = it
                    .next()
                    .and_then(|t| t.parse().ok())
                    .ok_or_else(|| Error::parse("stl", i + 1, "expected three numbers"))?;
            }
            Ok(v)
        };
        match kw {
            Some("facet") => {
                it.next(); // "normal"
                normal = xyz(&mut it)?;
                verts.clear();
            }
            Some("vertex") => verts.push(xyz(&mut it)?),
            Some("endfacet") => {
                if verts.len() != 3 {
                    return Err(Error::parse("stl", i + 1, "facet without three vertices"));
                }
                triangles.push([verts[0], verts[1], verts[2]]);
                normals.push(normal);
            }
            _ => {}
        }
    }
    if triangles.is_empty() {
        return Err(Error::Invalid("STL has no facets".into()));
    }
    Ok(Stl { triangles, normals })
}
