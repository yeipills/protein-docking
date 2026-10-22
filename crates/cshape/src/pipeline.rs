//! Runs a computation and writes its output files. Shared by the CLI and the server.

use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, bail};
use cs_core::corrected::{self, Params};
use cs_core::io::msms::MsmsSurface;
use cs_core::io::stl::Stl;
use cs_core::progress::Progress;
use cs_core::{Layer, Mode, export, legacy};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spec {
    pub name: String,
    pub mode: Mode,
    /// Corrected-mode parameters (ignored in legacy mode, whose parameters are fixed).
    #[serde(default)]
    pub params: Params,
    /// Legacy mode: also write Script04's ten per-cell intermediate files (large).
    #[serde(default)]
    pub legacy_intermediate: bool,
}

pub struct Inputs {
    pub vert: PathBuf,
    pub face: PathBuf,
    pub stl: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub summary: serde_json::Value,
    pub timings_ms: Timings,
    pub files: Vec<OutputFile>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Timings {
    pub read: u64,
    pub compute: u64,
    pub write: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputFile {
    pub name: String,
    pub bytes: u64,
    pub kind: String,
}

pub const VIEWER_FILE: &str = "viewer.bin";
pub const SUMMARY_FILE: &str = "summary.json";

/// Keeps `[A-Za-z0-9_.-]`, replaces anything else, and never returns an empty or dot name.
pub fn sanitize_name(name: &str) -> String {
    let s: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "_.-".contains(c) {
                c
            } else {
                '_'
            }
        })
        .take(80)
        .collect();
    let s = s.trim_start_matches('.').to_string();
    if s.is_empty() { "protein".into() } else { s }
}

pub fn execute(
    spec: &Spec,
    inputs: &Inputs,
    out_dir: &Path,
    progress: &dyn Progress,
) -> anyhow::Result<Report> {
    let name = sanitize_name(&spec.name);
    std::fs::create_dir_all(out_dir).with_context(|| format!("creating {}", out_dir.display()))?;
    let mut timings = Timings::default();
    let mut files: Vec<OutputFile> = Vec::new();
    let mut write = |file: String, kind: &str, data: &[u8]| -> anyhow::Result<()> {
        let path = out_dir.join(&file);
        std::fs::write(&path, data).with_context(|| format!("writing {}", path.display()))?;
        files.push(OutputFile {
            name: file,
            bytes: data.len() as u64,
            kind: kind.into(),
        });
        Ok(())
    };

    let t = Instant::now();
    let surface = MsmsSurface::read(&inputs.vert, &inputs.face)?;
    let stl = match (&spec.mode, &inputs.stl) {
        (Mode::Legacy, Some(p)) => Some(Stl::read(p)?),
        (Mode::Legacy, None) => {
            bail!("legacy mode needs the surface STL (the original scripts cast rays against it)")
        }
        _ => None,
    };
    timings.read = t.elapsed().as_millis() as u64;

    let t = Instant::now();
    let (shapes, legacy_result) = match spec.mode {
        Mode::Corrected => (corrected::run(&surface, &spec.params, progress)?, None),
        Mode::Legacy => {
            let r = legacy::run(&surface, stl.as_ref().unwrap(), progress)?;
            (r.shapes.clone(), Some(r))
        }
    };
    timings.compute = t.elapsed().as_millis() as u64;

    let t = Instant::now();
    for (layer, text) in export::unity_layers(&shapes) {
        write(
            format!("{name}_cs_{}_unity.txt", layer.name()),
            "unity",
            text.as_bytes(),
        )?;
    }
    write(
        format!("{name}_resumen_cs_unity.txt"),
        "unity",
        export::unity_summary(&shapes).as_bytes(),
    )?;
    if let Some(r) = &legacy_result {
        write(
            "centroidesPCS.txt".into(),
            "legacy",
            legacy::write_centroids(r).as_bytes(),
        )?;
        write(
            format!("{name}_rayos_contexto.txt"),
            "legacy",
            legacy::write_rayos_contexto(r).as_bytes(),
        )?;
        write(
            format!("{name}_cr_totales.txt"),
            "legacy",
            legacy::write_cr_totales(r).as_bytes(),
        )?;
        if spec.legacy_intermediate {
            for layer in Layer::ALL {
                let text = legacy::write_cs_intermediate(r, layer);
                write(
                    format!("{name}_{}.txt", layer.name()),
                    "legacy",
                    text.as_bytes(),
                )?;
            }
        }
    }
    let faces = surface.faces()?;
    write(
        VIEWER_FILE.into(),
        "viewer",
        &export::viewer_blob(&surface.vertices, &faces, &shapes),
    )?;

    let mut summary = serde_json::to_value(export::summary(&shapes))?;
    if spec.mode == Mode::Corrected {
        summary["params"] = serde_json::to_value(&spec.params)?;
    }
    summary["input"] = serde_json::json!({
        "vertices": surface.vertices.len(),
        "faces": surface.faces_raw.len(),
    });
    timings.write = t.elapsed().as_millis() as u64;

    let report = Report {
        summary,
        timings_ms: timings,
        files,
    };
    let json = serde_json::to_vec_pretty(&report)?;
    std::fs::write(out_dir.join(SUMMARY_FILE), json)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::sanitize_name;

    #[test]
    fn names_are_safe() {
        assert_eq!(sanitize_name("1AHW_l_u"), "1AHW_l_u");
        assert_eq!(sanitize_name("../../etc/passwd"), "_.._etc_passwd");
        assert_eq!(sanitize_name("..."), "protein");
        assert_eq!(sanitize_name("a b/c"), "a_b_c");
    }
}
