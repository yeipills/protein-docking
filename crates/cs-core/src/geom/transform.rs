//! Rigid transforms, optimal superposition (Kabsch) and a small seeded RNG.

#![allow(clippy::needless_range_loop)] // 3x3 / 4x4 matrix code reads best indexed

use serde::{Deserialize, Serialize};

use super::vec3::*;

pub type M3 = [[f64; 3]; 3];

/// `x -> rot * x + trans`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rigid {
    pub rot: M3,
    pub trans: V3,
}

impl Default for Rigid {
    fn default() -> Self {
        Rigid::IDENTITY
    }
}

impl Rigid {
    pub const IDENTITY: Rigid = Rigid {
        rot: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        trans: [0.0; 3],
    };

    #[inline]
    pub fn apply(&self, p: V3) -> V3 {
        add(mat_vec(&self.rot, p), self.trans)
    }

    #[inline]
    pub fn apply_vector(&self, v: V3) -> V3 {
        mat_vec(&self.rot, v)
    }

    /// `self ∘ other`: apply `other` first, then `self`.
    pub fn compose(&self, other: &Rigid) -> Rigid {
        Rigid {
            rot: mat_mul(&self.rot, &other.rot),
            trans: self.apply(other.trans),
        }
    }

    pub fn inverse(&self) -> Rigid {
        let rt = transpose(&self.rot);
        Rigid {
            rot: rt,
            trans: scale(mat_vec(&rt, self.trans), -1.0),
        }
    }

    pub fn translation(t: V3) -> Rigid {
        Rigid {
            rot: Rigid::IDENTITY.rot,
            trans: t,
        }
    }

    /// Rotation by `angle` radians about unit `axis` through the origin.
    pub fn axis_angle(axis: V3, angle: f64) -> Rigid {
        Rigid {
            rot: axis_angle_matrix(normalize(axis), angle),
            trans: [0.0; 3],
        }
    }

    /// Rotation `rot` about the point `center`.
    pub fn about(center: V3, rot: M3) -> Rigid {
        Rigid {
            rot,
            trans: sub(center, mat_vec(&rot, center)),
        }
    }
}

pub fn mat_vec(m: &M3, v: V3) -> V3 {
    [dot(m[0], v), dot(m[1], v), dot(m[2], v)]
}

pub fn mat_mul(a: &M3, b: &M3) -> M3 {
    let mut r = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            r[i][j] = a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j];
        }
    }
    r
}

pub fn transpose(m: &M3) -> M3 {
    [
        [m[0][0], m[1][0], m[2][0]],
        [m[0][1], m[1][1], m[2][1]],
        [m[0][2], m[1][2], m[2][2]],
    ]
}

pub fn axis_angle_matrix(a: V3, angle: f64) -> M3 {
    let (s, c) = angle.sin_cos();
    let t = 1.0 - c;
    [
        [
            t * a[0] * a[0] + c,
            t * a[0] * a[1] - s * a[2],
            t * a[0] * a[2] + s * a[1],
        ],
        [
            t * a[0] * a[1] + s * a[2],
            t * a[1] * a[1] + c,
            t * a[1] * a[2] - s * a[0],
        ],
        [
            t * a[0] * a[2] - s * a[1],
            t * a[1] * a[2] + s * a[0],
            t * a[2] * a[2] + c,
        ],
    ]
}

/// Rotation that maps unit vector `from` onto unit vector `to`.
pub fn rotation_between(from: V3, to: V3) -> M3 {
    let (f, t) = (normalize(from), normalize(to));
    let c = dot(f, t);
    let axis = cross(f, t);
    let s = norm(axis);
    if s < 1e-12 {
        if c > 0.0 {
            return Rigid::IDENTITY.rot;
        }
        // Opposite vectors: rotate 180° about any perpendicular axis.
        let p = if f[0].abs() < 0.9 {
            [1.0, 0.0, 0.0]
        } else {
            [0.0, 1.0, 0.0]
        };
        return axis_angle_matrix(normalize(cross(f, p)), std::f64::consts::PI);
    }
    axis_angle_matrix(scale(axis, 1.0 / s), s.atan2(c))
}

