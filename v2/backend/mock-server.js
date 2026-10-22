const express = require('express');
const cors = require('cors');
const multer = require('multer');
const fs = require('fs').promises;
const path = require('path');
const WebSocket = require('ws');

const app = express();
const PORT = process.env.PORT || 8080;

// Middleware
app.use(cors());
app.use(express.json());

// File upload configuration
const upload = multer({
    dest: '/app/uploads/',
    limits: { fileSize: 500 * 1024 * 1024 } // 500MB
});

// In-memory job storage
const jobs = new Map();
const files = new Map();

// Initialize with sample files
const SAMPLE_FILES = [
    { id: '2f6d', filename: '2f6d.pdb', atoms: 4775, size_mb: 0.5 },
    { id: '4jet', filename: '4jet.pdb', atoms: 14784, size_mb: 1.2 },
    { id: '6izr', filename: '6izr.pdb', atoms: 7952, size_mb: 0.8 },
    { id: '6qtl', filename: '6qtl.pdb', atoms: 21304, size_mb: 2.1 },
    { id: '8e5d', filename: '8e5d.pdb', atoms: 84360, size_mb: 8.7 }
];

SAMPLE_FILES.forEach(f => {
    files.set(f.id, {
        ...f,
        uploaded_at: new Date().toISOString()
    });
});

// Health check
app.get('/health', (req, res) => {
    res.json({
        status: 'healthy',
        version: '2.0.0-node-mock',
        timestamp: new Date().toISOString(),
        capabilities: {
            cpu_cores: require('os').cpus().length,
            simd_support: ['Mock SIMD'],
            memory_allocator: 'V8'
        }
    });
});

// Root endpoint
app.get('/', (req, res) => {
    res.json({
        message: '🧬 Protein Docking v2 - Mock Backend (Node.js)',
        status: 'active',
        performance_mode: 'DEVELOPMENT',
        features: {
            simd_vectorization: false,
            memory_pooling: false,
            parallel_processing: true,
            ultra_optimized: false
        },
        endpoints: {
            health: '/health',
            api: '/api/v1'
        }
    });
});

// Metrics endpoint
app.get('/metrics', (req, res) => {
    res.set('Content-Type', 'text/plain');
    res.send('# HELP backend_status Status of backend\n# TYPE backend_status gauge\nbackend_status 1\n');
});

// API Info
app.get('/api/v1', (req, res) => {
    res.json({
        api_version: 'v1',
        endpoints: {
            ping: '/api/v1/ping',
            files: '/api/v1/files',
            upload: '/api/v1/upload',
            jobs: '/api/v1/jobs',
            system: '/api/v1/system/stats'
        }
    });
});

// Ping
app.get('/api/v1/ping', (req, res) => {
    res.json({
        message: 'pong',
        timestamp: new Date().toISOString(),
        performance: 'MOCK'
    });
});

// List files
app.get('/api/v1/files', (req, res) => {
    const fileList = Array.from(files.values()).map(f => ({
        id: f.id,
        filename: f.filename,
        size_mb: f.size_mb,
        atoms: f.atoms,
        description: `Protein structure ${f.id}`,
        uploaded_at: f.uploaded_at
    }));

    res.json({
        success: true,
        files: fileList,
        total: fileList.length,
        timestamp: new Date().toISOString()
    });
});

// Upload file
app.post('/api/v1/upload', upload.single('file'), async (req, res) => {
    try {
        if (!req.file) {
            return res.status(400).json({
                success: false,
                error: 'No file uploaded'
            });
        }

        const fileId = Date.now().toString();
        const fileInfo = {
            id: fileId,
            filename: req.file.originalname,
            size_mb: (req.file.size / (1024 * 1024)).toFixed(2),
            atoms: Math.floor(Math.random() * 10000) + 1000,
            uploaded_at: new Date().toISOString()
        };

        files.set(fileId, fileInfo);

        res.json({
            success: true,
            file: fileInfo
        });
    } catch (error) {
        res.status(500).json({
            success: false,
            error: error.message
        });
    }
});

