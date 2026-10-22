<script lang="ts">
  import { onMount } from 'svelte';
  import Hero from '$components/Hero.svelte';
  import FeatureGrid from '$components/FeatureGrid.svelte';
  import PerformanceStats from '$components/PerformanceStats.svelte';
  import QuickStart from '$components/QuickStart.svelte';
  import { performanceStore } from '$stores/performance.ts';
  import { systemStore, checkSystemCapabilities } from '$stores/system.ts';
  
  let mounted = false;
  
  onMount(() => {
    mounted = true;
    
    // Initialize system checks
    checkSystemCapabilities();
    
    // Track page load performance
    performanceStore.trackPageLoad('home');
  });
</script>

<svelte:head>
  <title>Protein Docking v2 - Ultra Performance Edition</title>
  <meta name="description" content="Next-generation protein docking with 200x performance improvements using Rust, GPU acceleration, and SIMD optimization." />
  <meta name="keywords" content="protein docking, molecular docking, bioinformatics, high performance computing, GPU acceleration, SIMD" />
</svelte:head>

<div class="page-home" class:mounted>
  <!-- Hero section -->
  <Hero />
  
  <!-- Performance statistics -->
  <section class="section-performance">
    <div class="container">
      <h2>🚀 Performance Achievements</h2>
      <PerformanceStats />
    </div>
  </section>
  
  <!-- Feature highlights -->
  <section class="section-features">
    <div class="container">
      <h2>⚡ Ultra-Optimized Features</h2>
      <FeatureGrid />
    </div>
  </section>
  
  <!-- Quick start guide -->
  <section class="section-quickstart">
    <div class="container">
      <h2>🎯 Quick Start</h2>
      <QuickStart />
    </div>
  </section>
  
  <!-- System capabilities -->
  <section class="section-capabilities">
    <div class="container">
      <h2>🔧 System Capabilities</h2>
      <div class="capabilities-grid">
        {#each Object.entries($systemStore.capabilities) as [feature, supported]}
          <div class="capability-item" class:supported>
            <div class="capability-icon">
              {supported ? '✅' : '❌'}
            </div>
            <div class="capability-info">
              <h3>{feature.replace(/([A-Z])/g, ' $1').replace(/^./, str => str.toUpperCase())}</h3>
              <p class="capability-status">
                {supported ? 'Supported' : 'Not Available'}
              </p>
            </div>
          </div>
        {/each}
      </div>
    </div>
  </section>
</div>

<style>
  .page-home {
    opacity: 0;
    transform: translateY(20px);
    transition: opacity 0.6s ease-out, transform 0.6s ease-out;
  }
  
  .page-home.mounted {
    opacity: 1;
    transform: translateY(0);
  }
  
  .container {
    max-width: 1200px;
    margin: 0 auto;
    padding: 0 2rem;
  }
  
  section {
    padding: 4rem 0;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  }
  
  section:last-child {
    border-bottom: none;
  }
  
  h2 {
    text-align: center;
    font-size: 2.5rem;
    margin-bottom: 3rem;
    background: linear-gradient(45deg, #00ff88, #00ccff);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }
  
  .section-performance {
    background: rgba(0, 255, 136, 0.05);
  }
  
  .section-features {
    background: rgba(0, 204, 255, 0.05);
  }
  
  .section-quickstart {
    background: rgba(255, 195, 0, 0.05);
  }
  
  .section-capabilities {
    background: rgba(138, 43, 226, 0.05);
  }
  
  .capabilities-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 1.5rem;
    margin-top: 2rem;
  }
  
  .capability-item {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 1.5rem;
    background: rgba(255, 255, 255, 0.05);
    border-radius: 12px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    transition: all 0.3s ease;
  }
  
  .capability-item:hover {
    transform: translateY(-2px);
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
  }
  
  .capability-item.supported {
    border-color: rgba(0, 255, 136, 0.3);
    background: rgba(0, 255, 136, 0.05);
  }
  
  .capability-icon {
    font-size: 2rem;
    min-width: 3rem;
    text-align: center;
  }
  
  .capability-info h3 {
    margin: 0 0 0.5rem 0;
    font-size: 1.2rem;
    color: white;
  }
  
  .capability-status {
    margin: 0;
    font-size: 0.9rem;
    color: #888;
  }
  
  .supported .capability-status {
    color: #00ff88;
  }
  
  /* Performance optimizations */
  .page-home {
    will-change: opacity, transform;
  }
  
  .capability-item {
    will-change: transform;
    backface-visibility: hidden;
  }
  
  /* Responsive design */
  @media (max-width: 768px) {
    .container {
      padding: 0 1rem;
    }
    
    section {
      padding: 2rem 0;
    }
    
    h2 {
      font-size: 2rem;
      margin-bottom: 2rem;
    }
    
    .capabilities-grid {
      grid-template-columns: 1fr;
      gap: 1rem;
    }
    
    .capability-item {
      padding: 1rem;
    }
  }
  
  /* Accessibility */
  @media (prefers-reduced-motion: reduce) {
    .page-home {
      transition: none;
    }
    
    .capability-item {
      transition: none;
    }
    
    .capability-item:hover {
      transform: none;
    }
  }
</style>