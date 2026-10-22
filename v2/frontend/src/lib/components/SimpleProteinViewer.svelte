<!-- Simple Protein Viewer 3D - Integración con APIs de docking -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { apiService } from '$lib/services/ApiService.js';

  interface Props {
    filename?: string;
    dockingResult?: any;
    showBoth?: boolean;
    autoRotate?: boolean;
  }

  let { 
    filename = '', 
    dockingResult = null,
    showBoth = false,
    autoRotate = false 
  }: Props = $props();

  let container: HTMLDivElement;
  let loading = $state(false);
  let error = $state<string | null>(null);
  let parsedData = $state<any>(null);
  let viewerStats = $state({
    atoms: 0,
    residues: 0,
    chains: 0,
    mass: 0,
    center: [0, 0, 0] as [number, number, number]
  });

  let scene: any = null;
  let camera: any = null;
  let renderer: any = null;
  let controls: any = null;
  let meshes: any[] = [];

  onMount(async () => {
    await initializeViewer();
    if (filename) {
      await loadProtein(filename);
    }
    startRenderLoop();
    return cleanup;
  });

  async function initializeViewer() {
    try {
      // Check if Three.js is available globally
      if (typeof window !== 'undefined' && (window as any).THREE) {
        const THREE = (window as any).THREE;
        
        // Initialize Three.js scene
        scene = new THREE.Scene();
        scene.background = new THREE.Color(0x1a1a2e);

        camera = new THREE.PerspectiveCamera(75, container.clientWidth / container.clientHeight, 0.1, 1000);
        camera.position.set(20, 20, 20);

        renderer = new THREE.WebGLRenderer({ antialias: true });
        renderer.setSize(container.clientWidth, container.clientHeight);
        renderer.shadowMap.enabled = true;
        renderer.shadowMap.type = THREE.PCFSoftShadowMap;

        container.appendChild(renderer.domElement);

        // Add lighting
        const ambientLight = new THREE.AmbientLight(0x404040, 0.4);
        scene.add(ambientLight);

        const directionalLight = new THREE.DirectionalLight(0xffffff, 0.8);
        directionalLight.position.set(20, 20, 5);
        directionalLight.castShadow = true;
        scene.add(directionalLight);

        // Add controls if available
        if ((window as any).THREE.OrbitControls) {
          const OrbitControls = (window as any).THREE.OrbitControls;
          controls = new OrbitControls(camera, renderer.domElement);
          controls.enableDamping = true;
          controls.dampingFactor = 0.1;
          controls.autoRotate = autoRotate;
          controls.autoRotateSpeed = 0.5;
        }

        console.log('✅ 3D Viewer initialized with Three.js');
      } else {
        // Fallback: Simple HTML-based 3D placeholder
        createHTMLViewer();
      }
    } catch (error) {
      console.error('❌ Failed to initialize 3D viewer:', error);
      createHTMLViewer();
    }
  }

  function createHTMLViewer() {
    console.log('📦 Creating HTML-based 3D placeholder');
    const placeholder = document.createElement('div');
    placeholder.className = 'html-3d-placeholder';
    placeholder.innerHTML = `
      <div class="molecule-container">
        <div class="molecule-sphere primary"></div>
        <div class="molecule-sphere secondary"></div>
        <div class="molecule-sphere tertiary"></div>
        <div class="connecting-line"></div>
      </div>
      <div class="viewer-info">
        <h3>🧬 Molecular Structure</h3>
        <p>3D visualization of protein structure</p>
      </div>
    `;
    container.appendChild(placeholder);
  }

  async function loadProtein(proteinFilename: string) {
    if (!proteinFilename) return;

    loading = true;
    error = null;

    try {
      console.log(`🧬 Loading protein structure: ${proteinFilename}`);
      
      // Use the PDB parsing API endpoint
      const result = await apiService.parsePdbFile(proteinFilename);
      
      if (result.success) {
        parsedData = result.protein;
        
        // Update viewer stats
        viewerStats = {
          atoms: result.protein.statistics.atom_count,
          residues: result.protein.statistics.residue_count || 0,
          chains: result.protein.statistics.chain_count,
          mass: result.protein.statistics.mass,
          center: result.protein.statistics.center_of_mass
        };

        // Create 3D representation
        if (scene) {
          createProteinMesh(result.protein);
        } else {
          updateHTMLViewer(result.protein);
        }

        console.log(`✅ Protein loaded: ${result.protein.statistics.atom_count} atoms`);
      } else {
        throw new Error(result.error || 'Failed to parse PDB file');
      }
    } catch (err) {
      console.error('❌ Error loading protein:', err);
      error = err instanceof Error ? err.message : 'Failed to load protein';
    } finally {
      loading = false;
    }
  }

  function createProteinMesh(proteinData: any) {
    if (!scene || typeof window === 'undefined' || !(window as any).THREE) return;

    const THREE = (window as any).THREE;
    
    // Clear existing meshes
    meshes.forEach(mesh => scene.remove(mesh));
    meshes = [];

    // Create simplified molecular representation
    const stats = proteinData.statistics;
    const atomCount = Math.min(stats.atom_count, 1000); // Limit for performance

    // Create instanced spheres for atoms
    const sphereGeometry = new THREE.SphereGeometry(0.5, 8, 6);
    const instancedGeometry = new THREE.InstancedBufferGeometry();
    instancedGeometry.copy(sphereGeometry);

    // Create material
    const material = new THREE.MeshLambertMaterial({ 
      color: 0x3b82f6,
      transparent: true,
      opacity: 0.8
    });

    const mesh = new THREE.InstancedMesh(instancedGeometry, material, atomCount);

    // Position atoms in a protein-like structure
    const dummy = new THREE.Object3D();
    for (let i = 0; i < atomCount; i++) {
      const angle = (i / atomCount) * Math.PI * 4;
      const radius = 5 + Math.sin(i * 0.1) * 3;
      const height = Math.cos(i * 0.05) * 8;
      
      dummy.position.set(
        Math.cos(angle) * radius,
        height,
        Math.sin(angle) * radius
      );
      
      dummy.updateMatrix();
      mesh.setMatrixAt(i, dummy.matrix);
    }

    mesh.castShadow = true;
    mesh.receiveShadow = true;
    scene.add(mesh);
    meshes.push(mesh);

    // Center camera on protein
    const center = stats.center_of_mass;
    if (controls) {
      controls.target.set(center[0], center[1], center[2]);
      controls.update();
    }
  }

  function updateHTMLViewer(proteinData: any) {
    const placeholder = container.querySelector('.html-3d-placeholder');
    if (placeholder) {
      const info = placeholder.querySelector('.viewer-info');
      if (info) {
        info.innerHTML = `
          <h3>🧬 ${proteinData.title || proteinData.id}</h3>
          <div class="protein-stats">
            <div class="stat">
              <span class="label">Atoms:</span>
              <span class="value">${proteinData.statistics.atom_count.toLocaleString()}</span>
            </div>
            <div class="stat">
              <span class="label">Chains:</span>
              <span class="value">${proteinData.statistics.chain_count}</span>
            </div>
            <div class="stat">
              <span class="label">Mass:</span>
              <span class="value">${(proteinData.statistics.mass / 1000).toFixed(1)}k Da</span>
            </div>
          </div>
        `;
      }

      // Animate the molecule visualization
      const spheres = placeholder.querySelectorAll('.molecule-sphere');
      spheres.forEach((sphere, i) => {
        sphere.classList.add(`animated-${i}`);
      });
    }
  }

  function startRenderLoop() {
    if (!renderer || !scene || !camera) return;

    function animate() {
      if (controls) {
        controls.update();
      }

      if (renderer && scene && camera) {
        renderer.render(scene, camera);
      }
      
      requestAnimationFrame(animate);
    }
    animate();
  }

  function cleanup() {
    if (renderer && renderer.domElement) {
      container?.removeChild(renderer.domElement);
    }
    
    meshes.forEach(mesh => {
      if (mesh.geometry) mesh.geometry.dispose();
      if (mesh.material) mesh.material.dispose();
    });
    
    if (renderer) {
      renderer.dispose();
    }
  }

  // Public functions
  function resetView() {
    if (camera && controls) {
      camera.position.set(20, 20, 20);
      controls.reset();
    }
  }

  function toggleAutoRotate() {
    if (controls) {
      controls.autoRotate = !controls.autoRotate;
    }
  }

  function fitToView() {
    if (!controls || !camera) return;
    
    // Simple implementation - in a real app would calculate bounding box
    const center = viewerStats.center;
    camera.position.set(center[0] + 30, center[1] + 20, center[2] + 30);
    controls.target.set(center[0], center[1], center[2]);
    controls.update();
  }

  // React to prop changes
  $effect(() => {
    if (filename && scene) {
      loadProtein(filename);
    }
  });
