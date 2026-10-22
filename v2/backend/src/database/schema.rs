// Database Schema - Definiciones de esquema y queries optimizadas
// Tipos customizados, stored procedures, views materializadas

use sqlx::{PgPool, Row};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

/// Estadísticas agregadas de jobs
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct JobStatistics {
    pub total_jobs: i64,
    pub queued_jobs: i64,
    pub running_jobs: i64,
    pub completed_jobs: i64,
    pub failed_jobs: i64,
    pub cancelled_jobs: i64,
    pub avg_completion_time_seconds: Option<f64>,
    pub total_poses_evaluated: i64,
    pub avg_score: Option<f64>,
    pub best_score: Option<f64>,
    pub worst_score: Option<f64>,
}

/// Estadísticas de usuario
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct UserStatistics {
    pub user_id: String,
    pub total_jobs: i64,
    pub completed_jobs: i64,
    pub failed_jobs: i64,
    pub total_computation_time_ms: i64,
    pub avg_score: Option<f64>,
    pub best_score: Option<f64>,
    pub files_uploaded: i64,
    pub last_activity: DateTime<Utc>,
}

/// Trending de performance temporal
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PerformanceTrend {
    pub date: chrono::NaiveDate,
    pub jobs_completed: i64,
    pub avg_completion_time_seconds: Option<f64>,
    pub avg_poses_per_second: Option<f64>,
    pub total_poses_evaluated: i64,
}

/// Schema de queries avanzadas
impl JobStatistics {
    /// Obtener estadísticas globales del sistema
    pub async fn get_global_stats(pool: &PgPool) -> anyhow::Result<Self> {
        let stats = sqlx::query_as!(
            JobStatistics,
            r#"
            WITH job_counts AS (
                SELECT 
                    COUNT(*) as total_jobs,
                    COUNT(*) FILTER (WHERE status = 'queued') as queued_jobs,
                    COUNT(*) FILTER (WHERE status = 'running') as running_jobs,
                    COUNT(*) FILTER (WHERE status = 'completed') as completed_jobs,
                    COUNT(*) FILTER (WHERE status = 'failed') as failed_jobs,
                    COUNT(*) FILTER (WHERE status = 'cancelled') as cancelled_jobs
                FROM docking_jobs
            ),
            completion_times AS (
                SELECT 
                    AVG(EXTRACT(EPOCH FROM (completed_at - started_at))) as avg_completion_time_seconds
                FROM docking_jobs 
                WHERE status = 'completed' 
                AND started_at IS NOT NULL 
                AND completed_at IS NOT NULL
            ),
            pose_stats AS (
                SELECT 
                    SUM((computation_stats->>'total_poses_evaluated')::bigint) as total_poses_evaluated,
                    AVG(score) as avg_score,
                    MIN(score) as best_score,
                    MAX(score) as worst_score
                FROM docking_jobs dj
                LEFT JOIN docking_results dr ON dj.job_id = dr.job_id
                WHERE dj.status = 'completed'
            )
            SELECT 
                jc.total_jobs,
                jc.queued_jobs,
                jc.running_jobs,
                jc.completed_jobs,
                jc.failed_jobs,
                jc.cancelled_jobs,
                ct.avg_completion_time_seconds,
                COALESCE(ps.total_poses_evaluated, 0) as total_poses_evaluated,
                ps.avg_score,
                ps.best_score,
                ps.worst_score
            FROM job_counts jc
            CROSS JOIN completion_times ct
            CROSS JOIN pose_stats ps
            "#
        )
        .fetch_one(pool)
        .await?;
        
        Ok(stats)
    }
    
