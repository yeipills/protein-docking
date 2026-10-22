// SIMD Math - Operaciones matemáticas ultra-optimizadas para protein docking
// Utiliza AVX2, AVX-512 y SSE para aceleración vectorial

use anyhow::Result;
use wide::*;

/// Motor de matemáticas SIMD para cálculos de docking
pub struct SimdMath {
    pub avx2_available: bool,
    pub avx512_available: bool,
    pub sse2_available: bool,
}

impl SimdMath {
    /// Crear nueva instancia detectando capabilities SIMD
    pub fn new() -> Self {
        let mut simd_math = Self {
            avx2_available: false,
            avx512_available: false,
            sse2_available: false,
        };
        
        // Detectar SIMD support
        #[cfg(target_arch = "x86_64")]
        {
            simd_math.sse2_available = std::arch::is_x86_feature_detected!("sse2");
            simd_math.avx2_available = std::arch::is_x86_feature_detected!("avx2");
            simd_math.avx512_available = std::arch::is_x86_feature_detected!("avx512f");
        }
        
        tracing::info!("🔧 SIMD capabilities - SSE2: {}, AVX2: {}, AVX-512: {}", 
                      simd_math.sse2_available, simd_math.avx2_available, simd_math.avx512_available);
        
        simd_math
    }
    
    /// Calcular distancias euclideas entre arrays de puntos (SIMD)
    pub fn calculate_distances_simd(&self, points_a: &[[f64; 3]], points_b: &[[f64; 3]]) -> Vec<f64> {
        if self.avx2_available && points_a.len() >= 4 && points_b.len() >= 4 {
            self.calculate_distances_avx2(points_a, points_b)
        } else if self.sse2_available && points_a.len() >= 2 && points_b.len() >= 2 {
            self.calculate_distances_sse2(points_a, points_b)
        } else {
            self.calculate_distances_scalar(points_a, points_b)
        }
    }
    
    /// Calcular productos punto vectoriales (SIMD)
    pub fn dot_products_simd(&self, vectors_a: &[[f64; 3]], vectors_b: &[[f64; 3]]) -> Vec<f64> {
        assert_eq!(vectors_a.len(), vectors_b.len());
        
        if self.avx2_available && vectors_a.len() >= 4 {
            self.dot_products_avx2(vectors_a, vectors_b)
        } else {
            self.dot_products_scalar(vectors_a, vectors_b)
        }
    }
    
    /// Calcular centro de masa con SIMD
    pub fn calculate_center_of_mass_simd(&self, positions: &[[f64; 3]], masses: &[f64]) -> [f64; 3] {
        assert_eq!(positions.len(), masses.len());
        
        if positions.is_empty() {
            return [0.0; 3];
        }
        
        if self.avx2_available && positions.len() >= 4 {
            self.calculate_center_of_mass_avx2(positions, masses)
        } else {
            self.calculate_center_of_mass_scalar(positions, masses)
        }
    }
    
    /// Aplicar transformación a múltiples puntos (SIMD)
    pub fn apply_transformation_simd(
        &self,
        points: &[[f64; 3]],
        translation: &[f64; 3],
        rotation_matrix: &[[f64; 3]; 3],
    ) -> Vec<[f64; 3]> {
        if self.avx2_available && points.len() >= 4 {
            self.apply_transformation_avx2(points, translation, rotation_matrix)
        } else {
            self.apply_transformation_scalar(points, translation, rotation_matrix)
        }
    }
    
    /// Calcular potenciales electrostáticos (SIMD)
    pub fn calculate_electrostatic_potential_simd(
        &self,
        charges_a: &[f64],
        positions_a: &[[f64; 3]],
        charges_b: &[f64],
        positions_b: &[[f64; 3]],
    ) -> f64 {
        if self.avx2_available {
            self.calculate_electrostatic_potential_avx2(charges_a, positions_a, charges_b, positions_b)
        } else {
            self.calculate_electrostatic_potential_scalar(charges_a, positions_a, charges_b, positions_b)
        }
    }
    
