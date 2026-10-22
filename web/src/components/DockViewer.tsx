import { useEffect, useMemo, useState } from 'react'
import { Canvas } from '@react-three/fiber'
import { OrbitControls } from '@react-three/drei'
import * as THREE from 'three'
import type { RigidT } from '../api'
import type { DockViewerData, MeshData } from '../dockData'

interface Props {
  data: DockViewerData
  pose: RigidT | null
  native: RigidT | null
  label: string
}

function matrix(t: RigidT): THREE.Matrix4 {
  const r = t.rot
  return new THREE.Matrix4().set(
    r[0][0], r[0][1], r[0][2], t.trans[0],
    r[1][0], r[1][1], r[1][2], t.trans[1],
    r[2][0], r[2][1], r[2][2], t.trans[2],
    0, 0, 0, 1,
  )
}

function useGeometry(m: MeshData) {
  const g = useMemo(() => {
    const geo = new THREE.BufferGeometry()
    geo.setAttribute('position', new THREE.BufferAttribute(m.vertices, 3))
    geo.setIndex(new THREE.BufferAttribute(m.faces, 1))
    geo.computeVertexNormals()
    geo.computeBoundingSphere()
    return geo
  }, [m])
  useEffect(() => () => g.dispose(), [g])
  return g
}

export function DockViewer({ data, pose, native, label }: Props) {
  const [opacity, setOpacity] = useState(0.55)
  const [showNative, setShowNative] = useState(true)
  const receptor = useGeometry(data.receptor)
  const ligand = useGeometry(data.ligand)
  const poseMatrix = useMemo(() => (pose ? matrix(pose) : null), [pose])
  // Frame the receptor and the first pose shown (the camera then stays where the user puts it).
  const [sphere] = useState(() => {
    const s = receptor.boundingSphere!.clone()
    if (poseMatrix) s.union(ligand.boundingSphere!.clone().applyMatrix4(poseMatrix))
    return s
  })
  const nativeMatrix = useMemo(() => (native ? matrix(native) : null), [native])

  return (
    <div className="viewer">
      <div className="viewer-canvas">
        <Canvas
          dpr={[1, 2]}
          gl={{ antialias: true, alpha: true }}
          camera={{
            position: [sphere.center.x, sphere.center.y, sphere.center.z + sphere.radius * 2.6],
            near: 0.5,
            far: sphere.radius * 30,
            fov: 40,
          }}
        >
          <ambientLight intensity={0.55} />
          <directionalLight position={[1, 1.5, 2]} intensity={1.6} />
          <directionalLight position={[-2, -1, -1]} intensity={0.5} color="#7aa7d6" />
          <mesh geometry={receptor} renderOrder={2}>
            <meshStandardMaterial
              color="#9aa3ad"
              roughness={0.55}
              transparent={opacity < 1}
              opacity={opacity}
              depthWrite={opacity >= 1}
              side={THREE.DoubleSide}
            />
          </mesh>
          {poseMatrix && (
            <mesh geometry={ligand} matrixAutoUpdate={false} matrix={poseMatrix} renderOrder={1}>
              <meshStandardMaterial color="#f2b134" roughness={0.45} emissive="#5a3a00" emissiveIntensity={0.25} />
            </mesh>
          )}
          {showNative && nativeMatrix && (
            <mesh geometry={ligand} matrixAutoUpdate={false} matrix={nativeMatrix} renderOrder={3}>
              <meshBasicMaterial color="#8ccf7e" wireframe transparent opacity={0.35} depthWrite={false} />
            </mesh>
          )}
          <OrbitControls makeDefault enableDamping dampingFactor={0.12} target={sphere.center} />
        </Canvas>
        <div className="viewer-hud">
          <span className="mono">{label}</span>
        </div>
        <p className="viewer-hint">
          Gris: receptor · ámbar: ligando en la pose elegida{native ? ' · verde: posición real' : ''}
        </p>
      </div>
      <aside className="viewer-panel">
        <section>
          <h4>Receptor</h4>
          <input type="range" min={0} max={1} step={0.01} value={opacity} onChange={(e) => setOpacity(Number(e.target.value))} />
        </section>
        {native && (
          <section>
            <h4>Referencia</h4>
            <label className="check">
              <input type="checkbox" checked={showNative} onChange={(e) => setShowNative(e.target.checked)} />
              Mostrar la posición real del ligando
            </label>
          </section>
        )}
      </aside>
    </div>
  )
}