    /// Obtener estadísticas por rango de tiempo
    pub async fn get_stats_by_timerange(
        pool: &PgPool,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> anyhow::Result<Self> {
        let stats = sqlx::query_as!(
            JobStatistics,
            r#"
            WITH job_counts AS (
                SELECT 
                    COUNT(*) as total_jobs,
                    COUNT(*) FILTER (WHERE status = 'queued') as queued_jobs,
                    COUNT(*) FILTER (WHERE status = 'running') as running_jobs,
                    COUNT(*) FILTER (WHERE status = 'completed') as completed_jobs,
                    COUNT(*) FILTER (WHERE status = 'failed') as failed_jobs,
                    COUNT(*) FILTER (WHERE status = 'cancelled') as cancelled_jobs
                FROM docking_jobs
                WHERE created_at BETWEEN $1 AND $2
            ),
            completion_times AS (
                SELECT 
                    AVG(EXTRACT(EPOCH FROM (completed_at - started_at))) as avg_completion_time_seconds
                FROM docking_jobs 
                WHERE status = 'completed' 
                AND started_at IS NOT NULL 
                AND completed_at IS NOT NULL
                AND created_at BETWEEN $1 AND $2
            ),
            pose_stats AS (
                SELECT 
                    SUM((computation_stats->>'total_poses_evaluated')::bigint) as total_poses_evaluated,
                    AVG(score) as avg_score,
                    MIN(score) as best_score,
                    MAX(score) as worst_score
                FROM docking_jobs dj
                LEFT JOIN docking_results dr ON dj.job_id = dr.job_id
                WHERE dj.status = 'completed'
                AND dj.created_at BETWEEN $1 AND $2
            )
            SELECT 
                jc.total_jobs,
                jc.queued_jobs,
                jc.running_jobs,
                jc.completed_jobs,
                jc.failed_jobs,
                jc.cancelled_jobs,
                ct.avg_completion_time_seconds,
                COALESCE(ps.total_poses_evaluated, 0) as total_poses_evaluated,
                ps.avg_score,
                ps.best_score,
                ps.worst_score
            FROM job_counts jc
            CROSS JOIN completion_times ct
            CROSS JOIN pose_stats ps
            "#,
            start_date,
            end_date
        )
        .fetch_one(pool)
        .await?;
        
        Ok(stats)
    }
}

impl UserStatistics {
    /// Obtener estadísticas por usuario
    pub async fn get_user_stats(
        pool: &PgPool,
        user_id: &str,
    ) -> anyhow::Result<Option<Self>> {
        let stats = sqlx::query_as!(
            UserStatistics,
            r#"
            WITH user_jobs AS (
                SELECT 
                    user_id,
                    COUNT(*) as total_jobs,
                    COUNT(*) FILTER (WHERE status = 'completed') as completed_jobs,
                    COUNT(*) FILTER (WHERE status = 'failed') as failed_jobs,
                    SUM(
                        CASE 
                            WHEN computation_stats IS NOT NULL 
                            THEN (computation_stats->>'computation_time_ms')::bigint 
                            ELSE 0 
                        END
                    ) as total_computation_time_ms,
                    MAX(created_at) as last_activity
                FROM docking_jobs 
                WHERE user_id = $1
                GROUP BY user_id
            ),
            user_scores AS (
                SELECT 
                    AVG(dr.score) as avg_score,
                    MIN(dr.score) as best_score
                FROM docking_jobs dj
                JOIN docking_results dr ON dj.job_id = dr.job_id
                WHERE dj.user_id = $1
                AND dj.status = 'completed'
            ),
            user_files AS (
                SELECT COUNT(*) as files_uploaded
                FROM protein_files pf
                WHERE EXISTS (
                    SELECT 1 FROM docking_jobs dj 
                    WHERE (dj.receptor_file_id = pf.file_id OR dj.ligand_file_id = pf.file_id)
                    AND dj.user_id = $1
                )
            )
            SELECT 
                uj.user_id,
                uj.total_jobs,
                uj.completed_jobs,
                uj.failed_jobs,
                uj.total_computation_time_ms,
                us.avg_score,
                us.best_score,
                uf.files_uploaded,
                uj.last_activity
            FROM user_jobs uj
            CROSS JOIN user_scores us
            CROSS JOIN user_files uf
            "#,
            user_id
        )
        .fetch_optional(pool)
        .await?;
        
        Ok(stats)
    }
    
