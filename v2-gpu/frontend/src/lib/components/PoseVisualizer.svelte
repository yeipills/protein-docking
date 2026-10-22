<!-- Pose Visualizer - Visualización 3D interactiva de poses de docking -->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import * as THREE from 'three';
  import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
  import type { DockingPose } from '../algorithms/DockingAlgorithm.js';
  import type { PDBStructure, PDBAtom } from '../utils/PDBParser.js';

  interface Props {
    receptor: PDBStructure | null;
    ligand: PDBStructure | null;
    selectedPose: DockingPose | null;
    showReceptor?: boolean;
    showLigand?: boolean;
    showContacts?: boolean;
    showLabels?: boolean;
    representationType?: 'cartoon' | 'spacefill' | 'sticks' | 'surface';
  }

  let {
    receptor,
    ligand,
    selectedPose,
    showReceptor = true,
    showLigand = true,
    showContacts = true,
    showLabels = false,
    representationType = 'spacefill'
  }: Props = $props();

  let container: HTMLDivElement;
  let scene: THREE.Scene;
  let camera: THREE.PerspectiveCamera;
  let renderer: THREE.WebGLRenderer;
  let controls: OrbitControls;
  
  let receptorGroup: THREE.Group;
  let ligandGroup: THREE.Group;
  let contactsGroup: THREE.Group;
  let labelsGroup: THREE.Group;
  
  let isInitialized = $state(false);

  // Reactive updates cuando cambia la pose seleccionada
  $effect(() => {
    if (isInitialized && selectedPose) {
      updateVisualization();
    }
  });

  $effect(() => {
    if (isInitialized) {
      toggleReceptorVisibility(showReceptor);
    }
  });

  $effect(() => {
    if (isInitialized) {
      toggleLigandVisibility(showLigand);
    }
  });

  $effect(() => {
    if (isInitialized) {
      toggleContactsVisibility(showContacts);
    }
  });

  $effect(() => {
    if (isInitialized) {
      toggleLabelsVisibility(showLabels);
    }
  });

  onMount(() => {
    initThreeJS();
    initializeMolecularVisualization();
    isInitialized = true;
    
    if (selectedPose) {
      updateVisualization();
    }
  });

  onDestroy(() => {
    if (renderer) {
      renderer.dispose();
    }
    if (controls) {
      controls.dispose();
    }
  });

  function initThreeJS() {
    // Scene
    scene = new THREE.Scene();
    scene.background = new THREE.Color(0x1a1a1a);

    // Camera
    camera = new THREE.PerspectiveCamera(
      75,
      container.clientWidth / container.clientHeight,
      0.1,
      1000
    );
    camera.position.set(20, 20, 20);

    // Renderer
    renderer = new THREE.WebGLRenderer({ 
      antialias: true,
      alpha: true
    });
    renderer.setSize(container.clientWidth, container.clientHeight);
    renderer.shadowMap.enabled = true;
    renderer.shadowMap.type = THREE.PCFSoftShadowMap;
    container.appendChild(renderer.domElement);

    // Controls
    controls = new OrbitControls(camera, renderer.domElement);
    controls.enableDamping = true;
    controls.dampingFactor = 0.1;
    controls.enableZoom = true;
    controls.autoRotate = false;

    // Lighting
    const ambientLight = new THREE.AmbientLight(0x404040, 0.6);
    scene.add(ambientLight);

    const directionalLight = new THREE.DirectionalLight(0xffffff, 0.8);
    directionalLight.position.set(10, 10, 5);
    directionalLight.castShadow = true;
    scene.add(directionalLight);

    const pointLight = new THREE.PointLight(0xffffff, 0.5, 100);
    pointLight.position.set(-10, -10, -10);
    scene.add(pointLight);

    // Start render loop
    animate();
  }

  function initializeMolecularVisualization() {
    // Initialize groups
    receptorGroup = new THREE.Group();
    ligandGroup = new THREE.Group();
    contactsGroup = new THREE.Group();
    labelsGroup = new THREE.Group();

    scene.add(receptorGroup);
    scene.add(ligandGroup);
    scene.add(contactsGroup);
    scene.add(labelsGroup);

    // Create receptor visualization
    if (receptor) {
      createReceptorVisualization();
    }
  }

  function createReceptorVisualization() {
    if (!receptor) return;

    receptorGroup.clear();

    // Create simplified receptor representation
    const receptorGeometry = new THREE.BufferGeometry();
    const positions: number[] = [];
    const colors: number[] = [];

    receptor.atoms.forEach(atom => {
      positions.push(atom.x, atom.y, atom.z);
      
      // Color by element
      const color = getElementColor(atom.element);
      colors.push(color.r, color.g, color.b);
    });

    receptorGeometry.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
    receptorGeometry.setAttribute('color', new THREE.Float32BufferAttribute(colors, 3));

    const receptorMaterial = new THREE.PointsMaterial({
      size: 0.5,
      vertexColors: true,
      transparent: true,
      opacity: 0.6
    });

    const receptorPoints = new THREE.Points(receptorGeometry, receptorMaterial);
    receptorGroup.add(receptorPoints);
  }

  function updateVisualization() {
    if (!selectedPose || !ligand) return;

    // Clear existing ligand visualization
    ligandGroup.clear();
    contactsGroup.clear();
    labelsGroup.clear();

    // Transform ligand atoms according to pose
    const transformedLigand = transformLigandAtoms(ligand.atoms, selectedPose);

    // Create ligand visualization
    createLigandVisualization(transformedLigand);

    // Create contacts visualization
    if (showContacts && selectedPose.contacts) {
      createContactsVisualization(selectedPose.contacts, transformedLigand);
    }

    // Create labels
    if (showLabels) {
      createLabelsVisualization(transformedLigand);
    }

    // Center camera on ligand
    centerCameraOnLigand(transformedLigand);
  }

  function transformLigandAtoms(atoms: PDBAtom[], pose: DockingPose): PDBAtom[] {
    // Apply transformation matrix to ligand atoms
    const matrix = new THREE.Matrix4();
    matrix.fromArray(pose.transformation_matrix);

    return atoms.map(atom => {
      const position = new THREE.Vector3(atom.x, atom.y, atom.z);
      position.applyMatrix4(matrix);

      return {
        ...atom,
        x: position.x,
        y: position.y,
        z: position.z
      };
    });
  }

  function createLigandVisualization(transformedLigand: PDBAtom[]) {
    if (representationType === 'spacefill') {
      createSpacefillRepresentation(transformedLigand);
    } else if (representationType === 'sticks') {
      createSticksRepresentation(transformedLigand);
    } else {
      createSpacefillRepresentation(transformedLigand); // Default
    }
  }

  function createSpacefillRepresentation(atoms: PDBAtom[]) {
    atoms.forEach(atom => {
      const radius = getVdWRadius(atom.element) * 0.8;
      const geometry = new THREE.SphereGeometry(radius, 16, 12);
      
      const color = getElementColor(atom.element);
      const material = new THREE.MeshPhongMaterial({
        color: new THREE.Color(color.r, color.g, color.b),
        shininess: 30,
        transparent: true,
        opacity: 0.9
      });

      const sphere = new THREE.Mesh(geometry, material);
      sphere.position.set(atom.x, atom.y, atom.z);
      sphere.castShadow = true;
      sphere.receiveShadow = true;

      // Store atom data for interaction
      (sphere as any).atomData = atom;

      ligandGroup.add(sphere);
    });
  }

  function createSticksRepresentation(atoms: PDBAtom[]) {
    // For simplicity, create spheres for atoms and cylinders for bonds
    atoms.forEach(atom => {
      const radius = 0.3;
      const geometry = new THREE.SphereGeometry(radius, 12, 8);
      
      const color = getElementColor(atom.element);
      const material = new THREE.MeshPhongMaterial({
        color: new THREE.Color(color.r, color.g, color.b),
        shininess: 30
      });

      const sphere = new THREE.Mesh(geometry, material);
      sphere.position.set(atom.x, atom.y, atom.z);
      sphere.castShadow = true;

      ligandGroup.add(sphere);
    });

    // TODO: Add bonds between atoms (would need bond calculation)
  }

  function createContactsVisualization(contacts: any[], transformedLigand: PDBAtom[]) {
    contacts.forEach(contact => {
      // Create line between receptor and ligand atoms
      const receptorPos = new THREE.Vector3(
        contact.receptor_atom.position[0],
        contact.receptor_atom.position[1],
        contact.receptor_atom.position[2]
      );

      const ligandPos = new THREE.Vector3(
        contact.ligand_atom.position[0],
        contact.ligand_atom.position[1],
        contact.ligand_atom.position[2]
      );

      const points = [receptorPos, ligandPos];
      const geometry = new THREE.BufferGeometry().setFromPoints(points);

      // Color based on interaction type
      let color = 0xffffff;
      switch (contact.interaction_type) {
        case 'hydrogen_bond':
          color = 0x00ff00; // Green
          break;
        case 'van_der_waals':
          color = 0xffff00; // Yellow
          break;
        case 'electrostatic':
          color = 0xff0000; // Red
          break;
        case 'hydrophobic':
          color = 0x00ffff; // Cyan
          break;
      }

      const material = new THREE.LineBasicMaterial({
        color: color,
        transparent: true,
        opacity: 0.6,
        linewidth: 2
      });

      const line = new THREE.Line(geometry, material);
      contactsGroup.add(line);

      // Add interaction label
      const labelGeometry = new THREE.SphereGeometry(0.1, 8, 6);
      const labelMaterial = new THREE.MeshBasicMaterial({ color });
      const label = new THREE.Mesh(labelGeometry, labelMaterial);
      
      const midPoint = new THREE.Vector3()
        .addVectors(receptorPos, ligandPos)
        .multiplyScalar(0.5);
      label.position.copy(midPoint);
      
      contactsGroup.add(label);
    });
  }

  function createLabelsVisualization(atoms: PDBAtom[]) {
    // TODO: Implement text labels using THREE.js text geometry or CSS3D
    // For now, create small markers
    atoms.forEach(atom => {
      const geometry = new THREE.SphereGeometry(0.2, 8, 6);
      const material = new THREE.MeshBasicMaterial({
        color: 0xffffff,
        transparent: true,
        opacity: 0.8
      });

      const marker = new THREE.Mesh(geometry, material);
      marker.position.set(atom.x, atom.y + 2, atom.z);
      labelsGroup.add(marker);
    });
  }

  function centerCameraOnLigand(atoms: PDBAtom[]) {
    if (atoms.length === 0) return;

    // Calculate bounding box
    const box = new THREE.Box3();
    atoms.forEach(atom => {
      box.expandByPoint(new THREE.Vector3(atom.x, atom.y, atom.z));
    });

    const center = box.getCenter(new THREE.Vector3());
    const size = box.getSize(new THREE.Vector3());
    const maxDim = Math.max(size.x, size.y, size.z);

    // Position camera
    controls.target.copy(center);
    camera.position.copy(center);
    camera.position.z += maxDim * 2;
    
    controls.update();
  }

  function toggleReceptorVisibility(visible: boolean) {
    if (receptorGroup) {
      receptorGroup.visible = visible;
    }
  }

  function toggleLigandVisibility(visible: boolean) {
    if (ligandGroup) {
      ligandGroup.visible = visible;
    }
  }

  function toggleContactsVisibility(visible: boolean) {
    if (contactsGroup) {
      contactsGroup.visible = visible;
    }
  }

  function toggleLabelsVisibility(visible: boolean) {
    if (labelsGroup) {
      labelsGroup.visible = visible;
    }
  }

  function getElementColor(element: string): {r: number, g: number, b: number} {
    const colors: Record<string, {r: number, g: number, b: number}> = {
      'C': { r: 0.3, g: 0.3, b: 0.3 },  // Dark gray
      'N': { r: 0.2, g: 0.2, b: 1.0 },  // Blue
      'O': { r: 1.0, g: 0.2, b: 0.2 },  // Red
      'S': { r: 1.0, g: 1.0, b: 0.2 },  // Yellow
      'P': { r: 1.0, g: 0.6, b: 0.2 },  // Orange
      'H': { r: 1.0, g: 1.0, b: 1.0 },  // White
      'F': { r: 0.2, g: 1.0, b: 0.2 },  // Green
      'CL': { r: 0.2, g: 1.0, b: 0.2 }, // Green
      'BR': { r: 0.6, g: 0.2, b: 0.2 }, // Dark red
      'I': { r: 0.4, g: 0.2, b: 0.6 }   // Purple
    };

    return colors[element.toUpperCase()] || { r: 0.8, g: 0.8, b: 0.8 };
  }

  function getVdWRadius(element: string): number {
    const radii: Record<string, number> = {
      'H': 1.20, 'C': 1.70, 'N': 1.55, 'O': 1.52, 'F': 1.47,
      'P': 1.80, 'S': 1.80, 'CL': 1.75, 'BR': 1.85, 'I': 1.98
    };
    return radii[element.toUpperCase()] || 1.5;
  }

  function animate() {
    requestAnimationFrame(animate);
    
    if (controls) {
      controls.update();
    }
    
    if (renderer && scene && camera) {
      renderer.render(scene, camera);
    }
  }

  function handleResize() {
    if (!container || !camera || !renderer) return;

    const width = container.clientWidth;
    const height = container.clientHeight;

    camera.aspect = width / height;
    camera.updateProjectionMatrix();
    renderer.setSize(width, height);
  }

  // Handle window resize
  onMount(() => {
    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  });
