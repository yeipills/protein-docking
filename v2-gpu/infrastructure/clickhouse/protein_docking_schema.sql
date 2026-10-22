-- ClickHouse Schema for Protein Docking Analytics
-- Esquema ultra-optimizado para analytics de protein docking

-- Database creation
CREATE DATABASE IF NOT EXISTS protein_docking_analytics;
USE protein_docking_analytics;

-- Main jobs table with optimized partitioning
CREATE TABLE IF NOT EXISTS docking_jobs (
    job_id String,
    user_id String,
    created_at DateTime('UTC'),
    started_at Nullable(DateTime('UTC')),
    completed_at Nullable(DateTime('UTC')),
    status Enum8('pending' = 1, 'running' = 2, 'completed' = 3, 'failed' = 4),
    job_type Enum8('simple' = 1, 'complex' = 2, 'batch' = 3),
    protein_a_name String,
    protein_b_name String,
    protein_a_size UInt32,
    protein_b_size UInt32,
    computation_time_ms UInt64,
    memory_usage_mb UInt64,
    gpu_utilization_percent Float32,
    cpu_cores_used UInt8,
    result_score Nullable(Float64),
    result_rmsd Nullable(Float64),
    error_message Nullable(String),
    algorithm_version String,
    instance_id String,
    -- Partitioning by date for efficient querying
    date Date MATERIALIZED toDate(created_at)
) ENGINE = MergeTree()
PARTITION BY toYYYYMM(created_at)
ORDER BY (date, status, job_type, created_at)
TTL created_at + INTERVAL 90 DAY DELETE
SETTINGS index_granularity = 8192;

-- Performance metrics table
CREATE TABLE IF NOT EXISTS performance_metrics (
    timestamp DateTime('UTC'),
    instance_id String,
    metric_name String,
    metric_value Float64,
    metric_unit String,
    labels Map(String, String),
    -- Partitioning by hour for real-time analytics
    date Date MATERIALIZED toDate(timestamp),
    hour UInt8 MATERIALIZED toHour(timestamp)
) ENGINE = MergeTree()
PARTITION BY (date, hour)
ORDER BY (timestamp, instance_id, metric_name)
TTL timestamp + INTERVAL 30 DAY DELETE
SETTINGS index_granularity = 8192;

-- Algorithm performance table
CREATE TABLE IF NOT EXISTS algorithm_performance (
    timestamp DateTime('UTC'),
    job_id String,
    algorithm_name String,
    operation_type Enum8('simd_ops' = 1, 'spatial_search' = 2, 'gpu_kernel' = 3, 'memory_pool' = 4),
    execution_time_ms UInt64,
    operations_count UInt64,
    memory_allocated_mb UInt64,
    simd_instructions_used String,
    gpu_device_id Nullable(String),
    optimization_level Enum8('O0' = 0, 'O1' = 1, 'O2' = 2, 'O3' = 3),
    date Date MATERIALIZED toDate(timestamp)
) ENGINE = MergeTree()
PARTITION BY toYYYYMM(timestamp)
ORDER BY (date, algorithm_name, operation_type, timestamp)
TTL timestamp + INTERVAL 60 DAY DELETE
SETTINGS index_granularity = 8192;

-- System resources table
CREATE TABLE IF NOT EXISTS system_resources (
    timestamp DateTime('UTC'),
    instance_id String,
    cpu_utilization_percent Float32,
    memory_usage_mb UInt64,
    memory_total_mb UInt64,
    gpu_utilization_percent Float32,
    gpu_memory_used_mb UInt64,
    gpu_memory_total_mb UInt64,
    disk_io_read_mb_per_sec Float32,
    disk_io_write_mb_per_sec Float32,
    network_rx_mb_per_sec Float32,
    network_tx_mb_per_sec Float32,
    load_average_1min Float32,
    load_average_5min Float32,
    load_average_15min Float32,
    date Date MATERIALIZED toDate(timestamp),
    hour UInt8 MATERIALIZED toHour(timestamp)
) ENGINE = MergeTree()
PARTITION BY (date, hour)
ORDER BY (timestamp, instance_id)
TTL timestamp + INTERVAL 7 DAY DELETE
SETTINGS index_granularity = 8192;

