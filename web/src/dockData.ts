// Decoder for dock_viewer.bin (see crates/cshape/src/docking.rs, `dock_viewer_blob`).

export interface MeshData {
  vertices: Float32Array
  faces: Uint32Array
}

export interface DockViewerData {
  receptor: MeshData
  /** In the uploaded ligand's coordinates; poses move it onto the receptor. */
  ligand: MeshData
}

export function parseDockViewer(buf: ArrayBuffer): DockViewerData {
  const view = new DataView(buf)
  const magic = new TextDecoder().decode(new Uint8Array(buf, 0, 4))
  if (magic !== 'DCK1') throw new Error(`dock_viewer.bin: unexpected header ${JSON.stringify(magic)}`)
  let off = 4
  const mesh = (): MeshData => {
    const nv = view.getUint32(off, true)
    const nf = view.getUint32(off + 4, true)
    off += 8
    const vertices = new Float32Array(nv * 3)
    for (let i = 0; i < nv * 3; i++) vertices[i] = view.getFloat32(off + i * 4, true)
    off += nv * 12
    const faces = new Uint32Array(nf * 3)
    for (let i = 0; i < nf * 3; i++) faces[i] = view.getUint32(off + i * 4, true)
    off += nf * 12
    return { vertices, faces }
  }
  const receptor = mesh()
  const ligand = mesh()
  if (off !== buf.byteLength) throw new Error(`dock_viewer.bin: ${buf.byteLength - off} trailing bytes`)
  return { receptor, ligand }
}