</script>

<!-- Pose Visualizer Component -->
<div class="pose-visualizer">
  <div class="visualization-container" bind:this={container}>
    <!-- Loading state -->
    {#if !isInitialized}
      <div class="loading-overlay">
        <div class="loading-spinner"></div>
        <p>Inicializando visualización 3D...</p>
      </div>
    {/if}

    <!-- No data state -->
    {#if isInitialized && !selectedPose}
      <div class="no-data-overlay">
        <p>Selecciona una pose para visualizar</p>
      </div>
    {/if}
  </div>

  <!-- Visualization controls -->
  <div class="visualization-controls">
    <div class="control-section">
      <h4>Visibilidad</h4>
      <label>
        <input type="checkbox" bind:checked={showReceptor}>
        Receptor
      </label>
      <label>
        <input type="checkbox" bind:checked={showLigand}>
        Ligando
      </label>
      <label>
        <input type="checkbox" bind:checked={showContacts}>
        Contactos
      </label>
      <label>
        <input type="checkbox" bind:checked={showLabels}>
        Etiquetas
      </label>
    </div>

    <div class="control-section">
      <h4>Representación</h4>
      <select bind:value={representationType}>
        <option value="spacefill">Spacefill</option>
        <option value="sticks">Sticks</option>
        <option value="cartoon">Cartoon</option>
        <option value="surface">Surface</option>
      </select>
    </div>

    {#if selectedPose}
      <div class="control-section pose-info">
        <h4>Información de la Pose</h4>
        <div class="info-grid">
          <div class="info-item">
            <span>ID:</span>
            <span>{selectedPose.id}</span>
          </div>
          <div class="info-item">
            <span>Energía:</span>
            <span>{selectedPose.energy.toFixed(2)} kcal/mol</span>
          </div>
          <div class="info-item">
            <span>Score:</span>
            <span>{(selectedPose.score * 100).toFixed(1)}%</span>
          </div>
          <div class="info-item">
            <span>Contactos:</span>
            <span>{selectedPose.contacts?.length || 0}</span>
          </div>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .pose-visualizer {
    display: flex;
    height: 600px;
    background: var(--surface-color);
    border-radius: 12px;
    overflow: hidden;
    border: 1px solid var(--border-color);
  }

  .visualization-container {
    flex: 1;
    position: relative;
    background: #1a1a1a;
  }

  .loading-overlay, .no-data-overlay {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    background: rgba(26, 26, 26, 0.9);
    color: white;
    z-index: 10;
  }

  .loading-spinner {
    width: 40px;
    height: 40px;
    border: 4px solid rgba(255, 255, 255, 0.3);
    border-top: 4px solid var(--primary-color);
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin-bottom: 1rem;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  .visualization-controls {
    width: 280px;
    background: var(--background-secondary);
    padding: 1.5rem;
    border-left: 1px solid var(--border-color);
    overflow-y: auto;
  }

  .control-section {
    margin-bottom: 1.5rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid var(--border-color);
  }

  .control-section:last-child {
    border-bottom: none;
    margin-bottom: 0;
  }

  .control-section h4 {
    margin: 0 0 0.75rem 0;
    color: var(--text-primary);
    font-size: 0.95rem;
    font-weight: 600;
  }

  .control-section label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
    font-size: 0.9rem;
    color: var(--text-primary);
    cursor: pointer;
  }

  .control-section input[type="checkbox"] {
    margin: 0;
  }

  .control-section select {
    width: 100%;
    padding: 0.5rem;
    border: 1px solid var(--border-color);
    border-radius: 6px;
    background: var(--surface-color);
    color: var(--text-primary);
    font-size: 0.9rem;
  }

  .pose-info .info-grid {
    display: grid;
    gap: 0.5rem;
  }

  .info-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.85rem;
  }

  .info-item span:first-child {
    color: var(--text-secondary);
    font-weight: 500;
  }

  .info-item span:last-child {
    color: var(--text-primary);
    font-weight: 600;
    font-family: 'Courier New', monospace;
  }

  /* Responsive */
  @media (max-width: 1024px) {
    .pose-visualizer {
      flex-direction: column;
      height: auto;
    }

    .visualization-container {
      height: 400px;
    }

    .visualization-controls {
      width: 100%;
      border-left: none;
      border-top: 1px solid var(--border-color);
    }
  }

  @media (max-width: 768px) {
    .visualization-controls {
      padding: 1rem;
    }

    .visualization-container {
      height: 300px;
    }
  }
</style>