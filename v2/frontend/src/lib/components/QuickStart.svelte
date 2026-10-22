<!-- Quick Start Guide Component -->
<script lang="ts">
  const steps = [
    {
      number: 1,
      icon: '📤',
      title: 'Upload Files',
      description: 'Upload your receptor protein and ligand molecule files (PDB, MOL, SDF formats)',
      action: 'Go to Upload',
      link: '/app'
    },
    {
      number: 2,
      icon: '🎛️',
      title: 'Configure Parameters',
      description: 'Set docking parameters like exhaustiveness, number of modes, and precision level',
      action: 'Create Job',
      link: '/app'
    },
    {
      number: 3,
      icon: '🚀',
      title: 'Run Simulation',
      description: 'Start the ultra-fast docking simulation with real-time progress monitoring',
      action: 'Monitor Jobs',
      link: '/app'
    },
    {
      number: 4,
      icon: '🔬',
      title: 'View Results',
      description: 'Explore results in the interactive 3D viewer with pose analysis and scoring',
      action: 'Open Viewer',
      link: '/app'
    }
  ];
</script>

<div class="quick-start">
  <div class="steps-container">
    {#each steps as step, index}
      <div class="step-card" style="--delay: {index * 0.1}s">
        <div class="step-number">
          <span>{step.number}</span>
        </div>
        
        <div class="step-content">
          <div class="step-header">
            <span class="step-icon">{step.icon}</span>
            <h3 class="step-title">{step.title}</h3>
          </div>
          
          <p class="step-description">{step.description}</p>
          
          <a href={step.link} class="step-action">
            {step.action} →
          </a>
        </div>
        
        {#if index < steps.length - 1}
          <div class="step-connector"></div>
        {/if}
      </div>
    {/each}
  </div>
  
  <div class="quick-tips">
    <h4>💡 Pro Tips</h4>
    <div class="tips-grid">
      <div class="tip-item">
        <span class="tip-icon">⚡</span>
        <span class="tip-text">Use GPU acceleration for 10x faster processing</span>
      </div>
      <div class="tip-item">
        <span class="tip-icon">🎯</span>
        <span class="tip-text">Start with "Balanced" preset for optimal results</span>
      </div>
      <div class="tip-item">
        <span class="tip-icon">📊</span>
        <span class="tip-text">Monitor real-time progress with WebSocket updates</span>
      </div>
      <div class="tip-item">
        <span class="tip-icon">🔍</span>
        <span class="tip-text">Use 3D viewer controls for detailed analysis</span>
      </div>
    </div>
  </div>
</div>

<style>
  .quick-start {
    width: 100%;
  }
  
  .steps-container {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 2rem;
    margin-bottom: 3rem;
    position: relative;
  }
  
  .step-card {
    position: relative;
    background: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 16px;
    padding: 2rem;
    backdrop-filter: blur(10px);
    transition: all 0.3s ease;
    animation: slideInUp 0.6s ease-out forwards;
    animation-delay: var(--delay);
    opacity: 0;
    transform: translateY(30px);
  }
  
  @keyframes slideInUp {
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
  
  .step-card:hover {
    transform: translateY(-4px);
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.3);
    border-color: var(--accent-primary);
  }
  
  .step-number {
    position: absolute;
    top: -12px;
    left: 50%;
    transform: translateX(-50%);
    width: 40px;
    height: 40px;
    background: var(--accent-primary);
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 700;
    color: white;
    font-size: 1.125rem;
    box-shadow: 0 4px 12px rgba(59, 130, 246, 0.4);
  }
  
  .step-content {
    text-align: center;
    margin-top: 1rem;
  }
  
  .step-header {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 1rem;
  }
  
  .step-icon {
    font-size: 2.5rem;
    margin-bottom: 0.5rem;
  }
  
  .step-title {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 600;
    color: white;
  }
  
  .step-description {
    margin: 0 0 1.5rem 0;
    color: rgba(255, 255, 255, 0.8);
    line-height: 1.6;
    font-size: 0.95rem;
  }
  
  .step-action {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1.5rem;
    background: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 8px;
    color: white;
    text-decoration: none;
    font-weight: 500;
    transition: all 0.3s ease;
    backdrop-filter: blur(10px);
  }
  
  .step-action:hover {
    background: var(--accent-primary);
    border-color: var(--accent-primary);
    transform: translateY(-1px);
  }
  
  .step-connector {
    position: absolute;
    top: 50%;
    right: -1rem;
    width: 2rem;
    height: 2px;
    background: linear-gradient(90deg, var(--accent-primary), transparent);
    transform: translateY(-50%);
    opacity: 0.5;
  }
  
  .quick-tips {
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 12px;
    padding: 2rem;
    backdrop-filter: blur(10px);
  }
  
  .quick-tips h4 {
    margin: 0 0 1.5rem 0;
    color: white;
    font-size: 1.25rem;
    font-weight: 600;
    text-align: center;
  }
  
  .tips-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
    gap: 1rem;
  }
  
  .tip-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 1rem;
    background: rgba(255, 255, 255, 0.05);
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    transition: all 0.3s ease;
  }
  
  .tip-item:hover {
    background: rgba(255, 255, 255, 0.1);
    transform: translateY(-2px);
  }
  
  .tip-icon {
    font-size: 1.5rem;
    flex-shrink: 0;
  }
  
  .tip-text {
    color: rgba(255, 255, 255, 0.9);
    font-size: 0.875rem;
    line-height: 1.5;
  }
  
  @media (max-width: 1200px) {
    .step-connector {
      display: none;
    }
  }
  
  @media (max-width: 768px) {
    .steps-container {
      grid-template-columns: 1fr;
      gap: 1.5rem;
    }
    
    .step-card {
      padding: 1.5rem;
    }
    
    .tips-grid {
      grid-template-columns: 1fr;
    }
    
    .tip-item {
      padding: 0.75rem;
    }
  }
</style>