import React, { useEffect, useRef } from 'react';
import * as THREE from 'three';
import { STLLoader } from 'three/examples/jsm/loaders/STLLoader.js';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';

const StlPreview = ({ file }) => {
    const mountRef = useRef(null);

    useEffect(() => {
        // Crear escena, cámara y renderer
        const scene = new THREE.Scene();
        const camera = new THREE.PerspectiveCamera(75, window.innerWidth / window.innerHeight, 0.1, 1000);
        const renderer = new THREE.WebGLRenderer();

        // Configurar tamaño del renderer y añadirlo al documento
        renderer.setSize(800, 600);  // Ajusta estos valores según tus necesidades
        mountRef.current.appendChild(renderer.domElement);

        // Añadir controles de órbita
        const controls = new OrbitControls(camera, renderer.domElement);
        controls.enableZoom = true;  // Habilita el zoom

        // Configurar luces y añadirlas a la escena
        const pointLight = new THREE.PointLight(0xffffff, 1, 0);
        pointLight.position.set(50, 50, 50);
        const ambientLight = new THREE.AmbientLight(0x404040);
        scene.add(pointLight, ambientLight);

        // Cargar archivo STL
        const loader = new STLLoader();
        loader.load(URL.createObjectURL(file), (geometry) => {
            const material = new THREE.MeshBasicMaterial({ color: 0x00ff00 });
            const mesh = new THREE.Mesh(geometry, material);
            scene.add(mesh);
        });

        // Configurar posición de la cámara
        camera.position.z = 5;

        // Crear loop de animación
        const animate = function () {
            requestAnimationFrame(animate);
            controls.update();
            renderer.render(scene, camera);
        };

        animate();

        // Función de limpieza
        return () => {
            mountRef.current.removeChild(renderer.domElement);
        };
    }, [file]);

    return <div id="stl-preview" ref={mountRef}></div>;
}

export default StlPreview;
