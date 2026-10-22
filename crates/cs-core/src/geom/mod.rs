pub mod bvh;
pub mod grid;
pub mod sdf;
pub mod vec3;

/// `n` nearly uniform unit directions on the sphere (Fibonacci lattice).
pub fn fibonacci_sphere(n: usize) -> Vec<vec3::V3> {
    let golden = std::f64::consts::PI * (3.0 - 5f64.sqrt());
    (0..n)
        .map(|i| {
            let z = 1.0 - (2.0 * i as f64 + 1.0) / n as f64;
            let r = (1.0 - z * z).max(0.0).sqrt();
            let phi = golden * i as f64;
            [r * phi.cos(), r * phi.sin(), z]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn fibonacci_is_unit_and_balanced() {
        let d = super::fibonacci_sphere(500);
        let mut c = [0.0; 3];
        for v in &d {
            assert!((super::vec3::norm(*v) - 1.0).abs() < 1e-12);
            c = super::vec3::add(c, *v);
        }
        assert!(super::vec3::norm(c) / 500.0 < 1e-2);
    }
}
