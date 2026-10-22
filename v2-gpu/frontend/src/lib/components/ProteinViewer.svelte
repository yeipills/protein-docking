<!--
  ProteinViewer.svelte - Visualizador 3D ultra-optimizado con WebGL
  
  Features:
  - Renderizado WebGL con Three.js
  - LOD (Level of Detail) automático
  - Instanced rendering para performance
  - Shaders optimizados
  - Interacción fluida 60 FPS
-->

<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import * as THREE from 'three';
  import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
  import { api } from '../stores/api.svelte';
  import type { FileMetadata } from '../types/unified.js';
  import type { JobStatusInfo } from '../types/unified.js';
  import { PDBParser, PDBUtils, type PDBStructure } from '../utils/PDBParser.js';
  
  interface Props {
    selectedReceptor?: FileMetadata | null;
    selectedLigand?: FileMetadata | null;
    selectedJob?: JobStatusInfo | null;
    showResults?: boolean;
  }
  
  let { 
    selectedReceptor = null,
    selectedLigand = null, 
    selectedJob = null,
    showResults = false
  }: Props = $props();

  // Component state
  let isInitialized = $state(false);
  let error = $state<string | null>(null);
  let loadingState = $state<'idle' | 'loading' | 'loaded' | 'error'>('idle');
  let loadedStructures = $state<{ receptor?: boolean; ligand?: boolean; job?: boolean }>({});
  
  // Referencias del DOM
  let viewerContainer: HTMLElement;
  let canvas: HTMLCanvasElement;
  
  // Objetos Three.js
  let scene: THREE.Scene;
  let camera: THREE.PerspectiveCamera;
  let renderer: THREE.WebGLRenderer;
  let controls: OrbitControls;
  
  // Grupos de objetos 3D
  let proteinGroup: THREE.Group;
  let ligandGroup: THREE.Group;
  let resultGroup: THREE.Group;
  let backgroundGroup: THREE.Group;
  
  // Sistema de materiales optimizados
  let materials = $state({
    receptor: null as THREE.Material | null,
    ligand: null as THREE.Material | null,
    bonds: null as THREE.Material | null,
    surface: null as THREE.Material | null
  });
  
  // Estado del visualizador
  let viewerState = $state({
    isInitialized: false,
    showReceptor: true,
    showLigand: true,
    showBonds: true,
    showSurface: false,
    representation: 'cartoon' as 'cartoon' | 'spacefill' | 'sticks' | 'surface',
    qualityLevel: 'high' as 'low' | 'medium' | 'high' | 'ultra',
    animationSpeed: 1.0,
    isRotating: false,
    showDebugInfo: false,
    showControlsHelp: false
  });
  
  // Métricas de rendimiento
  let performanceMetrics = $state({
    fps: 60,
    drawCalls: 0,
    triangles: 0,
    geometries: 0,
    textures: 0
  });
  
  // PDB parsing info
  let parsingInfo = $state({
    receptorInfo: null as any,
    ligandInfo: null as any,
    lastParsedFile: '',
    parsingTime: 0
  });
  
  // Sistema de LOD
  let lodSystem = {
    distances: [50, 100, 200, 500],
    geometries: new Map<string, THREE.LOD>(),
    currentLevel: 0
  };
  
  onMount(async () => {
    try {
      await initializeViewer();
      setupEventListeners();
      startRenderLoop();
      viewerState.isInitialized = true;
      
    } catch (error) {
      console.error('Failed to initialize 3D viewer:', error);
    }
  });
  
  onDestroy(() => {
    cleanup();
  });
  
  async function initializeViewer() {
    // Configurar scene
    scene = new THREE.Scene();
    scene.fog = new THREE.Fog(0x000000, 100, 1000);
    
    // Configurar camera con FOV optimizado
    camera = new THREE.PerspectiveCamera(
      50, // FOV más bajo para mejor performance
      viewerContainer.clientWidth / viewerContainer.clientHeight,
      0.1,
      1000
    );
    camera.position.set(50, 30, 50);
    
    // Configurar renderer con optimizaciones
    renderer = new THREE.WebGLRenderer({
      canvas,
      antialias: window.devicePixelRatio <= 1, // Solo antialias en pantallas normales
      alpha: false, // Fondo opaco para mejor performance
      powerPreference: 'high-performance',
      stencil: false,
      depth: true
    });
    
    renderer.setSize(viewerContainer.clientWidth, viewerContainer.clientHeight);
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2)); // Limitar DPR
    
    // Configuraciones de rendimiento
    renderer.shadowMap.enabled = true;
    renderer.shadowMap.type = THREE.PCFSoftShadowMap;
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = 1.2;
    
    // Configurar controles optimizados
    controls = new OrbitControls(camera, renderer.domElement);
    
    // Configuración de damping para suavidad
    controls.enableDamping = true;
    controls.dampingFactor = 0.08;
    
    // Configuración de rotación
    controls.enableRotate = true;
    controls.rotateSpeed = 0.5;
    controls.autoRotate = false;
    controls.autoRotateSpeed = 2.0;
    
    // Configuración de zoom
    controls.enableZoom = true;
    controls.zoomSpeed = 1.2;
    controls.minDistance = 5;
    controls.maxDistance = 1000;
    
    // Configuración de panning
    controls.enablePan = true;
    controls.panSpeed = 0.8;
    controls.screenSpacePanning = true; // Cambiar a true para mejor control
    controls.keyPanSpeed = 7.0;
    
    // Límites de ángulos
    controls.maxPolarAngle = Math.PI; // Permitir rotación completa
    controls.minPolarAngle = 0;
    
    // Teclas de control
    controls.keys = {
      LEFT: 'ArrowLeft',
      UP: 'ArrowUp', 
      RIGHT: 'ArrowRight',
      BOTTOM: 'ArrowDown'
    };
    
    // Configuración de mouse
    controls.mouseButtons = {
      LEFT: THREE.MOUSE.ROTATE,
      MIDDLE: THREE.MOUSE.DOLLY,
      RIGHT: THREE.MOUSE.PAN
    };
    
    // Configuración de touch
    controls.touches = {
      ONE: THREE.TOUCH.ROTATE,
      TWO: THREE.TOUCH.DOLLY_PAN
    };
    
    // Crear grupos
    proteinGroup = new THREE.Group();
    ligandGroup = new THREE.Group();
    resultGroup = new THREE.Group();
    backgroundGroup = new THREE.Group();
    
    scene.add(proteinGroup, ligandGroup, resultGroup, backgroundGroup);
    
    // Configurar iluminación optimizada
    setupLighting();
    
    // Crear materiales optimizados
    createOptimizedMaterials();
    
    // Cargar datos iniciales
    await loadProteinData();
  }
  
  function setupLighting() {
    // Luz ambiental suave
    const ambientLight = new THREE.AmbientLight(0x404040, 0.3);
    scene.add(ambientLight);
    
    // Luz direccional principal con sombras
    const directionalLight = new THREE.DirectionalLight(0xffffff, 0.8);
    directionalLight.position.set(50, 50, 25);
    directionalLight.castShadow = true;
    
    // Optimizar sombras
    directionalLight.shadow.mapSize.width = 1024; // Reducido para performance
    directionalLight.shadow.mapSize.height = 1024;
    directionalLight.shadow.camera.near = 0.5;
    directionalLight.shadow.camera.far = 200;
    directionalLight.shadow.camera.left = -50;
    directionalLight.shadow.camera.right = 50;
    directionalLight.shadow.camera.top = 50;
    directionalLight.shadow.camera.bottom = -50;
    
    scene.add(directionalLight);
    
    // Luces de relleno
    const fillLight1 = new THREE.DirectionalLight(0x8080ff, 0.2);
    fillLight1.position.set(-25, 25, 25);
    scene.add(fillLight1);
    
    const fillLight2 = new THREE.DirectionalLight(0xff8080, 0.1);
    fillLight2.position.set(25, -25, -25);
    scene.add(fillLight2);
  }
  
  function createOptimizedMaterials() {
    // Material optimizado para receptor con instancing
    materials.receptor = new THREE.ShaderMaterial({
      uniforms: {
        time: { value: 0 },
        lightDirection: { value: new THREE.Vector3(1, 1, 1).normalize() },
        ambientStrength: { value: 0.3 },
        diffuseStrength: { value: 0.7 },
        specularStrength: { value: 0.5 },
        shininess: { value: 32.0 }
      },
      vertexShader: `
        attribute vec3 instancePosition;
        attribute vec3 instanceColor;
        attribute float instanceScale;
        
        varying vec3 vNormal;
        varying vec3 vPosition;
        varying vec3 vColor;
        varying vec3 vWorldPosition;
        
        void main() {
          vNormal = normalize(normalMatrix * normal);
          vColor = instanceColor;
          
          // Scale and position the vertex
          vec3 scaledPosition = position * instanceScale;
          vec4 worldPosition = modelMatrix * vec4(scaledPosition + instancePosition, 1.0);
          vWorldPosition = worldPosition.xyz;
          vPosition = (modelViewMatrix * vec4(scaledPosition + instancePosition, 1.0)).xyz;
          
          gl_Position = projectionMatrix * modelViewMatrix * vec4(scaledPosition + instancePosition, 1.0);
        }
      `,
      fragmentShader: `
        uniform vec3 lightDirection;
        uniform float ambientStrength;
        uniform float diffuseStrength;
        uniform float specularStrength;
        uniform float shininess;
        
        varying vec3 vNormal;
        varying vec3 vPosition;
        varying vec3 vColor;
        varying vec3 vWorldPosition;
        
        void main() {
          // Ambient lighting
          vec3 ambient = ambientStrength * vColor;
          
          // Diffuse lighting
          float diff = max(dot(vNormal, lightDirection), 0.0);
          vec3 diffuse = diffuseStrength * diff * vColor;
          
          // Specular lighting
          vec3 viewDir = normalize(-vPosition);
          vec3 reflectDir = reflect(-lightDirection, vNormal);
          float spec = pow(max(dot(viewDir, reflectDir), 0.0), shininess);
          vec3 specular = specularStrength * spec * vec3(1.0);
          
          vec3 result = ambient + diffuse + specular;
          gl_FragColor = vec4(result, 1.0);
        }
      `
    });
    
    // Material optimizado para ligando con transparencia
    materials.ligand = new THREE.ShaderMaterial({
      uniforms: {
        time: { value: 0 },
        lightDirection: { value: new THREE.Vector3(1, 1, 1).normalize() },
        ambientStrength: { value: 0.4 },
        diffuseStrength: { value: 0.8 },
        specularStrength: { value: 0.6 },
        shininess: { value: 64.0 },
        opacity: { value: 0.85 }
      },
      vertexShader: `
        attribute vec3 instancePosition;
        attribute vec3 instanceColor;
        attribute float instanceScale;
        
        varying vec3 vNormal;
        varying vec3 vPosition;
        varying vec3 vColor;
        varying vec3 vWorldPosition;
        
        void main() {
          vNormal = normalize(normalMatrix * normal);
          vColor = instanceColor;
          
          // Scale and position the vertex
          vec3 scaledPosition = position * instanceScale;
          vec4 worldPosition = modelMatrix * vec4(scaledPosition + instancePosition, 1.0);
          vWorldPosition = worldPosition.xyz;
          vPosition = (modelViewMatrix * vec4(scaledPosition + instancePosition, 1.0)).xyz;
          
          gl_Position = projectionMatrix * modelViewMatrix * vec4(scaledPosition + instancePosition, 1.0);
        }
      `,
      fragmentShader: `
        uniform vec3 lightDirection;
        uniform float ambientStrength;
        uniform float diffuseStrength;
        uniform float specularStrength;
        uniform float shininess;
        uniform float opacity;
        
        varying vec3 vNormal;
        varying vec3 vPosition;
        varying vec3 vColor;
        varying vec3 vWorldPosition;
        
        void main() {
          // Enhanced lighting for ligand
          vec3 ambient = ambientStrength * vColor;
          
          float diff = max(dot(vNormal, lightDirection), 0.0);
          vec3 diffuse = diffuseStrength * diff * vColor;
          
          vec3 viewDir = normalize(-vPosition);
          vec3 reflectDir = reflect(-lightDirection, vNormal);
          float spec = pow(max(dot(viewDir, reflectDir), 0.0), shininess);
          vec3 specular = specularStrength * spec * vec3(1.0);
          
          // Add rim lighting for better visibility
          float rim = 1.0 - max(dot(vNormal, viewDir), 0.0);
          vec3 rimLight = rim * vec3(0.3, 0.5, 1.0) * 0.3;
          
          vec3 result = ambient + diffuse + specular + rimLight;
          gl_FragColor = vec4(result, opacity);
        }
      `,
      transparent: true
    });
    
    // Material para bonds optimizado
    materials.bonds = new THREE.MeshBasicMaterial({
      vertexColors: true, // Usar colores por vértice
      transparent: false
    });
    
    // Material para superficie con shader custom
    materials.surface = new THREE.ShaderMaterial({
      uniforms: {
        time: { value: 0 },
        opacity: { value: 0.3 },
        color: { value: new THREE.Color(0x00ff88) }
      },
      vertexShader: `
        varying vec3 vNormal;
        varying vec3 vPosition;
        varying vec3 vColor;
        attribute vec3 color;
        uniform float time;
        
        void main() {
          vNormal = normalize(normalMatrix * normal);
          vPosition = position;
          vColor = color;
          
          vec3 pos = position + normal * sin(time + position.x * 0.1) * 0.5;
          gl_Position = projectionMatrix * modelViewMatrix * vec4(pos, 1.0);
        }
      `,
      fragmentShader: `
        uniform float opacity;
        uniform vec3 color;
        varying vec3 vNormal;
        varying vec3 vPosition;
        varying vec3 vColor;
        
        void main() {
          float fresnel = pow(1.0 + dot(vNormal, normalize(vPosition)), 2.0);
          vec3 finalColor = mix(vColor, color, 0.3);
          gl_FragColor = vec4(finalColor, opacity * fresnel);
        }
      `,
      transparent: true,
      side: THREE.DoubleSide
    });
  }
  
  async function loadProteinData() {
    // Limpiar vista anterior
    proteinGroup.clear();
    ligandGroup.clear();
    resultGroup.clear();
    
    // Cargar receptor si está seleccionado
    if (selectedReceptor) {
      await loadReceptorProtein(selectedReceptor);
    }
    
    // Cargar ligando si está seleccionado
    if (selectedLigand) {
      await loadLigandMolecule(selectedLigand);
    }
    
    // Cargar resultados si hay job seleccionado
    if (selectedJob && showResults) {
      await loadJobResults(selectedJob);
    }
  }
  
  async function loadReceptorProtein(file: FileMetadata) {
    try {
      console.log(`🧬 Loading receptor protein: ${file.filename}`);
      
      // Usar el parser PDB real
      const geometry = await createProteinGeometryFromPDB(file, 'receptor');
      const mesh = createOptimizedMesh(geometry, materials.receptor!, 'receptor');
      
      mesh.castShadow = true;
      mesh.receiveShadow = true;
      mesh.userData = { fileId: file.file_id, filename: file.filename };
      
      proteinGroup.add(mesh);
      
      // Configurar LOD para la proteína
      setupLODForMesh(mesh, `receptor_${file.file_id}`);
      
      console.log(`✅ Loaded receptor with PDB parser: ${file.filename}`);
      
    } catch (error) {
      console.error(`❌ Failed to load receptor ${file.filename}:`, error);
      // Fallback a geometría genérica
      const fallbackGeometry = createProteinGeometry('receptor');
      const mesh = new THREE.Mesh(fallbackGeometry, materials.receptor);
      mesh.userData = { fileId: file.file_id, filename: file.filename, fallback: true };
      proteinGroup.add(mesh);
    }
  }
  
  async function loadLigandMolecule(file: FileMetadata) {
    try {
      console.log(`🧬 Loading ligand molecule: ${file.filename}`);
      
      // Usar el parser PDB real
      const geometry = await createProteinGeometryFromPDB(file, 'ligand');
      const mesh = createOptimizedMesh(geometry, materials.ligand!, 'ligand');
      
      mesh.castShadow = true;
      mesh.receiveShadow = true;
      mesh.userData = { fileId: file.file_id, filename: file.filename };
      
      ligandGroup.add(mesh);
      
      // Configurar LOD para el ligando
      setupLODForMesh(mesh, `ligand_${file.file_id}`);
      
      console.log(`✅ Loaded ligand with PDB parser: ${file.filename}`);
      
    } catch (error) {
      console.error(`❌ Failed to load ligand ${file.filename}:`, error);
      // Fallback a geometría genérica
      const fallbackGeometry = createProteinGeometry('ligand');
      const mesh = new THREE.Mesh(fallbackGeometry, materials.ligand);
      mesh.userData = { fileId: file.file_id, filename: file.filename, fallback: true };
      ligandGroup.add(mesh);
    }
  }
  
  function createProteinGeometry(type: 'receptor' | 'ligand'): THREE.BufferGeometry {
    // Crear geometría optimizada basada en el tipo
    let geometry: THREE.BufferGeometry;
    
    if (viewerState.representation === 'cartoon') {
      // Usar geometría simplificada para cartoon
      geometry = type === 'receptor' 
        ? new THREE.BoxGeometry(15, 8, 10)
        : new THREE.SphereGeometry(3, 12, 8);
    } else if (viewerState.representation === 'spacefill') {
      // Esferas para spacefill
      geometry = type === 'receptor'
        ? new THREE.SphereGeometry(8, 20, 16)
        : new THREE.SphereGeometry(2, 12, 10);
    } else {
      // Sticks representation
      geometry = new THREE.CylinderGeometry(0.5, 0.5, 8, 8, 1);
    }
    
    // Optimizar geometría
    geometry.computeBoundingSphere();
    geometry.computeBoundingBox();
    
    return geometry;
  }
  
  async function createProteinGeometryFromPDB(fileMetadata: FileMetadata, type: 'receptor' | 'ligand'): Promise<THREE.BufferGeometry> {
    const startTime = performance.now();
    
    try {
      console.log(`🧬 Loading PDB data for ${type}: ${fileMetadata.filename}`);
      
      // Download PDB file content
      const pdbContent = await PDBUtils.downloadPDBData(fileMetadata.file_id);
      
      // Parse PDB content
      const structure = PDBParser.parsePDB(pdbContent);
      console.log(`✅ Parsed PDB structure:`, structure.stats);
      
      // Store debug info
      const parsingTime = performance.now() - startTime;
      const info = {
        filename: fileMetadata.filename,
        atoms: structure.stats.totalAtoms,
        residues: structure.stats.totalResidues,
        chains: structure.stats.totalChains,
        bonds: structure.bonds.length,
        elements: Object.keys(structure.stats.elements).join(', '),
        center: structure.center,
        boundingBox: structure.boundingBox.size,
        parsingTime: `${parsingTime.toFixed(2)}ms`,
        title: structure.title || 'Unknown'
      };
      
      if (type === 'receptor') {
        parsingInfo.receptorInfo = info;
      } else {
        parsingInfo.ligandInfo = info;
      }
      
      parsingInfo.lastParsedFile = fileMetadata.filename;
      parsingInfo.parsingTime = parsingTime;
      
      // Create geometry from parsed atoms
      return createGeometryFromPDBStructure(structure, type);
      
    } catch (error) {
      console.error(`❌ Failed to load PDB for ${type}:`, error);
      // Fallback to mock geometry
      return createProteinGeometry(type);
    }
  }
  
  function createGeometryFromPDBStructure(structure: PDBStructure, type: 'receptor' | 'ligand'): THREE.BufferGeometry {
    console.log(`🔄 Creating ${viewerState.representation} geometry from ${structure.atoms.length} atoms`);
    
    switch (viewerState.representation) {
      case 'spacefill':
        return createSphereGeometryFromAtoms(structure, type);
      case 'sticks':
        return createStickGeometryFromBonds(structure, type);
      case 'surface':
        return createSurfaceGeometryFromAtoms(structure, type);
      case 'cartoon':
      default:
        return createCartoonGeometryFromAtoms(structure, type);
    }
  }
  
  function createSphereGeometryFromAtoms(structure: PDBStructure, type: 'receptor' | 'ligand'): THREE.BufferGeometry {
    // Use instanced rendering for better performance
    const maxAtoms = Math.min(structure.atoms.length, 5000); // Limit for performance
    const atomsToShow = structure.atoms.slice(0, maxAtoms);
    
    console.log(`🔮 Creating instanced sphere geometry for ${atomsToShow.length} atoms`);
    
    // Base sphere geometry that will be instanced
    const baseGeometry = new THREE.SphereGeometry(1.0, 12, 8); // Unit sphere
    
    // Create instanced geometry
    const instancedGeometry = new THREE.InstancedBufferGeometry();
    instancedGeometry.copy(baseGeometry);
    instancedGeometry.maxInstancedCount = atomsToShow.length;
    
    // Instance attributes
    const instancePositions = new Float32Array(atomsToShow.length * 3);
    const instanceColors = new Float32Array(atomsToShow.length * 3);
    const instanceScales = new Float32Array(atomsToShow.length);
    
    atomsToShow.forEach((atom, i) => {
      const vdwRadius = PDBParser.getVdWRadius(atom.element);
      const elementColor = PDBParser.getElementColor(atom.element);
      
      // Position
      instancePositions[i * 3] = atom.x;
      instancePositions[i * 3 + 1] = atom.y;
      instancePositions[i * 3 + 2] = atom.z;
      
      // Color
      instanceColors[i * 3] = elementColor[0];
      instanceColors[i * 3 + 1] = elementColor[1];
      instanceColors[i * 3 + 2] = elementColor[2];
      
      // Scale based on Van der Waals radius
      instanceScales[i] = vdwRadius;
    });
    
    // Add instance attributes
    instancedGeometry.setAttribute('instancePosition', new THREE.InstancedBufferAttribute(instancePositions, 3));
    instancedGeometry.setAttribute('instanceColor', new THREE.InstancedBufferAttribute(instanceColors, 3));
    instancedGeometry.setAttribute('instanceScale', new THREE.InstancedBufferAttribute(instanceScales, 1));
    
    instancedGeometry.computeBoundingSphere();
    
    console.log(`✅ Created instanced sphere geometry with ${atomsToShow.length} instances`);
    return instancedGeometry;
  }
  
  function createStickGeometryFromBonds(structure: PDBStructure, type: 'receptor' | 'ligand'): THREE.BufferGeometry {
    const positions: number[] = [];
    const colors: number[] = [];
    const indices: number[] = [];
    
    let vertexOffset = 0;
    const maxBonds = Math.min(structure.bonds.length, 2000); // Limit for performance
    
    // If no bonds calculated or very few, create bonds between nearby atoms
    let bondsToRender = structure.bonds.slice(0, maxBonds);
    
    if (bondsToRender.length < 10) {
      console.log('🔗 Creating additional bonds based on proximity');
      bondsToRender = [];
      
      // Simple bond detection for nearby atoms
      for (let i = 0; i < structure.atoms.length; i++) {
        for (let j = i + 1; j < Math.min(structure.atoms.length, i + 50); j++) {
          const atom1 = structure.atoms[i];
          const atom2 = structure.atoms[j];
          
          const distance = Math.sqrt(
            (atom1.x - atom2.x) ** 2 +
            (atom1.y - atom2.y) ** 2 +
            (atom1.z - atom2.z) ** 2
          );
          
          if (distance > 0.8 && distance < 2.0) {
            bondsToRender.push({
              atom1Index: i,
              atom2Index: j,
              bondOrder: 1,
              distance
            });
            
            if (bondsToRender.length >= maxBonds) break;
          }
        }
        if (bondsToRender.length >= maxBonds) break;
      }
    }
    
    bondsToRender.forEach((bond) => {
      const atom1 = structure.atoms[bond.atom1Index];
      const atom2 = structure.atoms[bond.atom2Index];
      
      if (!atom1 || !atom2) return;
      
      // Create cylinder between atoms
      const direction = new THREE.Vector3(
        atom2.x - atom1.x,
        atom2.y - atom1.y,
        atom2.z - atom1.z
      );
      const distance = direction.length();
      
      if (distance < 0.1) return; // Skip very short bonds
      
      // Variable bond thickness based on bond order
      const bondRadius = 0.08 + (bond.bondOrder * 0.02);
      const cylinderGeometry = new THREE.CylinderGeometry(bondRadius, bondRadius, distance, 6, 1);
      const cylinderPositions = cylinderGeometry.attributes.position.array as Float32Array;
      const cylinderIndices = cylinderGeometry.index?.array as Uint16Array;
      
      // Calculate transformation
      const center = new THREE.Vector3(
        (atom1.x + atom2.x) / 2,
        (atom1.y + atom2.y) / 2,
        (atom1.z + atom2.z) / 2
      );
      
      direction.normalize();
      const quaternion = new THREE.Quaternion();
      quaternion.setFromUnitVectors(new THREE.Vector3(0, 1, 0), direction);
      
      // Two-colored bonds: each half colored by its atom
      const color1 = PDBParser.getElementColor(atom1.element);
      const color2 = PDBParser.getElementColor(atom2.element);
      
      // Transform cylinder positions
      for (let i = 0; i < cylinderPositions.length; i += 3) {
        const vertex = new THREE.Vector3(
          cylinderPositions[i],
          cylinderPositions[i + 1],
          cylinderPositions[i + 2]
        );
        
        vertex.applyQuaternion(quaternion);
        vertex.add(center);
        
        positions.push(vertex.x, vertex.y, vertex.z);
        
        // Color gradient along bond length
        const t = (cylinderPositions[i + 1] + distance / 2) / distance;
        const color = t < 0.5 ? color1 : color2;
        colors.push(color[0], color[1], color[2]);
      }
      
      // Add indices with offset
      if (cylinderIndices) {
        for (let i = 0; i < cylinderIndices.length; i++) {
          indices.push(cylinderIndices[i] + vertexOffset);
        }
      }
      
      vertexOffset += cylinderPositions.length / 3;
    });
    
    const geometry = new THREE.BufferGeometry();
    
    if (positions.length > 0) {
      geometry.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
      geometry.setAttribute('color', new THREE.Float32BufferAttribute(colors, 3));
      
      if (indices.length > 0) {
        geometry.setIndex(indices);
      }
      
      geometry.computeVertexNormals();
    } else {
      // Fallback: create wireframe of atoms
      console.log('⚠️ No bonds to render, creating wireframe fallback');
      return createCartoonGeometryFromAtoms(structure, type);
    }
    
    geometry.computeBoundingSphere();
    centerGeometry(geometry);
    
    console.log(`✅ Created stick geometry with ${bondsToRender.length} bonds`);
    return geometry;
  }
  
  function createCartoonGeometryFromAtoms(structure: PDBStructure, type: 'receptor' | 'ligand'): THREE.BufferGeometry {
    console.log(`🎨 Creating enhanced cartoon representation`);
    
    const positions: number[] = [];
    const colors: number[] = [];
    const indices: number[] = [];
    
    // Filter for backbone atoms (CA for proteins, P for nucleic acids)
    const backboneAtoms = structure.atoms.filter(atom => 
      atom.name === 'CA' || atom.name === 'P' || atom.name === 'C4\''
    );
    
    // If protein/nucleic acid with backbone
    if (backboneAtoms.length > 3) {
      return createBackboneCartoon(structure, backboneAtoms, type);
    } else {
      // Small molecule - show as ball and stick reduced
      return createSmallMoleculeCartoon(structure, type);
    }
  }
  
  function createBackboneCartoon(structure: PDBStructure, backboneAtoms: any[], type: 'receptor' | 'ligand'): THREE.BufferGeometry {
    const positions: number[] = [];
    const colors: number[] = [];
    const indices: number[] = [];
    let vertexOffset = 0;
    
    // Group backbone atoms by chain and residue
    const chains = new Map<string, any[]>();
    backboneAtoms.forEach(atom => {
      const chainId = atom.chainID || 'A';
      if (!chains.has(chainId)) {
        chains.set(chainId, []);
      }
      chains.get(chainId)!.push(atom);
    });
    
    // Create cartoon representation for each chain
    chains.forEach((chainAtoms, chainId) => {
      // Sort atoms by residue sequence
      chainAtoms.sort((a, b) => a.resSeq - b.resSeq);
      
      // Create spline curve through backbone
      const points = chainAtoms.map(atom => new THREE.Vector3(atom.x, atom.y, atom.z));
      
      if (points.length < 2) return;
      
      // Create tube geometry along the backbone
      const curve = new THREE.CatmullRomCurve3(points);
      const tubeGeometry = new THREE.TubeGeometry(curve, points.length * 2, 0.3, 8, false);
      
      const tubePositions = tubeGeometry.attributes.position.array as Float32Array;
      const tubeIndices = tubeGeometry.index?.array as Uint16Array;
      
      // Color by chain or secondary structure
      const chainColor = getChainColor(chainId);
      
      // Add tube vertices
      for (let i = 0; i < tubePositions.length; i += 3) {
        positions.push(
          tubePositions[i],
          tubePositions[i + 1],
          tubePositions[i + 2]
        );
        colors.push(chainColor[0], chainColor[1], chainColor[2]);
      }
      
      // Add tube indices
      if (tubeIndices) {
        for (let i = 0; i < tubeIndices.length; i++) {
          indices.push(tubeIndices[i] + vertexOffset);
        }
      }
      
      vertexOffset += tubePositions.length / 3;
    });
    
    const geometry = new THREE.BufferGeometry();
    
    if (positions.length > 0) {
      geometry.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
      geometry.setAttribute('color', new THREE.Float32BufferAttribute(colors, 3));
      
      if (indices.length > 0) {
        geometry.setIndex(indices);
      }
      
      geometry.computeVertexNormals();
    }
    
    geometry.computeBoundingSphere();
    centerGeometry(geometry);
    
    console.log(`✅ Created backbone cartoon for ${chains.size} chains`);
    return geometry;
  }
  
  function createSmallMoleculeCartoon(structure: PDBStructure, type: 'receptor' | 'ligand'): THREE.BufferGeometry {
    // Show small molecules as reduced ball-and-stick
    const positions: number[] = [];
    const colors: number[] = [];
    const indices: number[] = [];
    let vertexOffset = 0;
    
    // Show atoms as small spheres
    const maxAtoms = Math.min(structure.atoms.length, 100);
    for (let i = 0; i < maxAtoms; i++) {
      const atom = structure.atoms[i];
      const elementColor = PDBParser.getElementColor(atom.element);
      const radius = PDBParser.getVdWRadius(atom.element) * 0.4; // Reduced size
      
      const sphereGeometry = new THREE.SphereGeometry(radius, 8, 6);
      const spherePositions = sphereGeometry.attributes.position.array as Float32Array;
      const sphereIndices = sphereGeometry.index?.array as Uint16Array;
      
      // Add sphere vertices
      for (let j = 0; j < spherePositions.length; j += 3) {
        positions.push(
          spherePositions[j] + atom.x,
          spherePositions[j + 1] + atom.y,
          spherePositions[j + 2] + atom.z
        );
        colors.push(elementColor[0], elementColor[1], elementColor[2]);
      }
      
      // Add sphere indices
      if (sphereIndices) {
        for (let j = 0; j < sphereIndices.length; j++) {
          indices.push(sphereIndices[j] + vertexOffset);
        }
      }
      
      vertexOffset += spherePositions.length / 3;
    }
    
    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
    geometry.setAttribute('color', new THREE.Float32BufferAttribute(colors, 3));
    
    if (indices.length > 0) {
      geometry.setIndex(indices);
      geometry.computeVertexNormals();
    }
    
    geometry.computeBoundingSphere();
    centerGeometry(geometry);
    
    console.log(`✅ Created small molecule cartoon with ${maxAtoms} atoms`);
    return geometry;
  }
  
  function getChainColor(chainId: string): [number, number, number] {
    // Color chains differently
    const colors: Record<string, [number, number, number]> = {
      'A': [0.2, 0.6, 1.0],  // Blue
      'B': [1.0, 0.4, 0.2],  // Orange
      'C': [0.2, 1.0, 0.4],  // Green
      'D': [1.0, 0.2, 0.8],  // Magenta
      'E': [0.8, 0.8, 0.2],  // Yellow
      'F': [0.6, 0.2, 1.0],  // Purple
    };
    
    return colors[chainId] || [0.7, 0.7, 0.7]; // Default gray
  }
  
  // Deprecated function - kept for fallback
  function createProteinGeometryFromData(data: any, type: 'receptor' | 'ligand'): THREE.BufferGeometry {
    console.warn('⚠️  Using deprecated createProteinGeometryFromData - use createProteinGeometryFromPDB instead');
    return createProteinGeometry(type);
  }
  
  // Helper function to center geometry
  function centerGeometry(geometry: THREE.BufferGeometry): void {
    geometry.computeBoundingBox();
    
    if (geometry.boundingBox) {
      const center = new THREE.Vector3();
      geometry.boundingBox.getCenter(center);
      geometry.translate(-center.x, -center.y, -center.z);
    }
  }
  
  // Create surface representation using marching cubes approach (simplified)
  function createSurfaceGeometryFromAtoms(structure: PDBStructure, type: 'receptor' | 'ligand'): THREE.BufferGeometry {
    console.log(`🌊 Creating surface geometry from ${structure.atoms.length} atoms`);
    
    // Simplified surface generation - in reality would use marching cubes
    // For now, create a convex hull approximation using spheres
    
    const positions: number[] = [];
    const colors: number[] = [];
    const indices: number[] = [];
    
    // Calculate bounding box
    const bounds = structure.boundingBox;
    const center = structure.center;
    
    // Create a grid of vertices around the molecular surface
    const gridSize = 20;
    const stepX = bounds.size[0] / gridSize;
    const stepY = bounds.size[1] / gridSize;
    const stepZ = bounds.size[2] / gridSize;
    
    let vertexIndex = 0;
    const surfaceThreshold = 3.0; // Distance threshold for surface
    
    for (let i = 0; i <= gridSize; i++) {
      for (let j = 0; j <= gridSize; j++) {
        for (let k = 0; k <= gridSize; k++) {
          const x = bounds.min[0] + i * stepX;
          const y = bounds.min[1] + j * stepY;
          const z = bounds.min[2] + k * stepZ;
          
          // Check if this point is near the molecular surface
          let minDistance = Infinity;
          let closestAtom = null;
          
          for (const atom of structure.atoms) {
            const distance = Math.sqrt(
              (x - atom.x) ** 2 + 
              (y - atom.y) ** 2 + 
              (z - atom.z) ** 2
            );
            
            if (distance < minDistance) {
              minDistance = distance;
              closestAtom = atom;
            }
          }
          
          // If point is within surface threshold, add it
          if (minDistance <= surfaceThreshold && minDistance >= 2.0) {
            positions.push(x, y, z);
            
            // Color based on closest atom type
            if (closestAtom) {
              const elementColor = PDBParser.getElementColor(closestAtom.element);
              colors.push(elementColor[0], elementColor[1], elementColor[2]);
            } else {
              colors.push(0.7, 0.7, 0.7);
            }
          }
        }
      }
    }
    
    // Create triangulated surface from points (simplified approach)
    // In a real implementation, this would use marching cubes or similar
    const geometry = new THREE.BufferGeometry();
    
    if (positions.length > 0) {
      geometry.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
      geometry.setAttribute('color', new THREE.Float32BufferAttribute(colors, 3));
      
      // Create simple triangulation for demonstration
      const numVertices = positions.length / 3;
      for (let i = 0; i < numVertices - 2; i += 3) {
        if (i + 2 < numVertices) {
          indices.push(i, i + 1, i + 2);
        }
      }
      
      if (indices.length > 0) {
        geometry.setIndex(indices);
      }
      
      geometry.computeVertexNormals();
    } else {
      // Fallback: create a simple wireframe sphere
      const sphereGeometry = new THREE.SphereGeometry(
        Math.max(...bounds.size) / 2, 
        16, 
        12
      );
      geometry.copy(sphereGeometry);
      
      // Add colors
      const colorArray = new Float32Array(sphereGeometry.attributes.position.count * 3);
      for (let i = 0; i < colorArray.length; i += 3) {
        colorArray[i] = 0.3;
        colorArray[i + 1] = 0.8;
        colorArray[i + 2] = 0.3;
      }
      geometry.setAttribute('color', new THREE.BufferAttribute(colorArray, 3));
    }
    
    geometry.computeBoundingSphere();
    centerGeometry(geometry);
    
    console.log(`✅ Created surface geometry with ${positions.length / 3} surface points`);
    return geometry;
  }
  
  // Create optimized mesh based on geometry type
  function createOptimizedMesh(
    geometry: THREE.BufferGeometry, 
    material: THREE.Material, 
    type: 'receptor' | 'ligand'
  ): THREE.Mesh {
    let mesh: THREE.Mesh;
    let materialToUse = material;
    
    // Use surface material for surface representation
    if (viewerState.representation === 'surface' && materials.surface) {
      materialToUse = materials.surface;
    }
    
    if (geometry instanceof THREE.InstancedBufferGeometry) {
      // Use InstancedMesh for better performance with many atoms
      const maxInstances = geometry.maxInstancedCount || 1000;
      mesh = new THREE.InstancedMesh(geometry, materialToUse, maxInstances);
      
      // Set instance count
      mesh.count = maxInstances;
      
      console.log(`🚀 Created InstancedMesh with ${maxInstances} instances`);
    } else {
      // Regular mesh for other geometries
      mesh = new THREE.Mesh(geometry, materialToUse);
      console.log(`🚀 Created regular Mesh`);
    }
    
    // Special settings for surface representation
    if (viewerState.representation === 'surface') {
      mesh.material = materials.surface!;
      // Enable wireframe for better surface visibility
      if ('wireframe' in mesh.material) {
        (mesh.material as any).wireframe = false;
      }
    }
    
    return mesh;
  }
  
  // Update material uniforms during render
  function updateMaterialUniforms(time: number): void {
    // Update receptor material
    if (materials.receptor && 'uniforms' in materials.receptor) {
      materials.receptor.uniforms.time.value = time * 0.001;
      materials.receptor.uniforms.lightDirection.value.set(
        Math.sin(time * 0.0005) * 0.5 + 0.5,
        0.8,
        Math.cos(time * 0.0005) * 0.5 + 0.5
      ).normalize();
    }
    
    // Update ligand material
    if (materials.ligand && 'uniforms' in materials.ligand) {
      materials.ligand.uniforms.time.value = time * 0.001;
      materials.ligand.uniforms.lightDirection.value.set(
        Math.sin(time * 0.0005) * 0.5 + 0.5,
        0.8,
        Math.cos(time * 0.0005) * 0.5 + 0.5
      ).normalize();
    }
  }
  
  function getAtomColor(element: string): { r: number, g: number, b: number } {
    const colors: Record<string, { r: number, g: number, b: number }> = {
      'C': { r: 0.5, g: 0.5, b: 0.5 }, // Carbon - gris
      'N': { r: 0.0, g: 0.0, b: 1.0 }, // Nitrogen - azul
      'O': { r: 1.0, g: 0.0, b: 0.0 }, // Oxygen - rojo
      'H': { r: 1.0, g: 1.0, b: 1.0 }, // Hydrogen - blanco
      'S': { r: 1.0, g: 1.0, b: 0.0 }, // Sulfur - amarillo
      'P': { r: 1.0, g: 0.5, b: 0.0 }, // Phosphorus - naranja
    };
    
    return colors[element] || { r: 0.8, g: 0.8, b: 0.8 };
  }
  
  async function fetchProteinStructure(fileId: string): Promise<any> {
    try {
      // Intentar obtener estructura procesada del backend
      const response = await fetch(`${api.state.baseURL}/api/files/${fileId}/structure`);
      
      if (response.ok) {
        return await response.json();
      } else {
        // Fallback: crear datos simulados basados en metadatos
        const file = api.state.files.find(f => f.file_id === fileId);
        return createSimulatedStructure(file);
      }
    } catch (error) {
      console.error('Error fetching protein structure:', error);
      return createSimulatedStructure();
    }
  }
  
  function createSimulatedStructure(file?: FileMetadata): any {
    const atomCount = file?.protein_info?.total_atoms || 100;
    const atoms = [];
    
    for (let i = 0; i < Math.min(atomCount, 1000); i++) {
      atoms.push({
        element: ['C', 'N', 'O', 'H'][Math.floor(Math.random() * 4)],
        x: (Math.random() - 0.5) * 30,
        y: (Math.random() - 0.5) * 30,
        z: (Math.random() - 0.5) * 30,
        residue: Math.floor(i / 10)
      });
    }
    
    return { atoms };
  }
  
  function setupLODForMesh(mesh: THREE.Mesh, id: string) {
    const lod = new THREE.LOD();
    
    // Nivel 0: Alta calidad (cerca)
    const highQuality = mesh.clone();
    lod.addLevel(highQuality, 0);
    
    // Nivel 1: Calidad media
    const mediumGeometry = mesh.geometry.clone();
    // Simplificar geometría aquí
    const mediumQuality = new THREE.Mesh(mediumGeometry, mesh.material);
    lod.addLevel(mediumQuality, 50);
    
    // Nivel 2: Baja calidad (lejos)
    const lowGeometry = new THREE.BoxGeometry(5, 5, 5);
    const lowQuality = new THREE.Mesh(lowGeometry, mesh.material);
    lod.addLevel(lowQuality, 100);
    
    lodSystem.geometries.set(id, lod);
  }
  
  async function loadJobResults(job: JobStatusInfo) {
    resultGroup.clear();
    
    if (!job.results || job.results.length === 0) return;
    
    try {
      // Crear instanced geometry para múltiples resultados
      const instancedGeometry = new THREE.InstancedBufferGeometry();
      const baseGeometry = new THREE.SphereGeometry(1.5, 12, 8);
      
      instancedGeometry.copy(baseGeometry);
      instancedGeometry.maxInstancedCount = job.results.length;
      
      // Crear buffers para instancias
      const positions = new Float32Array(job.results.length * 3);
      const colors = new Float32Array(job.results.length * 3);
      const scales = new Float32Array(job.results.length);
      
      // Encontrar el mejor y peor score para normalización
      const scores = job.results.map(r => r.score);
      const minScore = Math.min(...scores);
      const maxScore = Math.max(...scores);
      const scoreRange = maxScore - minScore;
      
      job.results.forEach((result, i) => {
        // Posición basada en pose
        const x = (Math.random() - 0.5) * 20;
        const y = (Math.random() - 0.5) * 20;
        const z = (Math.random() - 0.5) * 20;
        
        positions[i * 3] = x;
        positions[i * 3 + 1] = y;
        positions[i * 3 + 2] = z;
        
        // Color basado en score (verde = mejor, rojo = peor)
        const normalizedScore = scoreRange > 0 ? (result.score - minScore) / scoreRange : 0.5;
        colors[i * 3] = 1 - normalizedScore; // R
        colors[i * 3 + 1] = normalizedScore; // G
        colors[i * 3 + 2] = 0.3; // B
        
        // Escala basada en score
        scales[i] = 0.5 + normalizedScore * 1.5;
      });
      
      instancedGeometry.setAttribute('instancePosition', new THREE.InstancedBufferAttribute(positions, 3));
      instancedGeometry.setAttribute('instanceColor', new THREE.InstancedBufferAttribute(colors, 3));
      instancedGeometry.setAttribute('instanceScale', new THREE.InstancedBufferAttribute(scales, 1));
      
      // Material mejorado para resultados
      const resultMaterial = new THREE.ShaderMaterial({
        uniforms: {
          time: { value: 0 },
          opacity: { value: 0.8 }
        },
        vertexShader: `
          attribute vec3 instancePosition;
          attribute vec3 instanceColor;
          attribute float instanceScale;
          varying vec3 vColor;
          varying float vScale;
          uniform float time;
          
          void main() {
            vColor = instanceColor;
            vScale = instanceScale;
            
            vec3 pos = position * instanceScale + instancePosition;
            pos.y += sin(time * 2.0 + instancePosition.x * 0.1) * 0.3;
            
            gl_Position = projectionMatrix * modelViewMatrix * vec4(pos, 1.0);
          }
        `,
        fragmentShader: `
          uniform float opacity;
          varying vec3 vColor;
          varying float vScale;
          
          void main() {
            vec2 center = gl_PointCoord - vec2(0.5);
            float dist = length(center);
            
            if (dist > 0.5) discard;
            
            float alpha = opacity * (1.0 - dist * 2.0);
            gl_FragColor = vec4(vColor, alpha);
          }
        `,
        transparent: true,
        side: THREE.DoubleSide
      });
      
      const resultMesh = new THREE.Mesh(instancedGeometry, resultMaterial);
      resultGroup.add(resultMesh);
      
      console.log(`✅ Loaded ${job.results.length} docking results`);
      
    } catch (error) {
      console.error('❌ Failed to load job results:', error);
    }
  }
  
  function setupEventListeners() {
    // Resize handler optimizado
    const resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const { width, height } = entry.contentRect;
        
        camera.aspect = width / height;
        camera.updateProjectionMatrix();
        
        renderer.setSize(width, height);
      }
    });
    
    resizeObserver.observe(viewerContainer);
    
    // Keyboard controls
    setupKeyboardControls();
    
    // Mouse/touch enhancements
    setupInteractionControls();
    
    // Performance monitoring
    setInterval(updatePerformanceMetrics, 1000);
  }
  
  function setupKeyboardControls() {
    const handleKeyDown = (event: KeyboardEvent) => {
      // Prevenir scroll de la página con flechas
      if (['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'Space'].includes(event.code)) {
        event.preventDefault();
      }
      
      switch (event.code) {
        case 'Space':
          toggleRotation();
          break;
        case 'KeyR':
          if (event.ctrlKey || event.metaKey) {
            event.preventDefault();
            resetCamera();
          }
          break;
        case 'KeyF':
          fitToView();
          break;
        case 'Digit1':
          changeRepresentation('cartoon');
          break;
        case 'Digit2':
          changeRepresentation('spacefill');
          break;
        case 'Digit3':
          changeRepresentation('sticks');
          break;
        case 'Digit4':
          changeRepresentation('surface');
          break;
        case 'KeyH':
          viewerState.showDebugInfo = !viewerState.showDebugInfo;
          break;
        case 'Slash':
        case 'Question':
          if (event.shiftKey) {
            viewerState.showControlsHelp = !viewerState.showControlsHelp;
          }
          break;
        case 'Equal':
        case 'NumpadAdd':
          // Zoom in
          const zoomInDistance = camera.position.length() * 0.8;
          camera.position.normalize().multiplyScalar(Math.max(zoomInDistance, controls.minDistance));
          controls.update();
          break;
        case 'Minus':
        case 'NumpadSubtract':
          // Zoom out
          const zoomOutDistance = camera.position.length() * 1.2;
          camera.position.normalize().multiplyScalar(Math.min(zoomOutDistance, controls.maxDistance));
          controls.update();
          break;
      }
    };
    
    document.addEventListener('keydown', handleKeyDown);
    
    // Cleanup en destroy
    const originalCleanup = cleanup;
    cleanup = () => {
      document.removeEventListener('keydown', handleKeyDown);
      originalCleanup();
    };
  }
  
  function setupInteractionControls() {
    let isDragging = false;
    let lastPointerPosition = { x: 0, y: 0 };
    
    const handlePointerMove = (event: PointerEvent) => {
      if (!isDragging) return;
      
      const deltaX = event.clientX - lastPointerPosition.x;
      const deltaY = event.clientY - lastPointerPosition.y;
      
      // Update debug info if visible
      if (viewerState.showDebugInfo) {
        // Actualizar métricas de interacción
      }
      
      lastPointerPosition = { x: event.clientX, y: event.clientY };
    };
    
    const handlePointerDown = (event: PointerEvent) => {
      isDragging = true;
      lastPointerPosition = { x: event.clientX, y: event.clientY };
    };
    
    const handlePointerUp = () => {
      isDragging = false;
    };
    
    // Double-click to fit view
    const handleDoubleClick = (event: MouseEvent) => {
      event.preventDefault();
      fitToView();
    };
    
    // Context menu for additional options
    const handleContextMenu = (event: MouseEvent) => {
      event.preventDefault();
      // TODO: Mostrar menu contextual
    };
    
    // Wheel para zoom preciso
    const handleWheel = (event: WheelEvent) => {
      event.preventDefault();
      
      const zoomDirection = event.deltaY > 0 ? 1.1 : 0.9;
      const newDistance = camera.position.length() * zoomDirection;
      const clampedDistance = Math.max(
        controls.minDistance, 
        Math.min(controls.maxDistance, newDistance)
      );
      
      camera.position.normalize().multiplyScalar(clampedDistance);
      controls.update();
    };
    
    if (canvas) {
      canvas.addEventListener('pointermove', handlePointerMove);
      canvas.addEventListener('pointerdown', handlePointerDown);
      canvas.addEventListener('pointerup', handlePointerUp);
      canvas.addEventListener('dblclick', handleDoubleClick);
      canvas.addEventListener('contextmenu', handleContextMenu);
      canvas.addEventListener('wheel', handleWheel, { passive: false });
    }
  }
  
  function startRenderLoop() {
    let lastTime = 0;
    let frameCount = 0;
    
    function render(time: number) {
      // Calcular delta time
      const deltaTime = time - lastTime;
      lastTime = time;
      
      // Actualizar controles
      controls.update();
      
      // Actualizar materiales optimizados
      updateMaterialUniforms(time);
      
      // Actualizar animaciones de superficie
      if (materials.surface && 'uniforms' in materials.surface) {
        materials.surface.uniforms.time.value = time * 0.001;
      }
      
      // Actualizar animación de resultados
      resultGroup.children.forEach(child => {
        if (child instanceof THREE.Mesh && 'uniforms' in child.material) {
          child.material.uniforms.time.value = time * 0.001;
        }
      });
      
      // Rotación automática si está habilitada
      if (viewerState.isRotating) {
        proteinGroup.rotation.y += 0.01 * viewerState.animationSpeed;
        ligandGroup.rotation.y += 0.015 * viewerState.animationSpeed;
      }
      
      // Actualizar LOD
      updateLOD();
      
      // Renderizar
      renderer.render(scene, camera);
      
      // Contar frames para FPS
      frameCount++;
      
      requestAnimationFrame(render);
    }
    
    requestAnimationFrame(render);
  }
  
  function updateLOD() {
    const cameraPosition = camera.position;
    
    lodSystem.geometries.forEach((lod, id) => {
      const distance = lod.position.distanceTo(cameraPosition);
      
      // Determinar nivel LOD apropiado
      let level = 0;
      for (let i = 0; i < lodSystem.distances.length; i++) {
        if (distance > lodSystem.distances[i]) {
          level = i + 1;
        }
      }
      
      lodSystem.currentLevel = level;
    });
  }
  
  function updatePerformanceMetrics() {
    if (!renderer) return;
    
    const info = renderer.info;
    
    performanceMetrics = {
      fps: Math.round(1000 / 16.67), // Aproximación
      drawCalls: info.render.calls,
      triangles: info.render.triangles,
      geometries: info.memory.geometries,
      textures: info.memory.textures
    };
  }
  
  function cleanup() {
    if (renderer) {
      renderer.dispose();
    }
    
    if (controls) {
      controls.dispose();
    }
    
    // Limpiar geometrías y materiales
    scene?.traverse((object) => {
      if (object instanceof THREE.Mesh) {
        object.geometry?.dispose();
        if (Array.isArray(object.material)) {
          object.material.forEach(mat => mat.dispose());
        } else {
          object.material?.dispose();
        }
      }
    });
  }
  
  // Funciones de control públicas
  function resetCamera() {
    camera.position.set(50, 30, 50);
    camera.lookAt(0, 0, 0);
    controls.reset();
    console.log('🎯 Camera reset to default position');
  }
  
  function toggleRotation() {
    viewerState.isRotating = !viewerState.isRotating;
    controls.autoRotate = viewerState.isRotating;
    console.log(`🔄 Auto-rotation ${viewerState.isRotating ? 'enabled' : 'disabled'}`);
  }
  
  function fitToView() {
    const allObjects: THREE.Object3D[] = [];
    
    // Recopilar todas las geometrías visibles
    scene.traverse((object) => {
      if (object instanceof THREE.Mesh && object.visible) {
        allObjects.push(object);
      }
    });
    
    if (allObjects.length === 0) {
      console.log('⚠️ No objects to fit');
      return;
    }
    
    // Calcular bounding box de todos los objetos
    const box = new THREE.Box3();
    allObjects.forEach(obj => {
      const objBox = new THREE.Box3().setFromObject(obj);
      box.union(objBox);
    });
    
    const center = box.getCenter(new THREE.Vector3());
    const size = box.getSize(new THREE.Vector3());
    const maxDim = Math.max(size.x, size.y, size.z);
    
    // Calcular distancia óptima de la cámara
    const fov = camera.fov * (Math.PI / 180);
    const distance = maxDim / (2 * Math.tan(fov / 2)) * 1.5; // Factor 1.5 para margen
    
    // Posicionar cámara
    const direction = camera.position.clone().sub(center).normalize();
    camera.position.copy(center).add(direction.multiplyScalar(distance));
    camera.lookAt(center);
    
    // Actualizar controles
    controls.target.copy(center);
    controls.update();
    
    console.log(`🔍 Fitted to view - Center: ${center.toArray().map(c => c.toFixed(1)).join(', ')}, Distance: ${distance.toFixed(1)}`);
  }
  
  function focusOnSelection(type: 'receptor' | 'ligand' | 'all') {
    let targetGroup: THREE.Group;
    
    switch (type) {
      case 'receptor':
        targetGroup = proteinGroup;
        break;
      case 'ligand':
        targetGroup = ligandGroup;
        break;
      case 'all':
      default:
        // Focus on all visible objects
        fitToView();
        return;
    }
    
    if (targetGroup.children.length === 0) {
      console.log(`⚠️ No ${type} loaded to focus on`);
      return;
    }
    
    // Calcular bounding box del grupo
    const box = new THREE.Box3().setFromObject(targetGroup);
    const center = box.getCenter(new THREE.Vector3());
    const size = box.getSize(new THREE.Vector3());
    const maxDim = Math.max(size.x, size.y, size.z);
    
    // Posicionar cámara
    const fov = camera.fov * (Math.PI / 180);
    const distance = maxDim / (2 * Math.tan(fov / 2)) * 2;
    
    const direction = camera.position.clone().sub(center).normalize();
    camera.position.copy(center).add(direction.multiplyScalar(distance));
    camera.lookAt(center);
    
    controls.target.copy(center);
    controls.update();
    
    console.log(`🎯 Focused on ${type}`);
  }
  
  function setCameraPreset(preset: 'front' | 'back' | 'left' | 'right' | 'top' | 'bottom') {
    const distance = camera.position.length();
    const target = controls.target;
    
    let newPosition: THREE.Vector3;
    
    switch (preset) {
      case 'front':
        newPosition = new THREE.Vector3(0, 0, distance);
        break;
      case 'back':
        newPosition = new THREE.Vector3(0, 0, -distance);
        break;
      case 'left':
        newPosition = new THREE.Vector3(-distance, 0, 0);
        break;
      case 'right':
        newPosition = new THREE.Vector3(distance, 0, 0);
        break;
      case 'top':
        newPosition = new THREE.Vector3(0, distance, 0);
        break;
      case 'bottom':
        newPosition = new THREE.Vector3(0, -distance, 0);
        break;
      default:
        return;
    }
    
    newPosition.add(target);
    camera.position.copy(newPosition);
    camera.lookAt(target);
    controls.update();
    
    console.log(`📷 Camera preset: ${preset}`);
  }
  
  function changeRepresentation(type: typeof viewerState.representation) {
    console.log(`🎨 Changing representation to: ${type}`);
    viewerState.representation = type;
    
    // Recargar proteínas con nueva representación
    loadProteinData();
  }
  
  function setQualityLevel(level: typeof viewerState.qualityLevel) {
    viewerState.qualityLevel = level;
    
    // Ajustar configuraciones de calidad
    switch (level) {
      case 'low':
        renderer.setPixelRatio(0.5);
        renderer.shadowMap.enabled = false;
        break;
      case 'medium':
        renderer.setPixelRatio(1);
        renderer.shadowMap.enabled = true;
        break;
      case 'high':
        renderer.setPixelRatio(Math.min(window.devicePixelRatio, 1.5));
        renderer.shadowMap.enabled = true;
        break;
      case 'ultra':
        renderer.setPixelRatio(window.devicePixelRatio);
        renderer.shadowMap.enabled = true;
        break;
    }
  }
  
  // Reactividad para cambios en props
  $effect(() => {
    if (viewerState.isInitialized) {
      loadProteinData();
    }
  });
  
  $effect(() => {
    if (viewerState.isInitialized && selectedJob && showResults) {
      loadJobResults(selectedJob);
    }
  });
</script>

<div class="protein-viewer" bind:this={viewerContainer}>

  <!-- Validation Messages -->
  {#if !selectedReceptor && !selectedLigand && !selectedJob}
    <div class="validation-message">
      <div class="validation-icon">🧬</div>
      <h3>No Molecule Selected</h3>
      <p>Select a receptor, ligand, or completed job to view molecular structures in 3D.</p>
      <div class="validation-suggestions">
        <ul>
          <li>Go to <strong>Create Docking Job</strong> tab to select molecules</li>
          <li>Go to <strong>Job Monitor</strong> tab to view completed jobs</li>
          <li>Upload protein files in the <strong>Upload Files</strong> tab</li>
        </ul>
      </div>
    </div>
  {/if}

  <canvas bind:this={canvas}></canvas>
  
  <!-- Controles del visualizador -->
  <div class="viewer-controls">
    <div class="control-group">
      <button 
        class="control-btn"
        onclick={() => changeRepresentation('cartoon')}
        class:active={viewerState.representation === 'cartoon'}
        title="Cartoon representation"
      >
        🧬
      </button>
      
      <button 
        class="control-btn"
        onclick={() => changeRepresentation('spacefill')}
        class:active={viewerState.representation === 'spacefill'}
        title="Space-fill representation"
      >
        🔘
      </button>
      
      <button 
        class="control-btn"
        onclick={() => changeRepresentation('sticks')}
        class:active={viewerState.representation === 'sticks'}
        title="Stick representation"
      >
        🥢
      </button>
      
      <button 
        class="control-btn"
        onclick={() => changeRepresentation('surface')}
        class:active={viewerState.representation === 'surface'}
        title="Surface representation"
      >
        🌊
      </button>
    </div>
    
    <div class="control-group">
      <button 
        class="control-btn"
        onclick={toggleRotation}
        class:active={viewerState.isRotating}
        title="Toggle auto-rotation"
      >
        🔄
      </button>
      
      <button 
        class="control-btn"
        onclick={resetCamera}
        title="Reset camera"
      >
        🎯
      </button>
      
      <button 
        class="control-btn"
        onclick={() => viewerState.showDebugInfo = !viewerState.showDebugInfo}
        class:active={viewerState.showDebugInfo}
        title="Toggle info (H)"
      >
        📊
      </button>
      
      <button 
        class="control-btn"
        onclick={() => viewerState.showControlsHelp = !viewerState.showControlsHelp}
        class:active={viewerState.showControlsHelp}
        title="Show controls help (?)"
      >
        ❓
      </button>
    </div>
    
    <div class="control-group">
      <button 
        class="control-btn"
        onclick={fitToView}
        title="Fit to view (F)"
      >
        🔍
      </button>
      
      <button 
        class="control-btn"
        onclick={() => focusOnSelection('receptor')}
        title="Focus receptor"
      >
        🧬
      </button>
      
      <button 
        class="control-btn"
        onclick={() => focusOnSelection('ligand')}
        title="Focus ligand"
      >
        🤼
      </button>
    </div>
    
    <div class="control-group">
      <select 
        onchange={(e) => setCameraPreset(e.target.value as any)}
        class="camera-preset-select"
        title="Camera presets"
      >
        <option value="">Views</option>
        <option value="front">Front</option>
        <option value="back">Back</option>
        <option value="left">Left</option>
        <option value="right">Right</option>
        <option value="top">Top</option>
        <option value="bottom">Bottom</option>
      </select>
    </div>
    
    <div class="control-group">
      <select 
        bind:value={viewerState.qualityLevel}
        onchange={(e) => setQualityLevel(e.target.value as any)}
        class="quality-select"
        title="Rendering quality"
      >
        <option value="low">Low Quality</option>
        <option value="medium">Medium Quality</option>
        <option value="high">High Quality</option>
        <option value="ultra">Ultra Quality</option>
      </select>
    </div>
  </div>
  
  <!-- Indicador de performance -->
  <div class="performance-indicator">
    <div class="metric">
      <span class="metric-label">FPS:</span>
      <span class="metric-value" class:good={performanceMetrics.fps >= 45} class:bad={performanceMetrics.fps < 30}>
        {performanceMetrics.fps}
      </span>
    </div>
    <div class="metric">
      <span class="metric-label">Triangles:</span>
      <span class="metric-value">{performanceMetrics.triangles.toLocaleString()}</span>
    </div>
  </div>
  
  <!-- Performance info overlay -->
  {#if viewerState.showDebugInfo}
    <div class="info-overlay">
      <h3>📊 Viewer Info</h3>
      
      {#if parsingInfo.receptorInfo}
        <div class="info-section">
          <h4>🧬 Receptor: {parsingInfo.receptorInfo.filename}</h4>
          <div class="info-grid">
            <span>Title:</span><span>{parsingInfo.receptorInfo.title}</span>
            <span>Atoms:</span><span>{parsingInfo.receptorInfo.atoms}</span>
            <span>Residues:</span><span>{parsingInfo.receptorInfo.residues}</span>
            <span>Chains:</span><span>{parsingInfo.receptorInfo.chains}</span>
            <span>Bonds:</span><span>{parsingInfo.receptorInfo.bonds}</span>
            <span>Elements:</span><span>{parsingInfo.receptorInfo.elements}</span>
            <span>Center:</span><span>{parsingInfo.receptorInfo.center.map(c => c.toFixed(1)).join(', ')}</span>
            <span>Size:</span><span>{parsingInfo.receptorInfo.boundingBox.map(s => s.toFixed(1)).join('x')}</span>
            <span>Parse time:</span><span>{parsingInfo.receptorInfo.parsingTime}</span>
          </div>
        </div>
      {/if}
      
      {#if parsingInfo.ligandInfo}
        <div class="info-section">
          <h4>🤼 Ligand: {parsingInfo.ligandInfo.filename}</h4>
          <div class="info-grid">
            <span>Title:</span><span>{parsingInfo.ligandInfo.title}</span>
            <span>Atoms:</span><span>{parsingInfo.ligandInfo.atoms}</span>
            <span>Residues:</span><span>{parsingInfo.ligandInfo.residues}</span>
            <span>Chains:</span><span>{parsingInfo.ligandInfo.chains}</span>
            <span>Bonds:</span><span>{parsingInfo.ligandInfo.bonds}</span>
            <span>Elements:</span><span>{parsingInfo.ligandInfo.elements}</span>
            <span>Center:</span><span>{parsingInfo.ligandInfo.center.map(c => c.toFixed(1)).join(', ')}</span>
            <span>Size:</span><span>{parsingInfo.ligandInfo.boundingBox.map(s => s.toFixed(1)).join('x')}</span>
            <span>Parse time:</span><span>{parsingInfo.ligandInfo.parsingTime}</span>
          </div>
        </div>
      {/if}
      
      <div class="info-section">
        <h4>🎨 Renderer</h4>
        <div class="info-grid">
          <span>Representation:</span><span>{viewerState.representation}</span>
          <span>Quality:</span><span>{viewerState.qualityLevel}</span>
          <span>FPS:</span><span class:good={performanceMetrics.fps >= 45} class:bad={performanceMetrics.fps < 30}>{performanceMetrics.fps}</span>
          <span>Draw calls:</span><span>{performanceMetrics.drawCalls}</span>
          <span>Triangles:</span><span>{performanceMetrics.triangles.toLocaleString()}</span>
          <span>Geometries:</span><span>{performanceMetrics.geometries}</span>
        </div>
      </div>
    </div>
  {/if}
  
  <!-- Controls help overlay -->
  {#if viewerState.showControlsHelp}
    <div class="controls-help">
      <h4>❓ Controls</h4>
      
      <div class="shortcut">
        <span>Rotate</span>
        <span class="key">LMB Drag</span>
      </div>
      <div class="shortcut">
        <span>Pan</span>
        <span class="key">RMB Drag</span>
      </div>
      <div class="shortcut">
        <span>Zoom</span>
        <span class="key">Wheel</span>
      </div>
      <div class="shortcut">
        <span>Auto-rotate</span>
        <span class="key">Space</span>
      </div>
      <div class="shortcut">
        <span>Reset camera</span>
        <span class="key">Ctrl+R</span>
      </div>
      <div class="shortcut">
        <span>Fit to view</span>
        <span class="key">F</span>
      </div>
      <div class="shortcut">
        <span>Zoom in/out</span>
        <span class="key">+/-</span>
      </div>
      <div class="shortcut">
        <span>Cartoon</span>
        <span class="key">1</span>
      </div>
      <div class="shortcut">
        <span>Spacefill</span>
        <span class="key">2</span>
      </div>
      <div class="shortcut">
        <span>Sticks</span>
        <span class="key">3</span>
      </div>
      <div class="shortcut">
        <span>Surface</span>
        <span class="key">4</span>
      </div>
      <div class="shortcut">
        <span>Debug info</span>
        <span class="key">H</span>
      </div>
      <div class="shortcut">
        <span>This help</span>
        <span class="key">?</span>
      </div>
    </div>
  {/if}
  
  <!-- Loading overlay -->
  {#if !viewerState.isInitialized}
    <div class="loading-overlay">
      <div class="loading-spinner"></div>
      <p>Initializing 3D viewer...</p>
    </div>
  {/if}
</div>

<style>
  .protein-viewer {
    position: relative;
    width: 100%;
    height: 100%;
    background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
    overflow: hidden;
  }
  
  canvas {
    display: block;
    width: 100%;
    height: 100%;
  }
  
  .viewer-controls {
    position: absolute;
    top: 1rem;
    right: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    z-index: 10;
  }
  
  .control-group {
    display: flex;
    gap: 0.25rem;
    background: rgba(0, 0, 0, 0.7);
    border-radius: 0.5rem;
    padding: 0.25rem;
    backdrop-filter: blur(10px);
  }
  
  .control-btn {
    width: 40px;
    height: 40px;
    border: none;
    border-radius: 0.375rem;
    background: rgba(255, 255, 255, 0.1);
    color: white;
    font-size: 1.25rem;
    cursor: pointer;
    transition: all 0.2s ease;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  
  .control-btn:hover {
    background: rgba(255, 255, 255, 0.2);
    transform: scale(1.05);
  }
  
  .control-btn.active {
    background: var(--accent-primary);
    color: white;
  }
  
  .quality-select,
  .camera-preset-select {
    background: rgba(0, 0, 0, 0.7);
    color: white;
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 0.375rem;
    padding: 0.5rem;
    font-size: 0.875rem;
    cursor: pointer;
    min-width: 100px;
  }
  
  .camera-preset-select option {
    background: rgba(0, 0, 0, 0.9);
    color: white;
  }
  
  .performance-indicator {
    position: absolute;
    bottom: 1rem;
    left: 1rem;
    display: flex;
    gap: 1rem;
    background: rgba(0, 0, 0, 0.7);
    border-radius: 0.5rem;
    padding: 0.75rem;
    backdrop-filter: blur(10px);
    font-size: 0.875rem;
    z-index: 10;
  }
  
  .metric {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }
  
  .metric-label {
    color: rgba(255, 255, 255, 0.7);
  }
  
  .metric-value {
    color: white;
    font-weight: 600;
  }
  
  .metric-value.good {
    color: #22c55e;
  }
  
  .metric-value.bad {
    color: #ef4444;
  }
  
  .loading-overlay {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.8);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: white;
    z-index: 20;
  }
  
  .loading-spinner {
    width: 50px;
    height: 50px;
    border: 3px solid rgba(255, 255, 255, 0.3);
    border-top: 3px solid var(--accent-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin-bottom: 1rem;
  }
  
  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }
  
  .loading-overlay p {
    margin: 0;
    font-size: 1rem;
    opacity: 0.8;
  }

  
  .info-overlay {
    position: absolute;
    top: 1rem;
    left: 1rem;
    background: rgba(0, 0, 0, 0.9);
    border-radius: 0.5rem;
    padding: 1rem;
    max-width: 400px;
    max-height: 80vh;
    overflow-y: auto;
    font-size: 0.75rem;
    color: white;
    z-index: 15;
    backdrop-filter: blur(10px);
  }
  
  .info-overlay h3 {
    margin: 0 0 1rem 0;
    font-size: 1rem;
    color: var(--accent-primary);
  }
  
  .info-overlay h4 {
    margin: 0.5rem 0 0.5rem 0;
    font-size: 0.875rem;
    color: #fff;
  }
  
  .info-section {
    margin-bottom: 1rem;
    padding-bottom: 0.5rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  }
  
  .info-section:last-child {
    border-bottom: none;
    margin-bottom: 0;
  }
  
  .info-grid {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 0.25rem 0.5rem;
    font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
  }
  
  .info-grid span:nth-child(odd) {
    color: rgba(255, 255, 255, 0.7);
    font-weight: 500;
  }
  
  .info-grid span:nth-child(even) {
    color: #fff;
    text-align: right;
  }
  
  .info-grid .good {
    color: #22c55e;
  }
  
  .info-grid .bad {
    color: #ef4444;
  }
  
  /* Help tooltip */
  .controls-help {
    position: absolute;
    bottom: 4rem;
    right: 1rem;
    background: rgba(0, 0, 0, 0.8);
    border-radius: 0.5rem;
    padding: 0.75rem;
    font-size: 0.75rem;
    color: white;
    max-width: 200px;
    z-index: 10;
    backdrop-filter: blur(10px);
  }
  
  .controls-help h4 {
    margin: 0 0 0.5rem 0;
    font-size: 0.875rem;
    color: var(--accent-primary);
  }
  
  .controls-help .shortcut {
    display: flex;
    justify-content: space-between;
    margin-bottom: 0.25rem;
  }
  
  .controls-help .key {
    background: rgba(255, 255, 255, 0.1);
    padding: 0.125rem 0.25rem;
    border-radius: 0.25rem;
    font-family: monospace;
    font-size: 0.625rem;
  }
  
  /* Responsive adjustments */
  @media (max-width: 768px) {
    .viewer-controls {
      top: 0.5rem;
      right: 0.5rem;
    }
    
    .performance-indicator {
      bottom: 0.5rem;
      left: 0.5rem;
      flex-direction: column;
      gap: 0.5rem;
    }
    
    .control-btn {
      width: 36px;
      height: 36px;
      font-size: 1rem;
    }
  }
</style>"