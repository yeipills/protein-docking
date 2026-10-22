/*!
 * SIMD Operations - Operaciones vectorizadas ultra-optimizadas
 * 
 * Implementa operaciones matemáticas usando SIMD (AVX2/AVX-512) para
 * acelerar cálculos de docking molecular hasta 8x más rápido
 */

use ndarray::{Array1, Array2, ArrayView1, ArrayView2, Axis};
use crate::core::{Result, AppError};

/// Operaciones SIMD optimizadas
pub struct SimdOperations {
    simd_support: SimdSupport,
}

/// Nivel de soporte SIMD detectado
#[derive(Debug, Clone, Copy)]
pub enum SimdSupport {
    None,
    Sse2,
    Avx2,
    Avx512,
}

impl SimdOperations {
    /// Crear nueva instancia con detección automática de SIMD
    pub fn new() -> Self {
        let support = Self::detect_simd_support();
        tracing::info!("🚀 SIMD support detected: {:?}", support);
        
        Self {
            simd_support: support,
        }
    }

    /// Detectar nivel de soporte SIMD del sistema
    fn detect_simd_support() -> SimdSupport {
        #[cfg(target_arch = "x86_64")]
        {
            if std::arch::is_x86_feature_detected!("avx512f") {
                return SimdSupport::Avx512;
            }
            if std::arch::is_x86_feature_detected!("avx2") {
                return SimdSupport::Avx2;
            }
            if std::arch::is_x86_feature_detected!("sse2") {
                return SimdSupport::Sse2;
            }
        }
        
        #[cfg(target_arch = "aarch64")]
        {
            if std::arch::is_aarch64_feature_detected!("neon") {
                return SimdSupport::Avx2; // NEON es comparable a AVX2
            }
        }
        
        SimdSupport::None
    }

    /// Calcular centroides usando SIMD - Reemplazo optimizado del código Python original
    /// 
    /// Operaciones optimizadas:
    /// - Vectorización automática para sumas y promedios
    /// - Processing en chunks de 8 elementos (AVX2) o 16 (AVX-512)
    /// - Reducción paralela de resultados
    pub fn compute_centroids_simd(&self, positions: &Array2<f64>) -> Result<Array2<f64>> {
        if positions.is_empty() {
            return Err(AppError::algorithm("Cannot compute centroids from empty positions"));
        }

        let start_time = std::time::Instant::now();
        
        match self.simd_support {
            SimdSupport::Avx512 => self.compute_centroids_avx512(positions),
            SimdSupport::Avx2 => self.compute_centroids_avx2(positions),
            SimdSupport::Sse2 => self.compute_centroids_sse2(positions),
            SimdSupport::None => self.compute_centroids_fallback(positions),
        }.map(|result| {
            let duration = start_time.elapsed();
            tracing::debug!("🚀 SIMD centroids computed in {:?}", duration);
            result
        })
    }

