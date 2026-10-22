// Memory Pool - Sistema de pooling de memoria ultra-optimizado para proteínas
// Zero-allocation durante runtime, pre-allocated pools

use anyhow::{Result, anyhow};
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use tracing::{info, debug, warn};

/// Pool de memoria especializado para estructuras de proteínas
pub struct ProteinMemoryPool {
    atom_pools: Vec<Mutex<VecDeque<Vec<AtomData>>>>,
    coordinate_pools: Vec<Mutex<VecDeque<Vec<[f64; 3]>>>>,
    distance_matrix_pools: Vec<Mutex<VecDeque<Vec<f64>>>>,
    transformation_pools: Vec<Mutex<VecDeque<TransformationBuffer>>>,
    scoring_result_pools: Vec<Mutex<VecDeque<ScoringResultBuffer>>>,
    pool_stats: Arc<Mutex<PoolStatistics>>,
}

/// Datos de átomo optimizados para cálculos
#[derive(Debug, Clone)]
pub struct AtomData {
    pub position: [f64; 3],
    pub charge: f64,
    pub radius: f64,
    pub mass: f64,
    pub element_type: u8, // Índice en lugar de string
}

/// Buffer para transformaciones 3D
#[derive(Debug, Clone)]
pub struct TransformationBuffer {
    pub positions_original: Vec<[f64; 3]>,
    pub positions_transformed: Vec<[f64; 3]>,
    pub rotation_matrix: [[f64; 3]; 3],
    pub translation: [f64; 3],
    pub quaternion: [f64; 4],
}

/// Buffer para resultados de scoring
#[derive(Debug, Clone)]
pub struct ScoringResultBuffer {
    pub energy_components: Vec<f64>,
    pub pairwise_energies: Vec<f64>,
    pub gradients: Vec<[f64; 3]>,
    pub hessian: Vec<f64>,
}

/// Estadísticas del pool de memoria
#[derive(Debug, Clone)]
pub struct PoolStatistics {
    pub total_allocations: u64,
    pub total_deallocations: u64,
    pub current_usage_bytes: u64,
    pub peak_usage_bytes: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub pool_efficiency_percent: f64,
}

/// Handle para buffer prestado del pool
pub struct PooledBuffer<T> {
    buffer: Option<T>,
    pool_index: usize,
    return_fn: Box<dyn FnOnce(T, usize) + Send>,
}

impl ProteinMemoryPool {
    /// Crear nuevo pool con tamaños optimizados
    pub fn new(max_protein_size: usize) -> Arc<Self> {
        info!("🧠 Initializing ProteinMemoryPool for proteins up to {} atoms", max_protein_size);
        
        // Crear pools para diferentes tamaños
        let size_categories = vec![100, 500, 1000, 5000, 10000, 50000];
        let mut atom_pools = Vec::new();
        let mut coordinate_pools = Vec::new();
        let mut distance_matrix_pools = Vec::new();
        let mut transformation_pools = Vec::new();
        let mut scoring_result_pools = Vec::new();
        
        for &size in &size_categories {
            if size <= max_protein_size {
                // Pre-alocar buffers para evitar allocations en runtime
                let mut atom_pool = VecDeque::new();
                let mut coord_pool = VecDeque::new();
                let mut dist_pool = VecDeque::new();
                let mut transform_pool = VecDeque::new();
                let mut scoring_pool = VecDeque::new();
                
                // Pre-alocar algunos buffers de cada tipo
                for _ in 0..10 {
                    atom_pool.push_back(Vec::with_capacity(size));
                    coord_pool.push_back(Vec::with_capacity(size));
                    dist_pool.push_back(Vec::with_capacity(size * size));
                    
                    transform_pool.push_back(TransformationBuffer {
                        positions_original: Vec::with_capacity(size),
                        positions_transformed: Vec::with_capacity(size),
                        rotation_matrix: [[0.0; 3]; 3],
                        translation: [0.0; 3],
                        quaternion: [1.0, 0.0, 0.0, 0.0],
                    });
                    
                    scoring_pool.push_back(ScoringResultBuffer {
                        energy_components: Vec::with_capacity(10),
                        pairwise_energies: Vec::with_capacity(size * 100),
                        gradients: Vec::with_capacity(size),
                        hessian: Vec::with_capacity(size * size),
                    });
                }
                
                atom_pools.push(Mutex::new(atom_pool));
                coordinate_pools.push(Mutex::new(coord_pool));
                distance_matrix_pools.push(Mutex::new(dist_pool));
                transformation_pools.push(Mutex::new(transform_pool));
                scoring_result_pools.push(Mutex::new(scoring_pool));
            }
        }
        
        let pool = Arc::new(Self {
            atom_pools,
            coordinate_pools,
            distance_matrix_pools,
            transformation_pools,
            scoring_result_pools,
            pool_stats: Arc::new(Mutex::new(PoolStatistics {
                total_allocations: 0,
                total_deallocations: 0,
                current_usage_bytes: 0,
                peak_usage_bytes: 0,
                cache_hits: 0,
                cache_misses: 0,
                pool_efficiency_percent: 100.0,
            })),
        });
        
        info!("✅ ProteinMemoryPool initialized with {} size categories", pool.atom_pools.len());
        pool
    }
    
