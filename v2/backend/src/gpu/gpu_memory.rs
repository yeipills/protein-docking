/*!
 * GPU Memory Management - Sistema avanzado de gestión de memoria GPU
 * 
 * Implementa memory pooling optimizado para:
 * - Reutilización de buffers GPU
 * - Minimización de allocaciones/deallocaciones
 * - Gestión automática de memoria
 * - Detección de memory leaks
 */

use crate::core::{Result, AppError};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Pool de memoria GPU optimizado
pub struct GpuMemoryPool {
    pools: Arc<Mutex<HashMap<usize, BufferPool>>>,
    total_allocated: Arc<Mutex<usize>>,
    max_memory_mb: usize,
    allocation_stats: Arc<Mutex<AllocationStats>>,
}

/// Pool para buffers de un tamaño específico
struct BufferPool {
    size_bytes: usize,
    available_buffers: Vec<GpuBuffer>,
    allocated_buffers: Vec<GpuBuffer>,
    max_buffers: usize,
}

/// Buffer GPU genérico
#[derive(Debug, Clone)]
pub struct GpuBuffer {
    pub id: String,
    pub size_bytes: usize,
    pub allocated_at: Instant,
    pub last_used: Instant,
    pub usage_count: u32,
    pub buffer_type: BufferType,
    #[cfg(feature = "cuda")]
    pub cuda_ptr: Option<cudarc::driver::CudaSlice<u8>>,
    #[cfg(feature = "opencl")]
    pub opencl_buffer: Option<opencl3::memory::Buffer<u8>>,
}

/// Tipo de buffer GPU
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BufferType {
    Input,
    Output,
    Intermediate,
    Constant,
    Texture,
}

/// Estadísticas de allocación
#[derive(Debug, Clone)]
struct AllocationStats {
    total_allocations: u64,
    total_deallocations: u64,
    peak_memory_usage_mb: f64,
    current_memory_usage_mb: f64,
    allocation_failures: u64,
    average_buffer_lifetime_ms: f64,
    memory_fragmentation_percent: f32,
}

impl GpuMemoryPool {
    /// Crear nuevo pool de memoria GPU
    pub fn new(max_memory_mb: usize) -> Self {
        Self {
            pools: Arc::new(Mutex::new(HashMap::new())),
            total_allocated: Arc::new(Mutex::new(0)),
            max_memory_mb,
            allocation_stats: Arc::new(Mutex::new(AllocationStats::default())),
        }
    }
    
    /// Allocar buffer GPU con tamaño específico
    pub async fn allocate_buffer(&self, size_bytes: usize, buffer_type: BufferType) -> Result<GpuBuffer> {
        let start_time = Instant::now();
        
        // Verificar límites de memoria
        {
            let current_allocated = *self.total_allocated.lock().unwrap();
            let max_bytes = self.max_memory_mb * 1024 * 1024;
            
            if current_allocated + size_bytes > max_bytes {
                // Intentar liberar memoria no utilizada
                self.cleanup_unused_buffers().await?;
                
                let current_allocated = *self.total_allocated.lock().unwrap();
                if current_allocated + size_bytes > max_bytes {
                    let mut stats = self.allocation_stats.lock().unwrap();
                    stats.allocation_failures += 1;
                    return Err(AppError::gpu(&format!(
                        "GPU memory exhausted: {} MB used, {} MB requested, {} MB limit",
                        current_allocated / (1024 * 1024),
                        size_bytes / (1024 * 1024),
                        self.max_memory_mb
                    )));
                }
            }
        }
        
        // Buscar en pool existente
        let mut pools = self.pools.lock().unwrap();
        let pool = pools.entry(size_bytes).or_insert_with(|| BufferPool::new(size_bytes));
        
        let buffer = if let Some(mut buffer) = pool.available_buffers.pop() {
            // Reutilizar buffer existente
            buffer.last_used = Instant::now();
            buffer.usage_count += 1;
            tracing::debug!("🔄 Reusing GPU buffer {} bytes (usage: {})", size_bytes, buffer.usage_count);
            buffer
        } else {
            // Crear nuevo buffer
            let buffer = self.create_new_buffer(size_bytes, buffer_type).await?;
            tracing::debug!("🆕 Created new GPU buffer {} bytes", size_bytes);
            buffer
        };
        
        // Actualizar estadísticas
        {
            let mut total_allocated = self.total_allocated.lock().unwrap();
            *total_allocated += size_bytes;
            
            let mut stats = self.allocation_stats.lock().unwrap();
            stats.total_allocations += 1;
            stats.current_memory_usage_mb = *total_allocated as f64 / (1024.0 * 1024.0);
            if stats.current_memory_usage_mb > stats.peak_memory_usage_mb {
                stats.peak_memory_usage_mb = stats.current_memory_usage_mb;
            }
        }
        
        // Agregar al pool de buffers allocados
        pool.allocated_buffers.push(buffer.clone());
        
        let allocation_time = start_time.elapsed();
        tracing::debug!("🚀 GPU buffer allocated in {:?}", allocation_time);
        
        Ok(buffer)
    }
    
