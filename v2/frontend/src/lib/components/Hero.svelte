<!-- Hero Section Component -->
<script lang="ts">
  import { onMount } from 'svelte';
  
  let mounted = $state(false);
  
  onMount(() => {
    mounted = true;
  });
</script>

<section class="hero" class:mounted>
  <div class="hero-background">
    <div class="gradient-orb orb-1"></div>
    <div class="gradient-orb orb-2"></div>
    <div class="gradient-orb orb-3"></div>
  </div>
  
  <div class="hero-content">
    <div class="hero-badge">
      <span class="badge-text">Ultra Performance Edition</span>
      <span class="badge-version">v2.0.0</span>
    </div>
    
    <h1 class="hero-title">
      <span class="title-line">Next-Generation</span>
      <span class="title-line gradient-text">Protein Docking</span>
    </h1>
    
    <p class="hero-description">
      Experience 200x performance improvements with Rust-powered algorithms, 
      GPU acceleration, and real-time 3D visualization for molecular docking simulations.
    </p>
    
    <div class="hero-stats">
      <div class="stat-item">
        <span class="stat-number">200x</span>
        <span class="stat-label">Faster Processing</span>
      </div>
      <div class="stat-item">
        <span class="stat-number">60 FPS</span>
        <span class="stat-label">Real-time 3D</span>
      </div>
      <div class="stat-item">
        <span class="stat-number">SIMD</span>
        <span class="stat-label">Optimization</span>
      </div>
    </div>
    
    <div class="hero-actions">
      <a href="/app" class="btn btn-primary">
        🚀 Launch Application
      </a>
      <a href="#features" class="btn btn-secondary">
        📖 Learn More
      </a>
    </div>
  </div>
</section>

<style>
  .hero {
    position: relative;
    min-height: 80vh;
    display: flex;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 4rem 2rem;
    overflow: hidden;
    opacity: 0;
    transform: translateY(30px);
    transition: all 0.8s ease-out;
  }
  
  .hero.mounted {
    opacity: 1;
    transform: translateY(0);
  }
  
  .hero-background {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: -1;
  }
  
  .gradient-orb {
    position: absolute;
    border-radius: 50%;
    filter: blur(40px);
    opacity: 0.6;
    animation: float 20s ease-in-out infinite;
  }
  
  .orb-1 {
    width: 300px;
    height: 300px;
    background: radial-gradient(circle, #3b82f6, transparent);
    top: 10%;
    left: 10%;
    animation-delay: 0s;
  }
  
  .orb-2 {
    width: 200px;
    height: 200px;
    background: radial-gradient(circle, #10b981, transparent);
    top: 60%;
    right: 15%;
    animation-delay: -7s;
  }
  
  .orb-3 {
    width: 250px;
    height: 250px;
    background: radial-gradient(circle, #8b5cf6, transparent);
    bottom: 20%;
    left: 20%;
    animation-delay: -14s;
  }
  
  @keyframes float {
    0%, 100% { transform: translateY(0px) rotate(0deg); }
    33% { transform: translateY(-20px) rotate(120deg); }
    66% { transform: translateY(10px) rotate(240deg); }
  }
  
  .hero-content {
    max-width: 800px;
    z-index: 1;
  }
  
  .hero-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    background: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 50px;
    backdrop-filter: blur(10px);
    margin-bottom: 2rem;
    animation: glow 3s ease-in-out infinite alternate;
  }
  
  @keyframes glow {
    from { box-shadow: 0 0 20px rgba(59, 130, 246, 0.3); }
    to { box-shadow: 0 0 30px rgba(59, 130, 246, 0.6); }
  }
  
  .badge-text {
    font-size: 0.875rem;
    font-weight: 600;
    color: white;
  }
  
  .badge-version {
    font-size: 0.75rem;
    padding: 0.25rem 0.5rem;
    background: var(--accent-primary);
    border-radius: 20px;
    color: white;
    font-weight: 600;
  }
  
  .hero-title {
    font-size: clamp(2.5rem, 8vw, 4.5rem);
    font-weight: 800;
    line-height: 1.1;
    margin: 0 0 1.5rem 0;
    color: white;
  }
  
  .title-line {
    display: block;
  }
  
  .gradient-text {
    background: linear-gradient(45deg, var(--accent-primary), #10b981, #8b5cf6);
    background-size: 200% 200%;
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    animation: gradient-shift 4s ease-in-out infinite;
  }
  
  @keyframes gradient-shift {
    0%, 100% { background-position: 0% 50%; }
    50% { background-position: 100% 50%; }
  }
  
  .hero-description {
    font-size: 1.25rem;
    line-height: 1.6;
    color: rgba(255, 255, 255, 0.8);
    margin: 0 0 3rem 0;
    max-width: 600px;
    margin-left: auto;
    margin-right: auto;
  }
  
  .hero-stats {
    display: flex;
    justify-content: center;
    gap: 3rem;
    margin-bottom: 3rem;
  }
  
  .stat-item {
    text-align: center;
  }
  
  .stat-number {
    display: block;
    font-size: 2rem;
    font-weight: 800;
    color: var(--accent-primary);
    margin-bottom: 0.5rem;
  }
  
  .stat-label {
    font-size: 0.875rem;
    color: rgba(255, 255, 255, 0.7);
    font-weight: 500;
  }
  
  .hero-actions {
    display: flex;
    gap: 1rem;
    justify-content: center;
    flex-wrap: wrap;
  }
  
  .btn {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 1rem 2rem;
    border-radius: 12px;
    font-size: 1rem;
    font-weight: 600;
    text-decoration: none;
    transition: all 0.3s ease;
    border: 2px solid transparent;
  }
  
  .btn-primary {
    background: var(--accent-primary);
    color: white;
  }
  
  .btn-primary:hover {
    background: #2563eb;
    transform: translateY(-2px);
    box-shadow: 0 8px 25px rgba(59, 130, 246, 0.4);
  }
  
  .btn-secondary {
    background: transparent;
    color: white;
    border-color: rgba(255, 255, 255, 0.3);
    backdrop-filter: blur(10px);
  }
  
  .btn-secondary:hover {
    background: rgba(255, 255, 255, 0.1);
    border-color: rgba(255, 255, 255, 0.5);
    transform: translateY(-2px);
  }
  
  @media (max-width: 768px) {
    .hero {
      padding: 2rem 1rem;
    }
    
    .hero-stats {
      gap: 1.5rem;
    }
    
    .hero-actions {
      flex-direction: column;
      align-items: center;
    }
    
    .btn {
      width: 100%;
      max-width: 280px;
    }
  }
</style>