/// Optimal rigid transform mapping `mobile` onto `target` (least squares, no reflection).
/// Returns the transform and the RMSD after applying it.
pub fn kabsch(mobile: &[V3], target: &[V3]) -> (Rigid, f64) {
    assert_eq!(mobile.len(), target.len(), "kabsch needs paired points");
    let n = mobile.len().max(1) as f64;
    let centroid = |p: &[V3]| scale(p.iter().fold([0.0; 3], |a, b| add(a, *b)), 1.0 / n);
    let (cm, ct) = (centroid(mobile), centroid(target));
    // Covariance H = Σ (m - cm)(t - ct)^T
    let mut h = [[0.0; 3]; 3];
    for (m, t) in mobile.iter().zip(target) {
        let (a, b) = (sub(*m, cm), sub(*t, ct));
        for i in 0..3 {
            for j in 0..3 {
                h[i][j] += a[i] * b[j];
            }
        }
    }
    let rot = kabsch_rotation(&h);
    let rigid = Rigid {
        rot,
        trans: sub(ct, mat_vec(&rot, cm)),
    };
    (rigid, rmsd(mobile, target, &rigid))
}

/// RMSD between `target` and `mobile` transformed by `t`.
pub fn rmsd(mobile: &[V3], target: &[V3], t: &Rigid) -> f64 {
    if mobile.is_empty() {
        return 0.0;
    }
    let s: f64 = mobile
        .iter()
        .zip(target)
        .map(|(m, x)| dist2(t.apply(*m), *x))
        .sum();
    (s / mobile.len() as f64).sqrt()
}

/// Rotation maximising trace(R H) via Horn's quaternion method (robust, no SVD needed).
fn kabsch_rotation(h: &M3) -> M3 {
    let (sxx, sxy, sxz) = (h[0][0], h[0][1], h[0][2]);
    let (syx, syy, syz) = (h[1][0], h[1][1], h[1][2]);
    let (szx, szy, szz) = (h[2][0], h[2][1], h[2][2]);
    let n = [
        [sxx + syy + szz, syz - szy, szx - sxz, sxy - syx],
        [syz - szy, sxx - syy - szz, sxy + syx, szx + sxz],
        [szx - sxz, sxy + syx, -sxx + syy - szz, syz + szy],
        [sxy - syx, szx + sxz, syz + szy, -sxx - syy + szz],
    ];
    let q = largest_eigenvector4(n);
    let (w, x, y, z) = (q[0], q[1], q[2], q[3]);
    [
        [
            w * w + x * x - y * y - z * z,
            2.0 * (x * y - w * z),
            2.0 * (x * z + w * y),
        ],
        [
            2.0 * (x * y + w * z),
            w * w - x * x + y * y - z * z,
            2.0 * (y * z - w * x),
        ],
        [
            2.0 * (x * z - w * y),
            2.0 * (y * z + w * x),
            w * w - x * x - y * y + z * z,
        ],
    ]
}

/// Eigenvector of the largest eigenvalue of a symmetric 4x4 matrix (Jacobi rotations).
fn largest_eigenvector4(mut a: [[f64; 4]; 4]) -> [f64; 4] {
    let mut v = [[0.0; 4]; 4];
    for (i, row) in v.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    for _ in 0..64 {
        let off: f64 = (0..4)
            .flat_map(|i| (0..4).map(move |j| (i, j)))
            .filter(|(i, j)| i != j)
            .map(|(i, j)| a[i][j] * a[i][j])
            .sum();
        if off < 1e-30 {
            break;
        }
        for p in 0..4 {
            for q in p + 1..4 {
                if a[p][q].abs() < 1e-300 {
                    continue;
                }
                let theta = (a[q][q] - a[p][p]) / (2.0 * a[p][q]);
                let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
                let t = if theta == 0.0 { 1.0 } else { t };
                let c = 1.0 / (t * t + 1.0).sqrt();
                let s = t * c;
                for k in 0..4 {
                    let (akp, akq) = (a[k][p], a[k][q]);
                    a[k][p] = c * akp - s * akq;
                    a[k][q] = s * akp + c * akq;
                }
                for k in 0..4 {
                    let (apk, aqk) = (a[p][k], a[q][k]);
                    a[p][k] = c * apk - s * aqk;
                    a[q][k] = s * apk + c * aqk;
                }
                for row in v.iter_mut() {
                    let (vp, vq) = (row[p], row[q]);
                    row[p] = c * vp - s * vq;
                    row[q] = s * vp + c * vq;
                }
            }
        }
    }
    let best = (0..4).max_by(|&i, &j| a[i][i].total_cmp(&a[j][j])).unwrap();
    let q = [v[0][best], v[1][best], v[2][best], v[3][best]];
    let n = (q.iter().map(|x| x * x).sum::<f64>()).sqrt();
    q.map(|x| x / n)
}

