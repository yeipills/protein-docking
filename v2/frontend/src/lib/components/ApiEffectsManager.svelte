<!--
  API Effects Manager - Maneja efectos reactivos del API Store
  
  Este componente se encarga de manejar los efectos reactivos ($effect)
  que no pueden ejecutarse a nivel de módulo en Svelte 5.
  
  Funcionalidades:
  - Performance monitoring automático
  - Cache optimization
  - Error detection y alertas
  - Auto-refresh de datos
-->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import api, { getPerformanceInsights, getSystemHealth } from '$lib/stores/api.svelte';
  
  // Estado local para tracking
  let performanceCheckInterval: number | undefined;
  let lastPerformanceCheck = $state(Date.now());
  let lastAlertTime = $state({ latency: 0, successRate: 0, errors: 0, files: 0, jobs: 0 });
  
  // Derived values usando funciones del store
  const performanceInsights = $derived(getPerformanceInsights());
  const systemHealth = $derived(getSystemHealth());
  
  // Todos los effects removidos para eliminar loops
  // El monitoring se hace a través de intervals en onMount
  
  onMount(() => {
    console.log('🔧 API Effects Manager mounted');
    
    // Setup performance check interval (menos frecuente)
    performanceCheckInterval = setInterval(() => {
      lastPerformanceCheck = Date.now();
      
      // Log performance summary every 10 minutes (más espaciado)
      if (api.state.connected) {
        const currentHealth = getSystemHealth();
        const currentInsights = getPerformanceInsights();
        
        // Solo log si hay problemas significativos
        if (currentHealth.qualityScore < 75) {
          console.warn('📊 Performance Issue:', {
            healthScore: currentHealth.qualityScore,
            connectivity: currentHealth.connectivity
          });
        }
      }
    }, 10 * 60 * 1000); // 10 minutes
  });
  
  onDestroy(() => {
    console.log('🔧 API Effects Manager unmounted');
    
    if (performanceCheckInterval) {
      clearInterval(performanceCheckInterval);
    }
  });
</script>

<!-- Este componente no renderiza nada visible -->
<div style="display: none;" data-component="ApiEffectsManager">
  <!-- Hidden component for API effects management -->
  <span>Last check: {new Date(lastPerformanceCheck).toLocaleTimeString()}</span>
</div>