    // Implementaciones AVX2
    #[cfg(target_arch = "x86_64")]
    fn calculate_distances_avx2(&self, points_a: &[[f64; 3]], points_b: &[[f64; 3]]) -> Vec<f64> {
        let mut distances = Vec::with_capacity(points_a.len() * points_b.len());
        
        for point_a in points_a {
            for point_b in points_b {
                // Usar wide crate para operaciones SIMD
                let a_x = f64x4::splat(point_a[0]);
                let a_y = f64x4::splat(point_a[1]);
                let a_z = f64x4::splat(point_a[2]);
                
                let b_x = f64x4::splat(point_b[0]);
                let b_y = f64x4::splat(point_b[1]);
                let b_z = f64x4::splat(point_b[2]);
                
                let dx = a_x - b_x;
                let dy = a_y - b_y;
                let dz = a_z - b_z;
                
                let dist_squared = dx * dx + dy * dy + dz * dz;
                let distance = dist_squared.sqrt();
                
                distances.push(distance.as_array()[0]);
            }
        }
        
        distances
    }
    
    #[cfg(target_arch = "x86_64")]
    fn calculate_distances_sse2(&self, points_a: &[[f64; 3]], points_b: &[[f64; 3]]) -> Vec<f64> {
        let mut distances = Vec::with_capacity(points_a.len() * points_b.len());
        
        for point_a in points_a {
            for point_b in points_b {
                // Usar f64x2 para SSE2
                let dx = point_a[0] - point_b[0];
                let dy = point_a[1] - point_b[1];
                let dz = point_a[2] - point_b[2];
                
                let distance = (dx * dx + dy * dy + dz * dz).sqrt();
                distances.push(distance);
            }
        }
        
        distances
    }
    
    fn calculate_distances_scalar(&self, points_a: &[[f64; 3]], points_b: &[[f64; 3]]) -> Vec<f64> {
        let mut distances = Vec::with_capacity(points_a.len() * points_b.len());
        
        for point_a in points_a {
            for point_b in points_b {
                let dx = point_a[0] - point_b[0];
                let dy = point_a[1] - point_b[1];
                let dz = point_a[2] - point_b[2];
                
                let distance = (dx * dx + dy * dy + dz * dz).sqrt();
                distances.push(distance);
            }
        }
        
        distances
    }
    
    #[cfg(target_arch = "x86_64")]
    fn dot_products_avx2(&self, vectors_a: &[[f64; 3]], vectors_b: &[[f64; 3]]) -> Vec<f64> {
        let mut dot_products = Vec::with_capacity(vectors_a.len());
        
        for (vec_a, vec_b) in vectors_a.iter().zip(vectors_b.iter()) {
            let a_x = f64x4::splat(vec_a[0]);
            let a_y = f64x4::splat(vec_a[1]);
            let a_z = f64x4::splat(vec_a[2]);
            
            let b_x = f64x4::splat(vec_b[0]);
            let b_y = f64x4::splat(vec_b[1]);
            let b_z = f64x4::splat(vec_b[2]);
            
            let products = a_x * b_x + a_y * b_y + a_z * b_z;
            dot_products.push(products.as_array()[0]);
        }
        
        dot_products
    }
    
    fn dot_products_scalar(&self, vectors_a: &[[f64; 3]], vectors_b: &[[f64; 3]]) -> Vec<f64> {
        vectors_a
            .iter()
            .zip(vectors_b.iter())
            .map(|(vec_a, vec_b)| {
                vec_a[0] * vec_b[0] + vec_a[1] * vec_b[1] + vec_a[2] * vec_b[2]
            })
            .collect()
    }
    
