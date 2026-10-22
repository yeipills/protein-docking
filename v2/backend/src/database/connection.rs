// Database Connection Manager - Pool de conexiones ultra-optimizado
// Auto-reconexión, health checks, métricas de performance

use sqlx::{PgPool, postgres::PgPoolOptions, Postgres};
use std::time::Duration;
use tracing::{info, warn, error};
use once_cell::sync::OnceCell;

static DATABASE_POOL: OnceCell<PgPool> = OnceCell::new();

/// Configuración de base de datos
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connection_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_lifetime: Duration,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            host: std::env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string()),
            port: std::env::var("DB_PORT")
                .unwrap_or_else(|_| "5432".to_string())
                .parse()
                .unwrap_or(5432),
            database: std::env::var("DB_NAME").unwrap_or_else(|_| "protein_docking".to_string()),
            username: std::env::var("DB_USER").unwrap_or_else(|_| "postgres".to_string()),
            password: std::env::var("DB_PASSWORD").unwrap_or_else(|_| "password".to_string()),
            max_connections: std::env::var("DB_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "50".to_string())
                .parse()
                .unwrap_or(50),
            min_connections: std::env::var("DB_MIN_CONNECTIONS")
                .unwrap_or_else(|_| "5".to_string())
                .parse()
                .unwrap_or(5),
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600), // 10 min
            max_lifetime: Duration::from_secs(3600), // 1 hour
        }
    }
}

/// Inicializar pool de conexiones
pub async fn initialize_database_pool(config: DatabaseConfig) -> anyhow::Result<PgPool> {
    let database_url = format!(
        "postgresql://{}:{}@{}:{}/{}",
        config.username, config.password, config.host, config.port, config.database
    );
    
    info!("🔌 Connecting to PostgreSQL: {}:{}/{}", config.host, config.port, config.database);
    
    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(config.connection_timeout)
        .idle_timeout(config.idle_timeout)
        .max_lifetime(config.max_lifetime)
        .test_before_acquire(true) // Health check antes de usar conexión
        .connect(&database_url)
        .await?;
    
    // Ejecutar migraciones
    run_migrations(&pool).await?;
    
    // Configurar pool global
    DATABASE_POOL.set(pool.clone())
        .map_err(|_| anyhow::anyhow!("Failed to set global database pool"))?;
    
    info!("✅ Database pool initialized with {} connections", config.max_connections);
    
    Ok(pool)
}

/// Obtener referencia al pool global
pub fn get_database_pool() -> anyhow::Result<&'static PgPool> {
    DATABASE_POOL.get()
        .ok_or_else(|| anyhow::anyhow!("Database pool not initialized"))
}

/// Health check de base de datos
pub async fn health_check() -> anyhow::Result<DatabaseHealth> {
    let pool = get_database_pool()?;
    let start = std::time::Instant::now();
    
    // Query simple para verificar conectividad
    let result = sqlx::query("SELECT 1 as health_check")
        .fetch_one(pool)
        .await;
    
    let duration = start.elapsed();
    
    match result {
        Ok(_) => {
            let health = DatabaseHealth {
                status: "healthy".to_string(),
                response_time_ms: duration.as_millis() as u64,
                active_connections: pool.size(),
                idle_connections: pool.num_idle(),
                max_connections: pool.options().get_max_connections(),
            };
            
            if duration.as_millis() > 1000 {
                warn!("Slow database response: {}ms", duration.as_millis());
            }
            
            Ok(health)
        },
        Err(e) => {
            error!("Database health check failed: {}", e);
            Err(anyhow::anyhow!("Database unhealthy: {}", e))
        }
    }
}

/// Estado de salud de la base de datos
#[derive(Debug, serde::Serialize)]
pub struct DatabaseHealth {
    pub status: String,
    pub response_time_ms: u64,
    pub active_connections: u32,
    pub idle_connections: u32,
    pub max_connections: u32,
}

