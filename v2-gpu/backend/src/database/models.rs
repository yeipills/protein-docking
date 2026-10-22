// Database Models - Modelos ultra-optimizados para PostgreSQL
// Mapped types, queries eficientes, validaciones automáticas

use serde::{Serialize, Deserialize};
use sqlx::{PgPool, Row, FromRow};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::algorithms::{ProteinStructure, DockingConfig, DockingResult};
use crate::api::handlers::jobs::{JobStatus, JobPriority, ComputationStats};

/// Modelo de archivo de proteína en base de datos
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProteinFileModel {
    pub id: Uuid,
    pub file_id: String,
    pub original_filename: String,
    pub file_path: String,
    pub file_size: i64,
    pub content_type: String,
    pub upload_timestamp: DateTime<Utc>,
    pub protein_structure: Option<serde_json::Value>, // JSONB
    pub metadata: serde_json::Value, // JSONB
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ProteinFileModel {
    /// Crear nuevo archivo en base de datos
    pub async fn create(
        pool: &PgPool,
        file_id: String,
        original_filename: String,
        file_path: String,
        file_size: i64,
        content_type: String,
        protein_structure: Option<ProteinStructure>,
    ) -> anyhow::Result<Self> {
        let protein_structure_json = protein_structure
            .map(|s| serde_json::to_value(s))
            .transpose()?;
        
        let record = sqlx::query_as!(
            ProteinFileModel,
            r#"
            INSERT INTO protein_files (
                file_id, original_filename, file_path, file_size, 
                content_type, protein_structure
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING 
                id, file_id, original_filename, file_path, file_size,
                content_type, upload_timestamp, protein_structure,
                metadata, created_at, updated_at
            "#,
            file_id,
            original_filename,
            file_path,
            file_size,
            content_type,
            protein_structure_json
        )
        .fetch_one(pool)
        .await?;
        
        Ok(record)
    }
    
    /// Buscar archivo por file_id
    pub async fn find_by_file_id(pool: &PgPool, file_id: &str) -> anyhow::Result<Option<Self>> {
        let record = sqlx::query_as!(
            ProteinFileModel,
            r#"
            SELECT 
                id, file_id, original_filename, file_path, file_size,
                content_type, upload_timestamp, protein_structure,
                metadata, created_at, updated_at
            FROM protein_files 
            WHERE file_id = $1
            "#,
            file_id
        )
        .fetch_optional(pool)
        .await?;
        
        Ok(record)
    }
    
    /// Listar archivos con paginación
    pub async fn list_paginated(
        pool: &PgPool,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<Vec<Self>> {
        let records = sqlx::query_as!(
            ProteinFileModel,
            r#"
            SELECT 
                id, file_id, original_filename, file_path, file_size,
                content_type, upload_timestamp, protein_structure,
                metadata, created_at, updated_at
            FROM protein_files 
            ORDER BY upload_timestamp DESC
            LIMIT $1 OFFSET $2
            "#,
            limit,
            offset
        )
        .fetch_all(pool)
        .await?;
        
        Ok(records)
    }
    
    /// Eliminar archivo de base de datos
    pub async fn delete_by_file_id(pool: &PgPool, file_id: &str) -> anyhow::Result<bool> {
        let result = sqlx::query!(
            "DELETE FROM protein_files WHERE file_id = $1",
            file_id
        )
        .execute(pool)
        .await?;
        
        Ok(result.rows_affected() > 0)
    }
    
    /// Obtener estructura de proteína parseada
    pub fn get_protein_structure(&self) -> anyhow::Result<Option<ProteinStructure>> {
        if let Some(json) = &self.protein_structure {
            let structure: ProteinStructure = serde_json::from_value(json.clone())?;
            Ok(Some(structure))
        } else {
            Ok(None)
        }
    }
    
    /// Actualizar estructura de proteína
    pub async fn update_protein_structure(
        &mut self,
        pool: &PgPool,
        structure: ProteinStructure,
    ) -> anyhow::Result<()> {
        let structure_json = serde_json::to_value(structure)?;
        
        sqlx::query!(
            r#"
            UPDATE protein_files 
            SET protein_structure = $1
            WHERE file_id = $2
            "#,
            structure_json,
            self.file_id
        )
        .execute(pool)
        .await?;
        
        self.protein_structure = Some(structure_json);
        Ok(())
    }
}

/// Modelo de job de docking en base de datos
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DockingJobModel {
    pub id: Uuid,
    pub job_id: String,
    pub user_id: Option<String>,
    pub receptor_file_id: String,
    pub ligand_file_id: String,
    pub config: serde_json::Value, // JSONB
    pub status: String,
    pub priority: String,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    pub progress_percent: f32,
    pub current_step: Option<String>,
    pub estimated_completion: Option<DateTime<Utc>>,
    pub results: Option<serde_json::Value>, // JSONB
    pub error_message: Option<String>,
    pub computation_stats: Option<serde_json::Value>, // JSONB
}

impl DockingJobModel {
    /// Crear nuevo job en base de datos
    pub async fn create(
        pool: &PgPool,
        job_id: String,
        user_id: Option<String>,
        receptor_file_id: String,
        ligand_file_id: String,
        config: DockingConfig,
        priority: JobPriority,
    ) -> anyhow::Result<Self> {
        let config_json = serde_json::to_value(config)?;
        let priority_str = match priority {
            JobPriority::Low => "low",
            JobPriority::Normal => "normal", 
            JobPriority::High => "high",
            JobPriority::Critical => "critical",
        };
        
        let record = sqlx::query_as!(
            DockingJobModel,
            r#"
            INSERT INTO docking_jobs (
                job_id, user_id, receptor_file_id, ligand_file_id,
                config, priority
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING 
                id, job_id, user_id, receptor_file_id, ligand_file_id,
                config, status, priority, created_at, started_at, completed_at,
                updated_at, progress_percent, current_step, estimated_completion,
                results, error_message, computation_stats
            "#,
            job_id,
            user_id,
            receptor_file_id,
            ligand_file_id,
            config_json,
            priority_str
        )
        .fetch_one(pool)
        .await?;
        
        Ok(record)
    }
    
    /// Buscar job por job_id
    pub async fn find_by_job_id(pool: &PgPool, job_id: &str) -> anyhow::Result<Option<Self>> {
        let record = sqlx::query_as!(
            DockingJobModel,
            r#"
            SELECT 
                id, job_id, user_id, receptor_file_id, ligand_file_id,
                config, status, priority, created_at, started_at, completed_at,
                updated_at, progress_percent, current_step, estimated_completion,
                results, error_message, computation_stats
            FROM docking_jobs 
            WHERE job_id = $1
            "#,
            job_id
        )
        .fetch_optional(pool)
        .await?;
        
        Ok(record)
    }
    
    /// Listar jobs con filtros
    pub async fn list_filtered(
        pool: &PgPool,
        user_id: Option<&str>,
        status: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<Vec<Self>> {
        let records = if let Some(uid) = user_id {
            if let Some(status_filter) = status {
                sqlx::query_as!(
                    DockingJobModel,
                    r#"
                    SELECT 
                        id, job_id, user_id, receptor_file_id, ligand_file_id,
                        config, status, priority, created_at, started_at, completed_at,
                        updated_at, progress_percent, current_step, estimated_completion,
                        results, error_message, computation_stats
                    FROM docking_jobs 
                    WHERE user_id = $1 AND status = $2
                    ORDER BY created_at DESC
                    LIMIT $3 OFFSET $4
                    "#,
                    uid,
                    status_filter,
                    limit,
                    offset
                )
                .fetch_all(pool)
                .await?
            } else {
                sqlx::query_as!(
                    DockingJobModel,
                    r#"
                    SELECT 
                        id, job_id, user_id, receptor_file_id, ligand_file_id,
                        config, status, priority, created_at, started_at, completed_at,
                        updated_at, progress_percent, current_step, estimated_completion,
                        results, error_message, computation_stats
                    FROM docking_jobs 
                    WHERE user_id = $1
                    ORDER BY created_at DESC
                    LIMIT $2 OFFSET $3
                    "#,
                    uid,
                    limit,
                    offset
                )
                .fetch_all(pool)
                .await?
            }
        } else if let Some(status_filter) = status {
            sqlx::query_as!(
                DockingJobModel,
                r#"
                SELECT 
                    id, job_id, user_id, receptor_file_id, ligand_file_id,
                    config, status, priority, created_at, started_at, completed_at,
                    updated_at, progress_percent, current_step, estimated_completion,
                    results, error_message, computation_stats
                FROM docking_jobs 
                WHERE status = $1
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
                "#,
                status_filter,
                limit,
                offset
            )
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query_as!(
                DockingJobModel,
                r#"
                SELECT 
                    id, job_id, user_id, receptor_file_id, ligand_file_id,
                    config, status, priority, created_at, started_at, completed_at,
                    updated_at, progress_percent, current_step, estimated_completion,
                    results, error_message, computation_stats
                FROM docking_jobs 
                ORDER BY created_at DESC
                LIMIT $1 OFFSET $2
                "#,
                limit,
                offset
            )
            .fetch_all(pool)
            .await?
        };
        
        Ok(records)
    }
    
    /// Obtener jobs en cola por prioridad
    pub async fn get_queued_jobs_by_priority(pool: &PgPool) -> anyhow::Result<Vec<Self>> {
        let records = sqlx::query_as!(
            DockingJobModel,
            r#"
            SELECT 
                id, job_id, user_id, receptor_file_id, ligand_file_id,
                config, status, priority, created_at, started_at, completed_at,
                updated_at, progress_percent, current_step, estimated_completion,
                results, error_message, computation_stats
            FROM docking_jobs 
            WHERE status = 'queued'
            ORDER BY 
                CASE priority
                    WHEN 'critical' THEN 1
                    WHEN 'high' THEN 2
                    WHEN 'normal' THEN 3
                    WHEN 'low' THEN 4
                END,
                created_at ASC
            "#
        )
        .fetch_all(pool)
        .await?;
        
        Ok(records)
    }
    
    /// Actualizar estado del job
    pub async fn update_status(
        &mut self,
        pool: &PgPool,
        new_status: JobStatus,
        current_step: Option<String>,
        progress_percent: Option<f32>,
    ) -> anyhow::Result<()> {
        let status_str = match new_status {
            JobStatus::Queued => "queued",
            JobStatus::Running => "running",
            JobStatus::Completed => "completed",
            JobStatus::Failed => "failed",
            JobStatus::Cancelled => "cancelled",
        };
        
        let started_at = if matches!(new_status, JobStatus::Running) && self.started_at.is_none() {
            Some(Utc::now())
        } else {
            self.started_at
        };
        
        let completed_at = if matches!(new_status, JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled) {
            Some(Utc::now())
        } else {
            self.completed_at
        };
        
        sqlx::query!(
            r#"
            UPDATE docking_jobs 
            SET status = $1, current_step = $2, progress_percent = $3,
                started_at = $4, completed_at = $5
            WHERE job_id = $6
            "#,
            status_str,
            current_step,
            progress_percent.unwrap_or(self.progress_percent),
            started_at,
            completed_at,
            self.job_id
        )
        .execute(pool)
        .await?;
        
        self.status = status_str.to_string();
        self.current_step = current_step;
        if let Some(progress) = progress_percent {
            self.progress_percent = progress;
        }
        self.started_at = started_at;
        self.completed_at = completed_at;
        
        Ok(())
    }
    
    /// Actualizar resultados del job
    pub async fn update_results(
        &mut self,
        pool: &PgPool,
        results: Vec<DockingResult>,
        computation_stats: ComputationStats,
    ) -> anyhow::Result<()> {
        let results_json = serde_json::to_value(results)?;
        let stats_json = serde_json::to_value(computation_stats)?;
        
        sqlx::query!(
            r#"
            UPDATE docking_jobs 
            SET results = $1, computation_stats = $2, status = 'completed',
                completed_at = NOW(), progress_percent = 100.0
            WHERE job_id = $3
            "#,
            results_json,
            stats_json,
            self.job_id
        )
        .execute(pool)
        .await?;
        
        self.results = Some(results_json);
        self.computation_stats = Some(stats_json);
        self.status = "completed".to_string();
        self.completed_at = Some(Utc::now());
        self.progress_percent = 100.0;
        
        Ok(())
    }
    
    /// Marcar job como fallido
    pub async fn mark_failed(
        &mut self,
        pool: &PgPool,
        error_message: String,
    ) -> anyhow::Result<()> {
        sqlx::query!(
            r#"
            UPDATE docking_jobs 
            SET status = 'failed', error_message = $1, completed_at = NOW()
            WHERE job_id = $2
            "#,
            error_message,
            self.job_id
        )
        .execute(pool)
        .await?;
        
        self.status = "failed".to_string();
        self.error_message = Some(error_message);
        self.completed_at = Some(Utc::now());
        
        Ok(())
    }
    
    /// Convertir a modelo de dominio
    pub fn to_domain_model(&self) -> anyhow::Result<crate::api::handlers::jobs::DockingJob> {
        let config: DockingConfig = serde_json::from_value(self.config.clone())?;
        
        let status = match self.status.as_str() {
            "queued" => JobStatus::Queued,
            "running" => JobStatus::Running,
            "completed" => JobStatus::Completed,
            "failed" => JobStatus::Failed,
            "cancelled" => JobStatus::Cancelled,
            _ => JobStatus::Queued,
        };
        
        let results = if let Some(results_json) = &self.results {
            Some(serde_json::from_value(results_json.clone())?)
        } else {
            None
        };
        
        let computation_stats = if let Some(stats_json) = &self.computation_stats {
            Some(serde_json::from_value(stats_json.clone())?)
        } else {
            None
        };
        
        Ok(crate::api::handlers::jobs::DockingJob {
            job_id: self.job_id.clone(),
            user_id: self.user_id.clone(),
            receptor_file_id: self.receptor_file_id.clone(),
            ligand_file_id: self.ligand_file_id.clone(),
            config,
            status,
            created_at: self.created_at,
            started_at: self.started_at,
            completed_at: self.completed_at,
            progress_percent: self.progress_percent,
            current_step: self.current_step.clone(),
            estimated_completion: self.estimated_completion,
            results,
            error_message: self.error_message.clone(),
            computation_stats,
        })
    }
}

/// Modelo de resultados detallados
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DockingResultModel {
    pub id: Uuid,
    pub job_id: String,
    pub pose_id: i32,
    pub score: f32,
    pub rmsd: Option<f32>,
    pub transformation: serde_json::Value, // JSONB
    pub energy_breakdown: serde_json::Value, // JSONB
    pub computation_time_ms: i64,
    pub iterations_used: Option<i32>,
    pub convergence_reached: bool,
    pub created_at: DateTime<Utc>,
}

impl DockingResultModel {
    /// Crear resultado en batch
    pub async fn create_batch(
        pool: &PgPool,
        job_id: &str,
        results: Vec<DockingResult>,
    ) -> anyhow::Result<()> {
        let mut tx = pool.begin().await?;
        
        for result in results {
            let transformation_json = serde_json::to_value(result.transformation)?;
            let energy_breakdown_json = serde_json::to_value(result.energy_breakdown)?;
            
            sqlx::query!(
                r#"
                INSERT INTO docking_results (
                    job_id, pose_id, score, rmsd, transformation,
                    energy_breakdown, computation_time_ms, iterations_used,
                    convergence_reached
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                "#,
                job_id,
                result.pose_id,
                result.score,
                result.rmsd,
                transformation_json,
                energy_breakdown_json,
                result.computation_time_ms as i64,
                result.iterations_used,
                result.convergence_reached
            )
            .execute(&mut *tx)
            .await?;
        }
        
        tx.commit().await?;
        Ok(())
    }
    
    /// Obtener mejores resultados para un job
    pub async fn get_top_results(
        pool: &PgPool,
        job_id: &str,
        limit: i64,
    ) -> anyhow::Result<Vec<DockingResult>> {
        let records = sqlx::query_as!(
            DockingResultModel,
            r#"
            SELECT 
                id, job_id, pose_id, score, rmsd, transformation,
                energy_breakdown, computation_time_ms, iterations_used,
                convergence_reached, created_at
            FROM docking_results 
            WHERE job_id = $1
            ORDER BY score ASC
            LIMIT $2
            "#,
            job_id,
            limit
        )
        .fetch_all(pool)
        .await?;
        
        let mut results = Vec::new();
        for record in records {
            let transformation = serde_json::from_value(record.transformation)?;
            let energy_breakdown = serde_json::from_value(record.energy_breakdown)?;
            
            results.push(DockingResult {
                pose_id: record.pose_id as u32,
                score: record.score,
                rmsd: record.rmsd,
                transformation,
                energy_breakdown,
                computation_time_ms: record.computation_time_ms as u64,
                iterations_used: record.iterations_used.map(|i| i as u32),
                convergence_reached: record.convergence_reached,
            });
        }
        
        Ok(results)
    }
}