    /// Obtener buffer de átomos del pool
    pub fn get_atom_buffer(&self, required_size: usize) -> Result<PooledBuffer<Vec<AtomData>>> {
        let pool_index = self.find_best_pool_index(required_size);
        
        let mut buffer = {
            let mut pool = self.atom_pools[pool_index].lock().unwrap();
            if let Some(mut buf) = pool.pop_front() {
                buf.clear();
                buf.reserve(required_size.saturating_sub(buf.capacity()));
                self.update_stats_hit();
                buf
            } else {
                self.update_stats_miss();
                Vec::with_capacity(required_size)
            }
        };
        
        let pool_ref = Arc::downgrade(&Arc::new(self));
        Ok(PooledBuffer {
            buffer: Some(buffer),
            pool_index,
            return_fn: Box::new(move |buf, idx| {
                if let Some(pool) = pool_ref.upgrade() {
                    pool.return_atom_buffer(buf, idx);
                }
            }),
        })
    }
    
    /// Obtener buffer de coordenadas del pool
    pub fn get_coordinate_buffer(&self, required_size: usize) -> Result<PooledBuffer<Vec<[f64; 3]>>> {
        let pool_index = self.find_best_pool_index(required_size);
        
        let buffer = {
            let mut pool = self.coordinate_pools[pool_index].lock().unwrap();
            if let Some(mut buf) = pool.pop_front() {
                buf.clear();
                buf.reserve(required_size.saturating_sub(buf.capacity()));
                self.update_stats_hit();
                buf
            } else {
                self.update_stats_miss();
                Vec::with_capacity(required_size)
            }
        };
        
        let pool_ref = Arc::downgrade(&Arc::new(self));
        Ok(PooledBuffer {
            buffer: Some(buffer),
            pool_index,
            return_fn: Box::new(move |buf, idx| {
                if let Some(pool) = pool_ref.upgrade() {
                    pool.return_coordinate_buffer(buf, idx);
                }
            }),
        })
    }
    
    /// Obtener buffer de matriz de distancias
    pub fn get_distance_matrix_buffer(&self, required_size: usize) -> Result<PooledBuffer<Vec<f64>>> {
        let pool_index = self.find_best_pool_index((required_size as f64).sqrt() as usize);
        
        let buffer = {
            let mut pool = self.distance_matrix_pools[pool_index].lock().unwrap();
            if let Some(mut buf) = pool.pop_front() {
                buf.clear();
                buf.reserve(required_size.saturating_sub(buf.capacity()));
                self.update_stats_hit();
                buf
            } else {
                self.update_stats_miss();
                Vec::with_capacity(required_size)
            }
        };
        
        let pool_ref = Arc::downgrade(&Arc::new(self));
        Ok(PooledBuffer {
            buffer: Some(buffer),
            pool_index,
            return_fn: Box::new(move |buf, idx| {
                if let Some(pool) = pool_ref.upgrade() {
                    pool.return_distance_matrix_buffer(buf, idx);
                }
            }),
        })
    }
    
    /// Obtener buffer de transformación
    pub fn get_transformation_buffer(&self, required_size: usize) -> Result<PooledBuffer<TransformationBuffer>> {
        let pool_index = self.find_best_pool_index(required_size);
        
        let buffer = {
            let mut pool = self.transformation_pools[pool_index].lock().unwrap();
            if let Some(mut buf) = pool.pop_front() {
                buf.positions_original.clear();
                buf.positions_transformed.clear();
                buf.positions_original.reserve(required_size.saturating_sub(buf.positions_original.capacity()));
                buf.positions_transformed.reserve(required_size.saturating_sub(buf.positions_transformed.capacity()));
                self.update_stats_hit();
                buf
            } else {
                self.update_stats_miss();
                TransformationBuffer {
                    positions_original: Vec::with_capacity(required_size),
                    positions_transformed: Vec::with_capacity(required_size),
                    rotation_matrix: [[0.0; 3]; 3],
                    translation: [0.0; 3],
                    quaternion: [1.0, 0.0, 0.0, 0.0],
                }
            }
        };
        
        let pool_ref = Arc::downgrade(&Arc::new(self));
        Ok(PooledBuffer {
            buffer: Some(buffer),
            pool_index,
            return_fn: Box::new(move |buf, idx| {
                if let Some(pool) = pool_ref.upgrade() {
                    pool.return_transformation_buffer(buf, idx);
                }
            }),
        })
    }
    
