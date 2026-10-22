import { useEffect, useMemo, useRef, useState } from 'react'
import { Canvas, useFrame, useThree, type ThreeEvent } from '@react-three/fiber'
import { OrbitControls } from '@react-three/drei'
import * as THREE from 'three'
import type { LayerName, Mode } from '../api'
import { LAYERS, displayLayer, distanceColor } from '../layers'
import { samplePosition, type ViewerData } from '../viewerData'

type ColorBy = 'layer' | 'distance'

interface Props {
  data: ViewerData
  mode: Mode
  occupancy: Partial<Record<LayerName, number>>
}

const DEFAULT_VISIBLE: LayerName[] = ['ses', 'in1', 'in2', 'in3', 'in4', 'out1', 'out2', 'out3', 'out4']

export function Viewer({ data, mode, occupancy }: Props) {
  const nShapes = data.centers.length / 3
  const [selected, setSelected] = useState(0)
  const [visible, setVisible] = useState(() => new Set<LayerName>(DEFAULT_VISIBLE))
  const [colorBy, setColorBy] = useState<ColorBy>('layer')
  const [showAll, setShowAll] = useState(false)
  const [showRays, setShowRays] = useState(true)
  const [opacity, setOpacity] = useState(0.28)
  const [focusTick, setFocusTick] = useState(0)

  const bounds = useMemo(() => {
    const box = new THREE.Box3().setFromArray(data.vertices)
    const sphere = box.getBoundingSphere(new THREE.Sphere())
    return { center: sphere.center, radius: sphere.radius }
  }, [data])

  const toggle = (name: LayerName) =>
    setVisible((prev) => {
      const next = new Set(prev)
      if (next.has(name)) next.delete(name)
      else next.add(name)
      return next
    })

  const selectedCenter = useMemo(
    () => new THREE.Vector3(data.centers[selected * 3], data.centers[selected * 3 + 1], data.centers[selected * 3 + 2]),
    [data, selected],
  )

  return (
    <div className="viewer">
      <div className="viewer-canvas">
        <Canvas
          dpr={[1, 2]}
          gl={{ antialias: true, alpha: true }}
          camera={{
            position: [bounds.center.x, bounds.center.y, bounds.center.z + bounds.radius * 2.4],
            near: 0.1,
            far: bounds.radius * 20,
            fov: 40,
          }}
        >
          <ambientLight intensity={0.55} />
          <directionalLight position={[1, 1.5, 2]} intensity={1.6} />
          <directionalLight position={[-2, -1, -1]} intensity={0.5} color="#7aa7d6" />
          <Surface data={data} opacity={opacity} />
          <Centers data={data} selected={selected} onSelect={setSelected} />
          {showAll ? (
            <SamplePoints data={data} rays={[0, nShapes * data.raysPerCenter]} visible={visible} colorBy={colorBy} size={0.18} />
          ) : (
            <>
              {showRays && <Rays data={data} shape={selected} />}
              <SamplePoints
                data={data}
                rays={[selected * data.raysPerCenter, (selected + 1) * data.raysPerCenter]}
                visible={visible}
                colorBy={colorBy}
                size={Math.max(0.06, (2.2 * 3) / data.samplesPerRay / 3)}
              />
            </>
          )}
          <Controls home={bounds.center} focus={selectedCenter} focusTick={focusTick} />
        </Canvas>
        <div className="viewer-hud">
          <span className="mono">
            forma <b>{selected + 1}</b> / {nShapes}
          </span>
          <span className="mono dim">
            ({selectedCenter.x.toFixed(2)}, {selectedCenter.y.toFixed(2)}, {selectedCenter.z.toFixed(2)}) Å
          </span>
        </div>
        <p className="viewer-hint">Arrastra para rotar · rueda para acercar · clic en un punto ámbar para elegir forma</p>
      </div>

      <aside className="viewer-panel">
        <section>
          <h4>Forma de contexto</h4>
          <div className="stepper">
            <button onClick={() => setSelected((s) => (s - 1 + nShapes) % nShapes)} aria-label="Anterior">
              ‹
            </button>
            <input
              type="range"
              min={0}
              max={nShapes - 1}
              value={selected}
              onChange={(e) => setSelected(Number(e.target.value))}
              disabled={showAll}
            />
            <button onClick={() => setSelected((s) => (s + 1) % nShapes)} aria-label="Siguiente">
              ›
            </button>
          </div>
          <div className="row">
            <button className="ghost small" onClick={() => setFocusTick((t) => t + 1)}>
              Centrar en la forma
            </button>
            <label className="check">
              <input type="checkbox" checked={showAll} onChange={(e) => setShowAll(e.target.checked)} />
              Todas las formas
            </label>
          </div>
          <label className="check">
            <input type="checkbox" checked={showRays} onChange={(e) => setShowRays(e.target.checked)} disabled={showAll} />
            Mostrar rayos
          </label>
        </section>

        <section>
          <h4>Color</h4>
          <div className="segmented small">
            <button className={colorBy === 'layer' ? 'on' : ''} onClick={() => setColorBy('layer')}>
              Capas
            </button>
            <button
              className={colorBy === 'distance' ? 'on' : ''}
              onClick={() => setColorBy('distance')}
              disabled={!data.distances}
              title={data.distances ? '' : 'Solo disponible en el modo corregido'}
            >
              Distancia
            </button>
          </div>
          {colorBy === 'distance' && (
            <div className="ramp">
              <span>−1.5 Å</span>
              <div className="ramp-bar" />
              <span>+1.5 Å</span>
            </div>
          )}
        </section>

        <section>
          <h4>Capas {mode === 'legacy' && <span className="tag">fiel</span>}</h4>
          <ul className="legend">
            {LAYERS.map((l) => (
              <li key={l.name}>
                <button className={visible.has(l.name) ? 'legend-item on' : 'legend-item'} onClick={() => toggle(l.name)}>
                  <i style={{ background: l.color }} />
                  <span>{l.label}</span>
                  <em className="mono">{occupancy[l.name] !== undefined ? `${(occupancy[l.name]! * 100).toFixed(1)}%` : ''}</em>
                </button>
              </li>
            ))}
          </ul>
        </section>

        <section>
          <h4>Superficie</h4>
          <input type="range" min={0} max={1} step={0.01} value={opacity} onChange={(e) => setOpacity(Number(e.target.value))} />
        </section>
      </aside>
    </div>
  )
}

