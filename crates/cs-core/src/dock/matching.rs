//! Exhaustive complementarity matching of receptor and ligand context shapes.
//!
//! For a receptor point `p`, a ligand point `q` and a rotation `k`, with the ligand rings
//! rotated by `k` (`L = rotl(L, k)`), per ring:
//!
//! - fit reward: `popcount((R.shell & L.inside) | (R.inside & L.shell))`: ligand matter in
//!   the receptor's exterior band and receptor matter in the ligand's band. The two sets are
//!   disjoint (`shell` and `inside` are), so one popcount counts both.
//! - surface: `popcount(R.surf & L.surf)`: both surfaces pass through the same sample, i.e.
//!   they coincide there (the paper's SES layer). Unlike the fill term it does not grow when
//!   a protrusion is merely pushed into a wide pocket.
//! - clash: `popcount((R.core & L.inside) | (R.inside & L.core))`: overlaps that reach deeper
//!   than `core` below either surface.
//!
//! `score = fill · Σ reward + surface · Σ surface - clash · Σ clash` ([`MatchWeights`]). All `K` rotations of a pair are computed
//! together so the compiler can vectorize across rotations (one SIMD lane per rotation).

use rayon::prelude::*;

use serde::{Deserialize, Serialize};

use super::descriptor::{Descriptors, K, Rings};
use crate::error::Result;
use crate::progress::{Progress, check};

/// A raw match: receptor point `p`, ligand point `q`, rotation `k`, integer score.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match {
    pub score: i32,
    pub p: u32,
    pub q: u32,
    pub k: u8,
}

impl Match {
    /// Total order: higher score first, then lower indices (keeps results deterministic).
    fn key(&self) -> (std::cmp::Reverse<i32>, u32, u32, u8) {
        (std::cmp::Reverse(self.score), self.p, self.q, self.k)
    }
}

/// Integer weights of the three match terms (see module docs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MatchWeights {
    pub fill: i32,
    pub surface: i32,
    pub clash: i32,
}

impl Default for MatchWeights {
    fn default() -> Self {
        Self {
            fill: 1,
            surface: 1,
            clash: 3,
        }
    }
}