    /// Obtener buffer de resultados de scoring
    pub fn get_scoring_buffer(&self, atom_count: usize) -> Result<PooledBuffer<ScoringResultBuffer>> {
        let pool_index = self.find_best_pool_index(atom_count);
        
        let buffer = {
            let mut pool = self.scoring_result_pools[pool_index].lock().unwrap();
            if let Some(mut buf) = pool.pop_front() {
                buf.energy_components.clear();
                buf.pairwise_energies.clear();
                buf.gradients.clear();
                buf.hessian.clear();
                self.update_stats_hit();
                buf
            } else {
                self.update_stats_miss();
                ScoringResultBuffer {
                    energy_components: Vec::with_capacity(10),
                    pairwise_energies: Vec::with_capacity(atom_count * 100),
                    gradients: Vec::with_capacity(atom_count),
                    hessian: Vec::with_capacity(atom_count * atom_count),
                }
            }
        };
        
        let pool_ref = Arc::downgrade(&Arc::new(self));
        Ok(PooledBuffer {
            buffer: Some(buffer),
            pool_index,
            return_fn: Box::new(move |buf, idx| {
                if let Some(pool) = pool_ref.upgrade() {
                    pool.return_scoring_buffer(buf, idx);
                }
            }),
        })
    }
    
    /// Encontrar el índice del pool más adecuado
    fn find_best_pool_index(&self, required_size: usize) -> usize {
        let size_thresholds = [100, 500, 1000, 5000, 10000, 50000];
        
        for (i, &threshold) in size_thresholds.iter().enumerate() {
            if required_size <= threshold && i < self.atom_pools.len() {
                return i;
            }
        }
        
        // Usar el pool más grande disponible
        self.atom_pools.len().saturating_sub(1)
    }
    
    /// Devolver buffer de átomos al pool
    fn return_atom_buffer(&self, buffer: Vec<AtomData>, pool_index: usize) {
        if pool_index < self.atom_pools.len() {
            let mut pool = self.atom_pools[pool_index].lock().unwrap();
            if pool.len() < 50 { // Limitar tamaño del pool
                pool.push_back(buffer);
            }
        }
        self.update_stats_return();
    }
    
    /// Devolver buffer de coordenadas al pool
    fn return_coordinate_buffer(&self, buffer: Vec<[f64; 3]>, pool_index: usize) {
        if pool_index < self.coordinate_pools.len() {
            let mut pool = self.coordinate_pools[pool_index].lock().unwrap();
            if pool.len() < 50 {
                pool.push_back(buffer);
            }
        }
        self.update_stats_return();
    }
    
    /// Devolver buffer de matriz de distancias al pool
    fn return_distance_matrix_buffer(&self, buffer: Vec<f64>, pool_index: usize) {
        if pool_index < self.distance_matrix_pools.len() {
            let mut pool = self.distance_matrix_pools[pool_index].lock().unwrap();
            if pool.len() < 20 { // Matrices son más grandes, menor límite
                pool.push_back(buffer);
            }
        }
        self.update_stats_return();
    }
    
    /// Devolver buffer de transformación al pool
    fn return_transformation_buffer(&self, buffer: TransformationBuffer, pool_index: usize) {
        if pool_index < self.transformation_pools.len() {
            let mut pool = self.transformation_pools[pool_index].lock().unwrap();
            if pool.len() < 30 {
                pool.push_back(buffer);
            }
        }
        self.update_stats_return();
    }
    
    /// Devolver buffer de scoring al pool
    fn return_scoring_buffer(&self, buffer: ScoringResultBuffer, pool_index: usize) {
        if pool_index < self.scoring_result_pools.len() {
            let mut pool = self.scoring_result_pools[pool_index].lock().unwrap();
            if pool.len() < 30 {
                pool.push_back(buffer);
            }
        }
        self.update_stats_return();
    }
    
    /// Actualizar estadísticas para cache hit
    fn update_stats_hit(&self) {
        let mut stats = self.pool_stats.lock().unwrap();
        stats.cache_hits += 1;
        stats.total_allocations += 1;
        self.update_efficiency(&mut stats);
    }
    
    /// Actualizar estadísticas para cache miss
    fn update_stats_miss(&self) {
        let mut stats = self.pool_stats.lock().unwrap();
        stats.cache_misses += 1;
        stats.total_allocations += 1;
        self.update_efficiency(&mut stats);
    }
    
    /// Actualizar estadísticas para devolución
    fn update_stats_return(&self) {
        let mut stats = self.pool_stats.lock().unwrap();
        stats.total_deallocations += 1;
    }
    