function Surface({ data, opacity }: { data: ViewerData; opacity: number }) {
  const geometry = useMemo(() => {
    const g = new THREE.BufferGeometry()
    g.setAttribute('position', new THREE.BufferAttribute(data.vertices, 3))
    g.setIndex(new THREE.BufferAttribute(data.faces, 1))
    g.computeVertexNormals()
    return g
  }, [data])
  useEffect(() => () => geometry.dispose(), [geometry])
  if (opacity <= 0.001) return null
  return (
    <mesh geometry={geometry} renderOrder={2}>
      <meshStandardMaterial
        color="#9aa3ad"
        roughness={0.55}
        metalness={0.05}
        transparent={opacity < 1}
        opacity={opacity}
        depthWrite={opacity >= 1}
        side={THREE.DoubleSide}
      />
    </mesh>
  )
}

function Centers({ data, selected, onSelect }: { data: ViewerData; selected: number; onSelect: (i: number) => void }) {
  const n = data.centers.length / 3
  const ref = useRef<THREE.InstancedMesh>(null)
  useEffect(() => {
    const mesh = ref.current
    if (!mesh) return
    const m = new THREE.Matrix4()
    const amber = new THREE.Color('#f2b134')
    const hot = new THREE.Color('#fff3d1')
    for (let i = 0; i < n; i++) {
      // The selected point is drawn small so it does not hide its own samples.
      const s = i === selected ? 0.45 : 1
      m.makeScale(s, s, s).setPosition(data.centers[i * 3], data.centers[i * 3 + 1], data.centers[i * 3 + 2])
      mesh.setMatrixAt(i, m)
      mesh.setColorAt(i, i === selected ? hot : amber)
    }
    mesh.instanceMatrix.needsUpdate = true
    if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true
    mesh.computeBoundingSphere()
  }, [data, n, selected])

  const onClick = (e: ThreeEvent<MouseEvent>) => {
    e.stopPropagation()
    if (e.instanceId !== undefined) onSelect(e.instanceId)
  }
  return (
    <instancedMesh
      ref={ref}
      args={[undefined, undefined, n]}
      onClick={onClick}
      onPointerOver={() => (document.body.style.cursor = 'pointer')}
      onPointerOut={() => (document.body.style.cursor = '')}
      renderOrder={1}
    >
      <sphereGeometry args={[0.55, 16, 12]} />
      <meshStandardMaterial emissive="#7a4a00" emissiveIntensity={0.4} roughness={0.4} />
    </instancedMesh>
  )
}