#[inline(always)]
fn kernel_generic(r: Rings, l: Rings, w: MatchWeights, out: &mut [i32; K]) {
    let mut fill = [0u32; K];
    let mut surf = [0u32; K];
    let mut clash = [0u32; K];
    for ring in 0..r.shell.len() {
        let (rs, ri, rc, rf) = (r.shell[ring], r.inside[ring], r.core[ring], r.surf[ring]);
        let (ls, li, lc, lf) = (l.shell[ring], l.inside[ring], l.core[ring], l.surf[ring]);
        for k in 0..K {
            let s = k as u32;
            let lin = li.rotate_left(s);
            let lsh = ls.rotate_left(s);
            let lco = lc.rotate_left(s);
            let lsf = lf.rotate_left(s);
            fill[k] += ((rs & lin) | (ri & lsh)).count_ones();
            surf[k] += (rf & lsf).count_ones();
            clash[k] += ((rc & lin) | (ri & lco)).count_ones();
        }
    }
    for k in 0..K {
        out[k] = w.fill * fill[k] as i32 + w.surface * surf[k] as i32 - w.clash * clash[k] as i32;
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2,popcnt,bmi1,bmi2")]
fn kernel_avx2(r: Rings, l: Rings, w: MatchWeights, out: &mut [i32; K]) {
    kernel_generic(r, l, w, out)
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f,avx512bw,avx512vl,avx2,popcnt,bmi1,bmi2")]
fn kernel_avx512(r: Rings, l: Rings, w: MatchWeights, out: &mut [i32; K]) {
    kernel_generic(r, l, w, out)
}

/// Instruction-set specialization chosen once at run time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Isa {
    Generic,
    #[cfg(target_arch = "x86_64")]
    Avx2,
    #[cfg(target_arch = "x86_64")]
    Avx512,
}

impl Isa {
    pub fn detect() -> Isa {
        #[cfg(target_arch = "x86_64")]
        {
            if std::arch::is_x86_feature_detected!("avx512f")
                && std::arch::is_x86_feature_detected!("avx512bw")
                && std::arch::is_x86_feature_detected!("avx512vl")
                && std::arch::is_x86_feature_detected!("avx2")
                && std::arch::is_x86_feature_detected!("popcnt")
                && std::arch::is_x86_feature_detected!("bmi1")
                && std::arch::is_x86_feature_detected!("bmi2")
            {
                return Isa::Avx512;
            }
            if std::arch::is_x86_feature_detected!("avx2")
                && std::arch::is_x86_feature_detected!("popcnt")
                && std::arch::is_x86_feature_detected!("bmi1")
                && std::arch::is_x86_feature_detected!("bmi2")
            {
                return Isa::Avx2;
            }
        }
        Isa::Generic
    }
}

/// Scores of all `K` rotations of receptor point `p` against ligand point `q`.
#[inline]
pub fn pair_scores(
    isa: Isa,
    rec: &Descriptors,
    p: usize,
    lig: &Descriptors,
    q: usize,
    w: MatchWeights,
    out: &mut [i32; K],
) {
    let (r, l) = (rec.rings(p), lig.rings(q));
    match isa {
        Isa::Generic => kernel_generic(r, l, w, out),
        // SAFETY: these variants are only constructed by `Isa::detect` after checking that
        // the CPU supports every feature the function is compiled with.
        #[cfg(target_arch = "x86_64")]
        Isa::Avx2 => unsafe { kernel_avx2(r, l, w, out) },
        #[cfg(target_arch = "x86_64")]
        Isa::Avx512 => unsafe { kernel_avx512(r, l, w, out) },
    }
}

/// Enumerates every `(p, q, k)`. For each receptor point keeps the best `per_point` ligand
/// points (best rotation of each), then returns the best `max_total` of all of those, sorted.
/// Keeping a few per receptor point spreads the candidates over the whole receptor surface
/// instead of letting one deep pocket take them all.
pub fn enumerate(
    rec: &Descriptors,
    lig: &Descriptors,
    w: MatchWeights,
    per_point: usize,
    max_total: usize,
    progress: &dyn Progress,
    progress_span: (f32, f32),
) -> Result<Vec<Match>> {
    let isa = Isa::detect();
    let per_point = per_point.max(1);
    let n = rec.len();
    let done = std::sync::atomic::AtomicUsize::new(0);
    let lists: Vec<Vec<Match>> = (0..n)
        .into_par_iter()
        .map(|p| {
            check(progress)?;
            let mut scores = [0i32; K];
            let mut best: Vec<Match> = Vec::with_capacity(per_point + 1);
            let mut floor = i32::MIN;
            for q in 0..lig.len() {
                pair_scores(isa, rec, p, lig, q, w, &mut scores);
                let (mut bk, mut bs) = (0usize, scores[0]);
                for (k, &s) in scores.iter().enumerate().skip(1) {
                    if s > bs {
                        bs = s;
                        bk = k;
                    }
                }
                if best.len() == per_point && bs <= floor {
                    continue;
                }
                let m = Match {
                    score: bs,
                    p: p as u32,
                    q: q as u32,
                    k: bk as u8,
                };
                let pos = best.partition_point(|x| x.key() < m.key());
                best.insert(pos, m);
                best.truncate(per_point);
                if best.len() == per_point {
                    floor = best[per_point - 1].score;
                }
            }
            let d = done.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            if d % 64 == 0 {
                let f = d as f32 / n as f32;
                progress.report(
                    "matching",
                    progress_span.0 + (progress_span.1 - progress_span.0) * f,
                );
            }
            Ok(best)
        })
        .collect::<Result<_>>()?;
    let mut all: Vec<Match> = lists.into_iter().flatten().collect();
    all.sort_unstable_by_key(|m| m.key());
    all.truncate(max_total);
    Ok(all)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::transform::Rng;

    fn random_set(n: usize, rings: usize, rng: &mut Rng) -> Descriptors {
        let mut d = Descriptors {
            points: vec![[0.0; 3]; n],
            frames: vec![[[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]; n],
            n_rings: rings,
            shell: Vec::new(),
            inside: Vec::new(),
            core: Vec::new(),
            surf: Vec::new(),
        };
        for _ in 0..n * rings {
            let inside = rng.next_u64() as u32;
            let core = inside & rng.next_u64() as u32;
            let shell = !inside & rng.next_u64() as u32;
            let surf = (inside & !core | shell) & rng.next_u64() as u32;
            d.inside.push(inside);
            d.core.push(core);
            d.shell.push(shell);
            d.surf.push(surf);
        }
        d
    }

    const W: MatchWeights = MatchWeights {
        fill: 2,
        surface: 5,
        clash: 3,
    };

    /// Bit-by-bit reference of the score definition.
    fn reference(rec: &Descriptors, p: usize, lig: &Descriptors, q: usize, k: usize) -> i32 {
        let (r, l) = (rec.rings(p), lig.rings(q));
        let mut s = 0;
        for i in 0..rec.n_rings {
            for a in 0..K {
                let b = (a + K - k) % K; // ligand bit landing on receptor bit a
                let bit = |w: u32, i: usize| (w >> i) & 1 == 1;
                if (bit(r.shell[i], a) && bit(l.inside[i], b))
                    || (bit(r.inside[i], a) && bit(l.shell[i], b))
                {
                    s += W.fill;
                }
                if bit(r.surf[i], a) && bit(l.surf[i], b) {
                    s += W.surface;
                }
                if (bit(r.core[i], a) && bit(l.inside[i], b))
                    || (bit(r.inside[i], a) && bit(l.core[i], b))
                {
                    s -= W.clash;
                }
            }
        }
        s
    }

    #[test]
    fn all_kernels_match_reference() {
        let mut rng = Rng::new(9);
        let rec = random_set(4, 36, &mut rng);
        let lig = random_set(5, 36, &mut rng);
        let mut isas = vec![Isa::Generic];
        let detected = Isa::detect();
        if detected != Isa::Generic {
            isas.push(detected);
        }
        #[cfg(target_arch = "x86_64")]
        if detected == Isa::Avx512 {
            isas.push(Isa::Avx2);
        }
        for isa in isas {
            for p in 0..rec.len() {
                for q in 0..lig.len() {
                    let mut out = [0; K];
                    pair_scores(isa, &rec, p, &lig, q, W, &mut out);
                    for (k, &s) in out.iter().enumerate() {
                        assert_eq!(s, reference(&rec, p, &lig, q, k), "{isa:?} k={k}");
                    }
                }
            }
        }
    }

    #[test]
    fn enumeration_keeps_best_per_point() {
        let mut rng = Rng::new(10);
        let rec = random_set(6, 12, &mut rng);
        let lig = random_set(9, 12, &mut rng);
        let all = enumerate(
            &rec,
            &lig,
            W,
            2,
            100,
            &crate::progress::NoProgress,
            (0.0, 1.0),
        )
        .unwrap();
        assert_eq!(all.len(), 12);
        assert!(all.windows(2).all(|w| w[0].score >= w[1].score));
        for m in &all {
            let mut out = [0; K];
            pair_scores(
                Isa::Generic,
                &rec,
                m.p as usize,
                &lig,
                m.q as usize,
                W,
                &mut out,
            );
            assert_eq!(*out.iter().max().unwrap(), m.score);
        }
    }
}
