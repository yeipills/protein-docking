const express = require('express');
const cors = require('cors');
const multer = require('multer');
const fs = require('fs');
const path = require('path');

const app = express();
const PORT = 3001;

// Middleware
app.use(cors({
  origin: ['http://localhost:3000', 'http://127.0.0.1:3000'],
  credentials: true,
  methods: ['GET', 'POST', 'PUT', 'DELETE', 'OPTIONS'],
  allowedHeaders: ['Content-Type', 'Authorization', 'X-Requested-With']
}));
app.use(express.json());

// Storage for uploaded files
const storage = multer.memoryStorage();
const upload = multer({ storage });

// In-memory storage for files and jobs
let files = [];
let fileIdCounter = 1;
let jobs = [];
let jobIdCounter = 1;

// Create uploads directory if it doesn't exist
const uploadsDir = path.join(__dirname, 'uploads');
if (!fs.existsSync(uploadsDir)) {
  fs.mkdirSync(uploadsDir);
}

// Health check
app.get('/api/health', (req, res) => {
  res.json({
    status: 'ok',
    version: '3.0.0',
    timestamp: new Date().toISOString()
  });
});

// Ping endpoint
app.get('/api/ping', (req, res) => {
  res.json({
    message: 'pong',
    timestamp: new Date().toISOString(),
    performance: '0.1ms'
  });
});

// Upload file
app.post('/api/upload', upload.single('file'), (req, res) => {
  try {
    const file = req.file;
    if (!file) {
      return res.status(400).json({ error: 'No file uploaded' });
    }

    const fileId = `file_${fileIdCounter++}`;
    const filename = file.originalname;
    const fileSize = file.size;
    
    // Save file to disk
    const filePath = path.join(uploadsDir, `${fileId}_${filename}`);
    fs.writeFileSync(filePath, file.buffer);

    // Mock protein info parsing
    const proteinInfo = {
      name: filename.replace(/\.[^/.]+$/, ''),
      pdb_id: null,
      total_atoms: Math.floor(Math.random() * 5000) + 100,
      total_residues: Math.floor(Math.random() * 500) + 10,
      chains: Math.floor(Math.random() * 4) + 1,
      has_ligands: Math.random() > 0.5,
      resolution: Math.random() * 2 + 1
    };

    const fileMetadata = {
      file_id: fileId,
      filename: filename,
      file_size: fileSize,
      upload_timestamp: new Date().toISOString(),
      protein_info: proteinInfo,
      status: 'processed'
    };

    files.push(fileMetadata);

    console.log(`✅ File uploaded: ${filename} (ID: ${fileId})`);

    res.json({
      success: true,
      file_id: fileId,
      filename: filename,
      file_size: fileSize,
      protein_info: proteinInfo
    });
  } catch (error) {
    console.error('Upload error:', error);
    res.status(500).json({ error: 'Upload failed' });
  }
});

// List files
app.get('/api/files', (req, res) => {
  const limit = parseInt(req.query.limit) || 100;
  const offset = parseInt(req.query.offset) || 0;
  
  const paginatedFiles = files.slice(offset, offset + limit);
  
  console.log(`📋 Files requested: ${paginatedFiles.length} of ${files.length} total`);
  
  res.json({
    success: true,
    files: paginatedFiles,
    total: files.length
  });
});

// Get specific file
app.get('/api/files/:fileId', (req, res) => {
  const file = files.find(f => f.file_id === req.params.fileId);
  if (!file) {
    return res.status(404).json({ error: 'File not found' });
  }
  res.json(file);
});

// Delete file
app.delete('/api/files/:fileId', (req, res) => {
  const fileIndex = files.findIndex(f => f.file_id === req.params.fileId);
  if (fileIndex === -1) {
    return res.status(404).json({ error: 'File not found' });
  }

  const file = files[fileIndex];
  files.splice(fileIndex, 1);

  // Delete file from disk
  const filePath = path.join(uploadsDir, `${file.file_id}_${file.filename}`);
  if (fs.existsSync(filePath)) {
    fs.unlinkSync(filePath);
  }

  console.log(`🗑️ File deleted: ${file.filename} (ID: ${file.file_id})`);

  res.json({ success: true });
});

// Download file
app.get('/api/files/:fileId/download', (req, res) => {
  const file = files.find(f => f.file_id === req.params.fileId);
  if (!file) {
    return res.status(404).json({ error: 'File not found' });
  }

  const filePath = path.join(uploadsDir, `${file.file_id}_${file.filename}`);
  if (!fs.existsSync(filePath)) {
    return res.status(404).json({ error: 'File not found on disk' });
  }

  res.download(filePath, file.filename);
});