function Rays({ data, shape }: { data: ViewerData; shape: number }) {
  const geometry = useMemo(() => {
    const n = data.raysPerCenter
    const pos = new Float32Array(n * 6)
    for (let r = 0; r < n; r++) {
      const g = shape * n + r
      for (let k = 0; k < 3; k++) {
        pos[r * 6 + k] = data.centers[shape * 3 + k]
        pos[r * 6 + 3 + k] = data.rayEnds[g * 3 + k]
      }
    }
    const geo = new THREE.BufferGeometry()
    geo.setAttribute('position', new THREE.BufferAttribute(pos, 3))
    return geo
  }, [data, shape])
  useEffect(() => () => geometry.dispose(), [geometry])
  return (
    <lineSegments geometry={geometry}>
      <lineBasicMaterial color="#e8e2d4" transparent opacity={0.16} depthWrite={false} />
    </lineSegments>
  )
}

let discTexture: THREE.Texture | null = null
function disc(): THREE.Texture {
  if (discTexture) return discTexture
  const c = document.createElement('canvas')
  c.width = c.height = 64
  const ctx = c.getContext('2d')!
  const g = ctx.createRadialGradient(32, 32, 0, 32, 32, 32)
  g.addColorStop(0, 'rgba(255,255,255,1)')
  g.addColorStop(0.7, 'rgba(255,255,255,1)')
  g.addColorStop(1, 'rgba(255,255,255,0)')
  ctx.fillStyle = g
  ctx.beginPath()
  ctx.arc(32, 32, 32, 0, Math.PI * 2)
  ctx.fill()
  discTexture = new THREE.CanvasTexture(c)
  return discTexture
}

function SamplePoints({
  data,
  rays,
  visible,
  colorBy,
  size,
}: {
  data: ViewerData
  rays: [number, number]
  visible: Set<LayerName>
  colorBy: ColorBy
  size: number
}) {
  const geometry = useMemo(() => {
    const [r0, r1] = rays
    const S = data.samplesPerRay
    const pos: number[] = []
    const col: number[] = []
    const p: [number, number, number] = [0, 0, 0]
    const c = new THREE.Color()
    for (let r = r0; r < r1; r++) {
      for (let s = 0; s < S; s++) {
        const idx = r * S + s
        const layer = displayLayer(data.layers[idx], visible)
        if (!layer) continue
        samplePosition(data, r, s, p)
        pos.push(p[0], p[1], p[2])
        if (colorBy === 'distance' && data.distances) {
          const [cr, cg, cb] = distanceColor(data.distances[idx])
          col.push(cr, cg, cb)
        } else {
          c.set(layer.color)
          col.push(c.r, c.g, c.b)
        }
      }
    }
    const geo = new THREE.BufferGeometry()
    geo.setAttribute('position', new THREE.Float32BufferAttribute(pos, 3))
    geo.setAttribute('color', new THREE.Float32BufferAttribute(col, 3))
    return geo
  }, [data, rays[0], rays[1], visible, colorBy])
  useEffect(() => () => geometry.dispose(), [geometry])
  return (
    <points geometry={geometry} renderOrder={3}>
      <pointsMaterial
        size={size}
        sizeAttenuation
        vertexColors
        map={disc()}
        alphaTest={0.5}
        transparent={false}
        toneMapped={false}
      />
    </points>
  )
}

/** Orbit controls that glide to the selected shape when asked. */
function Controls({ home, focus, focusTick }: { home: THREE.Vector3; focus: THREE.Vector3; focusTick: number }) {
  const controls = useRef<React.ComponentRef<typeof OrbitControls>>(null)
  const goal = useRef<THREE.Vector3 | null>(null)
  const { camera } = useThree()

  useEffect(() => {
    controls.current?.target.copy(home)
  }, [home])

  useEffect(() => {
    if (focusTick > 0) goal.current = focus.clone()
    // Only a new request moves the camera, not every change of selection.
  }, [focusTick])

  useFrame((_, dt) => {
    const c = controls.current
    if (!c || !goal.current) return
    const k = 1 - Math.exp(-dt * 6)
    const delta = goal.current.clone().sub(c.target).multiplyScalar(k)
    c.target.add(delta)
    camera.position.add(delta)
    // Also move in so the shape fills the view.
    const toTarget = camera.position.clone().sub(c.target)
    const want = 14
    toTarget.setLength(toTarget.length() + (want - toTarget.length()) * k)
    camera.position.copy(c.target).add(toTarget)
    if (c.target.distanceTo(goal.current) < 0.01 && Math.abs(toTarget.length() - want) < 0.05) goal.current = null
    c.update()
  })

  return <OrbitControls ref={controls} makeDefault enableDamping dampingFactor={0.12} />
}
