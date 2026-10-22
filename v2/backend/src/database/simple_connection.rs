// Simplified Database Connection - Fallback sin PostgreSQL
// Para desarrollo sin base de datos activa

use tracing::{info, warn};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::simple_models::{SimpleProteinFileModel, SimpleDockingJobModel};

/// Pool simulado para desarrollo
#[derive(Debug, Clone)]
pub struct SimplePool {
    files: Arc<Mutex<HashMap<String, SimpleProteinFileModel>>>,
    jobs: Arc<Mutex<HashMap<String, SimpleDockingJobModel>>>,
}

impl SimplePool {
    pub fn new() -> Self {
        info!("📝 Using simple in-memory database pool for development");
        Self {
            files: Arc::new(Mutex::new(HashMap::new())),
            jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    pub fn store_file(&self, file: SimpleProteinFileModel) -> anyhow::Result<()> {
        let mut files = self.files.lock().map_err(|_| anyhow::anyhow!("Lock poisoned"))?;
        files.insert(file.file_id.clone(), file);
        Ok(())
    }
    
    pub fn get_file(&self, file_id: &str) -> anyhow::Result<Option<SimpleProteinFileModel>> {
        let files = self.files.lock().map_err(|_| anyhow::anyhow!("Lock poisoned"))?;
        Ok(files.get(file_id).cloned())
    }
    
    pub fn store_job(&self, job: SimpleDockingJobModel) -> anyhow::Result<()> {
        let mut jobs = self.jobs.lock().map_err(|_| anyhow::anyhow!("Lock poisoned"))?;
        jobs.insert(job.job_id.clone(), job);
        Ok(())
    }
    
    pub fn get_job(&self, job_id: &str) -> anyhow::Result<Option<SimpleDockingJobModel>> {
        let jobs = self.jobs.lock().map_err(|_| anyhow::anyhow!("Lock poisoned"))?;
        Ok(jobs.get(job_id).cloned())
    }
}

static mut SIMPLE_POOL: Option<SimplePool> = None;
static INIT: std::sync::Once = std::sync::Once::new();

/// Inicializar pool simple
pub fn initialize_simple_pool() -> anyhow::Result<SimplePool> {
    unsafe {
        INIT.call_once(|| {
            SIMPLE_POOL = Some(SimplePool::new());
        });
        
        Ok(SIMPLE_POOL.as_ref().unwrap().clone())
    }
}

/// Obtener pool simple global
pub fn get_simple_pool() -> anyhow::Result<SimplePool> {
    unsafe {
        SIMPLE_POOL.as_ref()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Simple pool not initialized"))
    }
}

/// Health check simplificado
#[derive(Debug, serde::Serialize)]
pub struct SimpleHealth {
    pub status: String,
    pub storage_type: String,
    pub files_count: usize,
    pub jobs_count: usize,
}

pub fn simple_health_check() -> anyhow::Result<SimpleHealth> {
    let pool = get_simple_pool()?;
    
    let files_count = pool.files.lock()
        .map_err(|_| anyhow::anyhow!("Lock poisoned"))?
        .len();
        
    let jobs_count = pool.jobs.lock()
        .map_err(|_| anyhow::anyhow!("Lock poisoned"))?
        .len();
    
    Ok(SimpleHealth {
        status: "healthy".to_string(),
        storage_type: "in-memory".to_string(),
        files_count,
        jobs_count,
    })
}