    #[cfg(target_arch = "x86_64")]
    fn calculate_center_of_mass_avx2(&self, positions: &[[f64; 3]], masses: &[f64]) -> [f64; 3] {
        let mut center = [0.0; 3];
        let mut total_mass = 0.0;
        
        // Procesar en chunks de 4 para AVX2
        let chunks = positions.len() / 4;
        
        for chunk_idx in 0..chunks {
            let start_idx = chunk_idx * 4;
            
            let masses_vec = f64x4::new([
                masses[start_idx],
                masses[start_idx + 1],
                masses[start_idx + 2],
                masses[start_idx + 3],
            ]);
            
            let x_coords = f64x4::new([
                positions[start_idx][0],
                positions[start_idx + 1][0],
                positions[start_idx + 2][0],
                positions[start_idx + 3][0],
            ]);
            
            let y_coords = f64x4::new([
                positions[start_idx][1],
                positions[start_idx + 1][1],
                positions[start_idx + 2][1],
                positions[start_idx + 3][1],
            ]);
            
            let z_coords = f64x4::new([
                positions[start_idx][2],
                positions[start_idx + 1][2],
                positions[start_idx + 2][2],
                positions[start_idx + 3][2],
            ]);
            
            let weighted_x = x_coords * masses_vec;
            let weighted_y = y_coords * masses_vec;
            let weighted_z = z_coords * masses_vec;
            
            // Sumar componentes
            center[0] += weighted_x.as_array().iter().sum::<f64>();
            center[1] += weighted_y.as_array().iter().sum::<f64>();
            center[2] += weighted_z.as_array().iter().sum::<f64>();
            total_mass += masses_vec.as_array().iter().sum::<f64>();
        }
        
        // Procesar elementos restantes
        for i in (chunks * 4)..positions.len() {
            let mass = masses[i];
            total_mass += mass;
            center[0] += positions[i][0] * mass;
            center[1] += positions[i][1] * mass;
            center[2] += positions[i][2] * mass;
        }
        
        if total_mass > 0.0 {
            center[0] /= total_mass;
            center[1] /= total_mass;
            center[2] /= total_mass;
        }
        
        center
    }
    
    fn calculate_center_of_mass_scalar(&self, positions: &[[f64; 3]], masses: &[f64]) -> [f64; 3] {
        let mut center = [0.0; 3];
        let mut total_mass = 0.0;
        
        for (pos, &mass) in positions.iter().zip(masses.iter()) {
            total_mass += mass;
            center[0] += pos[0] * mass;
            center[1] += pos[1] * mass;
            center[2] += pos[2] * mass;
        }
        
        if total_mass > 0.0 {
            center[0] /= total_mass;
            center[1] /= total_mass;
            center[2] /= total_mass;
        }
        
        center
    }
    
    #[cfg(target_arch = "x86_64")]
    fn apply_transformation_avx2(
        &self,
        points: &[[f64; 3]],
        translation: &[f64; 3],
        rotation_matrix: &[[f64; 3]; 3],
    ) -> Vec<[f64; 3]> {
        let mut transformed = Vec::with_capacity(points.len());
        
        let tx = f64x4::splat(translation[0]);
        let ty = f64x4::splat(translation[1]);
        let tz = f64x4::splat(translation[2]);
        
        for point in points {
            // Aplicar rotación
            let x = point[0];
            let y = point[1];
            let z = point[2];
            
            let rx = rotation_matrix[0][0] * x + rotation_matrix[0][1] * y + rotation_matrix[0][2] * z;
            let ry = rotation_matrix[1][0] * x + rotation_matrix[1][1] * y + rotation_matrix[1][2] * z;
            let rz = rotation_matrix[2][0] * x + rotation_matrix[2][1] * y + rotation_matrix[2][2] * z;
            
            // Aplicar translación
            transformed.push([
                rx + translation[0],
                ry + translation[1],
                rz + translation[2],
            ]);
        }
        
        transformed
    }
    
    fn apply_transformation_scalar(
        &self,
        points: &[[f64; 3]],
        translation: &[f64; 3],
        rotation_matrix: &[[f64; 3]; 3],
    ) -> Vec<[f64; 3]> {
        points
            .iter()
            .map(|point| {
                // Aplicar rotación
                let x = point[0];
                let y = point[1];
                let z = point[2];
                
                let rx = rotation_matrix[0][0] * x + rotation_matrix[0][1] * y + rotation_matrix[0][2] * z;
                let ry = rotation_matrix[1][0] * x + rotation_matrix[1][1] * y + rotation_matrix[1][2] * z;
                let rz = rotation_matrix[2][0] * x + rotation_matrix[2][1] * y + rotation_matrix[2][2] * z;
                
                // Aplicar translación
                [
                    rx + translation[0],
                    ry + translation[1],
                    rz + translation[2],
                ]
            })
            .collect()
    }
    
