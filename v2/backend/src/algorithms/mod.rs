/*!
 * Algorithms - Algoritmos de docking ultra-optimizados
 * 
 * Módulo principal que integra todas las optimizaciones:
 * - Docking engine con SIMD y GPU
 * - Spatial indexing con KD-Trees
 * - Paralelización masiva con Rayon
 * - Memory pooling optimizado
 */

pub mod docking_engine;
pub mod spatial_index;
pub mod simd_ops;
pub mod parallel_engine;
pub mod protein_parser;
pub mod docking_core;
pub mod simd_math;
pub mod memory_pool;

#[cfg(feature = "gpu")]
pub mod gpu_algorithms;

pub use docking_engine::DockingEngine;
pub use spatial_index::{SpatialIndex, KdTree};
pub use simd_ops::SimdOperations;
pub use parallel_engine::{ParallelEngine, ParallelEngineStats};
pub use protein_parser::{ProteinParser, ProteinStructure, Atom, Residue};
pub use docking_core::{DockingEngine as CoreDockingEngine, DockingResult, DockingConfig};
pub use simd_math::SimdMath;
pub use memory_pool::ProteinMemoryPool;

use crate::core::Result;
use std::sync::Arc;

/// Facade principal para todos los algoritmos de docking
pub struct AlgorithmFacade {
    docking_engine: Arc<DockingEngine>,
    parallel_engine: Arc<ParallelEngine>,
    spatial_index: Arc<SpatialIndex>,
    simd_ops: SimdOperations,
}

impl AlgorithmFacade {
    /// Crear facade con todas las optimizaciones
    pub async fn new() -> Result<Arc<Self>> {
        tracing::info!("🚀 Initializing algorithm facade with all optimizations...");
        
        // Inicializar motor de docking
        #[cfg(feature = "gpu")]
        let docking_engine = DockingEngine::new(None).await?;
        #[cfg(not(feature = "gpu"))]
        let docking_engine = DockingEngine::new().await?;
        
        // Inicializar motor paralelo
        let parallel_engine = ParallelEngine::new().await?;
        
        // Inicializar componentes auxiliares
        let spatial_index = Arc::new(SpatialIndex::new());
        let simd_ops = SimdOperations::new();
        
        let facade = Arc::new(Self {
            docking_engine,
            parallel_engine,
            spatial_index,
            simd_ops,
        });
        
        tracing::info!("✅ Algorithm facade initialized successfully");
        Ok(facade)
    }
    
    /// Obtener estadísticas de rendimiento completas
    pub fn get_comprehensive_stats(&self) -> AlgorithmStats {
        let parallel_stats = self.parallel_engine.get_performance_stats();
        let simd_info = self.simd_ops.get_simd_info();
        
        AlgorithmStats {
            parallel_stats,
            simd_info,
            active_computations: self.docking_engine.active_computations(),
            spatial_index_built: self.spatial_index.is_built(),
        }
    }
    
    /// Getters para componentes individuales
    pub fn docking_engine(&self) -> &Arc<DockingEngine> {
        &self.docking_engine
    }
    
    pub fn parallel_engine(&self) -> &Arc<ParallelEngine> {
        &self.parallel_engine
    }
    
    pub fn spatial_index(&self) -> &Arc<SpatialIndex> {
        &self.spatial_index
    }
    
    pub fn simd_ops(&self) -> &SimdOperations {
        &self.simd_ops
    }
}

/// Estadísticas completas de algoritmos
#[derive(Debug, Clone)]
pub struct AlgorithmStats {
    pub parallel_stats: ParallelEngineStats,
    pub simd_info: crate::algorithms::simd_ops::SimdInfo,
    pub active_computations: usize,
    pub spatial_index_built: bool,
}