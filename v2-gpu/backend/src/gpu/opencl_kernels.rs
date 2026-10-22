/*!
 * OpenCL Kernels - Kernels OpenCL para compatibilidad multi-GPU
 * 
 * Proporciona aceleración GPU usando OpenCL para sistemas sin CUDA:
 * - AMD GPUs, Intel GPUs, Apple Silicon
 * - Compatibilidad cross-platform
 */

use crate::core::{Result, AppError};
use nalgebra::Vector3;

/// Contexto OpenCL para operaciones de docking
#[cfg(feature = "opencl")]
pub struct OpenClContext {
    device_id: usize,
    context: opencl3::context::Context,
    queue: opencl3::command_queue::CommandQueue,
    program: opencl3::program::Program,
}

#[cfg(feature = "opencl")]
impl OpenClContext {
    /// Crear nuevo contexto OpenCL
    pub async fn new() -> Result<Self> {
        use opencl3::device::{get_all_devices, CL_DEVICE_TYPE_GPU};
        use opencl3::context::Context;
        use opencl3::command_queue::CommandQueue;
        use opencl3::program::{Program, CL_STD_2_0};
        
        // Obtener dispositivos GPU
        let devices = get_all_devices(CL_DEVICE_TYPE_GPU).map_err(|e| {
            AppError::gpu(&format!("Failed to get OpenCL devices: {:?}", e))
        })?;
        
        if devices.is_empty() {
            return Err(AppError::gpu("No OpenCL GPU devices found"));
        }
        
        let device = devices[0];
        let device_id = device.id().map_err(|e| {
            AppError::gpu(&format!("Failed to get device ID: {:?}", e))
        })?;
        
        // Crear contexto
        let context = Context::from_device(&device).map_err(|e| {
            AppError::gpu(&format!("Failed to create OpenCL context: {:?}", e))
        })?;
        
        // Crear command queue
        let queue = CommandQueue::create_default(&context, device.clone()).map_err(|e| {
            AppError::gpu(&format!("Failed to create command queue: {:?}", e))
        })?;
        
        // Compilar kernels
        let kernel_source = include_str!("../../../opencl/kernels.cl");
        let program = Program::create_and_build_from_source(&context, kernel_source, CL_STD_2_0).map_err(|e| {
            AppError::gpu(&format!("Failed to build OpenCL program: {:?}", e))
        })?;
        
        tracing::info!("🚀 OpenCL context initialized for device {}", device_id);
        
        Ok(Self {
            device_id,
            context,
            queue,
            program,
        })
    }
    