// System stats
app.get('/api/system/stats', (req, res) => {
  res.json({
    memory: {
      total: 32 * 1024 * 1024 * 1024, // 32GB
      used: Math.floor(Math.random() * 16) * 1024 * 1024 * 1024,
      available: Math.floor(Math.random() * 16) * 1024 * 1024 * 1024
    },
    cpu: {
      cores: 16,
      usage: Math.random() * 100
    },
    gpu: {
      available: true,
      model: 'RTX 5090',
      memory_total: 24 * 1024 * 1024 * 1024,
      memory_used: Math.floor(Math.random() * 12) * 1024 * 1024 * 1024
    },
    disk: {
      total: 1024 * 1024 * 1024 * 1024, // 1TB
      used: Math.floor(Math.random() * 512) * 1024 * 1024 * 1024,
      available: Math.floor(Math.random() * 512) * 1024 * 1024 * 1024
    }
  });
});

// Jobs endpoints (mock)
app.get('/api/jobs', (req, res) => {
  console.log('📋 Mock Server: GET /api/jobs - Returning jobs:', jobs.length);
  res.json({
    success: true,
    jobs: jobs,
    total: jobs.length
  });
});

app.post('/api/jobs', (req, res) => {
  const jobId = `job_${Date.now()}`;
  const newJob = {
    job_id: jobId,
    status: 'queued',
    progress_percent: 0,
    current_step: 'Initializing',
    created_at: new Date().toISOString(),
    poses_evaluated: 0,
    poses_remaining: 1000,
    receptor_file_id: req.body.receptor_file_id,
    ligand_file_id: req.body.ligand_file_id,
    config: req.body.config || {}
  };
  
  jobs.push(newJob);
  console.log('🚀 Mock Server: Created job:', jobId, 'Total jobs:', jobs.length);
  
  // Simulate job progress
  simulateJobProgress(jobId);
  
  res.json({
    success: true,
    job_id: jobId,
    status: 'queued',
    estimated_duration_seconds: 300,
    queue_position: jobs.length,
    websocket_url: `ws://localhost:3001/ws/jobs/${jobId}`
  });
});

// Simulate job progress
function simulateJobProgress(jobId) {
  const job = jobs.find(j => j.job_id === jobId);
  if (!job) return;
  
  console.log('⏳ Mock Server: Starting job simulation for:', jobId);
  
  setTimeout(() => {
    job.status = 'running';
    job.current_step = 'Preparing molecules';
    job.progress_percent = 10;
    console.log('▶️ Mock Server: Job started:', jobId);
  }, 2000);
  
  setTimeout(() => {
    job.current_step = 'Calculating poses';
    job.progress_percent = 50;
    job.poses_evaluated = 500;
    job.poses_remaining = 500;
    console.log('🔄 Mock Server: Job progress:', jobId, '50%');
  }, 10000);
  
  setTimeout(() => {
    job.status = 'completed';
    job.current_step = 'Completed';
    job.progress_percent = 100;
    job.poses_evaluated = 1000;
    job.poses_remaining = 0;
    job.completed_at = new Date().toISOString();
    job.results = [
      { pose_id: 1, score: -8.5, rmsd: 1.2, energy: -45.2 },
      { pose_id: 2, score: -7.8, rmsd: 1.5, energy: -42.1 },
      { pose_id: 3, score: -7.3, rmsd: 1.8, energy: -39.8 }
    ];
    console.log('✅ Mock Server: Job completed:', jobId);
  }, 30000);
}

// Connection test
app.get('/api/test', (req, res) => {
  res.json({
    success: true,
    message: 'Backend is running',
    timestamp: new Date().toISOString()
  });
});

// WebSocket mock (basic implementation)
const http = require('http');
const WebSocket = require('ws');

const server = http.createServer(app);

// Only create WebSocket server if ws module is available
try {
  const wss = new WebSocket.Server({ server });
  
  wss.on('connection', (ws) => {
    console.log('🔌 WebSocket client connected');
    
    ws.on('close', () => {
      console.log('🔌 WebSocket client disconnected');
    });
    
    // Send periodic updates
    const interval = setInterval(() => {
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({
          type: 'system_stats',
          data: {
            timestamp: new Date().toISOString(),
            files_count: files.length
          }
        }));
      }
    }, 5000);
    
    ws.on('close', () => {
      clearInterval(interval);
    });
  });
} catch (error) {
  console.log('WebSocket not available, continuing without it');
}

server.listen(PORT, () => {
  console.log(`🚀 Mock backend server running on http://localhost:${PORT}`);
  console.log(`📁 Files stored in: ${uploadsDir}`);
  console.log(`📋 Current files: ${files.length}`);
});