// Submit docking job
app.post('/api/v1/jobs', async (req, res) => {
    try {
        const { receptor_filename, ligand_filename, parameters } = req.body;

        if (!receptor_filename || !ligand_filename) {
            return res.status(400).json({
                success: false,
                error: 'receptor_filename and ligand_filename are required'
            });
        }

        const jobId = `job_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;

        const job = {
            id: jobId,
            receptor_filename,
            ligand_filename,
            parameters: parameters || {},
            status: 'pending',
            progress: 0,
            created_at: new Date().toISOString(),
            started_at: null,
            completed_at: null,
            result: null
        };

        jobs.set(jobId, job);

        // Simulate async processing
        setTimeout(() => processJob(jobId), 1000);

        res.json({
            success: true,
            job_id: jobId,
            message: 'Job submitted successfully'
        });
    } catch (error) {
        res.status(500).json({
            success: false,
            error: error.message
        });
    }
});

// Process job (simulation)
async function processJob(jobId) {
    const job = jobs.get(jobId);
    if (!job) return;

    job.status = 'running';
    job.started_at = new Date().toISOString();

    // Simulate progress
    for (let i = 0; i <= 100; i += 10) {
        await new Promise(resolve => setTimeout(resolve, 200));
        job.progress = i;
    }

    // Complete job
    job.status = 'completed';
    job.completed_at = new Date().toISOString();
    job.progress = 100;
    job.result = {
        poses_count: 10,
        best_pose: {
            id: 1,
            energy: -8.5 + Math.random() * 2,
            contacts: 15,
            clash_score: 0.2,
            rmsd: 1.5
        },
        computation_stats: {
            total_time_ms: 2000,
            parsing_time_ms: 100,
            docking_time_ms: 1800,
            scoring_time_ms: 100
        }
    };
}

// Get job status
app.get('/api/v1/jobs/:job_id', (req, res) => {
    const job = jobs.get(req.params.job_id);

    if (!job) {
        return res.status(404).json({
            success: false,
            error: 'Job not found'
        });
    }

    res.json({
        success: true,
        job
    });
});

// List all jobs
app.get('/api/v1/jobs', (req, res) => {
    const jobList = Array.from(jobs.values());

    res.json({
        success: true,
        jobs: jobList,
        total: jobList.length
    });
});

// Delete job
app.delete('/api/v1/jobs/:job_id', (req, res) => {
    const deleted = jobs.delete(req.params.job_id);

    if (!deleted) {
        return res.status(404).json({
            success: false,
            error: 'Job not found'
        });
    }

    res.json({
        success: true,
        message: 'Job deleted successfully'
    });
});

// Queue status
app.get('/api/v1/queue', (req, res) => {
    const jobList = Array.from(jobs.values());

    res.json({
        success: true,
        queue: {
            pending: jobList.filter(j => j.status === 'pending').length,
            running: jobList.filter(j => j.status === 'running').length,
            completed: jobList.filter(j => j.status === 'completed').length,
            failed: jobList.filter(j => j.status === 'failed').length,
            total: jobList.length
        }
    });
});

// System stats
app.get('/api/v1/system/stats', (req, res) => {
    const os = require('os');

    res.json({
        success: true,
        system: {
            cpu_cores: os.cpus().length,
            memory: {
                total_mb: (os.totalmem() / 1024 / 1024).toFixed(0),
                free_mb: (os.freemem() / 1024 / 1024).toFixed(0),
                used_mb: ((os.totalmem() - os.freemem()) / 1024 / 1024).toFixed(0)
            },
            uptime_seconds: os.uptime(),
            platform: os.platform(),
            node_version: process.version
        },
        timestamp: new Date().toISOString()
    });
});

// Start HTTP server
const server = app.listen(PORT, '0.0.0.0', () => {
    console.log('🚀 Mock Protein Docking Backend started');
    console.log(`📍 Server: http://0.0.0.0:${PORT}`);
    console.log('🧬 Performance mode: DEVELOPMENT (Mock)');
    console.log('⚡ Ready to accept requests');
});

// WebSocket server for real-time updates
const wss = new WebSocket.Server({ server, path: '/ws' });

wss.on('connection', (ws) => {
    console.log('WebSocket client connected');

    ws.send(JSON.stringify({
        type: 'connected',
        message: 'WebSocket connection established'
    }));

    // Send periodic updates
    const interval = setInterval(() => {
        if (ws.readyState === WebSocket.OPEN) {
            ws.send(JSON.stringify({
                type: 'stats',
                data: {
                    jobs_total: jobs.size,
                    timestamp: new Date().toISOString()
                }
            }));
        }
    }, 5000);

    ws.on('close', () => {
        clearInterval(interval);
        console.log('WebSocket client disconnected');
    });
});

console.log('🔌 WebSocket server ready at ws://0.0.0.0:' + PORT + '/ws');
