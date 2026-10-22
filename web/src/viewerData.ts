// Decoder for viewer.bin (see crates/cs-core/src/export.rs, `viewer_blob`).

export interface ViewerData {
  vertices: Float32Array
  faces: Uint32Array
  centers: Float32Array
  raysPerCenter: number
  rayEnds: Float32Array
  samplesPerRay: number
  sampleT: Float32Array
  layers: Uint16Array
  distances: Float32Array | null
}

export function parseViewer(buf: ArrayBuffer): ViewerData {
  const view = new DataView(buf)
  const magic = new TextDecoder().decode(new Uint8Array(buf, 0, 4))
  if (magic !== 'CSV1') throw new Error(`viewer.bin: unexpected header ${JSON.stringify(magic)}`)
  let off = 4
  const u32 = () => {
    const v = view.getUint32(off, true)
    off += 4
    return v
  }
  // Copies keep the arrays independent of alignment and of the source buffer.
  const f32 = (n: number) => {
    const a = new Float32Array(n)
    for (let i = 0; i < n; i++) a[i] = view.getFloat32(off + i * 4, true)
    off += n * 4
    return a
  }
  const u32s = (n: number) => {
    const a = new Uint32Array(n)
    for (let i = 0; i < n; i++) a[i] = view.getUint32(off + i * 4, true)
    off += n * 4
    return a
  }

  const nv = u32()
  const nf = u32()
  const vertices = f32(nv * 3)
  const faces = u32s(nf * 3)
  const nc = u32()
  const centers = f32(nc * 3)
  const raysPerCenter = u32()
  const rayEnds = f32(nc * raysPerCenter * 3)
  const samplesPerRay = u32()
  const sampleT = f32(samplesPerRay)
  const ns = nc * raysPerCenter * samplesPerRay
  const layers = new Uint16Array(ns)
  for (let i = 0; i < ns; i++) layers[i] = view.getUint16(off + i * 2, true)
  off += ns * 2
  off = (off + 3) & ~3
  const hasDistances = u32()
  const distances = hasDistances ? f32(ns) : null
  if (off !== buf.byteLength) throw new Error(`viewer.bin: ${buf.byteLength - off} trailing bytes`)
  return { vertices, faces, centers, raysPerCenter, rayEnds, samplesPerRay, sampleT, layers, distances }
}

/** Position of sample `s` of ray `r` (global ray index). */
export function samplePosition(d: ViewerData, r: number, s: number, out: [number, number, number]) {
  const c = Math.floor(r / d.raysPerCenter)
  const t = d.sampleT[s]
  for (let k = 0; k < 3; k++) {
    const o = d.centers[c * 3 + k]
    out[k] = o + (d.rayEnds[r * 3 + k] - o) * t
  }
  return out
}
