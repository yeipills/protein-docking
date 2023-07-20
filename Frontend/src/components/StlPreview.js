import React, { useEffect, useRef } from 'react';
import * as THREE from 'three';
import { STLLoader } from 'three/examples/jsm/loaders/STLLoader.js';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
//import { MeshLambertMaterial } from 'three';


const StlPreview = ({ file }) => {
    const mountRef = useRef(null);

    useEffect(() => {
        // Si no hay archivo, no intentes renderizarlo
        if (!file) {
            return;
        }

        const scene = new THREE.Scene();
        const camera = new THREE.PerspectiveCamera(75, window.innerWidth / window.innerHeight, 0.1, 1000);
        const renderer = new THREE.WebGLRenderer();


        renderer.setSize(700, 600);
        mountRef.current.appendChild(renderer.domElement);

        const controls = new OrbitControls(camera, renderer.domElement);
        controls.enableZoom = true;

        const pointLight = new THREE.PointLight(0xffffff, 1, 0);
        pointLight.position.set(50, 50, 50);
        const ambientLight = new THREE.AmbientLight(0x404040);
        scene.add(pointLight, ambientLight);

        const loader = new STLLoader();
        loader.load(URL.createObjectURL(file), (geometry) => {
            const material = new THREE.MeshLambertMaterial({ color: 0x00ff00 });
            const mesh = new THREE.Mesh(geometry, material);

            scene.add(mesh);

            const box = new THREE.Box3().setFromObject(mesh);
            const size = box.getSize(new THREE.Vector3()).length();
            const center = box.getCenter(new THREE.Vector3());

            controls.reset();

            mesh.position.x += mesh.position.x - center.x;
            mesh.position.y += mesh.position.y - center.y;
            mesh.position.z += mesh.position.z - center.z;
            controls.update();

            camera.position.copy(center);
            camera.position.x += size / 2.0;
            camera.position.y += size / 2.0;
            camera.position.z += size / 2.0;
            camera.near = size / 100;
            camera.far = size * 100;
            camera.updateProjectionMatrix();

            camera.lookAt(center);

        });

        const animate = function () {
            requestAnimationFrame(animate);
            controls.update();
            renderer.render(scene, camera);
        };

        animate();

        // Función de limpieza
        return () => {
            if (mountRef.current && renderer) {
                mountRef.current.removeChild(renderer.domElement);
            }
        };
    }, [file]);

    return <div id="stl-preview" ref={mountRef}></div>;
}

export default StlPreview;