    /// Obtener top usuarios por actividad
    pub async fn get_top_users(
        pool: &PgPool,
        limit: i64,
    ) -> anyhow::Result<Vec<Self>> {
        let users = sqlx::query_as!(
            UserStatistics,
            r#"
            WITH user_jobs AS (
                SELECT 
                    user_id,
                    COUNT(*) as total_jobs,
                    COUNT(*) FILTER (WHERE status = 'completed') as completed_jobs,
                    COUNT(*) FILTER (WHERE status = 'failed') as failed_jobs,
                    SUM(
                        CASE 
                            WHEN computation_stats IS NOT NULL 
                            THEN (computation_stats->>'computation_time_ms')::bigint 
                            ELSE 0 
                        END
                    ) as total_computation_time_ms,
                    MAX(created_at) as last_activity
                FROM docking_jobs 
                WHERE user_id IS NOT NULL
                GROUP BY user_id
            ),
            user_scores AS (
                SELECT 
                    dj.user_id,
                    AVG(dr.score) as avg_score,
                    MIN(dr.score) as best_score
                FROM docking_jobs dj
                JOIN docking_results dr ON dj.job_id = dr.job_id
                WHERE dj.user_id IS NOT NULL
                AND dj.status = 'completed'
                GROUP BY dj.user_id
            ),
            user_files AS (
                SELECT 
                    dj.user_id,
                    COUNT(DISTINCT pf.file_id) as files_uploaded
                FROM docking_jobs dj
                JOIN protein_files pf ON (dj.receptor_file_id = pf.file_id OR dj.ligand_file_id = pf.file_id)
                WHERE dj.user_id IS NOT NULL
                GROUP BY dj.user_id
            )
            SELECT 
                uj.user_id,
                uj.total_jobs,
                uj.completed_jobs,
                uj.failed_jobs,
                uj.total_computation_time_ms,
                COALESCE(us.avg_score, 0.0) as avg_score,
                us.best_score,
                COALESCE(uf.files_uploaded, 0) as files_uploaded,
                uj.last_activity
            FROM user_jobs uj
            LEFT JOIN user_scores us ON uj.user_id = us.user_id
            LEFT JOIN user_files uf ON uj.user_id = uf.user_id
            ORDER BY uj.total_jobs DESC, uj.completed_jobs DESC
            LIMIT $1
            "#,
            limit
        )
        .fetch_all(pool)
        .await?;
        
        Ok(users)
    }
}

impl PerformanceTrend {
    /// Obtener trending de performance por días
    pub async fn get_daily_trends(
        pool: &PgPool,
        days: i32,
    ) -> anyhow::Result<Vec<Self>> {
        let trends = sqlx::query_as!(
            PerformanceTrend,
            r#"
            WITH date_series AS (
                SELECT generate_series(
                    CURRENT_DATE - INTERVAL '%s days',
                    CURRENT_DATE,
                    INTERVAL '1 day'
                )::date as date
            ),
            daily_jobs AS (
                SELECT 
                    completed_at::date as date,
                    COUNT(*) as jobs_completed,
                    AVG(EXTRACT(EPOCH FROM (completed_at - started_at))) as avg_completion_time_seconds,
                    SUM((computation_stats->>'total_poses_evaluated')::bigint) as total_poses_evaluated,
                    AVG((computation_stats->>'poses_per_second')::numeric) as avg_poses_per_second
                FROM docking_jobs 
                WHERE status = 'completed'
                AND completed_at >= CURRENT_DATE - INTERVAL '%s days'
                AND started_at IS NOT NULL
                AND computation_stats IS NOT NULL
                GROUP BY completed_at::date
            )
            SELECT 
                ds.date,
                COALESCE(dj.jobs_completed, 0) as jobs_completed,
                dj.avg_completion_time_seconds,
                dj.avg_poses_per_second,
                COALESCE(dj.total_poses_evaluated, 0) as total_poses_evaluated
            FROM date_series ds
            LEFT JOIN daily_jobs dj ON ds.date = dj.date
            ORDER BY ds.date DESC
            "#,
            &days.to_string(),
            &days.to_string()
        )
        .fetch_all(pool)
        .await?;
        
        Ok(trends)
    }
}

/// Queries especializadas para performance
pub struct PerformanceQueries;

impl PerformanceQueries {
    /// Buscar jobs con scores similares (clustering)
    pub async fn find_similar_scores(
        pool: &PgPool,
        target_score: f32,
        tolerance: f32,
        limit: i64,
    ) -> anyhow::Result<Vec<String>> {
        let job_ids = sqlx::query_scalar!(
            r#"
            SELECT DISTINCT dj.job_id
            FROM docking_jobs dj
            JOIN docking_results dr ON dj.job_id = dr.job_id
            WHERE dr.score BETWEEN $1 AND $2
            AND dj.status = 'completed'
            ORDER BY ABS(dr.score - $3)
            LIMIT $4
            "#,
            target_score - tolerance,
            target_score + tolerance,
            target_score,
            limit
        )
        .fetch_all(pool)
        .await?;
        
        Ok(job_ids)
    }
    
