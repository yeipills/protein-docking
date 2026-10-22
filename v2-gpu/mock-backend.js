#!/usr/bin/env node

// Mock Backend Simple para Protein Docking v2
// Provee endpoints básicos para que el frontend funcione

const express = require('express');
const cors = require('cors');
const multer = require('multer');
const path = require('path');
const fs = require('fs');
const http = require('http');
const WebSocket = require('ws');

const app = express();
const upload = multer({ dest: 'uploads/' });

// Middleware - CORS configurado para desarrollo
app.use(cors({
  origin: ['http://localhost:3000', 'http://127.0.0.1:3000'],
  credentials: true,
  methods: ['GET', 'POST', 'PUT', 'DELETE', 'OPTIONS'],
  allowedHeaders: ['Content-Type', 'Authorization', 'X-Requested-With']
}));
app.use(express.json());
app.use(express.static('uploads'));

// Crear directorio uploads si no existe
if (!fs.existsSync('uploads')) {
    fs.mkdirSync('uploads');
}

// Mock data
let proteins = [];
let jobs = [];
let jobId = 1;

// Health check
app.get('/health', (req, res) => {
    res.json({ 
        status: 'healthy', 
        timestamp: new Date().toISOString(),
        service: 'protein-docking-mock-backend',
        version: '2.0.0',
        capabilities: {
            cpu_cores: 12,
            simd_support: ['AVX2', 'SSE2'],
            memory_allocator: 'mimalloc',
            gpu_acceleration: true,
            parallel_docking: true
        }
    });
});

// API v1 routes
app.get('/api/v1/health', (req, res) => {
    res.json({ 
        status: 'healthy', 
        timestamp: new Date().toISOString(),
        service: 'protein-docking-mock-backend',
        version: '2.0.0',
        capabilities: {
            cpu_cores: 12,
            simd_support: ['AVX2', 'SSE2'],
            memory_allocator: 'mimalloc',
            gpu_acceleration: true,
            parallel_docking: true
        }
    });
});

app.get('/api/v1/ping', (req, res) => {
    res.json({ 
        message: 'pong',
        timestamp: new Date().toISOString(),
        performance: '< 1ms'
    });
});

// Upload endpoint (usado por el frontend)
app.post('/api/v1/upload', upload.single('file'), (req, res) => {
    if (!req.file) {
        return res.status(400).json({ error: 'No file uploaded' });
    }

    const fileId = 'file_' + (proteins.length + 1);
    const protein = {
        id: proteins.length + 1,
        name: req.file.originalname,
        filename: req.file.filename,
        size: req.file.size,
        uploaded_at: new Date().toISOString(),
        pdb_data: "MOCK PDB DATA - " + req.file.originalname,
        file_type: 'pdb',
        status: 'uploaded',
        file_id: fileId
    };

    proteins.push(protein);
    
    // Respuesta con la estructura esperada por el frontend
    res.json({
        file_id: fileId,
        filename: req.file.originalname,
        file_size: req.file.size,
        protein_info: {
            name: req.file.originalname.replace('.pdb', ''),
            pdb_id: null,
            total_atoms: Math.floor(Math.random() * 5000) + 1000,
            total_residues: Math.floor(Math.random() * 500) + 100,
            chains: Math.floor(Math.random() * 4) + 1,
            molecular_weight: Math.random() * 50000 + 10000,
            center_of_mass: [Math.random() * 10, Math.random() * 10, Math.random() * 10],
            bounding_box_size: [Math.random() * 20 + 10, Math.random() * 20 + 10, Math.random() * 20 + 10]
        },
        upload_time_ms: Math.floor(Math.random() * 1000) + 100,
        status: 'uploaded'
    });
});

// Proteins endpoints
app.get('/api/v1/proteins', (req, res) => {
    res.json(proteins);
});