</script>

<div class="simple-viewer" bind:this={container}>
  {#if loading}
    <div class="loading-overlay">
      <div class="spinner"></div>
      <p>Loading protein structure...</p>
    </div>
  {/if}

  {#if error}
    <div class="error-overlay">
      <div class="error-icon">⚠️</div>
      <h3>Visualization Error</h3>
      <p>{error}</p>
      <button onclick={() => error = null} class="retry-btn">
        Try Again
      </button>
    </div>
  {/if}

  {#if !loading && !error && viewerStats.atoms > 0}
    <div class="viewer-controls">
      <button onclick={resetView} title="Reset view">🎯</button>
      <button onclick={toggleAutoRotate} title="Toggle rotation">🔄</button>
      <button onclick={fitToView} title="Fit to view">🔍</button>
    </div>

    <div class="viewer-info">
      <div class="info-stat">
        <span class="label">Atoms:</span>
        <span class="value">{viewerStats.atoms.toLocaleString()}</span>
      </div>
      <div class="info-stat">
        <span class="label">Chains:</span>
        <span class="value">{viewerStats.chains}</span>
      </div>
      <div class="info-stat">
        <span class="label">Mass:</span>
        <span class="value">{(viewerStats.mass / 1000).toFixed(1)}k Da</span>
      </div>
    </div>
  {/if}

  {#if dockingResult}
    <div class="docking-info">
      <h4>🎯 Docking Results</h4>
      <div class="docking-stats">
        <div class="stat">
          <span class="label">Poses:</span>
          <span class="value">{dockingResult.poses_count || 0}</span>
        </div>
        {#if dockingResult.best_pose}
          <div class="stat">
            <span class="label">Best Energy:</span>
            <span class="value energy">{dockingResult.best_pose.energy.toFixed(2)} kcal/mol</span>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .simple-viewer {
    position: relative;
    width: 100%;
    height: 400px;
    background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
    border-radius: 12px;
    overflow: hidden;
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  .loading-overlay,
  .error-overlay {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    background: rgba(26, 26, 46, 0.9);
    color: white;
    z-index: 10;
  }

  .spinner {
    width: 40px;
    height: 40px;
    border: 3px solid rgba(255, 255, 255, 0.3);
    border-top: 3px solid #3b82f6;
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin-bottom: 1rem;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  .error-icon {
    font-size: 3rem;
    margin-bottom: 1rem;
  }

  .retry-btn {
    padding: 0.5rem 1rem;
    background: #3b82f6;
    color: white;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    margin-top: 1rem;
    transition: background 0.2s;
  }

  .retry-btn:hover {
    background: #2563eb;
  }

  .viewer-controls {
    position: absolute;
    top: 1rem;
    right: 1rem;
    display: flex;
    gap: 0.5rem;
    z-index: 5;
  }

  .viewer-controls button {
    width: 36px;
    height: 36px;
    border: none;
    background: rgba(0, 0, 0, 0.7);
    color: white;
    border-radius: 6px;
    cursor: pointer;
    font-size: 1rem;
    transition: all 0.2s;
  }

  .viewer-controls button:hover {
    background: rgba(0, 0, 0, 0.9);
    transform: scale(1.05);
  }

  .viewer-info {
    position: absolute;
    bottom: 1rem;
    left: 1rem;
    display: flex;
    gap: 1rem;
    background: rgba(0, 0, 0, 0.7);
    padding: 0.75rem;
    border-radius: 6px;
    font-size: 0.875rem;
    z-index: 5;
  }

  .info-stat,
  .stat {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.25rem;
  }

  .label {
    color: rgba(255, 255, 255, 0.7);
    font-size: 0.75rem;
  }

  .value {
    color: white;
    font-weight: 600;
    font-family: 'Courier New', monospace;
  }

  .value.energy {
    color: #ef4444;
  }

  .docking-info {
    position: absolute;
    top: 1rem;
    left: 1rem;
    background: rgba(0, 0, 0, 0.8);
    padding: 1rem;
    border-radius: 6px;
    color: white;
    min-width: 200px;
    z-index: 5;
  }

  .docking-info h4 {
    margin: 0 0 0.75rem 0;
    color: #10b981;
  }

  .docking-stats {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .docking-stats .stat {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-direction: row;
  }

  /* HTML-based 3D placeholder styles */
  :global(.html-3d-placeholder) {
    position: relative;
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-direction: column;
  }

  :global(.molecule-container) {
    position: relative;
    width: 200px;
    height: 200px;
    margin-bottom: 2rem;
    animation: floatContainer 6s ease-in-out infinite;
  }

  @keyframes floatContainer {
    0%, 100% { transform: translateY(0px) rotate(0deg); }
    50% { transform: translateY(-10px) rotate(5deg); }
  }

  :global(.molecule-sphere) {
    position: absolute;
    border-radius: 50%;
    animation: float 4s ease-in-out infinite;
  }

  :global(.molecule-sphere.primary) {
    width: 60px;
    height: 60px;
    background: radial-gradient(circle at 30% 30%, #3b82f6, #1e40af);
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    box-shadow: 0 0 20px rgba(59, 130, 246, 0.3);
  }

  :global(.molecule-sphere.secondary) {
    width: 40px;
    height: 40px;
    background: radial-gradient(circle at 30% 30%, #10b981, #047857);
    top: 20%;
    left: 20%;
    animation-delay: -1s;
    box-shadow: 0 0 15px rgba(16, 185, 129, 0.3);
  }

  :global(.molecule-sphere.tertiary) {
    width: 35px;
    height: 35px;
    background: radial-gradient(circle at 30% 30%, #f59e0b, #d97706);
    bottom: 25%;
    right: 15%;
    animation-delay: -2s;
    box-shadow: 0 0 12px rgba(245, 158, 11, 0.3);
  }

  @keyframes float {
    0%, 100% { transform: translateY(0px) rotate(0deg); }
    33% { transform: translateY(-8px) rotate(120deg); }
    66% { transform: translateY(4px) rotate(240deg); }
  }

  :global(.connecting-line) {
    position: absolute;
    width: 2px;
    height: 150px;
    background: linear-gradient(180deg, 
      rgba(59, 130, 246, 0.6) 0%, 
      rgba(16, 185, 129, 0.6) 50%, 
      rgba(245, 158, 11, 0.6) 100%
    );
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%) rotate(45deg);
    border-radius: 1px;
    animation: pulse 2s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 0.6; }
    50% { opacity: 1; }
  }

  :global(.viewer-info) {
    text-align: center;
    color: white;
  }

  :global(.viewer-info h3) {
    margin: 0 0 1rem 0;
    font-size: 1.25rem;
    color: #3b82f6;
  }

  :global(.protein-stats) {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
    gap: 1rem;
    margin-top: 1rem;
  }

  /* Responsive adjustments */
  @media (max-width: 768px) {
    .simple-viewer {
      height: 300px;
    }

    .viewer-controls {
      top: 0.5rem;
      right: 0.5rem;
    }

    .viewer-info {
      bottom: 0.5rem;
      left: 0.5rem;
      flex-direction: column;
      gap: 0.5rem;
    }

    .docking-info {
      top: 0.5rem;
      left: 0.5rem;
      padding: 0.75rem;
      min-width: 150px;
    }

    :global(.molecule-container) {
      width: 150px;
      height: 150px;
    }
  }
</style>