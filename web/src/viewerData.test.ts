import { describe, expect, it } from 'vitest'
import { parseViewer, samplePosition } from './viewerData'

/** Builds a blob in the layout written by `export::viewer_blob`. */
function blob(withDistances: boolean): ArrayBuffer {
  const bytes: number[] = []
  const u32 = (v: number) => {
    const b = new Uint8Array(new Uint32Array([v]).buffer)
    bytes.push(...b)
  }
  const f32 = (v: number) => bytes.push(...new Uint8Array(new Float32Array([v]).buffer))
  const u16 = (v: number) => bytes.push(...new Uint8Array(new Uint16Array([v]).buffer))
  bytes.push(...new TextEncoder().encode('CSV1'))
  u32(3) // vertices
  u32(1) // faces
  ;[0, 0, 0, 1, 0, 0, 0, 1, 0].forEach(f32)
  ;[0, 1, 2].forEach(u32)
  u32(1) // centers
  ;[1, 2, 3].forEach(f32)
  u32(2) // rays per center
  ;[4, 2, 3, 1, 2, 6].forEach(f32)
  u32(3) // samples per ray
  ;[0.25, 0.5, 0.75].forEach(f32)
  ;[1, 2, 4, 8, 16, 512].forEach(u16)
  while (bytes.length % 4) bytes.push(0)
  u32(withDistances ? 1 : 0)
  if (withDistances) [-0.5, 0, 0.5, 1, 1.5, 2].forEach(f32)
  return new Uint8Array(bytes).buffer
}

describe('parseViewer', () => {
  it('decodes every section', () => {
    const d = parseViewer(blob(true))
    expect(Array.from(d.faces)).toEqual([0, 1, 2])
    expect(d.raysPerCenter).toBe(2)
    expect(d.samplesPerRay).toBe(3)
    expect(Array.from(d.layers)).toEqual([1, 2, 4, 8, 16, 512])
    expect(d.distances?.[0]).toBeCloseTo(-0.5)
  })

  it('handles results without distances', () => {
    expect(parseViewer(blob(false)).distances).toBeNull()
  })

  it('places samples along their ray', () => {
    const d = parseViewer(blob(false))
    // Ray 0 goes from (1,2,3) to (4,2,3); sample 1 is halfway.
    expect(samplePosition(d, 0, 1, [0, 0, 0])).toEqual([2.5, 2, 3])
  })

  it('rejects other files', () => {
    expect(() => parseViewer(new TextEncoder().encode('nope').buffer)).toThrow()
  })
})