    /// Calcular centroides usando OpenCL
    pub async fn compute_centroids_opencl(&self, positions: &[Vector3<f64>]) -> Result<Vec<Vector3<f64>>> {
        use opencl3::kernel::{Kernel, ExecuteKernel};
        use opencl3::memory::{Buffer, CL_MEM_READ_ONLY, CL_MEM_WRITE_ONLY};
        
        let start_time = std::time::Instant::now();
        
        if positions.is_empty() {
            return Err(AppError::algorithm("Cannot compute centroids from empty positions"));
        }
        
        // Convertir posiciones a array plano
        let flat_positions: Vec<f64> = positions.iter()
            .flat_map(|p| [p.x, p.y, p.z])
            .collect();\n        \n        let n_points = positions.len();\n        let n_dims = 3;\n        \n        // Crear buffers\n        let positions_buffer = Buffer::<f64>::create(&self.context, CL_MEM_READ_ONLY, flat_positions.len(), std::ptr::null_mut()).map_err(|e| {\n            AppError::gpu(&format!(\"Failed to create positions buffer: {:?}\", e))\n        })?;\n        \n        let centroids_buffer = Buffer::<f64>::create(&self.context, CL_MEM_WRITE_ONLY, n_dims, std::ptr::null_mut()).map_err(|e| {\n            AppError::gpu(&format!(\"Failed to create centroids buffer: {:?}\", e))\n        })?;\n        \n        // Copiar datos a GPU\n        self.queue.enqueue_write_buffer(&positions_buffer, opencl3::types::CL_BLOCKING, 0, &flat_positions, &[]).map_err(|e| {\n            AppError::gpu(&format!(\"Failed to write positions to buffer: {:?}\", e))\n        })?;\n        \n        // Crear y configurar kernel\n        let kernel = Kernel::create(&self.program, \"compute_centroids\").map_err(|e| {\n            AppError::gpu(&format!(\"Failed to create kernel: {:?}\", e))\n        })?;\n        \n        // Establecer argumentos del kernel\n        ExecuteKernel::new(&kernel)\n            .set_arg(&positions_buffer)\n            .set_arg(&centroids_buffer)\n            .set_arg(&(n_points as i32))\n            .set_global_work_size(n_points)\n            .enqueue_nd_range(&self.queue).map_err(|e| {\n                AppError::gpu(&format!(\"Failed to execute kernel: {:?}\", e))\n            })?;\n        \n        // Leer resultados\n        let mut result_data = vec![0.0f64; n_dims];\n        self.queue.enqueue_read_buffer(&centroids_buffer, opencl3::types::CL_BLOCKING, 0, &mut result_data, &[]).map_err(|e| {\n            AppError::gpu(&format!(\"Failed to read centroids: {:?}\", e))\n        })?;\n        \n        // Convertir resultado\n        let centroids = vec![Vector3::new(\n            result_data[0] / n_points as f64,\n            result_data[1] / n_points as f64,\n            result_data[2] / n_points as f64,\n        )];\n        \n        let duration = start_time.elapsed();\n        tracing::info!(\"🚀 OpenCL centroids computed in {:?} for {} points\", duration, n_points);\n        \n        Ok(centroids)\n    }\n    \n    /// Búsqueda paralela de vecinos usando OpenCL\n    pub async fn parallel_neighbor_search_opencl(\n        &self,\n        query_points: &[Vector3<f64>],\n        reference_points: &[Vector3<f64>],\n        radius: f64,\n    ) -> Result<Vec<Vec<usize>>> {\n        use opencl3::kernel::{Kernel, ExecuteKernel};\n        use opencl3::memory::{Buffer, CL_MEM_READ_ONLY, CL_MEM_WRITE_ONLY};\n        \n        let start_time = std::time::Instant::now();\n        \n        if query_points.is_empty() || reference_points.is_empty() {\n            return Ok(vec![]);\n        }\n        \n        let n_query = query_points.len();\n        let n_ref = reference_points.len();\n        \n        // Convertir a arrays planos\n        let flat_query: Vec<f64> = query_points.iter()\n            .flat_map(|p| [p.x, p.y, p.z])\n            .collect();\n        \n        let flat_ref: Vec<f64> = reference_points.iter()\n            .flat_map(|p| [p.x, p.y, p.z])\n            .collect();\n        \n        // Crear buffers\n        let query_buffer = Buffer::<f64>::create(&self.context, CL_MEM_READ_ONLY, flat_query.len(), std::ptr::null_mut()).map_err(|e| {\n            AppError::gpu(&format!(\"Failed to create query buffer: {:?}\", e))\n        })?;\n        \n        let ref_buffer = Buffer::<f64>::create(&self.context, CL_MEM_READ_ONLY, flat_ref.len(), std::ptr::null_mut()).map_err(|e| {\n            AppError::gpu(&format!(\"Failed to create reference buffer: {:?}\", e))\n        })?;\n        \n        let max_neighbors = 100;\n        let neighbors_buffer = Buffer::<i32>::create(&self.context, CL_MEM_WRITE_ONLY, n_query * max_neighbors, std::ptr::null_mut()).map_err(|e| {\n            AppError::gpu(&format!(\"Failed to create neighbors buffer: {:?}\", e))\n        })?;\n        \n        let counts_buffer = Buffer::<i32>::create(&self.context, CL_MEM_WRITE_ONLY, n_query, std::ptr::null_mut()).map_err(|e| {\n            AppError::gpu(&format!(\"Failed to create counts buffer: {:?}\", e))\n        })?;\n        \n        // Copiar datos\n        self.queue.enqueue_write_buffer(&query_buffer, opencl3::types::CL_BLOCKING, 0, &flat_query, &[]).map_err(|e| {\n            AppError::gpu(&format!(\"Failed to write query points: {:?}\", e))\n        })?;\n        \n        self.queue.enqueue_write_buffer(&ref_buffer, opencl3::types::CL_BLOCKING, 0, &flat_ref, &[]).map_err(|e| {\n            AppError::gpu(&format!(\"Failed to write reference points: {:?}\", e))\n        })?;\n        \n        // Crear y ejecutar kernel\n        let kernel = Kernel::create(&self.program, \"parallel_neighbor_search\").map_err(|e| {\n            AppError::gpu(&format!(\"Failed to create neighbor search kernel: {:?}\", e))\n        })?;\n        \n        ExecuteKernel::new(&kernel)\n            .set_arg(&query_buffer)\n            .set_arg(&ref_buffer)\n            .set_arg(&neighbors_buffer)\n            .set_arg(&counts_buffer)\n            .set_arg(&(n_query as i32))\n            .set_arg(&(n_ref as i32))\n            .set_arg(&radius)\n            .set_arg(&(max_neighbors as i32))\n            .set_global_work_size(n_query)\n            .enqueue_nd_range(&self.queue).map_err(|e| {\n                AppError::gpu(&format!(\"Failed to execute neighbor search kernel: {:?}\", e))\n            })?;\n        \n        // Leer resultados\n        let mut neighbor_data = vec![0i32; n_query * max_neighbors];\n        let mut neighbor_counts = vec![0i32; n_query];\n        \n        self.queue.enqueue_read_buffer(&neighbors_buffer, opencl3::types::CL_BLOCKING, 0, &mut neighbor_data, &[]).map_err(|e| {\n            AppError::gpu(&format!(\"Failed to read neighbor data: {:?}\", e))\n        })?;\n        \n        self.queue.enqueue_read_buffer(&counts_buffer, opencl3::types::CL_BLOCKING, 0, &mut neighbor_counts, &[]).map_err(|e| {\n            AppError::gpu(&format!(\"Failed to read neighbor counts: {:?}\", e))\n        })?;\n        \n        // Convertir resultado\n        let mut results = Vec::with_capacity(n_query);\n        for i in 0..n_query {\n            let count = neighbor_counts[i] as usize;\n            let start_idx = i * max_neighbors;\n            let neighbors: Vec<usize> = neighbor_data[start_idx..start_idx + count]\n                .iter()\n                .map(|&idx| idx as usize)\n                .collect();\n            results.push(neighbors);\n        }\n        \n        let duration = start_time.elapsed();\n        tracing::info!(\"🚀 OpenCL neighbor search completed in {:?} for {} queries\", duration, n_query);\n        \n        Ok(results)\n    }\n    \n    /// Evaluación de conformaciones usando OpenCL\n    pub async fn evaluate_conformations_opencl(\n        &self,\n        conformations: &[super::ConformationData],\n        scoring_params: &super::ScoringParameters,\n    ) -> Result<Vec<f64>> {\n        use opencl3::kernel::{Kernel, ExecuteKernel};\n        use opencl3::memory::{Buffer, CL_MEM_READ_ONLY, CL_MEM_WRITE_ONLY};\n        \n        let start_time = std::time::Instant::now();\n        \n        if conformations.is_empty() {\n            return Ok(vec![]);\n        }\n        \n        let n_conformations = conformations.len();\n        \n        // Preparar datos simplificados para demostración\n        let mut receptor_centers = Vec::new();\n        let mut ligand_centers = Vec::new();\n        \n        for conf in conformations {\n            // Calcular centros de masa\n            let receptor_center = self.compute_center(&conf.receptor_atoms);\n            let ligand_center = self.compute_center(&conf.ligand_atoms);\n            \n            receptor_centers.extend([receptor_center.x, receptor_center.y, receptor_center.z]);\n            ligand_centers.extend([ligand_center.x, ligand_center.y, ligand_center.z]);\n        }\n        \n        // Crear buffers\n        let receptor_buffer = Buffer::<f64>::create(&self.context, CL_MEM_READ_ONLY, receptor_centers.len(), std::ptr::null_mut()).map_err(|e| {\n            AppError::gpu(&format!(\"Failed to create receptor buffer: {:?}\", e))\n        })?;\n        \n        let ligand_buffer = Buffer::<f64>::create(&self.context, CL_MEM_READ_ONLY, ligand_centers.len(), std::ptr::null_mut()).map_err(|e| {\n            AppError::gpu(&format!(\"Failed to create ligand buffer: {:?}\", e))\n        })?;\n        \n        let scores_buffer = Buffer::<f64>::create(&self.context, CL_MEM_WRITE_ONLY, n_conformations, std::ptr::null_mut()).map_err(|e| {\n            AppError::gpu(&format!(\"Failed to create scores buffer: {:?}\", e))\n        })?;\n        \n        // Copiar datos\n        self.queue.enqueue_write_buffer(&receptor_buffer, opencl3::types::CL_BLOCKING, 0, &receptor_centers, &[]).map_err(|e| {\n            AppError::gpu(&format!(\"Failed to write receptor data: {:?}\", e))\n        })?;\n        \n        self.queue.enqueue_write_buffer(&ligand_buffer, opencl3::types::CL_BLOCKING, 0, &ligand_centers, &[]).map_err(|e| {\n            AppError::gpu(&format!(\"Failed to write ligand data: {:?}\", e))\n        })?;\n        \n        // Crear y ejecutar kernel\n        let kernel = Kernel::create(&self.program, \"evaluate_conformations\").map_err(|e| {\n            AppError::gpu(&format!(\"Failed to create evaluation kernel: {:?}\", e))\n        })?;\n        \n        ExecuteKernel::new(&kernel)\n            .set_arg(&receptor_buffer)\n            .set_arg(&ligand_buffer)\n            .set_arg(&scores_buffer)\n            .set_arg(&(n_conformations as i32))\n            .set_arg(&scoring_params.van_der_waals_weight)\n            .set_arg(&scoring_params.electrostatic_weight)\n            .set_global_work_size(n_conformations)\n            .enqueue_nd_range(&self.queue).map_err(|e| {\n                AppError::gpu(&format!(\"Failed to execute evaluation kernel: {:?}\", e))\n            })?;\n        \n        // Leer resultados\n        let mut scores = vec![0.0f64; n_conformations];\n        self.queue.enqueue_read_buffer(&scores_buffer, opencl3::types::CL_BLOCKING, 0, &mut scores, &[]).map_err(|e| {\n            AppError::gpu(&format!(\"Failed to read scores: {:?}\", e))\n        })?;\n        \n        let duration = start_time.elapsed();\n        tracing::info!(\"🚀 OpenCL conformations evaluated in {:?} for {} conformations\", duration, n_conformations);\n        \n        Ok(scores)\n    }\n    \n    fn compute_center(&self, atoms: &[Vector3<f64>]) -> Vector3<f64> {\n        if atoms.is_empty() {\n            return Vector3::zeros();\n        }\n        \n        let sum = atoms.iter().fold(Vector3::zeros(), |acc, atom| acc + atom);\n        sum / atoms.len() as f64\n    }\n}\n\n/// Implementación de GpuOperations para OpenCL\n#[cfg(feature = \"opencl\")]\nimpl super::GpuOperations for OpenClContext {\n    async fn compute_centroids_gpu(&self, positions: &[Vector3<f64>]) -> Result<Vec<Vector3<f64>>> {\n        self.compute_centroids_opencl(positions).await\n    }\n    \n    async fn parallel_neighbor_search_gpu(\n        &self,\n        query_points: &[Vector3<f64>],\n        reference_points: &[Vector3<f64>],\n        radius: f64,\n    ) -> Result<Vec<Vec<usize>>> {\n        self.parallel_neighbor_search_opencl(query_points, reference_points, radius).await\n    }\n    \n    async fn evaluate_conformations_gpu(\n        &self,\n        conformations: &[super::ConformationData],\n        scoring_params: &super::ScoringParameters,\n    ) -> Result<Vec<f64>> {\n        self.evaluate_conformations_opencl(conformations, scoring_params).await\n    }\n    \n    fn get_gpu_info(&self) -> super::GpuInfo {\n        super::GpuInfo {\n            device_name: format!(\"OpenCL Device {}\", self.device_id),\n            compute_capability: (2, 0),\n            memory_total_mb: 8 * 1024, // 8GB típico\n            memory_free_mb: 6 * 1024,  // 6GB libre\n            cuda_cores: 0, // No aplica para OpenCL\n            supports_fp64: true,\n        }\n    }\n}\n\n/// Stub para cuando OpenCL no está disponible\n#[cfg(not(feature = \"opencl\"))]\npub struct OpenClContext;\n\n#[cfg(not(feature = \"opencl\"))]\nimpl OpenClContext {\n    pub async fn new() -> Result<Self> {\n        Err(AppError::gpu(\"OpenCL support not compiled\"))\n    }\n}\n\n/// Métricas de rendimiento específicas de OpenCL\n#[derive(Debug, Clone)]\npub struct OpenClPerformanceMetrics {\n    pub kernel_execution_time_ms: u64,\n    pub memory_transfer_time_ms: u64,\n    pub command_queue_overhead_ms: u64,\n    pub compute_unit_utilization: f32,\n}\n\n/// Información de dispositivo OpenCL\n#[derive(Debug, Clone)]\npub struct OpenClDeviceInfo {\n    pub device_name: String,\n    pub vendor: String,\n    pub driver_version: String,\n    pub opencl_version: String,\n    pub max_compute_units: u32,\n    pub max_work_group_size: usize,\n    pub global_memory_mb: u64,\n    pub local_memory_kb: u64,\n}"