<script lang="ts">
  import JobMonitor from '$lib/components/JobMonitor.svelte';
  import { apiService } from '$lib/services/ApiService.js';
  import { onMount } from 'svelte';

  let backendStatus: 'connected' | 'disconnected' | 'checking' = 'checking';

  onMount(async () => {
    await checkBackendConnection();
  });

  async function checkBackendConnection() {
    backendStatus = 'checking';
    try {
      const health = await apiService.getHealth();
      backendStatus = 'connected';
    } catch (error) {
      backendStatus = 'disconnected';
      console.error('Backend connection failed:', error);
    }
  }
</script>

<svelte:head>
  <title>Job Monitor - Protein Docking</title>
  <meta name="description" content="Real-time monitoring of protein docking jobs" />
</svelte:head>

<div class="min-h-screen bg-gray-50">
  <!-- Header -->
  <header class="bg-white shadow-sm border-b">
    <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
      <div class="flex justify-between items-center py-4">
        <div class="flex items-center space-x-4">
          <h1 class="text-xl font-semibold text-gray-900">🧬 Protein Docking - Job Monitor</h1>
          
          <!-- Backend Status Indicator -->
          <div class="flex items-center space-x-2">
            {#if backendStatus === 'connected'}
              <div class="w-2 h-2 bg-green-500 rounded-full"></div>
              <span class="text-sm text-green-600">Backend Connected</span>
            {:else if backendStatus === 'disconnected'}
              <div class="w-2 h-2 bg-red-500 rounded-full"></div>
              <span class="text-sm text-red-600">Backend Disconnected</span>
            {:else}
              <div class="w-2 h-2 bg-yellow-500 rounded-full animate-pulse"></div>
              <span class="text-sm text-yellow-600">Checking...</span>
            {/if}
          </div>
        </div>
        
        <nav class="flex space-x-4">
          <a href="/" class="text-gray-600 hover:text-gray-900 px-3 py-2 rounded-md text-sm font-medium">
            🏠 Home
          </a>
          <a href="/test" class="text-gray-600 hover:text-gray-900 px-3 py-2 rounded-md text-sm font-medium">
            🧪 Test APIs
          </a>
          <a href="/visualization" class="text-gray-600 hover:text-gray-900 px-3 py-2 rounded-md text-sm font-medium">
            🔍 3D Visualization
          </a>
          <a href="/jobs" class="text-blue-600 hover:text-blue-800 px-3 py-2 rounded-md text-sm font-medium bg-blue-50">
            📊 Jobs
          </a>
        </nav>
      </div>
    </div>
  </header>

  <!-- Main Content -->
  <main class="max-w-7xl mx-auto">
    {#if backendStatus === 'disconnected'}
      <div class="p-6">
        <div class="bg-red-50 border border-red-200 rounded-lg p-6">
          <div class="flex items-center">
            <div class="flex-shrink-0">
              <svg class="h-5 w-5 text-red-400" viewBox="0 0 20 20" fill="currentColor">
                <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clip-rule="evenodd" />
              </svg>
            </div>
            <div class="ml-3">
              <h3 class="text-sm font-medium text-red-800">Backend Connection Error</h3>
              <div class="mt-2 text-sm text-red-700">
                <p>Cannot connect to the protein docking backend. Please ensure the backend is running.</p>
                <div class="mt-2">
                  <button 
                    on:click={checkBackendConnection}
                    class="bg-red-100 hover:bg-red-200 text-red-800 px-3 py-1 rounded text-sm"
                  >
                    🔄 Retry Connection
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Troubleshooting Section -->
        <div class="mt-6 bg-white rounded-lg border p-6">
          <h3 class="text-lg font-semibold text-gray-900 mb-4">🛠️ Troubleshooting</h3>
          <div class="space-y-4">
            <div>
              <h4 class="font-medium text-gray-800">1. Verify Backend is Running</h4>
              <div class="text-sm text-gray-600 mt-1 font-mono bg-gray-100 p-2 rounded">
                docker-compose up -d backend
              </div>
            </div>
            
            <div>
              <h4 class="font-medium text-gray-800">2. Check Backend Health</h4>
              <div class="text-sm text-gray-600 mt-1 font-mono bg-gray-100 p-2 rounded">
                curl http://localhost:8080/health
              </div>
            </div>
            
            <div>
              <h4 class="font-medium text-gray-800">3. Backend URL Configuration</h4>
              <p class="text-sm text-gray-600 mt-1">
                Expected backend URL: <span class="font-mono bg-gray-100 px-1 rounded">http://localhost:8080</span>
              </p>
            </div>
          </div>
        </div>
      </div>
    {:else if backendStatus === 'connected'}
      <!-- Job Monitor Component -->
      <JobMonitor />
    {:else}
      <!-- Loading State -->
      <div class="p-6">
        <div class="flex items-center justify-center">
          <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
          <span class="ml-3 text-gray-600">Checking backend connection...</span>
        </div>
      </div>
    {/if}
  </main>

  <!-- Footer -->
  <footer class="bg-white border-t mt-12">
    <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-4">
      <div class="text-center text-sm text-gray-500">
        <p>🧬 Protein Docking v3.0 - Ultra-Optimized Backend with Real-time Job Monitoring</p>
        <p class="mt-1">
          <span class="font-mono">API Status: 
            {#if backendStatus === 'connected'}
              <span class="text-green-600">✅ Online</span>
            {:else if backendStatus === 'disconnected'}
              <span class="text-red-600">❌ Offline</span>
            {:else}
              <span class="text-yellow-600">⏳ Checking</span>
            {/if}
          </span>
        </p>
      </div>
    </div>
  </footer>
</div>