    /// Buscar proteínas con características similares
    pub async fn find_similar_proteins(
        pool: &PgPool,
        target_molecular_weight: f64,
        weight_tolerance: f64,
        min_atoms: i32,
        max_atoms: i32,
    ) -> anyhow::Result<Vec<String>> {
        let file_ids = sqlx::query_scalar!(
            r#"
            SELECT file_id
            FROM protein_files
            WHERE protein_structure IS NOT NULL
            AND (protein_structure->>'molecular_weight')::numeric BETWEEN $1 AND $2
            AND (protein_structure->'atoms')::jsonb ? '0' -- Has at least one atom
            AND jsonb_array_length(protein_structure->'atoms') BETWEEN $3 AND $4
            ORDER BY ABS((protein_structure->>'molecular_weight')::numeric - $5)
            "#,
            target_molecular_weight - weight_tolerance,
            target_molecular_weight + weight_tolerance,
            min_atoms,
            max_atoms,
            target_molecular_weight
        )
        .fetch_all(pool)
        .await?;
        
        Ok(file_ids)
    }
    
    /// Análisis de convergencia de algoritmos
    pub async fn analyze_convergence_patterns(
        pool: &PgPool,
        min_jobs: i64,
    ) -> anyhow::Result<Vec<ConvergenceAnalysis>> {
        let analysis = sqlx::query_as!(
            ConvergenceAnalysis,
            r#"
            WITH convergence_stats AS (
                SELECT 
                    (dj.config->>'sampling_strategy')::text as strategy,
                    AVG(dr.iterations_used::numeric) as avg_iterations,
                    COUNT(*) as job_count,
                    AVG(dr.score) as avg_score,
                    COUNT(*) FILTER (WHERE dr.convergence_reached = true) as converged_count,
                    AVG(dr.computation_time_ms::numeric) as avg_computation_time_ms
                FROM docking_jobs dj
                JOIN docking_results dr ON dj.job_id = dr.job_id
                WHERE dj.status = 'completed'
                AND dr.iterations_used IS NOT NULL
                GROUP BY (dj.config->>'sampling_strategy')::text
                HAVING COUNT(*) >= $1
            )
            SELECT 
                strategy,
                avg_iterations,
                job_count,
                avg_score,
                converged_count,
                avg_computation_time_ms,
                (converged_count::float / job_count::float * 100.0) as convergence_rate_percent
            FROM convergence_stats
            ORDER BY convergence_rate_percent DESC, avg_score ASC
            "#,
            min_jobs
        )
        .fetch_all(pool)
        .await?;
        
        Ok(analysis)
    }
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ConvergenceAnalysis {
    pub strategy: Option<String>,
    pub avg_iterations: Option<f64>,
    pub job_count: i64,
    pub avg_score: Option<f64>,
    pub converged_count: i64,
    pub avg_computation_time_ms: Option<f64>,
    pub convergence_rate_percent: Option<f64>,
}

/// Views materializadas para performance
pub struct MaterializedViews;

impl MaterializedViews {
    /// Crear view de estadísticas agregadas (actualizada cada hora)
    pub async fn create_job_stats_view(pool: &PgPool) -> anyhow::Result<()> {
        sqlx::query(r#"
            CREATE MATERIALIZED VIEW IF NOT EXISTS mv_job_statistics AS
            SELECT 
                DATE_TRUNC('hour', created_at) as hour_bucket,
                COUNT(*) as total_jobs,
                COUNT(*) FILTER (WHERE status = 'completed') as completed_jobs,
                COUNT(*) FILTER (WHERE status = 'failed') as failed_jobs,
                AVG(progress_percent) as avg_progress,
                AVG(
                    CASE 
                        WHEN status = 'completed' AND started_at IS NOT NULL AND completed_at IS NOT NULL
                        THEN EXTRACT(EPOCH FROM (completed_at - started_at))
                        ELSE NULL
                    END
                ) as avg_completion_time_seconds
            FROM docking_jobs
            WHERE created_at >= NOW() - INTERVAL '30 days'
            GROUP BY DATE_TRUNC('hour', created_at)
            WITH DATA;
            
            CREATE UNIQUE INDEX IF NOT EXISTS idx_mv_job_statistics_hour 
            ON mv_job_statistics (hour_bucket);
        "#)
        .execute(pool)
        .await?;
        
        Ok(())
    }
    
    /// Refrescar views materializadas
    pub async fn refresh_materialized_views(pool: &PgPool) -> anyhow::Result<()> {
        sqlx::query("REFRESH MATERIALIZED VIEW CONCURRENTLY mv_job_statistics")
            .execute(pool)
            .await?;
        
        Ok(())
    }
}