// Configuration
const API_URL = 'http://localhost:5000/api/v1';
let authToken = null;
let refreshToken = null;

// Initialize app
document.addEventListener('DOMContentLoaded', () => {
    // Check for existing session
    authToken = localStorage.getItem('access_token');
    refreshToken = localStorage.getItem('refresh_token');

    if (authToken) {
        updateUIForAuth(true);
        showPage('jobs');
        loadJobs();
    }

    // Setup event listeners
    setupEventListeners();
});

// Setup all event listeners
function setupEventListeners() {
    // Navigation
    document.querySelectorAll('[data-page]').forEach(link => {
        link.addEventListener('click', (e) => {
            e.preventDefault();
            showPage(e.target.getAttribute('data-page'));
        });
    });

    // Logout
    document.getElementById('navLogout').addEventListener('click', logout);

    // Forms
    document.getElementById('loginForm').addEventListener('submit', handleLogin);
    document.getElementById('registerForm').addEventListener('submit', handleRegister);
    document.getElementById('uploadForm').addEventListener('submit', handleUpload);
}

// Page Navigation
function showPage(pageName) {
    // Hide all pages
    document.querySelectorAll('.page').forEach(page => {
        page.classList.remove('active');
    });

    // Remove active from all nav links
    document.querySelectorAll('.nav-menu a').forEach(link => {
        link.classList.remove('active');
    });

    // Show selected page
    document.getElementById(pageName + 'Page').classList.add('active');

    // Update active nav link
    const activeLink = document.querySelector(`[data-page="${pageName}"]`);
    if (activeLink) {
        activeLink.classList.add('active');
    }
}

// Authentication
async function handleLogin(e) {
    e.preventDefault();

    const email = document.getElementById('loginEmail').value;
    const password = document.getElementById('loginPassword').value;

    try {
        const formData = new FormData();
        formData.append('username', email);
        formData.append('password', password);

        const response = await fetch(`${API_URL}/auth/login`, {
            method: 'POST',
            body: formData
        });

        const data = await response.json();

        if (response.ok) {
            authToken = data.access_token;
            refreshToken = data.refresh_token;

            localStorage.setItem('access_token', authToken);
            localStorage.setItem('refresh_token', refreshToken);

            updateUIForAuth(true);
            showToast('Login exitoso', 'success');
            showPage('jobs');
            loadJobs();
        } else {
            showToast(data.detail || 'Error en login', 'error');
        }
    } catch (error) {
        console.error('Login error:', error);
        showToast('Error de conexión', 'error');
    }
}

async function handleRegister(e) {
    e.preventDefault();

    const username = document.getElementById('registerUsername').value;
    const email = document.getElementById('registerEmail').value;
    const password = document.getElementById('registerPassword').value;

    try {
        const response = await fetch(`${API_URL}/auth/register`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({ username, email, password })
        });

        const data = await response.json();

        if (response.ok) {
            showToast('Registro exitoso. Por favor inicia sesión', 'success');
            showPage('login');
        } else {
            showToast(data.detail || 'Error en registro', 'error');
        }
    } catch (error) {
        console.error('Register error:', error);
        showToast('Error de conexión', 'error');
    }
}

function logout() {
    authToken = null;
    refreshToken = null;
    localStorage.removeItem('access_token');
    localStorage.removeItem('refresh_token');
    updateUIForAuth(false);
    showPage('home');
    showToast('Sesión cerrada', 'success');
}

function updateUIForAuth(isAuthenticated) {
    const loginLink = document.getElementById('navLogin');
    const logoutLink = document.getElementById('navLogout');
    const jobsLink = document.getElementById('navJobs');
    const uploadLink = document.getElementById('navUpload');

    if (isAuthenticated) {
        loginLink.style.display = 'none';
        logoutLink.style.display = 'block';
        jobsLink.style.display = 'block';
        uploadLink.style.display = 'block';
    } else {
        loginLink.style.display = 'block';
        logoutLink.style.display = 'none';
        jobsLink.style.display = 'none';
        uploadLink.style.display = 'none';
    }
}

