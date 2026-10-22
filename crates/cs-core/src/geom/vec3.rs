//! Minimal `[f64; 3]` vector helpers. Kept as free functions on plain arrays so data can be
//! shared with I/O and serialization code without conversions.

pub type V3 = [f64; 3];

#[inline]
pub fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

#[inline]
pub fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

#[inline]
pub fn scale(a: V3, s: f64) -> V3 {
    [a[0] * s, a[1] * s, a[2] * s]
}

#[inline]
pub fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
pub fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[inline]
pub fn norm2(a: V3) -> f64 {
    dot(a, a)
}

#[inline]
pub fn norm(a: V3) -> f64 {
    norm2(a).sqrt()
}

#[inline]
pub fn normalize(a: V3) -> V3 {
    let n = norm(a);
    if n > 0.0 { scale(a, 1.0 / n) } else { [0.0; 3] }
}

#[inline]
pub fn dist2(a: V3, b: V3) -> f64 {
    norm2(sub(a, b))
}
