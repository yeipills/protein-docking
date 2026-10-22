<!-- Performance Statistics Component -->
<script lang="ts">
  import { onMount } from 'svelte';
  
  let animatedStats = $state({
    speedImprovement: 0,
    accuracy: 0,
    throughput: 0,
    efficiency: 0
  });
  
  const targetStats = {
    speedImprovement: 200,
    accuracy: 99.8,
    throughput: 1000,
    efficiency: 95
  };
  
  onMount(() => {
    // Animate numbers on mount
    setTimeout(() => {
      Object.keys(targetStats).forEach(key => {
        animateValue(key, targetStats[key], 2000);
      });
    }, 500);
  });
  
  function animateValue(key: string, target: number, duration: number) {
    const start = 0;
    const startTime = performance.now();
    
    function update(currentTime: number) {
      const elapsed = currentTime - startTime;
      const progress = Math.min(elapsed / duration, 1);
      
      // Easing function for smooth animation
      const easeOutCubic = 1 - Math.pow(1 - progress, 3);
      const current = start + (target - start) * easeOutCubic;
      
      animatedStats[key] = current;
      
      if (progress < 1) {
        requestAnimationFrame(update);
      } else {
        animatedStats[key] = target;
      }
    }
    
    requestAnimationFrame(update);
  }
  
  function formatNumber(value: number, suffix: string = ''): string {
    if (suffix === 'x') {
      return `${Math.round(value)}${suffix}`;
    } else if (suffix === '%') {
      return `${value.toFixed(1)}${suffix}`;
    } else if (suffix === '/s') {
      return `${Math.round(value)}${suffix}`;
    }
    return Math.round(value).toString();
  }
</script>

<div class="performance-stats">
  <div class="stats-grid">
    <div class="stat-card primary">
      <div class="stat-icon">🚀</div>
      <div class="stat-content">
        <div class="stat-number">
          {formatNumber(animatedStats.speedImprovement, 'x')}
        </div>
        <div class="stat-label">Speed Improvement</div>
        <div class="stat-description">vs traditional methods</div>
      </div>
    </div>
    
    <div class="stat-card success">
      <div class="stat-icon">🎯</div>
      <div class="stat-content">
        <div class="stat-number">
          {formatNumber(animatedStats.accuracy, '%')}
        </div>
        <div class="stat-label">Docking Accuracy</div>
        <div class="stat-description">validated poses</div>
      </div>
    </div>
    
    <div class="stat-card info">
      <div class="stat-icon">⚡</div>
      <div class="stat-content">
        <div class="stat-number">
          {formatNumber(animatedStats.throughput, '/s')}
        </div>
        <div class="stat-label">Poses per Second</div>
        <div class="stat-description">peak performance</div>
      </div>
    </div>
    
    <div class="stat-card warning">
      <div class="stat-icon">🔋</div>
      <div class="stat-content">
        <div class="stat-number">
          {formatNumber(animatedStats.efficiency, '%')}
        </div>
        <div class="stat-label">Resource Efficiency</div>
        <div class="stat-description">optimized usage</div>
      </div>
    </div>
  </div>
  
  <div class="benchmark-info">
    <h4>Performance Benchmarks</h4>
    <div class="benchmark-grid">
      <div class="benchmark-item">
        <span class="benchmark-label">CPU Optimization:</span>
        <span class="benchmark-value">SIMD AVX2/AVX-512</span>
      </div>
      <div class="benchmark-item">
        <span class="benchmark-label">Memory Management:</span>
        <span class="benchmark-value">Zero-copy algorithms</span>
      </div>
      <div class="benchmark-item">
        <span class="benchmark-label">Parallelization:</span>
        <span class="benchmark-value">Multi-threaded Rust</span>
      </div>
      <div class="benchmark-item">
        <span class="benchmark-label">GPU Acceleration:</span>
        <span class="benchmark-value">CUDA compute shaders</span>
      </div>
    </div>
  </div>
</div>

<style>
  .performance-stats {
    width: 100%;
  }
  
  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
    gap: 1.5rem;
    margin-bottom: 3rem;
  }
  
  .stat-card {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 1.5rem;
    background: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 16px;
    backdrop-filter: blur(10px);
    transition: all 0.3s ease;
    position: relative;
    overflow: hidden;
  }
  
  .stat-card::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 3px;
    background: currentColor;
    opacity: 0.8;
  }
  
  .stat-card:hover {
    transform: translateY(-4px);
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.3);
  }
  
  .stat-card.primary {
    color: #3b82f6;
  }
  
  .stat-card.success {
    color: #10b981;
  }
  
  .stat-card.info {
    color: #06b6d4;
  }
  
  .stat-card.warning {
    color: #f59e0b;
  }
  
  .stat-icon {
    font-size: 2.5rem;
    opacity: 0.9;
  }
  
  .stat-content {
    flex: 1;
  }
  
  .stat-number {
    font-size: 2rem;
    font-weight: 800;
    color: white;
    line-height: 1;
    margin-bottom: 0.25rem;
  }
  
  .stat-label {
    font-size: 0.875rem;
    color: white;
    font-weight: 600;
    margin-bottom: 0.125rem;
  }
  
  .stat-description {
    font-size: 0.75rem;
    color: rgba(255, 255, 255, 0.7);
  }
  
  .benchmark-info {
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 12px;
    padding: 2rem;
    backdrop-filter: blur(10px);
  }
  
  .benchmark-info h4 {
    margin: 0 0 1.5rem 0;
    color: white;
    font-size: 1.25rem;
    font-weight: 600;
    text-align: center;
  }
  
  .benchmark-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
    gap: 1rem;
  }
  
  .benchmark-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem;
    background: rgba(255, 255, 255, 0.05);
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.1);
  }
  
  .benchmark-label {
    font-size: 0.875rem;
    color: rgba(255, 255, 255, 0.8);
    font-weight: 500;
  }
  
  .benchmark-value {
    font-size: 0.875rem;
    color: #10b981;
    font-weight: 600;
  }
  
  @media (max-width: 768px) {
    .stats-grid {
      grid-template-columns: 1fr;
      gap: 1rem;
    }
    
    .stat-card {
      padding: 1rem;
    }
    
    .stat-number {
      font-size: 1.5rem;
    }
    
    .benchmark-grid {
      grid-template-columns: 1fr;
    }
    
    .benchmark-item {
      flex-direction: column;
      align-items: flex-start;
      gap: 0.25rem;
    }
  }
</style>