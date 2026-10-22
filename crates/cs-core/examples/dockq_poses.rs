//! Generates perturbed ligand poses for validating [`cs_core::eval`] against the official
//! DockQ program (see `tools/benchmark/validate_dockq.py` and `docs/EVALUATION.md`).
//!
//! ```text
//! cargo run --release -p cs-core --example dockq_poses -- <out_dir> <prefix>...
//! ```
//!
//! Each `<prefix>` names a complex by the common prefix of its four files, e.g.
//! `data/complexes/1AHW/1AHW` for `1AHW_{r,l}_{u,b}.pdb`. For each complex it writes
//! `<id>_native.pdb` (bound receptor as chain A, bound ligand as chain B), one
//! `<id>_poseK.pdb` per pose (unbound receptor as chain A, posed unbound ligand as chain B)
//! and writes `<out_dir>/poses.tsv` with one line per pose: the pose, our metrics on the
//! coordinates as written (rounded to 0.001 Å, first chain mapping), our DockQ on the exact
//! coordinates and the best DockQ over all chain mappings (what `Native::evaluate` reports).
//!
//! Chains are merged and residues renumbered because DockQ v2 scores every chain pair
//! separately, whereas the docking benchmark (and CAPRI) treat each partner as one body.
//! Only the protein residues our evaluator uses are written, so both programs see the
//! same atoms and the comparison isolates the metric computations.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use cs_core::eval::Native;
use cs_core::eval::align::ChainPair;
use cs_core::eval::residues::protein_residues;
use cs_core::geom::transform::{Rigid, Rng};
use cs_core::geom::vec3::*;
use cs_core::io::pdb::{Atom, Structure, write_atom};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        return Err("usage: dockq_poses <out_dir> <prefix>...".into());
    }
    let out = PathBuf::from(&args[0]);
    std::fs::create_dir_all(&out)?;
    let mut tsv = String::from(
        "case\tpose\trot\ttrans\tfnat\tfnonnat\tirmsd\tlrmsd\tdockq\tcapri\tnat_correct\tnat_total\texact_dockq\tbest_dockq\n",
    );
    for prefix in &args[1..] {
        let id = Path::new(prefix)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let read = |s: &str| Structure::read(Path::new(&format!("{prefix}_{s}.pdb")));
        let (rb, lb, ru, lu) = (read("r_b")?, read("l_b")?, read("r_u")?, read("l_u")?);
        let native = Native::new(&rb, &lb, &ru, &lu)?;
        let info = native.info();
        let all_b = |s: &Structure| chain_order(s);
        let native_pdb = merged(&rb, &all_b(&rb), 'A') + &merged(&lb, &all_b(&lb), 'B');
        std::fs::write(out.join(format!("{id}_native.pdb")), native_pdb + "END\n")?;
        let rec_pdb = merged(&ru, &model_chains(&info.receptor_chains), 'A');

        let mut rng = Rng::new(id.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
            (h ^ b as u64).wrapping_mul(0x100_0000_01b3)
        }));
        for (k, pose) in poses(&native, &lu, &mut rng).iter().enumerate() {
            let moved = Structure {
                atoms: lu
                    .atoms
                    .iter()
                    .map(|a| Atom {
                        pos: pose.apply(a.pos),
                        ..a.clone()
                    })
                    .collect(),
            };
            let pdb = rec_pdb.clone()
                + &merged(&moved, &model_chains(&info.ligand_chains), 'B')
                + "END\n";
            std::fs::write(out.join(format!("{id}_pose{k}.pdb")), pdb)?;
            // DockQ reads coordinates rounded to 0.001 Å; score the same rounded ligand
            // with the chain mapping written to the files (the first one).
            let rounded = Structure {
                atoms: moved
                    .atoms
                    .iter()
                    .map(|a| Atom {
                        pos: a.pos.map(|x| format!("{x:.3}").parse().unwrap()),
                        ..a.clone()
                    })
                    .collect(),
            };
            let same = |x: &[ChainPair], y: &[ChainPair]| {
                x.iter()
                    .map(|p| (p.a, p.b))
                    .eq(y.iter().map(|p| (p.a, p.b)))
            };
            let as_written = Native::new(&rb, &lb, &ru, &rounded)?;
            let k0 = as_written
                .chain_mappings()
                .position(|(r, l)| same(r, &info.receptor_chains) && same(l, &info.ligand_chains))
                .expect("same chain mapping");
            let m = as_written.evaluate_each(&Rigid::IDENTITY)[k0];
            let exact = native.evaluate_each(pose)[0];
            let best = native.evaluate(pose);
            let rot: Vec<String> = pose
                .rot
                .iter()
                .flatten()
                .map(|x| format!("{x:.17e}"))
                .collect();
            let trans: Vec<String> = pose.trans.iter().map(|x| format!("{x:.17e}")).collect();
            writeln!(
                tsv,
                "{id}\t{k}\t{}\t{}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{}\t{}\t{}\t{:.6}\t{:.6}",
                rot.join(","),
                trans.join(","),
                m.fnat,
                m.fnonnat,
                m.irmsd,
                m.lrmsd,
                m.dockq,
                m.capri.name(),
                m.nat_correct,
                m.nat_total,
                exact.dockq,
                best.dockq
            )?;
        }
        eprintln!("{id}: {info:?}");
    }
    std::fs::write(out.join("poses.tsv"), tsv)?;
    Ok(())
}

