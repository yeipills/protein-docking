import type { LayerName } from './api'

export interface LayerInfo {
  name: LayerName
  bit: number
  label: string
  color: string
}

// Diverging palette: cool blues inside the surface, amber on it, warm reds outside.
// Order matches the bit positions written by the core (Layer::ALL).
export const LAYERS: LayerInfo[] = [
  { name: 'ses', bit: 1 << 0, label: 'SES', color: '#f2b134' },
  { name: 'in1', bit: 1 << 1, label: 'in 1', color: '#8fd3f4' },
  { name: 'in2', bit: 1 << 2, label: 'in 2', color: '#5fb3d9' },
  { name: 'in3', bit: 1 << 3, label: 'in 3', color: '#3a86c8' },
  { name: 'in4', bit: 1 << 4, label: 'in 4', color: '#2a5aa8' },
  { name: 'out1', bit: 1 << 5, label: 'out 1', color: '#f6a77c' },
  { name: 'out2', bit: 1 << 6, label: 'out 2', color: '#ec7c5a' },
  { name: 'out3', bit: 1 << 7, label: 'out 3', color: '#d6533f' },
  { name: 'out4', bit: 1 << 8, label: 'out 4', color: '#a8322f' },
  { name: 'vol', bit: 1 << 9, label: 'volumen', color: '#6c7a96' },
]

export const LAYER_BY_NAME = Object.fromEntries(LAYERS.map((l) => [l.name, l])) as Record<LayerName, LayerInfo>

/** First visible layer (in LAYERS order) the sample belongs to, or null. */
export function displayLayer(mask: number, visible: Set<LayerName>): LayerInfo | null {
  for (const l of LAYERS) if (visible.has(l.name) && mask & l.bit) return l
  return null
}

/** Diverging colour for a signed distance in Å (blue inside, red outside). */
export function distanceColor(d: number, range = 1.5): [number, number, number] {
  const t = Math.max(-1, Math.min(1, d / range))
  const mid: [number, number, number] = [0.95, 0.69, 0.2]
  const inside: [number, number, number] = [0.16, 0.35, 0.66]
  const outside: [number, number, number] = [0.66, 0.2, 0.18]
  const end = t < 0 ? inside : outside
  const a = Math.abs(t)
  return [mid[0] + (end[0] - mid[0]) * a, mid[1] + (end[1] - mid[1]) * a, mid[2] + (end[2] - mid[2]) * a]
}