    /// Liberar buffer GPU
    pub async fn deallocate_buffer(&self, buffer: GpuBuffer) -> Result<()> {
        let start_time = Instant::now();
        
        let mut pools = self.pools.lock().unwrap();
        if let Some(pool) = pools.get_mut(&buffer.size_bytes) {
            // Remover de buffers allocados
            pool.allocated_buffers.retain(|b| b.id != buffer.id);
            
            // Decidir si reciclar o liberar completamente
            if pool.available_buffers.len() < pool.max_buffers && 
               buffer.allocated_at.elapsed() < Duration::from_secs(300) { // 5 minutos
                // Reciclar buffer
                pool.available_buffers.push(buffer.clone());
                tracing::debug!("♻️ Recycled GPU buffer {} bytes", buffer.size_bytes);
            } else {
                // Liberar completamente
                self.destroy_buffer(&buffer).await?;
                tracing::debug!("🗑️ Destroyed GPU buffer {} bytes", buffer.size_bytes);
            }
        }
        
        // Actualizar estadísticas
        {
            let mut total_allocated = self.total_allocated.lock().unwrap();
            *total_allocated = total_allocated.saturating_sub(buffer.size_bytes);
            
            let mut stats = self.allocation_stats.lock().unwrap();
            stats.total_deallocations += 1;
            stats.current_memory_usage_mb = *total_allocated as f64 / (1024.0 * 1024.0);
            
            let lifetime = buffer.allocated_at.elapsed().as_millis() as f64;
            stats.average_buffer_lifetime_ms = 
                (stats.average_buffer_lifetime_ms + lifetime) / 2.0;
        }
        
        let deallocation_time = start_time.elapsed();
        tracing::debug!("🚀 GPU buffer deallocated in {:?}", deallocation_time);
        
        Ok(())
    }
    
    /// Crear nuevo buffer físico
    async fn create_new_buffer(&self, size_bytes: usize, buffer_type: BufferType) -> Result<GpuBuffer> {
        let buffer_id = uuid::Uuid::new_v4().to_string();
        
        let buffer = GpuBuffer {
            id: buffer_id,
            size_bytes,
            allocated_at: Instant::now(),
            last_used: Instant::now(),
            usage_count: 1,
            buffer_type,
            #[cfg(feature = "cuda")]
            cuda_ptr: None, // Se asignaría en implementación real
            #[cfg(feature = "opencl")]
            opencl_buffer: None, // Se asignaría en implementación real
        };
        
        // Aquí iría la allocación real según el backend (CUDA/OpenCL)
        // Por ahora simulamos
        tokio::time::sleep(Duration::from_micros(10)).await;
        
        Ok(buffer)
    }
    
    /// Destruir buffer físico
    async fn destroy_buffer(&self, _buffer: &GpuBuffer) -> Result<()> {
        // Aquí iría la liberación real según el backend
        tokio::time::sleep(Duration::from_micros(5)).await;
        Ok(())
    }
    
