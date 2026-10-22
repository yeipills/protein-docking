-- PostgreSQL initialization script for Protein Docking v2
-- Creates database schema and initial tables

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Create schema for protein docking
CREATE SCHEMA IF NOT EXISTS protein_docking;

-- Set search path
SET search_path TO protein_docking, public;

-- Table: proteins
CREATE TABLE IF NOT EXISTS proteins (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    pdb_id VARCHAR(10) UNIQUE,
    filename VARCHAR(255) NOT NULL,
    title TEXT,
    atom_count INTEGER,
    chain_count INTEGER,
    residue_count INTEGER,
    molecular_weight DOUBLE PRECISION,
    file_path TEXT,
    file_size BIGINT,
    uploaded_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    metadata JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Table: docking_jobs
CREATE TABLE IF NOT EXISTS docking_jobs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    receptor_id UUID REFERENCES proteins(id) ON DELETE CASCADE,
    ligand_id UUID REFERENCES proteins(id) ON DELETE CASCADE,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    parameters JSONB,
    started_at TIMESTAMP WITH TIME ZONE,
    completed_at TIMESTAMP WITH TIME ZONE,
    error_message TEXT,
    progress DOUBLE PRECISION DEFAULT 0.0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Table: docking_results
CREATE TABLE IF NOT EXISTS docking_results (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    job_id UUID REFERENCES docking_jobs(id) ON DELETE CASCADE,
    pose_number INTEGER NOT NULL,
    energy DOUBLE PRECISION NOT NULL,
    clash_score DOUBLE PRECISION,
    rmsd DOUBLE PRECISION,
    transformation_matrix JSONB,
    contacts JSONB,
    energy_components JSONB,
    coordinates JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Table: computation_stats
CREATE TABLE IF NOT EXISTS computation_stats (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    job_id UUID REFERENCES docking_jobs(id) ON DELETE CASCADE,
    total_time_ms BIGINT,
    parsing_time_ms BIGINT,
    docking_time_ms BIGINT,
    scoring_time_ms BIGINT,
    memory_used_mb DOUBLE PRECISION,
    cpu_usage DOUBLE PRECISION,
    gpu_used BOOLEAN DEFAULT false,
    simd_used BOOLEAN DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Table: users (for future authentication)
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    username VARCHAR(100) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    is_active BOOLEAN DEFAULT true,
    is_admin BOOLEAN DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_proteins_pdb_id ON proteins(pdb_id);
CREATE INDEX IF NOT EXISTS idx_proteins_uploaded_at ON proteins(uploaded_at DESC);
CREATE INDEX IF NOT EXISTS idx_docking_jobs_status ON docking_jobs(status);
CREATE INDEX IF NOT EXISTS idx_docking_jobs_created_at ON docking_jobs(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_docking_results_job_id ON docking_results(job_id);
CREATE INDEX IF NOT EXISTS idx_docking_results_energy ON docking_results(energy);
CREATE INDEX IF NOT EXISTS idx_computation_stats_job_id ON computation_stats(job_id);

-- Update trigger function
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Apply update triggers
CREATE TRIGGER update_proteins_updated_at
    BEFORE UPDATE ON proteins
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_docking_jobs_updated_at
    BEFORE UPDATE ON docking_jobs
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Insert sample data for testing
INSERT INTO proteins (pdb_id, filename, title, atom_count, chain_count) VALUES
    ('2F6D', '2f6d.pdb', 'Sample Protein 2F6D', 4775, 2),
    ('4JET', '4jet.pdb', 'Sample Protein 4JET', 14784, 4),
    ('6IZR', '6izr.pdb', 'Sample Protein 6IZR', 7952, 2)
ON CONFLICT (pdb_id) DO NOTHING;

-- Grant permissions
GRANT ALL PRIVILEGES ON SCHEMA protein_docking TO protein_user;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA protein_docking TO protein_user;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA protein_docking TO protein_user;

-- Set default schema for protein_user
ALTER ROLE protein_user SET search_path TO protein_docking, public;

-- Create view for job statistics
CREATE OR REPLACE VIEW job_statistics AS
SELECT
    status,
    COUNT(*) as count,
    AVG(EXTRACT(EPOCH FROM (completed_at - started_at))) as avg_duration_seconds,
    MIN(created_at) as first_job,
    MAX(created_at) as last_job
FROM docking_jobs
WHERE status IN ('completed', 'failed')
GROUP BY status;

GRANT SELECT ON job_statistics TO protein_user;

-- Success message
DO $$
BEGIN
    RAISE NOTICE 'Protein Docking database initialized successfully!';
END $$;
