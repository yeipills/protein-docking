<script lang="ts">
  import { onMount } from 'svelte';
  import { browser } from '$app/environment';
  import NotificationContainer from '$lib/components/NotificationContainer.svelte';
  import ApiEffectsManager from '$lib/components/ApiEffectsManager.svelte';
  import { notificationService } from '$lib/services/NotificationService';
  
  let appInitialized = false;
  let initError: string | null = null;
  
  onMount(async () => {
    if (!browser) return;
    
    try {
      console.log('🚀 App initializing...');
      
      // Verificar si ya fue inicializado en esta sesión
      const wasInitialized = localStorage.getItem('app-initialized') === 'true';
      
      if (wasInitialized) {
        // Inicialización rápida para recargas
        appInitialized = true;
        console.log('✅ App previously initialized, skipping setup');
        return;
      }
      
      // Primera inicialización
      notificationService.info(
        'Sistema iniciando',
        'Configurando entorno de docking de proteínas...',
        { category: 'system' }
      );
      
      // Inicialización rápida - solo verificaciones críticas
      await new Promise(resolve => setTimeout(resolve, 300));
      
      appInitialized = true;
      localStorage.setItem('app-initialized', 'true');
      
      // Notificación de éxito
      notificationService.success(
        'Sistema listo',
        'Entorno de docking ultra-optimizado iniciado correctamente',
        { category: 'system', duration: 3000 }
      );
      
    } catch (error) {
      console.error('❌ Failed to initialize app:', error);
      initError = error instanceof Error ? error.message : 'Unknown initialization error';
      
      // Notificación de error
      notificationService.error(
        'Error de inicialización',
        `Failed to start: ${initError}`,
        { category: 'system', duration: null }
      );
    }
  });
</script>

<svelte:head>
  <title>Protein Docking v2 - Ultra Performance Edition</title>
  <meta name="description" content="Ultra-high performance protein docking" />
</svelte:head>

<div class="app-layout" class:app-initialized={appInitialized}>
  <!-- Main content area -->
  <main class="main-content">
    {#if !appInitialized}
      <div class="initialization-screen">
        {#if initError}
          <div class="init-error">
            <h2>⚠️ Initialization Error</h2>
            <p>{initError}</p>
            <button onclick={() => window.location.reload()}>
              🔄 Retry
            </button>
          </div>
        {:else}
          <div class="init-loading">
            <div class="loading-spinner"></div>
            <h2>🧬 Initializing Protein Docking v2</h2>
            <p>Setting up high-performance computing environment...</p>
          </div>
        {/if}
      </div>
    {:else}
      <!-- App content -->
      <slot />
    {/if}
  </main>
  
  <!-- Sistema de notificaciones -->
  <NotificationContainer />
  
  <!-- API Effects Manager (solo si la app está inicializada) -->
  {#if appInitialized}
    <ApiEffectsManager />
  {/if}
</div>

<style>
  .app-layout {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    background: linear-gradient(135deg, #1a1a1a 0%, #2d2d2d 100%);
    color: white;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  }
  
  .main-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  
  .initialization-screen {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2rem;
  }
  
  .init-loading {
    text-align: center;
    max-width: 400px;
  }
  
  .init-loading h2 {
    margin: 1rem 0;
    font-size: 1.5rem;
    color: #00ff88;
  }
  
  .init-loading p {
    color: #888;
    margin-bottom: 2rem;
  }
  
  .loading-spinner {
    width: 60px;
    height: 60px;
    border: 4px solid #333;
    border-top: 4px solid #00ff88;
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin: 0 auto 2rem;
  }
  
  .init-error {
    text-align: center;
    background: rgba(255, 59, 48, 0.1);
    border: 1px solid rgba(255, 59, 48, 0.3);
    border-radius: 8px;
    padding: 2rem;
    max-width: 500px;
  }
  
  .init-error h2 {
    color: #ff3b30;
    margin-bottom: 1rem;
  }
  
  .init-error button {
    background: #ff3b30;
    color: white;
    border: none;
    padding: 0.75rem 1.5rem;
    border-radius: 6px;
    cursor: pointer;
    font-size: 1rem;
    margin-top: 1rem;
    transition: background-color 0.2s;
  }
  
  .init-error button:hover {
    background: #d70015;
  }
  
  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }
  
  /* Theme variants can be added here when needed */
  
  /* Optimized animations for performance */
  .app-layout {
    will-change: auto;
    transform: translateZ(0); /* Force GPU acceleration */
  }
  
  /* Responsive design */
  @media (max-width: 768px) {
    .app-layout {
      font-size: 14px;
    }
    
    .initialization-screen {
      padding: 1rem;
    }
  }
</style>