app.post('/api/v1/proteins/upload', upload.single('file'), (req, res) => {
    if (!req.file) {
        return res.status(400).json({ error: 'No file uploaded' });
    }

    const protein = {
        id: proteins.length + 1,
        name: req.file.originalname,
        filename: req.file.filename,
        size: req.file.size,
        uploaded_at: new Date().toISOString(),
        pdb_data: "MOCK PDB DATA - " + req.file.originalname
    };

    proteins.push(protein);
    
    res.json({
        message: 'Protein uploaded successfully',
        protein: protein
    });
});

// Jobs endpoints
app.get('/api/v1/jobs', (req, res) => {
    res.json({
        jobs: jobs,
        total: jobs.length
    });
});

// System stats endpoint
app.get('/api/v1/system/stats', (req, res) => {
    res.json({
        cpu_usage: Math.random() * 100,
        memory_usage: Math.random() * 8192,
        gpu_usage: Math.random() * 100,
        system_load_percent: Math.random() * 100,
        disk_usage: Math.random() * 1024,
        network_io: {
            bytes_sent: Math.floor(Math.random() * 1000000),
            bytes_received: Math.floor(Math.random() * 1000000)
        },
        active_jobs: jobs.filter(j => j.status === 'running').length,
        completed_jobs: jobs.filter(j => j.status === 'completed').length,
        total_jobs: jobs.length,
        uptime: Math.floor(Date.now() / 1000),
        version: '2.0.0-mock',
        processes: Math.floor(Math.random() * 200) + 50,
        threads: Math.floor(Math.random() * 500) + 100
    });
});

app.post('/api/v1/jobs', (req, res) => {
    const { protein_a_id, protein_b_id, parameters } = req.body;
    
    const job = {
        id: jobId++,
        protein_a_id,
        protein_b_id,
        status: 'running',
        progress: 0,
        parameters: parameters || {},
        created_at: new Date().toISOString(),
        estimated_completion: new Date(Date.now() + 5 * 60 * 1000).toISOString()
    };

    jobs.push(job);

    // Simular progreso del job
    simulateJobProgress(job);

    res.json({
        message: 'Docking job started',
        job: job
    });
});

app.get('/api/v1/jobs/:id', (req, res) => {
    const job = jobs.find(j => j.id === parseInt(req.params.id));
    if (!job) {
        return res.status(404).json({ error: 'Job not found' });
    }
    res.json(job);
});

// Simular progreso de job
function simulateJobProgress(job) {
    const interval = setInterval(() => {
        job.progress += Math.random() * 20;
        
        if (job.progress >= 100) {
            job.progress = 100;
            job.status = 'completed';
            job.completed_at = new Date().toISOString();
            job.result = {
                binding_affinity: -8.5 + Math.random() * 5,
                rmsd: Math.random() * 3,
                score: Math.random() * 100,
                coordinates: generateMockCoordinates()
            };
            clearInterval(interval);
        }
    }, 2000);
}

// Generar coordenadas mock para visualización
function generateMockCoordinates() {
    const coords = [];
    for (let i = 0; i < 50; i++) {
        coords.push({
            x: Math.random() * 20 - 10,
            y: Math.random() * 20 - 10, 
            z: Math.random() * 20 - 10,
            element: Math.random() > 0.5 ? 'C' : 'N'
        });
    }
    return coords;
}

// Files endpoints
app.get('/api/v1/files', (req, res) => {
    res.json({
        files: proteins.map(p => ({
            file_id: p.file_id || `file_${p.id}`,
            filename: p.name,
            file_size: p.size,
            content_type: 'chemical/x-pdb',
            upload_timestamp: p.uploaded_at,
            protein_info: {
                name: p.name.replace('.pdb', ''),
                pdb_id: null,
                total_atoms: Math.floor(Math.random() * 5000) + 1000,
                total_residues: Math.floor(Math.random() * 500) + 100,
                chains: Math.floor(Math.random() * 4) + 1,
                molecular_weight: Math.random() * 50000 + 10000,
                center_of_mass: [Math.random() * 10, Math.random() * 10, Math.random() * 10],
                bounding_box_size: [Math.random() * 20 + 10, Math.random() * 20 + 10, Math.random() * 20 + 10]
            },
            status: p.status
        })),
        total: proteins.length
    });
});