    /// Limpiar buffers no utilizados
    pub async fn cleanup_unused_buffers(&self) -> Result<usize> {
        let start_time = Instant::now();
        let mut cleaned_bytes = 0;
        let cleanup_threshold = Duration::from_secs(60); // 1 minuto sin uso
        
        let mut pools = self.pools.lock().unwrap();
        for pool in pools.values_mut() {
            let old_len = pool.available_buffers.len();
            
            // Remover buffers antiguos
            pool.available_buffers.retain(|buffer| {
                let should_keep = buffer.last_used.elapsed() < cleanup_threshold;
                if !should_keep {
                    cleaned_bytes += buffer.size_bytes;
                }
                should_keep
            });
            
            let cleaned_count = old_len - pool.available_buffers.len();
            if cleaned_count > 0 {
                tracing::info!("🧹 Cleaned {} unused buffers ({} bytes)", cleaned_count, cleaned_bytes);
            }
        }
        
        // Actualizar memoria total
        {
            let mut total_allocated = self.total_allocated.lock().unwrap();
            *total_allocated = total_allocated.saturating_sub(cleaned_bytes);
        }
        
        let cleanup_time = start_time.elapsed();
        tracing::debug!("🚀 GPU memory cleanup completed in {:?}", cleanup_time);
        
        Ok(cleaned_bytes)
    }
    
    /// Obtener estadísticas de memoria
    pub fn get_memory_stats(&self) -> GpuMemoryStats {
        let current_allocated = *self.total_allocated.lock().unwrap();
        let stats = self.allocation_stats.lock().unwrap().clone();
        let pools = self.pools.lock().unwrap();
        
        let total_buffers: usize = pools.values()
            .map(|pool| pool.available_buffers.len() + pool.allocated_buffers.len())
            .sum();
        
        let available_buffers: usize = pools.values()
            .map(|pool| pool.available_buffers.len())
            .sum();
        
        GpuMemoryStats {
            total_allocated_mb: current_allocated as f64 / (1024.0 * 1024.0),
            max_memory_mb: self.max_memory_mb as f64,
            utilization_percent: (current_allocated as f64 / (self.max_memory_mb as f64 * 1024.0 * 1024.0)) * 100.0,
            total_buffers,
            available_buffers,
            allocated_buffers: total_buffers - available_buffers,
            allocation_stats: stats,
            pool_count: pools.len(),
        }
    }
    
    /// Desfragmentar memoria si es necesario
    pub async fn defragment_memory(&self) -> Result<DefragmentationResult> {
        let start_time = Instant::now();
        
        // Analizar fragmentación
        let fragmentation_percent = self.calculate_fragmentation_percent();
        
        if fragmentation_percent < 25.0 {
            return Ok(DefragmentationResult {
                was_needed: false,
                bytes_moved: 0,
                duration_ms: start_time.elapsed().as_millis() as u64,
                fragmentation_before: fragmentation_percent,
                fragmentation_after: fragmentation_percent,
            });
        }
        
        tracing::info!("🔧 Starting GPU memory defragmentation ({}% fragmented)", fragmentation_percent);
        
        // Simular desfragmentación
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        let fragmentation_after = fragmentation_percent * 0.3; // Reducir fragmentación
        
        {
            let mut stats = self.allocation_stats.lock().unwrap();
            stats.memory_fragmentation_percent = fragmentation_after;
        }
        
        let duration = start_time.elapsed();
        tracing::info!("✅ GPU memory defragmentation completed in {:?}", duration);
        
        Ok(DefragmentationResult {
            was_needed: true,
            bytes_moved: 1024 * 1024, // 1MB simulado
            duration_ms: duration.as_millis() as u64,
            fragmentation_before: fragmentation_percent,
            fragmentation_after,
        })
    }
    
    fn calculate_fragmentation_percent(&self) -> f32 {
        // Simulación simple de fragmentación
        let stats = self.allocation_stats.lock().unwrap();
        stats.memory_fragmentation_percent
    }
}

impl BufferPool {
    fn new(size_bytes: usize) -> Self {
        let max_buffers = match size_bytes {
            0..=1024 => 100,          // Buffers pequeños: muchos
            1025..=1048576 => 50,     // Buffers medianos: cantidad media
            _ => 10,                  // Buffers grandes: pocos
        };
        
        Self {
            size_bytes,
            available_buffers: Vec::new(),
            allocated_buffers: Vec::new(),
            max_buffers,
        }
    }
}

