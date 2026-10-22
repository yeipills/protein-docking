//! Scores a docked model against the native (bound) complex with CAPRI measures and DockQ.
//!
//! ```text
//! cargo run --release -p cs-core --example evaluate -- \
//!     <native_receptor.pdb> <native_ligand.pdb> <receptor.pdb> <ligand.pdb> [--json]
//! cargo run --release -p cs-core --example evaluate -- \
//!     <native_receptor.pdb> <native_ligand.pdb> <model.pdb> --ligand-chains C [--json]
//! ```
//!
//! The model is either a receptor and a ligand file (e.g. the unbound receptor and the
//! docked ligand) or a single complex split by chain IDs. Residues are matched to the native
//! by sequence, so chain IDs and numbering do not need to agree.

use std::path::Path;
use std::process::ExitCode;

use cs_core::eval::Native;
use cs_core::geom::transform::Rigid;
use cs_core::io::pdb::Structure;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            eprintln!(
                "usage: evaluate <native_receptor.pdb> <native_ligand.pdb> \
                 (<receptor.pdb> <ligand.pdb> | <model.pdb> --ligand-chains IDS) [--json]"
            );
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    let mut ligand_chains: Option<String> = None;
    let mut json = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--json" => json = true,
            "--ligand-chains" => {
                ligand_chains = Some(args.next().ok_or("--ligand-chains needs a value")?)
            }
            _ if a.starts_with("--") => return Err(format!("unknown option {a}").into()),
            _ => files.push(a),
        }
    }
    let read = |p: &str| Structure::read(Path::new(p));
    let (receptor, ligand) = match (files.len(), &ligand_chains) {
        (4, None) => (read(&files[2])?, read(&files[3])?),
        (3, Some(ids)) => {
            let model = read(&files[2])?;
            let ids: Vec<char> = ids.chars().filter(|c| *c != ',').collect();
            let (lig, rec): (Vec<_>, Vec<_>) = model
                .atoms
                .into_iter()
                .partition(|a| ids.contains(&a.chain));
            if lig.is_empty() || rec.is_empty() {
                return Err(
                    "--ligand-chains must select some but not all chains of the model".into(),
                );
            }
            (Structure { atoms: rec }, Structure { atoms: lig })
        }
        _ => return Err("expected 4 PDB files, or 3 with --ligand-chains".into()),
    };
    let native = Native::new(&read(&files[0])?, &read(&files[1])?, &receptor, &ligand)?;
    let m = native.evaluate(&Rigid::IDENTITY);
    if json {
        let out = serde_json::json!({ "metrics": m, "native": native.info() });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        let info = native.info();
        let chains = |c: &[cs_core::eval::align::ChainPair]| {
            c.iter()
                .map(|p| format!("{}→{}", p.a, p.b))
                .collect::<Vec<_>>()
                .join(" ")
        };
        println!(
            "receptor chains (native→model): {}",
            chains(&info.receptor_chains)
        );
        println!(
            "ligand chains   (native→model): {}",
            chains(&info.ligand_chains)
        );
        println!(
            "matched residues: receptor {}/{}, ligand {}/{}",
            info.receptor_matched,
            info.receptor_residues,
            info.ligand_matched,
            info.ligand_residues
        );
        println!(
            "fnat     {:.3}  ({}/{})",
            m.fnat, m.nat_correct, m.nat_total
        );
        println!("fnonnat  {:.3}", m.fnonnat);
        println!("iRMSD    {:.3} Å", m.irmsd);
        println!("LRMSD    {:.3} Å", m.lrmsd);
        println!("DockQ    {:.3}", m.dockq);
        println!("CAPRI    {}", m.capri.name());
    }
    Ok(())
}