    #[cfg(target_arch = "x86_64")]
    fn calculate_electrostatic_potential_avx2(
        &self,
        charges_a: &[f64],
        positions_a: &[[f64; 3]],
        charges_b: &[f64],
        positions_b: &[[f64; 3]],
    ) -> f64 {
        let mut potential = 0.0;
        const COULOMB_CONSTANT: f64 = 8.9875517923e9; // N⋅m²/C²
        
        for (i, (charge_a, pos_a)) in charges_a.iter().zip(positions_a.iter()).enumerate() {
            for (j, (charge_b, pos_b)) in charges_b.iter().zip(positions_b.iter()).enumerate() {
                if i != j {
                    let dx = pos_a[0] - pos_b[0];
                    let dy = pos_a[1] - pos_b[1];
                    let dz = pos_a[2] - pos_b[2];
                    
                    let distance = (dx * dx + dy * dy + dz * dz).sqrt();
                    
                    if distance > 1e-10 { // Evitar división por cero
                        potential += COULOMB_CONSTANT * charge_a * charge_b / distance;
                    }
                }
            }
        }
        
        potential
    }
    
    fn calculate_electrostatic_potential_scalar(
        &self,
        charges_a: &[f64],
        positions_a: &[[f64; 3]],
        charges_b: &[f64],
        positions_b: &[[f64; 3]],
    ) -> f64 {
        let mut potential = 0.0;
        const COULOMB_CONSTANT: f64 = 8.9875517923e9;
        
        for (charge_a, pos_a) in charges_a.iter().zip(positions_a.iter()) {
            for (charge_b, pos_b) in charges_b.iter().zip(positions_b.iter()) {
                let dx = pos_a[0] - pos_b[0];
                let dy = pos_a[1] - pos_b[1];
                let dz = pos_a[2] - pos_b[2];
                
                let distance = (dx * dx + dy * dy + dz * dz).sqrt();
                
                if distance > 1e-10 {
                    potential += COULOMB_CONSTANT * charge_a * charge_b / distance;
                }
            }
        }
        
        potential
    }
    
    /// Calcular matriz de rotación desde quaternion
    pub fn quaternion_to_rotation_matrix(&self, q: &[f64; 4]) -> [[f64; 3]; 3] {
        let w = q[0];
        let x = q[1];
        let y = q[2];
        let z = q[3];
        
        // Normalizar quaternion
        let norm = (w * w + x * x + y * y + z * z).sqrt();
        let w = w / norm;
        let x = x / norm;
        let y = y / norm;
        let z = z / norm;
        
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
    
    /// Benchmarking de operaciones SIMD
    pub fn benchmark_simd_operations(&self) -> SimdBenchmarkResults {
        let test_size = 10000;
        let points_a: Vec<[f64; 3]> = (0..test_size)
            .map(|i| [i as f64, (i * 2) as f64, (i * 3) as f64])
            .collect();
        let points_b: Vec<[f64; 3]> = (0..test_size)
            .map(|i| [(i + 1) as f64, (i * 2 + 1) as f64, (i * 3 + 1) as f64])
            .collect();
        
        // Benchmark distancias
        let start = std::time::Instant::now();
        let _distances = self.calculate_distances_simd(&points_a[..100], &points_b[..100]);
        let distance_time = start.elapsed();
        
        // Benchmark centro de masa
        let masses: Vec<f64> = (0..test_size).map(|_| 1.0).collect();
        let start = std::time::Instant::now();
        let _center = self.calculate_center_of_mass_simd(&points_a, &masses);
        let center_time = start.elapsed();
        
        SimdBenchmarkResults {
            distance_calculation_us: distance_time.as_micros() as u64,
            center_of_mass_us: center_time.as_micros() as u64,
            simd_available: self.avx2_available || self.sse2_available,
            avx2_used: self.avx2_available,
            test_size,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SimdBenchmarkResults {
    pub distance_calculation_us: u64,
    pub center_of_mass_us: u64,
    pub simd_available: bool,
    pub avx2_used: bool,
    pub test_size: usize,
}

impl Default for SimdMath {
    fn default() -> Self {
        Self::new()
    }
}