-- Cache performance table for Redis analytics
CREATE TABLE IF NOT EXISTS cache_performance (
    timestamp DateTime('UTC'),
    redis_node String,
    cache_hits UInt64,
    cache_misses UInt64,
    hit_rate_percent Float32,
    memory_usage_mb UInt64,
    keys_count UInt64,
    commands_per_second UInt64,
    connected_clients UInt16,
    date Date MATERIALIZED toDate(timestamp)
) ENGINE = MergeTree()
PARTITION BY toYYYYMM(timestamp)
ORDER BY (date, redis_node, timestamp)
TTL timestamp + INTERVAL 30 DAY DELETE
SETTINGS index_granularity = 8192;

-- Materialized views for real-time analytics

-- Jobs throughput per hour
CREATE MATERIALIZED VIEW IF NOT EXISTS jobs_throughput_hourly
ENGINE = SummingMergeTree()
PARTITION BY toYYYYMM(hour_start)
ORDER BY (hour_start, status, job_type)
AS SELECT
    toStartOfHour(created_at) as hour_start,
    status,
    job_type,
    count() as jobs_count,
    avg(computation_time_ms) as avg_computation_time_ms,
    max(computation_time_ms) as max_computation_time_ms,
    avg(memory_usage_mb) as avg_memory_usage_mb,
    avg(gpu_utilization_percent) as avg_gpu_utilization
FROM docking_jobs
GROUP BY hour_start, status, job_type;

-- Algorithm performance summary
CREATE MATERIALIZED VIEW IF NOT EXISTS algorithm_performance_summary
ENGINE = SummingMergeTree()
PARTITION BY toYYYYMM(date)
ORDER BY (date, algorithm_name, operation_type)
AS SELECT
    toDate(timestamp) as date,
    algorithm_name,
    operation_type,
    count() as operations_count,
    avg(execution_time_ms) as avg_execution_time_ms,
    sum(operations_count) as total_operations,
    avg(memory_allocated_mb) as avg_memory_mb,
    quantile(0.95)(execution_time_ms) as p95_execution_time_ms
FROM algorithm_performance
GROUP BY date, algorithm_name, operation_type;

-- System resources summary per hour
CREATE MATERIALIZED VIEW IF NOT EXISTS system_resources_hourly
ENGINE = SummingMergeTree()
PARTITION BY toYYYYMM(hour_start)
ORDER BY (hour_start, instance_id)
AS SELECT
    toStartOfHour(timestamp) as hour_start,
    instance_id,
    avg(cpu_utilization_percent) as avg_cpu_percent,
    avg(memory_usage_mb) as avg_memory_mb,
    max(memory_usage_mb) as max_memory_mb,
    avg(gpu_utilization_percent) as avg_gpu_percent,
    max(gpu_utilization_percent) as max_gpu_percent,
    avg(load_average_1min) as avg_load_1min
FROM system_resources
GROUP BY hour_start, instance_id;

-- Useful queries for monitoring and analytics

-- Top performing algorithms by execution time
-- SELECT algorithm_name, avg(execution_time_ms) as avg_time
-- FROM algorithm_performance
-- WHERE timestamp >= now() - INTERVAL 1 DAY
-- GROUP BY algorithm_name
-- ORDER BY avg_time ASC;

-- Jobs success rate over time
-- SELECT 
--     toStartOfHour(created_at) as hour,
--     countIf(status = 'completed') * 100.0 / count() as success_rate_percent
-- FROM docking_jobs
-- WHERE created_at >= now() - INTERVAL 24 HOUR
-- GROUP BY hour
-- ORDER BY hour;

-- GPU utilization trend
-- SELECT 
--     toStartOfMinute(timestamp) as minute,
--     avg(gpu_utilization_percent) as avg_gpu_utilization
-- FROM system_resources
-- WHERE timestamp >= now() - INTERVAL 1 HOUR
-- GROUP BY minute
-- ORDER BY minute;

-- Cache hit rate analysis
-- SELECT 
--     redis_node,
--     avg(hit_rate_percent) as avg_hit_rate,
--     max(commands_per_second) as peak_commands_per_sec
-- FROM cache_performance
-- WHERE timestamp >= now() - INTERVAL 1 DAY
-- GROUP BY redis_node
-- ORDER BY avg_hit_rate DESC;