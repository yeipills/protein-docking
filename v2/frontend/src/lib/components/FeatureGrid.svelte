<!-- Feature Grid Component - Resalta características del sistema -->
<script lang="ts">
  const features = [
    {
      icon: '🚀',
      title: 'Ultra Performance',
      description: '200x faster than traditional docking with Rust optimization'
    },
    {
      icon: '⚡',
      title: 'GPU Acceleration',
      description: 'CUDA-powered parallel processing for maximum speed'
    },
    {
      icon: '🧬',
      title: 'Advanced Algorithms',
      description: 'State-of-the-art Monte Carlo and genetic algorithms'
    },
    {
      icon: '🎯',
      title: 'High Precision',
      description: 'Atomic-level accuracy with SIMD-optimized calculations'
    },
    {
      icon: '📊',
      title: 'Real-time Results',
      description: 'Live progress tracking and instant result visualization'
    },
    {
      icon: '🔬',
      title: '3D Visualization',
      description: 'Interactive WebGL-powered molecular viewer'
    }
  ];
</script>

<div class="feature-grid">
  {#each features as feature}
    <div class="feature-card">
      <div class="feature-icon">{feature.icon}</div>
      <h3 class="feature-title">{feature.title}</h3>
      <p class="feature-description">{feature.description}</p>
    </div>
  {/each}
</div>

<style>
  .feature-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 2rem;
    margin-top: 2rem;
  }

  .feature-card {
    background: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 16px;
    padding: 2rem;
    text-align: center;
    transition: all 0.3s ease;
    backdrop-filter: blur(10px);
  }

  .feature-card:hover {
    transform: translateY(-4px);
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.3);
    border-color: var(--accent-primary);
  }

  .feature-icon {
    font-size: 3rem;
    margin-bottom: 1rem;
    display: block;
  }

  .feature-title {
    margin: 0 0 1rem 0;
    font-size: 1.25rem;
    font-weight: 600;
    color: white;
  }

  .feature-description {
    margin: 0;
    color: rgba(255, 255, 255, 0.8);
    line-height: 1.6;
  }

  @media (max-width: 768px) {
    .feature-grid {
      grid-template-columns: 1fr;
      gap: 1rem;
    }

    .feature-card {
      padding: 1.5rem;
    }
  }
</style>