    /// Actualizar eficiencia del pool
    fn update_efficiency(&self, stats: &mut PoolStatistics) {
        let total_requests = stats.cache_hits + stats.cache_misses;
        if total_requests > 0 {
            stats.pool_efficiency_percent = (stats.cache_hits as f64 / total_requests as f64) * 100.0;
        }
    }
    
    /// Obtener estadísticas actuales del pool
    pub fn get_statistics(&self) -> PoolStatistics {
        self.pool_stats.lock().unwrap().clone()
    }
    
    /// Limpiar pools (para testing)
    pub fn clear_pools(&self) {
        for pool in &self.atom_pools {
            pool.lock().unwrap().clear();
        }
        for pool in &self.coordinate_pools {
            pool.lock().unwrap().clear();
        }
        for pool in &self.distance_matrix_pools {
            pool.lock().unwrap().clear();
        }
        for pool in &self.transformation_pools {
            pool.lock().unwrap().clear();
        }
        for pool in &self.scoring_result_pools {
            pool.lock().unwrap().clear();
        }
        
        let mut stats = self.pool_stats.lock().unwrap();
        *stats = PoolStatistics {
            total_allocations: 0,
            total_deallocations: 0,
            current_usage_bytes: 0,
            peak_usage_bytes: 0,
            cache_hits: 0,
            cache_misses: 0,
            pool_efficiency_percent: 100.0,
        };
    }
    
    /// Obtener información de uso de memoria
    pub fn get_memory_usage(&self) -> MemoryUsageInfo {
        let mut total_buffers = 0;
        let mut estimated_memory = 0;
        
        for pool in &self.atom_pools {
            let count = pool.lock().unwrap().len();
            total_buffers += count;
            estimated_memory += count * std::mem::size_of::<AtomData>() * 1000; // Estimate
        }
        
        for pool in &self.coordinate_pools {
            let count = pool.lock().unwrap().len();
            total_buffers += count;
            estimated_memory += count * std::mem::size_of::<[f64; 3]>() * 1000;
        }
        
        for pool in &self.distance_matrix_pools {
            let count = pool.lock().unwrap().len();
            total_buffers += count;
            estimated_memory += count * std::mem::size_of::<f64>() * 10000; // Matrix
        }
        
        MemoryUsageInfo {
            total_buffers_pooled: total_buffers,
            estimated_memory_bytes: estimated_memory,
            pool_categories: self.atom_pools.len(),
        }
    }
}

/// Información de uso de memoria
#[derive(Debug, Clone)]
pub struct MemoryUsageInfo {
    pub total_buffers_pooled: usize,
    pub estimated_memory_bytes: usize,
    pub pool_categories: usize,
}

impl<T> PooledBuffer<T> {
    /// Obtener referencia al buffer
    pub fn get(&self) -> &T {
        self.buffer.as_ref().unwrap()
    }
    
    /// Obtener referencia mutable al buffer
    pub fn get_mut(&mut self) -> &mut T {
        self.buffer.as_mut().unwrap()
    }
    
    /// Tomar ownership del buffer (consume el PooledBuffer)
    pub fn take(mut self) -> T {
        self.buffer.take().unwrap()
    }
}

impl<T> Drop for PooledBuffer<T> {
    fn drop(&mut self) {
        if let Some(buffer) = self.buffer.take() {
            (self.return_fn)(buffer, self.pool_index);
        }
    }
}

/// Macro para uso conveniente del memory pool
#[macro_export]
macro_rules! with_pooled_buffer {
    ($pool:expr, $buffer_type:ident, $size:expr, $buffer_name:ident, $body:block) => {
        {
            let mut $buffer_name = $pool.$buffer_type($size)?;
            $body
        }
    };
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_memory_pool_basic() {
        let pool = ProteinMemoryPool::new(1000);
        
        // Test atom buffer
        {
            let buffer = pool.get_atom_buffer(100).unwrap();
            assert!(buffer.get().capacity() >= 100);
        } // Buffer returned automatically
        
        // Test coordinate buffer
        {
            let buffer = pool.get_coordinate_buffer(200).unwrap();
            assert!(buffer.get().capacity() >= 200);
        }
        
        let stats = pool.get_statistics();
        assert!(stats.total_allocations >= 2);
    }
    
    #[test]
    fn test_pool_efficiency() {
        let pool = ProteinMemoryPool::new(1000);
        
        // Primer uso debería ser cache miss
        {
            let _buffer = pool.get_atom_buffer(100).unwrap();
        }
        
        // Segundo uso debería ser cache hit
        {
            let _buffer = pool.get_atom_buffer(100).unwrap();
        }
        
        let stats = pool.get_statistics();
        assert!(stats.cache_hits >= 1);
        assert!(stats.cache_misses >= 1);
        assert!(stats.pool_efficiency_percent > 0.0);
    }
}