/// Poses spanning all CAPRI classes: the unbound ligand as given, the optimal rigid pose,
/// perturbations of it of growing size, and two random placements.
fn poses(native: &Native, ligand: &Structure, rng: &mut Rng) -> Vec<Rigid> {
    let best = native.optimal_pose();
    let n = ligand.atoms.len() as f64;
    let centroid = scale(
        ligand.atoms.iter().fold([0.0; 3], |c, a| add(c, a.pos)),
        1.0 / n,
    );
    let center = best.apply(centroid);
    let unit = |rng: &mut Rng| loop {
        let v = [0, 1, 2].map(|_| 2.0 * rng.next_f64() - 1.0);
        if norm2(v) > 0.01 && norm2(v) <= 1.0 {
            break normalize(v);
        }
    };
    let mut out = vec![Rigid::IDENTITY, best];
    for (deg, dist) in [
        (3.0, 0.5),
        (8.0, 1.5),
        (15.0, 3.0),
        (25.0, 4.0),
        (40.0, 6.0),
        (60.0, 9.0),
        (90.0, 14.0),
    ] {
        let axis = unit(rng);
        let rot = Rigid::about(center, Rigid::axis_angle(axis, f64::to_radians(deg)).rot);
        let shift = Rigid::translation(scale(unit(rng), dist));
        out.push(shift.compose(&rot).compose(&best));
    }
    for _ in 0..2 {
        let rot = Rigid::about(center, rng.rotation());
        let shift = Rigid::translation(scale(unit(rng), 25.0));
        out.push(shift.compose(&rot).compose(&best));
    }
    out
}

fn chain_order(s: &Structure) -> Vec<char> {
    s.chains()
}

/// Unbound chains in the order of the bound chains they map to.
fn model_chains(pairs: &[ChainPair]) -> Vec<char> {
    pairs.iter().map(|p| p.b).collect()
}

/// Protein residues of `chains` (in that order), as one chain `id` numbered from 1.
fn merged(s: &Structure, chains: &[char], id: char) -> String {
    let res = protein_residues(s);
    let mut out = String::new();
    let mut n = 0;
    for &c in chains {
        for r in res.iter().filter(|r| r.chain == c) {
            n += 1;
            for &i in &r.atoms {
                let mut a = s.atoms[i].clone();
                a.chain = id;
                a.res_seq = n;
                a.i_code = ' ';
                a.alt_loc = ' ';
                a.hetatm = false;
                write_atom(&mut out, &a);
            }
        }
    }
    out
}
