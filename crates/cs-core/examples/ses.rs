//! Solvent-excluded surface of a PDB file, written as MSMS `.vert`/`.face`.
//!
//! ```text
//! cargo run --release -p cs-core --example ses -- protein.pdb out/protein \
//!     [--probe 1.4] [--spacing 0.5] [--radii bondi|united] [--hydrogens] [--no-hetatm] \
//!     [--keep-cavities] [--reference msms/protein] [--verbose]
//! ```
//!
//! Writes `out/protein.vert` and `out/protein.face` and prints mesh statistics. With
//! `--reference`, also compares against an MSMS surface (`<prefix>.vert` + `.face`): area,
//! volume and the distance from each mesh's vertices to the other surface.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use cs_core::geom::sdf::SignedDistance;
use cs_core::io::msms::MsmsSurface;
use cs_core::io::pdb::Structure;
use cs_core::mesh::Mesh;
use cs_core::progress::Progress;
use cs_core::surface::{Radii, SesGrid, SesParams};
use rayon::prelude::*;

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let mut positional = Vec::new();
    let mut params = SesParams::default();
    let mut reference: Option<PathBuf> = None;
    let mut verbose = false;
    while let Some(a) = args.next() {
        let mut value = || args.next().ok_or(format!("{a} needs a value"));
        match a.as_str() {
            "--probe" => params.probe = value()?.parse()?,
            "--spacing" => params.grid_spacing = value()?.parse()?,
            "--radii" => {
                params.radii = match value()?.as_str() {
                    "bondi" => Radii::Bondi,
                    "united" => Radii::United,
                    other => return Err(format!("unknown radii set {other}").into()),
                }
            }
            "--hydrogens" => params.include_hydrogens = true,
            "--no-hetatm" => params.include_hetatm = false,
            "--keep-cavities" => params.fill_cavities = false,
            "--reference" => reference = Some(value()?.into()),
            "--verbose" => verbose = true,
            _ if a.starts_with("--") => return Err(format!("unknown option {a}").into()),
            _ => positional.push(a),
        }
    }
    let [pdb, out] = positional.as_slice() else {
        return Err("usage: ses <in.pdb> <out_prefix> [options]".into());
    };

    let structure = Structure::read(Path::new(pdb))?;
    let (centers, _) = params.spheres(&structure.atoms);
    let t0 = Instant::now();
    let stages = Stages {
        start: t0,
        on: verbose,
    };
    let grid = SesGrid::from_atoms(&structure.atoms, &params, &stages)?;
    let t_field = t0.elapsed();
    let mesh = grid.mesh(&stages)?;
    let t_total = t0.elapsed();

    let d = grid.dims();
    println!("atoms          {}", centers.len());
    println!(
        "grid           {}x{}x{} nodes, spacing {} Å, probe {} Å",
        d[0],
        d[1],
        d[2],
        grid.spacing(),
        grid.probe()
    );
    println!(
        "time           {:.3} s (field {:.3} s, mesh {:.3} s), {} threads",
        t_total.as_secs_f64(),
        t_field.as_secs_f64(),
        (t_total - t_field).as_secs_f64(),
        rayon::current_num_threads()
    );
    println!(
        "mesh           {} vertices, {} faces, {} edges not shared by exactly 2 faces",
        mesh.vertices.len(),
        mesh.faces.len(),
        open_edges(&mesh)
    );
    println!("area           {:.1} Å²", mesh.area());
    println!("volume         {:.1} Å³", mesh.volume());

    let out = PathBuf::from(out);
    if let Some(dir) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(out.with_extension("vert"), mesh.write_msms_vert())?;
    std::fs::write(out.with_extension("face"), mesh.write_msms_face())?;

    if let Some(r) = reference {
        let msms = MsmsSurface::read(&r.with_extension("vert"), &r.with_extension("face"))?;
        let reference = Mesh::from_msms(&msms)?;
        println!(
            "reference      area {:.1} Å² ({:+.2} %), volume {:.1} Å³ ({:+.2} %)",
            reference.area(),
            100.0 * (mesh.area() / reference.area() - 1.0),
            reference.volume(),
            100.0 * (mesh.volume() / reference.volume() - 1.0)
        );
        let to_ref = distances(&mesh.vertices, &reference);
        let from_ref = distances(&reference.vertices, &mesh);
        print_stats("ours -> MSMS", &to_ref);
        print_stats("MSMS -> ours", &from_ref);
        print_stats("symmetric", &[to_ref, from_ref].concat());
    }
    Ok(())
}

/// Prints when each stage starts, with `--verbose`.
struct Stages {
    start: Instant,
    on: bool,
}

impl Progress for Stages {
    fn report(&self, stage: &str, _fraction: f32) {
        if self.on {
            eprintln!("{:8.3} s  {stage}", self.start.elapsed().as_secs_f64());
        }
    }
}

/// Edges not used by exactly two faces (0 for a closed manifold mesh).
fn open_edges(m: &Mesh) -> usize {
    let mut count: HashMap<(u32, u32), u32> = HashMap::new();
    for f in &m.faces {
        for k in 0..3 {
            let (a, b) = (f[k], f[(k + 1) % 3]);
            *count.entry((a.min(b), a.max(b))).or_default() += 1;
        }
    }
    count.values().filter(|&&c| c != 2).count()
}

fn distances(points: &[[f64; 3]], surface: &Mesh) -> Vec<f64> {
    let sdf = SignedDistance::new(surface.vertices.clone(), &surface.faces);
    points.par_iter().map(|&p| sdf.signed_distance(p)).collect()
}

fn print_stats(label: &str, d: &[f64]) {
    let mut a: Vec<f64> = d.iter().map(|x| x.abs()).collect();
    a.sort_by(f64::total_cmp);
    let mean = a.iter().sum::<f64>() / a.len() as f64;
    let signed = d.iter().sum::<f64>() / d.len() as f64;
    let q = |f: f64| a[((a.len() - 1) as f64 * f) as usize];
    println!(
        "{label:<14} mean |d| {mean:.3} Å, mean d {signed:+.3} Å, median {:.3}, p95 {:.3}, max {:.3} Å",
        q(0.5),
        q(0.95),
        a[a.len() - 1]
    );
}