/// Ejecutar migraciones automáticamente
async fn run_migrations(pool: &PgPool) -> anyhow::Result<()> {
    info!("🔄 Running database migrations...");
    
    // Crear extensiones necesarias
    let _ = sqlx::query("CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\"")
        .execute(pool)
        .await?;
    
    // Crear tabla de archivos
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS protein_files (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            file_id VARCHAR(255) UNIQUE NOT NULL,
            original_filename VARCHAR(255) NOT NULL,
            file_path VARCHAR(500) NOT NULL,
            file_size BIGINT NOT NULL,
            content_type VARCHAR(100) NOT NULL,
            upload_timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            
            -- Información de estructura proteica (JSON)
            protein_structure JSONB,
            
            -- Metadatos adicionales
            metadata JSONB DEFAULT '{}',
            
            -- Índices de timestamps
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
    "#)
    .execute(pool)
    .await?;
    
    // Crear tabla de jobs de docking
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS docking_jobs (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            job_id VARCHAR(255) UNIQUE NOT NULL,
            user_id VARCHAR(255),
            receptor_file_id VARCHAR(255) NOT NULL,
            ligand_file_id VARCHAR(255) NOT NULL,
            
            -- Configuración del docking (JSON)
            config JSONB NOT NULL,
            
            -- Estado del job
            status VARCHAR(50) NOT NULL DEFAULT 'queued',
            priority VARCHAR(20) NOT NULL DEFAULT 'normal',
            
            -- Timestamps
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            started_at TIMESTAMPTZ,
            completed_at TIMESTAMPTZ,
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            
            -- Progreso
            progress_percent REAL NOT NULL DEFAULT 0.0,
            current_step TEXT,
            estimated_completion TIMESTAMPTZ,
            
            -- Resultados y estadísticas
            results JSONB,
            error_message TEXT,
            computation_stats JSONB,
            
            -- Constraints
            CONSTRAINT valid_status CHECK (status IN ('queued', 'running', 'completed', 'failed', 'cancelled')),
            CONSTRAINT valid_priority CHECK (priority IN ('low', 'normal', 'high', 'critical')),
            CONSTRAINT valid_progress CHECK (progress_percent >= 0.0 AND progress_percent <= 100.0)
        )
    "#)
    .execute(pool)
    .await?;
    
    // Crear tabla de resultados detallados
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS docking_results (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            job_id VARCHAR(255) NOT NULL,
            pose_id INTEGER NOT NULL,
            score REAL NOT NULL,
            rmsd REAL,
            
            -- Transformación 3D (JSON)
            transformation JSONB NOT NULL,
            
            -- Breakdown de energía (JSON)
            energy_breakdown JSONB NOT NULL,
            
            -- Metadatos de computación
            computation_time_ms BIGINT NOT NULL,
            iterations_used INTEGER,
            convergence_reached BOOLEAN NOT NULL DEFAULT false,
            
            -- Timestamp
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            
            -- Índices compuestos
            UNIQUE(job_id, pose_id)
        )
    "#)
    .execute(pool)
    .await?;
    
    // Crear índices para performance
    create_database_indexes(pool).await?;
    
    // Crear triggers para updated_at automático
    create_database_triggers(pool).await?;
    
    info!("✅ Database migrations completed successfully");
    Ok(())
}

/// Crear índices optimizados
async fn create_database_indexes(pool: &PgPool) -> anyhow::Result<()> {
    let indexes = vec![
        // Protein files indexes
        "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_protein_files_file_id ON protein_files(file_id)",
        "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_protein_files_upload_timestamp ON protein_files(upload_timestamp DESC)",
        "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_protein_files_file_size ON protein_files(file_size)",
        
        // Docking jobs indexes
        "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_docking_jobs_job_id ON docking_jobs(job_id)",
        "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_docking_jobs_user_id ON docking_jobs(user_id)",
        "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_docking_jobs_status ON docking_jobs(status)",
        "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_docking_jobs_priority ON docking_jobs(priority)",
        "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_docking_jobs_created_at ON docking_jobs(created_at DESC)",
        "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_docking_jobs_completed_at ON docking_jobs(completed_at DESC)",
        "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_docking_jobs_status_priority ON docking_jobs(status, priority)",
        
        // Docking results indexes
        "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_docking_results_job_id ON docking_results(job_id)",
        "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_docking_results_score ON docking_results(score DESC)",
        "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_docking_results_job_pose ON docking_results(job_id, pose_id)",
        
        // JSONB indexes para búsquedas avanzadas
        "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_protein_files_structure_gin ON protein_files USING GIN(protein_structure)",
        "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_docking_jobs_config_gin ON docking_jobs USING GIN(config)",
        "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_docking_jobs_results_gin ON docking_jobs USING GIN(results)",
    ];
    
    for index_query in indexes {
        if let Err(e) = sqlx::query(index_query).execute(pool).await {
            // Los índices CONCURRENTLY pueden fallar si ya existen, logging pero no error crítico
            warn!("Index creation failed (possibly already exists): {}", e);
        }
    }
    
    Ok(())
}

/// Crear triggers para updated_at automático
async fn create_database_triggers(pool: &PgPool) -> anyhow::Result<()> {
    // Función de trigger para updated_at
    sqlx::query(r#"
        CREATE OR REPLACE FUNCTION update_updated_at_column()
        RETURNS TRIGGER AS $$
        BEGIN
            NEW.updated_at = NOW();
            RETURN NEW;
        END;
        $$ language 'plpgsql'
    "#)
    .execute(pool)
    .await?;
    
    // Triggers para cada tabla
    let triggers = vec![
        "DROP TRIGGER IF EXISTS update_protein_files_updated_at ON protein_files",
        "CREATE TRIGGER update_protein_files_updated_at BEFORE UPDATE ON protein_files FOR EACH ROW EXECUTE FUNCTION update_updated_at_column()",
        
        "DROP TRIGGER IF EXISTS update_docking_jobs_updated_at ON docking_jobs", 
        "CREATE TRIGGER update_docking_jobs_updated_at BEFORE UPDATE ON docking_jobs FOR EACH ROW EXECUTE FUNCTION update_updated_at_column()",
    ];
    
    for trigger_query in triggers {
        sqlx::query(trigger_query).execute(pool).await?;
    }
    
    Ok(())
}

/// Estadísticas de performance de base de datos
pub async fn get_database_stats() -> anyhow::Result<DatabaseStats> {
    let pool = get_database_pool()?;
    
    // Estadísticas de tablas
    let table_stats = sqlx::query_as::<_, TableStats>(r#"
        SELECT 
            schemaname,
            tablename,
            n_tup_ins as inserts,
            n_tup_upd as updates,
            n_tup_del as deletes,
            n_live_tup as live_tuples,
            n_dead_tup as dead_tuples
        FROM pg_stat_user_tables 
        WHERE schemaname = 'public'
        ORDER BY n_live_tup DESC
    "#)
    .fetch_all(pool)
    .await?;
    
    // Estadísticas de índices
    let index_stats = sqlx::query_as::<_, IndexStats>(r#"
        SELECT 
            schemaname,
            tablename,
            indexname,
            idx_tup_read as tuples_read,
            idx_tup_fetch as tuples_fetched
        FROM pg_stat_user_indexes 
        WHERE schemaname = 'public'
        ORDER BY idx_tup_read DESC
        LIMIT 20
    "#)
    .fetch_all(pool)
    .await?;
    
    Ok(DatabaseStats {
        connection_info: health_check().await?,
        table_stats,
        index_stats,
        generated_at: chrono::Utc::now(),
    })
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct TableStats {
    pub schemaname: String,
    pub tablename: String,
    pub inserts: i64,
    pub updates: i64,
    pub deletes: i64,
    pub live_tuples: i64,
    pub dead_tuples: i64,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct IndexStats {
    pub schemaname: String,
    pub tablename: String,
    pub indexname: String,
    pub tuples_read: i64,
    pub tuples_fetched: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct DatabaseStats {
    pub connection_info: DatabaseHealth,
    pub table_stats: Vec<TableStats>,
    pub index_stats: Vec<IndexStats>,
    pub generated_at: chrono::DateTime<chrono::Utc>,
}