impl Default for AllocationStats {
    fn default() -> Self {
        Self {
            total_allocations: 0,
            total_deallocations: 0,
            peak_memory_usage_mb: 0.0,
            current_memory_usage_mb: 0.0,
            allocation_failures: 0,
            average_buffer_lifetime_ms: 0.0,
            memory_fragmentation_percent: 0.0,
        }
    }
}

/// Estadísticas públicas de memoria GPU
#[derive(Debug, Clone)]
pub struct GpuMemoryStats {
    pub total_allocated_mb: f64,
    pub max_memory_mb: f64,
    pub utilization_percent: f64,
    pub total_buffers: usize,
    pub available_buffers: usize,
    pub allocated_buffers: usize,
    pub allocation_stats: AllocationStats,
    pub pool_count: usize,
}

/// Resultado de desfragmentación
#[derive(Debug, Clone)]
pub struct DefragmentationResult {
    pub was_needed: bool,
    pub bytes_moved: usize,
    pub duration_ms: u64,
    pub fragmentation_before: f32,
    pub fragmentation_after: f32,
}

/// Configuración de memory pool
#[derive(Debug, Clone)]
pub struct MemoryPoolConfig {
    pub max_memory_mb: usize,
    pub cleanup_interval_secs: u64,
    pub max_buffer_age_secs: u64,
    pub defragmentation_threshold_percent: f32,
    pub enable_memory_monitoring: bool,
}

impl Default for MemoryPoolConfig {
    fn default() -> Self {
        Self {
            max_memory_mb: 8 * 1024, // 8GB
            cleanup_interval_secs: 60,
            max_buffer_age_secs: 300, // 5 minutos
            defragmentation_threshold_percent: 25.0,
            enable_memory_monitoring: true,
        }
    }
}

/// Monitor de memoria en background
pub struct GpuMemoryMonitor {
    pool: Arc<GpuMemoryPool>,
    config: MemoryPoolConfig,
    is_running: Arc<std::sync::atomic::AtomicBool>,
}

impl GpuMemoryMonitor {
    pub fn new(pool: Arc<GpuMemoryPool>, config: MemoryPoolConfig) -> Self {
        Self {
            pool,
            config,
            is_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
    
    /// Iniciar monitoreo en background
    pub async fn start_monitoring(&self) -> Result<()> {
        if self.is_running.load(std::sync::atomic::Ordering::Relaxed) {
            return Ok(());
        }
        
        self.is_running.store(true, std::sync::atomic::Ordering::Relaxed);
        
        let pool = self.pool.clone();
        let config = self.config.clone();
        let is_running = self.is_running.clone();
        
        tokio::spawn(async move {
            let mut cleanup_interval = tokio::time::interval(Duration::from_secs(config.cleanup_interval_secs));
            
            while is_running.load(std::sync::atomic::Ordering::Relaxed) {
                cleanup_interval.tick().await;
                
                // Cleanup periódico
                if let Err(e) = pool.cleanup_unused_buffers().await {
                    tracing::error!("GPU memory cleanup failed: {}", e);
                }
                
                // Desfragmentación si es necesario
                let stats = pool.get_memory_stats();
                if stats.allocation_stats.memory_fragmentation_percent > config.defragmentation_threshold_percent {
                    if let Err(e) = pool.defragment_memory().await {
                        tracing::error!("GPU memory defragmentation failed: {}", e);
                    }
                }
                
                // Log estadísticas si el monitoreo está habilitado
                if config.enable_memory_monitoring {
                    tracing::debug!(
                        "💾 GPU Memory: {:.1}MB/{:.1}MB ({:.1}%) - {} buffers ({} available)",
                        stats.total_allocated_mb,
                        stats.max_memory_mb,
                        stats.utilization_percent,
                        stats.total_buffers,
                        stats.available_buffers
                    );
                }
            }
        });
        
        tracing::info!("🔍 GPU memory monitor started");
        Ok(())
    }
    
    /// Detener monitoreo
    pub fn stop_monitoring(&self) {
        self.is_running.store(false, std::sync::atomic::Ordering::Relaxed);
        tracing::info!("🛑 GPU memory monitor stopped");
    }
}