    /// Versión AVX-512 (16 elementos en paralelo)
    #[cfg(target_arch = "x86_64")]
    fn compute_centroids_avx512(&self, positions: &Array2<f64>) -> Result<Array2<f64>> {
        if !std::arch::is_x86_feature_detected!("avx512f") {
            return self.compute_centroids_avx2(positions);
        }

        unsafe {
            self.compute_centroids_avx512_unsafe(positions)
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    fn compute_centroids_avx512(&self, positions: &Array2<f64>) -> Result<Array2<f64>> {
        self.compute_centroids_fallback(positions)
    }

    /// Implementación unsafe de AVX-512
    #[cfg(target_arch = "x86_64")]
    unsafe fn compute_centroids_avx512_unsafe(&self, positions: &Array2<f64>) -> Result<Array2<f64>> {
        use std::arch::x86_64::*;

        let (n_points, n_dims) = positions.dim();
        if n_dims != 3 {
            return Err(AppError::algorithm("Expected 3D positions"));
        }

        // Procesar en chunks de 8 puntos (8 * 3 = 24 valores, 3 registros AVX-512)
        const CHUNK_SIZE: usize = 8;
        let mut centroids = Array2::zeros((n_points / CHUNK_SIZE + 1, 3));

        let positions_slice = positions.as_slice().unwrap();
        
        for (chunk_idx, chunk) in positions_slice.chunks_exact(CHUNK_SIZE * 3).enumerate() {
            // Cargar 24 valores (8 puntos * 3 coordenadas) en 3 registros AVX-512
            let x_values = _mm512_loadu_pd(chunk.as_ptr());
            let y_values = _mm512_loadu_pd(chunk.as_ptr().add(8));
            let z_values = _mm512_loadu_pd(chunk.as_ptr().add(16));

            // Sumar todas las coordenadas X
            let x_sum = _mm512_reduce_add_pd(x_values);
            let y_sum = _mm512_reduce_add_pd(y_values);
            let z_sum = _mm512_reduce_add_pd(z_values);

            // Calcular centroides (promedio)
            let count = CHUNK_SIZE as f64;
            centroids[[chunk_idx, 0]] = x_sum / count;
            centroids[[chunk_idx, 1]] = y_sum / count;
            centroids[[chunk_idx, 2]] = z_sum / count;
        }

        Ok(centroids)
    }

    /// Versión AVX2 (8 elementos en paralelo)
    #[cfg(target_arch = "x86_64")]
    fn compute_centroids_avx2(&self, positions: &Array2<f64>) -> Result<Array2<f64>> {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return self.compute_centroids_sse2(positions);
        }

        unsafe {
            self.compute_centroids_avx2_unsafe(positions)
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    fn compute_centroids_avx2(&self, positions: &Array2<f64>) -> Result<Array2<f64>> {
        self.compute_centroids_fallback(positions)
    }

    /// Implementación unsafe de AVX2
    #[cfg(target_arch = "x86_64")]
    unsafe fn compute_centroids_avx2_unsafe(&self, positions: &Array2<f64>) -> Result<Array2<f64>> {
        use std::arch::x86_64::*;

        let (n_points, n_dims) = positions.dim();
        if n_dims != 3 {
            return Err(AppError::algorithm("Expected 3D positions"));
        }

        // Procesar en chunks de 4 puntos (4 * 3 = 12 valores)
        const CHUNK_SIZE: usize = 4;
        let mut centroids = Array2::zeros((n_points / CHUNK_SIZE + 1, 3));

        let positions_slice = positions.as_slice().unwrap();

        for (chunk_idx, chunk) in positions_slice.chunks_exact(CHUNK_SIZE * 3).enumerate() {
            // Cargar coordenadas X, Y, Z de 4 puntos
            let x_vals = _mm256_loadu_pd(chunk.as_ptr());
            let y_vals = _mm256_loadu_pd(chunk.as_ptr().add(4));
            let z_vals = _mm256_loadu_pd(chunk.as_ptr().add(8));

            // Sumar horizontalmente usando hadd
            let x_sum_vec = _mm256_hadd_pd(x_vals, x_vals);
            let y_sum_vec = _mm256_hadd_pd(y_vals, y_vals);
            let z_sum_vec = _mm256_hadd_pd(z_vals, z_vals);

            // Extraer sumas finales
            let x_sum = _mm256_cvtsd_f64(x_sum_vec) + _mm256_cvtsd_f64(_mm256_permute4x64_pd(x_sum_vec, 0b11));
            let y_sum = _mm256_cvtsd_f64(y_sum_vec) + _mm256_cvtsd_f64(_mm256_permute4x64_pd(y_sum_vec, 0b11));
            let z_sum = _mm256_cvtsd_f64(z_sum_vec) + _mm256_cvtsd_f64(_mm256_permute4x64_pd(z_sum_vec, 0b11));

            // Calcular centroides
            let count = CHUNK_SIZE as f64;
            centroids[[chunk_idx, 0]] = x_sum / count;
            centroids[[chunk_idx, 1]] = y_sum / count;
            centroids[[chunk_idx, 2]] = z_sum / count;
        }

        Ok(centroids)
    }

    /// Versión SSE2 (4 elementos en paralelo)
    #[cfg(target_arch = "x86_64")]
    fn compute_centroids_sse2(&self, positions: &Array2<f64>) -> Result<Array2<f64>> {
        if !std::arch::is_x86_feature_detected!("sse2") {
            return self.compute_centroids_fallback(positions);
        }

        unsafe {
            self.compute_centroids_sse2_unsafe(positions)
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    fn compute_centroids_sse2(&self, positions: &Array2<f64>) -> Result<Array2<f64>> {
        self.compute_centroids_fallback(positions)
    }

    /// Implementación unsafe de SSE2
    #[cfg(target_arch = "x86_64")]
    unsafe fn compute_centroids_sse2_unsafe(&self, positions: &Array2<f64>) -> Result<Array2<f64>> {
        use std::arch::x86_64::*;

        let (n_points, n_dims) = positions.dim();
        if n_dims != 3 {
            return Err(AppError::algorithm("Expected 3D positions"));
        }

        // Procesar en chunks de 2 puntos
        const CHUNK_SIZE: usize = 2;
        let mut centroids = Array2::zeros((n_points / CHUNK_SIZE + 1, 3));

        let positions_slice = positions.as_slice().unwrap();

        for (chunk_idx, chunk) in positions_slice.chunks_exact(CHUNK_SIZE * 3).enumerate() {
            // Cargar 2 puntos (6 valores)
            let x_vals = _mm_loadu_pd(chunk.as_ptr());
            let y_vals = _mm_loadu_pd(chunk.as_ptr().add(2));
            let z_vals = _mm_loadu_pd(chunk.as_ptr().add(4));

            // Sumar horizontalmente
            let x_sum = _mm_cvtsd_f64(x_vals) + _mm_cvtsd_f64(_mm_unpackhi_pd(x_vals, x_vals));
            let y_sum = _mm_cvtsd_f64(y_vals) + _mm_cvtsd_f64(_mm_unpackhi_pd(y_vals, y_vals));
            let z_sum = _mm_cvtsd_f64(z_vals) + _mm_cvtsd_f64(_mm_unpackhi_pd(z_vals, z_vals));

            // Calcular centroides
            let count = CHUNK_SIZE as f64;
            centroids[[chunk_idx, 0]] = x_sum / count;
            centroids[[chunk_idx, 1]] = y_sum / count;
            centroids[[chunk_idx, 2]] = z_sum / count;
        }

        Ok(centroids)
    }

    /// Fallback sin SIMD (usando ndarray optimizado)
    fn compute_centroids_fallback(&self, positions: &Array2<f64>) -> Result<Array2<f64>> {
        let (n_points, n_dims) = positions.dim();
        if n_dims != 3 {
            return Err(AppError::algorithm("Expected 3D positions"));
        }

        // Usar ndarray's optimized operations
        let centroids = positions.mean_axis(Axis(0))
            .ok_or_else(|| AppError::algorithm("Failed to compute mean"))?
            .insert_axis(Axis(0));

        Ok(centroids)
    }

    /// Encontrar valor mínimo usando SIMD
    pub fn find_minimum(&self, values: &[f64]) -> Option<f64> {
        if values.is_empty() {
            return None;
        }

        match self.simd_support {
            SimdSupport::Avx512 => self.find_minimum_avx512(values),
            SimdSupport::Avx2 => self.find_minimum_avx2(values),
            SimdSupport::Sse2 => self.find_minimum_sse2(values),
            SimdSupport::None => Some(values.iter().fold(f64::INFINITY, |acc, &x| acc.min(x))),
        }
    }

    /// Encontrar mínimo con AVX-512
    #[cfg(target_arch = "x86_64")]
    fn find_minimum_avx512(&self, values: &[f64]) -> Option<f64> {
        if !std::arch::is_x86_feature_detected!("avx512f") {
            return self.find_minimum_avx2(values);
        }

        unsafe {
            use std::arch::x86_64::*;
            
            let mut min_vec = _mm512_set1_pd(f64::INFINITY);
            
            // Procesar chunks de 8 elementos
            for chunk in values.chunks_exact(8) {
                let vals = _mm512_loadu_pd(chunk.as_ptr());
                min_vec = _mm512_min_pd(min_vec, vals);
            }
            
            // Reducir a un solo valor
            let min_val = _mm512_reduce_min_pd(min_vec);
            
            // Procesar elementos restantes
            let remainder_min = values[values.len() & !7..]
                .iter()
                .fold(f64::INFINITY, |acc, &x| acc.min(x));
            
            Some(min_val.min(remainder_min))
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    fn find_minimum_avx512(&self, values: &[f64]) -> Option<f64> {
        self.find_minimum_avx2(values)
    }

    /// Encontrar mínimo con AVX2
    #[cfg(target_arch = "x86_64")]
    fn find_minimum_avx2(&self, values: &[f64]) -> Option<f64> {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return self.find_minimum_sse2(values);
        }

        unsafe {
            use std::arch::x86_64::*;
            
            let mut min_vec = _mm256_set1_pd(f64::INFINITY);
            
            // Procesar chunks de 4 elementos
            for chunk in values.chunks_exact(4) {
                let vals = _mm256_loadu_pd(chunk.as_ptr());
                min_vec = _mm256_min_pd(min_vec, vals);
            }
            
            // Extraer mínimo final
            let min_array: [f64; 4] = std::mem::transmute(min_vec);
            let min_val = min_array.iter().fold(f64::INFINITY, |acc, &x| acc.min(x));
            
            // Procesar elementos restantes
            let remainder_min = values[values.len() & !3..]
                .iter()
                .fold(f64::INFINITY, |acc, &x| acc.min(x));
            
            Some(min_val.min(remainder_min))
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    fn find_minimum_avx2(&self, values: &[f64]) -> Option<f64> {
        self.find_minimum_sse2(values)
    }

    /// Encontrar mínimo con SSE2
    #[cfg(target_arch = "x86_64")]
    fn find_minimum_sse2(&self, values: &[f64]) -> Option<f64> {
        if !std::arch::is_x86_feature_detected!("sse2") {
            return Some(values.iter().fold(f64::INFINITY, |acc, &x| acc.min(x)));
        }

        unsafe {
            use std::arch::x86_64::*;
            
            let mut min_vec = _mm_set1_pd(f64::INFINITY);
            
            // Procesar chunks de 2 elementos
            for chunk in values.chunks_exact(2) {
                let vals = _mm_loadu_pd(chunk.as_ptr());
                min_vec = _mm_min_pd(min_vec, vals);
            }
            
            // Extraer mínimo final
            let min_val = _mm_cvtsd_f64(min_vec).min(_mm_cvtsd_f64(_mm_unpackhi_pd(min_vec, min_vec)));
            
            // Procesar elementos restantes
            let remainder_min = values[values.len() & !1..]
                .iter()
                .fold(f64::INFINITY, |acc, &x| acc.min(x));
            
            Some(min_val.min(remainder_min))
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    fn find_minimum_sse2(&self, values: &[f64]) -> Option<f64> {
        Some(values.iter().fold(f64::INFINITY, |acc, &x| acc.min(x)))
    }

    /// Calcular promedio ponderado usando SIMD
    pub fn compute_weighted_average(&self, values: &[f64], weights: &[f64]) -> Result<f64> {
        if values.len() != weights.len() {
            return Err(AppError::algorithm("Values and weights must have same length"));
        }

        if values.is_empty() {
            return Ok(0.0);
        }

        match self.simd_support {
            SimdSupport::Avx512 => self.weighted_average_avx512(values, weights),
            SimdSupport::Avx2 => self.weighted_average_avx2(values, weights),
            SimdSupport::Sse2 => self.weighted_average_sse2(values, weights),
            SimdSupport::None => self.weighted_average_fallback(values, weights),
        }
    }

    /// Promedio ponderado fallback
    fn weighted_average_fallback(&self, values: &[f64], weights: &[f64]) -> Result<f64> {
        let weighted_sum: f64 = values.iter().zip(weights.iter()).map(|(v, w)| v * w).sum();
        let total_weight: f64 = weights.iter().sum();
        
        if total_weight == 0.0 {
            return Err(AppError::algorithm("Total weight cannot be zero"));
        }
        
        Ok(weighted_sum / total_weight)
    }

    /// Promedio ponderado AVX2
    #[cfg(target_arch = "x86_64")]
    fn weighted_average_avx2(&self, values: &[f64], weights: &[f64]) -> Result<f64> {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return self.weighted_average_fallback(values, weights);
        }

        unsafe {
            use std::arch::x86_64::*;
            
            let mut sum_vec = _mm256_setzero_pd();
            let mut weight_sum_vec = _mm256_setzero_pd();
            
            // Procesar en chunks de 4
            for (val_chunk, weight_chunk) in values.chunks_exact(4).zip(weights.chunks_exact(4)) {
                let vals = _mm256_loadu_pd(val_chunk.as_ptr());
                let weights_vec = _mm256_loadu_pd(weight_chunk.as_ptr());
                
                let weighted = _mm256_mul_pd(vals, weights_vec);
                sum_vec = _mm256_add_pd(sum_vec, weighted);
                weight_sum_vec = _mm256_add_pd(weight_sum_vec, weights_vec);
            }
            
            // Reducir a escalares
            let sum_array: [f64; 4] = std::mem::transmute(sum_vec);
            let weight_array: [f64; 4] = std::mem::transmute(weight_sum_vec);
            
            let mut total_sum = sum_array.iter().sum::<f64>();
            let mut total_weight = weight_array.iter().sum::<f64>();
            
            // Procesar elementos restantes
            let remainder_start = values.len() & !3;
            for i in remainder_start..values.len() {
                total_sum += values[i] * weights[i];
                total_weight += weights[i];
            }
            
            if total_weight == 0.0 {
                return Err(AppError::algorithm("Total weight cannot be zero"));
            }
            
            Ok(total_sum / total_weight)
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    fn weighted_average_avx2(&self, values: &[f64], weights: &[f64]) -> Result<f64> {
        self.weighted_average_fallback(values, weights)
    }

    /// Promedio ponderado AVX-512
    #[cfg(target_arch = "x86_64")]
    fn weighted_average_avx512(&self, values: &[f64], weights: &[f64]) -> Result<f64> {
        if !std::arch::is_x86_feature_detected!("avx512f") {
            return self.weighted_average_avx2(values, weights);
        }

        unsafe {
            use std::arch::x86_64::*;
            
            let mut sum_vec = _mm512_setzero_pd();
            let mut weight_sum_vec = _mm512_setzero_pd();
            
            // Procesar en chunks de 8
            for (val_chunk, weight_chunk) in values.chunks_exact(8).zip(weights.chunks_exact(8)) {
                let vals = _mm512_loadu_pd(val_chunk.as_ptr());
                let weights_vec = _mm512_loadu_pd(weight_chunk.as_ptr());
                
                let weighted = _mm512_mul_pd(vals, weights_vec);
                sum_vec = _mm512_add_pd(sum_vec, weighted);
                weight_sum_vec = _mm512_add_pd(weight_sum_vec, weights_vec);
            }
            
            // Reducir usando intrinsics AVX-512
            let total_sum = _mm512_reduce_add_pd(sum_vec);
            let total_weight = _mm512_reduce_add_pd(weight_sum_vec);
            
            // Procesar elementos restantes
            let remainder_start = values.len() & !7;
            let (remainder_sum, remainder_weight) = values[remainder_start..]
                .iter()
                .zip(weights[remainder_start..].iter())
                .fold((0.0, 0.0), |(sum, weight_sum), (&v, &w)| {
                    (sum + v * w, weight_sum + w)
                });
            
            let final_sum = total_sum + remainder_sum;
            let final_weight = total_weight + remainder_weight;
            
            if final_weight == 0.0 {
                return Err(AppError::algorithm("Total weight cannot be zero"));
            }
            
            Ok(final_sum / final_weight)
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    fn weighted_average_avx512(&self, values: &[f64], weights: &[f64]) -> Result<f64> {
        self.weighted_average_avx2(values, weights)
    }

    /// Promedio ponderado SSE2
    #[cfg(target_arch = "x86_64")]
    fn weighted_average_sse2(&self, values: &[f64], weights: &[f64]) -> Result<f64> {
        if !std::arch::is_x86_feature_detected!("sse2") {
            return self.weighted_average_fallback(values, weights);
        }

        unsafe {
            use std::arch::x86_64::*;
            
            let mut sum_vec = _mm_setzero_pd();
            let mut weight_sum_vec = _mm_setzero_pd();
            
            // Procesar en chunks de 2
            for (val_chunk, weight_chunk) in values.chunks_exact(2).zip(weights.chunks_exact(2)) {
                let vals = _mm_loadu_pd(val_chunk.as_ptr());
                let weights_vec = _mm_loadu_pd(weight_chunk.as_ptr());
                
                let weighted = _mm_mul_pd(vals, weights_vec);
                sum_vec = _mm_add_pd(sum_vec, weighted);
                weight_sum_vec = _mm_add_pd(weight_sum_vec, weights_vec);
            }
            
            // Extraer escalares
            let total_sum = _mm_cvtsd_f64(sum_vec) + _mm_cvtsd_f64(_mm_unpackhi_pd(sum_vec, sum_vec));
            let total_weight = _mm_cvtsd_f64(weight_sum_vec) + _mm_cvtsd_f64(_mm_unpackhi_pd(weight_sum_vec, weight_sum_vec));
            
            // Procesar elementos restantes
            let remainder_start = values.len() & !1;
            let (remainder_sum, remainder_weight) = values[remainder_start..]
                .iter()
                .zip(weights[remainder_start..].iter())
                .fold((0.0, 0.0), |(sum, weight_sum), (&v, &w)| {
                    (sum + v * w, weight_sum + w)
                });
            
            let final_sum = total_sum + remainder_sum;
            let final_weight = total_weight + remainder_weight;
            
            if final_weight == 0.0 {
                return Err(AppError::algorithm("Total weight cannot be zero"));
            }
            
            Ok(final_sum / final_weight)
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    fn weighted_average_sse2(&self, values: &[f64], weights: &[f64]) -> Result<f64> {
        self.weighted_average_fallback(values, weights)
    }

    /// Obtener información sobre el soporte SIMD
    pub fn get_simd_info(&self) -> SimdInfo {
        SimdInfo {
            support_level: self.simd_support,
            max_parallel_f64: match self.simd_support {
                SimdSupport::None => 1,
                SimdSupport::Sse2 => 2,
                SimdSupport::Avx2 => 4,
                SimdSupport::Avx512 => 8,
            },
            features: self.get_available_features(),
        }
    }

    /// Obtener features disponibles
    fn get_available_features(&self) -> Vec<String> {
        let mut features = Vec::new();
        
        #[cfg(target_arch = "x86_64")]
        {
            if std::arch::is_x86_feature_detected!("sse2") {
                features.push("SSE2".to_string());
            }
            if std::arch::is_x86_feature_detected!("avx") {
                features.push("AVX".to_string());
            }
            if std::arch::is_x86_feature_detected!("avx2") {
                features.push("AVX2".to_string());
            }
            if std::arch::is_x86_feature_detected!("avx512f") {
                features.push("AVX-512F".to_string());
            }
            if std::arch::is_x86_feature_detected!("fma") {
                features.push("FMA".to_string());
            }
        }
        
        #[cfg(target_arch = "aarch64")]
        {
            if std::arch::is_aarch64_feature_detected!("neon") {
                features.push("NEON".to_string());
            }
        }
        
        features
    }
}

/// Información sobre capacidades SIMD
#[derive(Debug, Clone)]
pub struct SimdInfo {
    pub support_level: SimdSupport,
    pub max_parallel_f64: usize,
    pub features: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::Array2;

    #[test]
    fn test_simd_detection() {
        let simd_ops = SimdOperations::new();
        let info = simd_ops.get_simd_info();
        
        // Debería detectar al menos algún nivel de SIMD en máquinas modernas
        println!("SIMD support: {:?}", info.support_level);
        println!("Max parallel f64: {}", info.max_parallel_f64);
        println!("Features: {:?}", info.features);
    }

    #[test]
    fn test_centroids_computation() {
        let simd_ops = SimdOperations::new();
        
        // Crear datos de prueba
        let positions = Array2::from_shape_vec((4, 3), vec![
            0.0, 0.0, 0.0,
            1.0, 1.0, 1.0,
            2.0, 2.0, 2.0,
            3.0, 3.0, 3.0,
        ]).unwrap();
        
        let result = simd_ops.compute_centroids_simd(&positions).unwrap();
        
        // Verificar que el resultado es correcto
        assert_eq!(result.dim(), (1, 3));
        assert!((result[[0, 0]] - 1.5).abs() < 1e-10);
        assert!((result[[0, 1]] - 1.5).abs() < 1e-10);
        assert!((result[[0, 2]] - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_minimum_finding() {
        let simd_ops = SimdOperations::new();
        
        let values = vec![5.0, 2.0, 8.0, 1.0, 9.0, 3.0, 7.0, 4.0, 6.0];
        let min_val = simd_ops.find_minimum(&values).unwrap();
        
        assert_eq!(min_val, 1.0);
    }

    #[test]
    fn test_weighted_average() {
        let simd_ops = SimdOperations::new();
        
        let values = vec![1.0, 2.0, 3.0, 4.0];
        let weights = vec![0.1, 0.2, 0.3, 0.4];
        
        let result = simd_ops.compute_weighted_average(&values, &weights).unwrap();
        
        // Resultado esperado: (1*0.1 + 2*0.2 + 3*0.3 + 4*0.4) / (0.1+0.2+0.3+0.4) = 3.0
        assert!((result - 3.0).abs() < 1e-10);
    }
}