/// SplitMix64: tiny, fast, reproducible RNG for sampling (not cryptographic).
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `[0, 1)`.
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniformly distributed random rotation (Shoemake's method).
    pub fn rotation(&mut self) -> M3 {
        let (u1, u2, u3) = (self.next_f64(), self.next_f64(), self.next_f64());
        let tau = std::f64::consts::TAU;
        let (a, b) = ((1.0 - u1).sqrt(), u1.sqrt());
        let (w, x, y, z) = (
            a * (tau * u2).sin(),
            a * (tau * u2).cos(),
            b * (tau * u3).sin(),
            b * (tau * u3).cos(),
        );
        [
            [
                1.0 - 2.0 * (y * y + z * z),
                2.0 * (x * y - w * z),
                2.0 * (x * z + w * y),
            ],
            [
                2.0 * (x * y + w * z),
                1.0 - 2.0 * (x * x + z * z),
                2.0 * (y * z - w * x),
            ],
            [
                2.0 * (x * z - w * y),
                2.0 * (y * z + w * x),
                1.0 - 2.0 * (x * x + y * y),
            ],
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: V3, b: V3) -> bool {
        (0..3).all(|k| (a[k] - b[k]).abs() < 1e-9)
    }

    #[test]
    fn compose_and_inverse() {
        let mut rng = Rng::new(7);
        let a = Rigid {
            rot: rng.rotation(),
            trans: [1.0, -2.0, 3.0],
        };
        let b = Rigid {
            rot: rng.rotation(),
            trans: [0.5, 0.0, -1.0],
        };
        let p = [0.3, 4.0, -2.0];
        assert!(close(a.compose(&b).apply(p), a.apply(b.apply(p))));
        assert!(close(a.inverse().apply(a.apply(p)), p));
    }

    #[test]
    fn kabsch_recovers_transform() {
        let mut rng = Rng::new(42);
        let pts: Vec<V3> = (0..50)
            .map(|_| {
                [
                    rng.next_f64() * 20.0,
                    rng.next_f64() * 20.0,
                    rng.next_f64() * 20.0,
                ]
            })
            .collect();
        let t = Rigid {
            rot: rng.rotation(),
            trans: [5.0, -3.0, 8.0],
        };
        let moved: Vec<V3> = pts.iter().map(|p| t.apply(*p)).collect();
        let (fit, r) = kabsch(&pts, &moved);
        assert!(r < 1e-9, "rmsd {r}");
        for i in 0..3 {
            assert!(close(fit.rot[i], t.rot[i]));
        }
    }

    #[test]
    fn rotation_between_maps_vectors() {
        for (f, t) in [
            ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
            ([0.0, 0.0, 1.0], [0.0, 0.0, -1.0]),
            ([1.0, 2.0, 3.0], [1.0, 2.0, 3.0]),
        ] {
            let r = rotation_between(f, t);
            assert!(close(mat_vec(&r, normalize(f)), normalize(t)));
        }
    }

    #[test]
    fn random_rotations_are_orthonormal() {
        let mut rng = Rng::new(1);
        for _ in 0..100 {
            let r = rng.rotation();
            let rrt = mat_mul(&r, &transpose(&r));
            for i in 0..3 {
                for j in 0..3 {
                    assert!((rrt[i][j] - if i == j { 1.0 } else { 0.0 }).abs() < 1e-12);
                }
            }
        }
    }
}