// Jobs Management
async function loadJobs() {
    const jobsList = document.getElementById('jobsList');

    try {
        const response = await fetch(`${API_URL}/jobs/`, {
            headers: {
                'Authorization': `Bearer ${authToken}`
            }
        });

        if (response.ok) {
            const jobs = await response.json();

            if (jobs.length === 0) {
                jobsList.innerHTML = '<p class="loading">No tienes trabajos aún. Sube una proteína para empezar.</p>';
                return;
            }

            jobsList.innerHTML = jobs.map(job => `
                <div class="job-card">
                    <div class="job-header">
                        <div class="job-title">Job #${job.id} - ${job.job_type}</div>
                        <div class="job-status status-${job.status}">${translateStatus(job.status)}</div>
                    </div>
                    <div class="job-info">
                        <div class="job-info-item">
                            <span class="job-info-label">Proteína</span>
                            <span class="job-info-value">${job.protein_id || 'N/A'}</span>
                        </div>
                        <div class="job-info-item">
                            <span class="job-info-label">Creado</span>
                            <span class="job-info-value">${formatDate(job.created_at)}</span>
                        </div>
                        <div class="job-info-item">
                            <span class="job-info-label">Progreso</span>
                            <span class="job-info-value">${job.progress}%</span>
                        </div>
                    </div>
                    ${job.progress > 0 ? `
                        <div class="progress-bar">
                            <div class="progress-fill" style="width: ${job.progress}%"></div>
                        </div>
                    ` : ''}
                    ${job.error_message ? `
                        <div style="color: var(--danger); margin-top: 1rem;">
                            Error: ${job.error_message}
                        </div>
                    ` : ''}
                </div>
            `).join('');

            // Auto-refresh if there are processing jobs
            if (jobs.some(job => job.status === 'processing' || job.status === 'pending')) {
                setTimeout(loadJobs, 5000); // Refresh every 5 seconds
            }
        } else if (response.status === 401) {
            // Token expired, try to refresh
            await refreshAccessToken();
            loadJobs(); // Retry
        } else {
            throw new Error('Failed to load jobs');
        }
    } catch (error) {
        console.error('Load jobs error:', error);
        jobsList.innerHTML = '<p class="loading">Error al cargar trabajos</p>';
    }
}

// Upload Protein
async function handleUpload(e) {
    e.preventDefault();

    const proteinName = document.getElementById('proteinName').value;
    const stlFile = document.getElementById('stlFile').files[0];
    const vertFile = document.getElementById('vertFile').files[0];
    const faceFile = document.getElementById('faceFile').files[0];
    const jobType = document.getElementById('jobType').value;

    if (!stlFile || !vertFile || !faceFile) {
        showToast('Por favor selecciona todos los archivos', 'warning');
        return;
    }

    const formData = new FormData();
    formData.append('stl_file', stlFile);
    formData.append('vertices_file', vertFile);
    formData.append('faces_file', faceFile);
    formData.append('name', proteinName);

    try {
        // Upload protein files
        showToast('Subiendo archivos...', 'warning');

        const uploadResponse = await fetch(`${API_URL}/proteins/`, {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${authToken}`
            },
            body: formData
        });

        if (!uploadResponse.ok) {
            throw new Error('Upload failed');
        }

        const protein = await uploadResponse.json();

        // Create job
        const jobResponse = await fetch(`${API_URL}/jobs/`, {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${authToken}`,
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({
                protein_id: protein.id,
                job_type: jobType
            })
        });

        if (!jobResponse.ok) {
            throw new Error('Job creation failed');
        }

        showToast('Proteína subida y procesamiento iniciado', 'success');

        // Reset form
        document.getElementById('uploadForm').reset();

        // Switch to jobs page
        showPage('jobs');
        loadJobs();
    } catch (error) {
        console.error('Upload error:', error);
        showToast('Error al subir proteína', 'error');
    }
}

// Token Refresh
async function refreshAccessToken() {
    try {
        const response = await fetch(`${API_URL}/auth/refresh`, {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${refreshToken}`
            }
        });

        if (response.ok) {
            const data = await response.json();
            authToken = data.access_token;
            localStorage.setItem('access_token', authToken);
            return true;
        } else {
            // Refresh token expired, logout
            logout();
            return false;
        }
    } catch (error) {
        console.error('Token refresh error:', error);
        logout();
        return false;
    }
}

// Utilities
function showToast(message, type = 'success') {
    const toast = document.getElementById('toast');
    toast.textContent = message;
    toast.className = `toast show ${type}`;

    setTimeout(() => {
        toast.classList.remove('show');
    }, 3000);
}

function translateStatus(status) {
    const translations = {
        'pending': 'Pendiente',
        'processing': 'Procesando',
        'completed': 'Completado',
        'failed': 'Fallido',
        'cancelled': 'Cancelado'
    };
    return translations[status] || status;
}

function formatDate(dateString) {
    const date = new Date(dateString);
    return date.toLocaleString('es-ES', {
        year: 'numeric',
        month: 'short',
        day: 'numeric',
        hour: '2-digit',
        minute: '2-digit'
    });
}
