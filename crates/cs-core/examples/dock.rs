//! Rigid-body docking of two PDB files by complementary context-shape matching.
//!
//! ```text
//! cargo run --release -p cs-core --example dock -- RECEPTOR.pdb LIGAND.pdb [options]
//!
//!   --out DIR            output directory (default out/dock)
//!   --surface atoms|ses  surface model: built-in atom SES grid (default) or SES mesh
//!   --params JSON|@FILE  DockParams overrides, e.g. '{"n_poses":50}'
//!   --complexes N        write the top N complexes as PDB (default 10)
//!   --randomize SEED     move the ligand by a seeded random rigid transform first (the
//!                        benchmark files are pre-superposed on the complex)
//!   --native R_B L_B     bound receptor and ligand: report CAPRI class and DockQ per pose
//! ```
//!
//! Writes `poses.json` (ranked poses, transforms relative to the ligand file as given, plus
//! run statistics) and `complex_NNN.pdb` (receptor + docked ligand).

use std::path::PathBuf;
use std::time::Instant;

use cs_core::dock::{self, DockInput, DockParams};
use cs_core::eval::Native;
use cs_core::geom::transform::{Rigid, Rng};
use cs_core::io::pdb::Structure;
use cs_core::progress::NoProgress;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: dock RECEPTOR.pdb LIGAND.pdb [--out DIR] [--surface atoms|ses] \
                 [--params JSON|@FILE] [--complexes N] [--randomize SEED] [--native R_B L_B]";
    let receptor_path = PathBuf::from(args.next().ok_or(usage)?);
    let ligand_path = PathBuf::from(args.next().ok_or(usage)?);
    let mut out = PathBuf::from("out/dock");
    let mut surface = String::from("atoms");
    let mut params = DockParams::default();
    let mut n_complexes = 10usize;
    let mut randomize: Option<u64> = None;
    let mut native_paths: Option<(PathBuf, PathBuf)> = None;
    while let Some(a) = args.next() {
        let mut val = || args.next().ok_or(format!("{a} needs a value"));
        match a.as_str() {
            "--out" => out = val()?.into(),
            "--surface" => surface = val()?,
            "--params" => {
                let v = val()?;
                let text = match v.strip_prefix('@') {
                    Some(path) => std::fs::read_to_string(path)?,
                    None => v,
                };
                params = serde_json::from_str(&text)?;
            }
            "--complexes" => n_complexes = val()?.parse()?,
            "--randomize" => randomize = Some(val()?.parse()?),
            "--native" => native_paths = Some((val()?.into(), val()?.into())),
            _ => return Err(format!("unknown argument {a}\n{usage}").into()),
        }
    }

    let t0 = Instant::now();
    let receptor = Structure::read(&receptor_path)?;
    let ligand_file = Structure::read(&ligand_path)?;
    // Optional random start; poses are reported relative to the ligand file either way.
    let start = match randomize {
        Some(seed) => {
            let mut rng = Rng::new(seed);
            let c = centroid(&ligand_file);
            Rigid::about(c, rng.rotation()).compose(&Rigid::translation([25.0, -15.0, 30.0]))
        }
        None => Rigid::IDENTITY,
    };
    let ligand = dock::transform_structure(&ligand_file, &start);
    let t_read = t0.elapsed().as_secs_f64();

    let t1 = Instant::now();
    let make = |s: &Structure| -> cs_core::Result<DockInput> {
        match surface.as_str() {
            "ses" => DockInput::from_ses(
                s.clone(),
                &cs_core::surface::SesParams::default(),
                &NoProgress,
            ),
            _ => DockInput::from_atoms(s.clone(), &dock::AtomSurfaceParams::default(), &NoProgress),
        }
    };
    let (r, l) = rayon::join(|| make(&receptor), || make(&ligand));
    let (rec_in, lig_in) = (r?, l?);
    let t_surface = t1.elapsed().as_secs_f64();

    let output = dock::run_with_stats(&rec_in, &lig_in, &params, &NoProgress)?;
    let poses: Vec<dock::Pose> = output
        .poses
        .iter()
        .map(|p| dock::Pose {
            transform: p.transform.compose(&start),
            ..p.clone()
        })
        .collect();

    let native = match &native_paths {
        Some((rb, lb)) => Some(Native::new(
            &Structure::read(rb)?,
            &Structure::read(lb)?,
            &receptor,
            &ligand_file,
        )?),
        None => None,
    };
    let metrics = native
        .as_ref()
        .map(|n| n.evaluate_many(&poses.iter().map(|p| p.transform).collect::<Vec<_>>()));

    std::fs::create_dir_all(&out)?;
    let json = serde_json::json!({
        "receptor": receptor_path,
        "ligand": ligand_path,
        "surface": surface,
        "params": params,
        "stats": output.stats,
        "seconds_surfaces": t_surface,
        "poses": poses,
        "metrics": metrics,
    });
    std::fs::write(out.join("poses.json"), serde_json::to_string_pretty(&json)?)?;
    for p in poses.iter().take(n_complexes) {
        let pdb = dock::complex_pdb(&receptor, &ligand_file, &p.transform);
        std::fs::write(out.join(format!("complex_{:03}.pdb", p.rank)), pdb)?;
    }

    let s = &output.stats;
    println!(
        "critical points: receptor {}, ligand {} (with tilted frames); pairs {}",
        s.receptor_points, s.ligand_points, s.pairs
    );
    println!(
        "time: read {:.2} s, surfaces {:.2} s, descriptors {:.2} s, matching {:.2} s \
         ({:.1} M pairs/s), refinement {:.2} s, docking total {:.2} s",
        t_read,
        t_surface,
        s.seconds_descriptors,
        s.seconds_matching,
        s.pairs as f64 / s.seconds_matching.max(1e-9) / 1e6,
        s.seconds_refinement,
        s.seconds_total
    );
    for (i, p) in poses.iter().take(10).enumerate() {
        let m = metrics
            .as_ref()
            .map(|m| format!("  {} DockQ {:.3}", m[i].capri.name(), m[i].dockq))
            .unwrap_or_default();
        println!(
            "#{:<3} score {:8.2}  contacts {:6.1}  clash {:5.2}  cluster {:3}{m}",
            p.rank, p.score, p.components.contacts, p.components.clash, p.cluster_size
        );
    }
    if let Some(n) = &native {
        // Score of the native placement itself (before and after local refinement): tells
        // whether a miss comes from the search or from the scoring function.
        let scorer = dock::scoring::Scorer::new(&receptor, &ligand, params.score_grid_spacing)?;
        let t = n.optimal_pose().compose(&start.inverse());
        let c = scorer.evaluate(&t, &params.score);
        let (tr, cr) = scorer.refine(&t, &params.score, &params.refine);
        println!(
            "native placement: score {:.2} (contacts {:.1}, clash {:.2}); refined {:.2}, DockQ {:.3}",
            c.total(&params.score),
            c.contacts,
            c.clash,
            cr.total(&params.score),
            n.evaluate(&tr.compose(&start)).dockq
        );
    }
    if let Some(m) = &metrics {
        let first = |c: cs_core::eval::Capri| m.iter().position(|x| x.capri >= c).map(|i| i + 1);
        println!(
            "first acceptable pose: rank {:?}; first medium: rank {:?}",
            first(cs_core::eval::Capri::Acceptable),
            first(cs_core::eval::Capri::Medium)
        );
    }
    println!("wrote {}", out.display());
    Ok(())
}

fn centroid(s: &Structure) -> [f64; 3] {
    let n = s.atoms.len().max(1) as f64;
    s.atoms.iter().fold([0.0; 3], |c, a| {
        [
            c[0] + a.pos[0] / n,
            c[1] + a.pos[1] / n,
            c[2] + a.pos[2] / n,
        ]
    })
}