// Docking endpoints
app.post('/api/v1/docking/dock', (req, res) => {
    const { proteinA, proteinB, parameters } = req.body;
    
    const job = {
        id: jobId++,
        protein_a: proteinA,
        protein_b: proteinB,
        status: 'processing',
        progress: 0,
        parameters: parameters || {},
        created_at: new Date().toISOString()
    };

    jobs.push(job);
    simulateJobProgress(job);

    res.json({
        message: 'Docking analysis started',
        job_id: job.id,
        estimated_time: '5 minutes'
    });
});

// Algorithms endpoints
app.get('/api/v1/algorithms', (req, res) => {
    res.json([
        {
            name: 'AutoDock Vina',
            version: '1.2.0',
            description: 'Molecular docking and virtual screening',
            supported: true
        },
        {
            name: 'SIMD Accelerated',
            version: '2.0.0', 
            description: 'Custom SIMD-optimized docking algorithm',
            supported: true
        },
        {
            name: 'GPU CUDA',
            version: '12.1',
            description: 'CUDA-accelerated molecular docking',
            supported: false
        }
    ]);
});

// Metrics endpoint
app.get('/metrics', (req, res) => {
    res.set('Content-Type', 'text/plain');
    res.send(`# HELP protein_docking_jobs_total Total number of docking jobs
# TYPE protein_docking_jobs_total counter
protein_docking_jobs_total ${jobs.length}

# HELP protein_docking_proteins_total Total number of uploaded proteins  
# TYPE protein_docking_proteins_total counter
protein_docking_proteins_total ${proteins.length}

# HELP protein_docking_jobs_running Currently running jobs
# TYPE protein_docking_jobs_running gauge
protein_docking_jobs_running ${jobs.filter(j => j.status === 'running').length}
`);
});

// Error handling
app.use((err, req, res, next) => {
    console.error(err.stack);
    res.status(500).json({ error: 'Something went wrong!' });
});

const PORT = process.env.PORT || 8080;

// Create HTTP server
const server = http.createServer(app);

// Create WebSocket server
const wss = new WebSocket.Server({ 
    server,
    path: '/ws/jobs'
});

// WebSocket handling
wss.on('connection', function connection(ws) {
    console.log('🔌 WebSocket client connected');
    
    // Send welcome message
    ws.send(JSON.stringify({
        type: 'Subscribed',
        data: { message: 'Connected to protein docking WebSocket' }
    }));
    
    // Handle messages
    ws.on('message', function incoming(message) {
        console.log('📨 WebSocket message:', message.toString());
    });
    
    // Handle disconnect
    ws.on('close', function close() {
        console.log('🔌 WebSocket client disconnected');
    });
    
    // Send periodic job updates (for demo)
    const interval = setInterval(() => {
        if (ws.readyState === WebSocket.OPEN) {
            const update = {
                type: 'JobProgress',
                data: {
                    job_id: 'demo-' + Math.floor(Math.random() * 1000),
                    progress_percent: Math.floor(Math.random() * 100),
                    status: Math.random() > 0.5 ? 'running' : 'completed',
                    current_step: 'Processing molecules'
                }
            };
            ws.send(JSON.stringify(update));
        }
    }, 5000);
    
    // Clean up interval on disconnect
    ws.on('close', () => {
        clearInterval(interval);
    });
});

server.listen(PORT, '0.0.0.0', () => {
    console.log(`🧬 Protein Docking Mock Backend started on port ${PORT}`);
    console.log(`📊 Health check: http://localhost:${PORT}/health`);
    console.log(`🔗 API: http://localhost:${PORT}/api/v1/`);
    console.log(`📈 Metrics: http://localhost:${PORT}/metrics`);
    console.log(`🔌 WebSocket: ws://localhost:${PORT}/ws/jobs`);
});

// Graceful shutdown
process.on('SIGTERM', () => {
    console.log('Received SIGTERM, shutting down gracefully');
    process.exit(0);
});

process.on('SIGINT', () => {
    console.log('Received SIGINT, shutting down gracefully');
    process.exit(0);
});