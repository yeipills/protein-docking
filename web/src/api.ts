// Typed client for the cshape HTTP API.

export type Mode = 'corrected' | 'legacy' | 'dock'
export type Status = 'queued' | 'running' | 'done' | 'failed' | 'cancelled'
export type LayerName = 'ses' | 'in1' | 'in2' | 'in3' | 'in4' | 'out1' | 'out2' | 'out3' | 'out4' | 'vol'

export interface Params {
  skip_face_types: number[]
  min_spacing: number
  radius: number
  rays: number
  samples: number
  layer_edges: number[]
  ses_half_width: number | null
}

export interface Summary {
  mode: Mode
  surface_points: number
  context_shapes: number
  rays_per_shape: number
  samples_per_ray: number
  radius: number
  layer_occupancy: { layer: LayerName; fraction: number }[]
  params?: Params
  input: { vertices: number; faces: number; atoms?: number | null }
}

export interface OutputFile {
  name: string
  bytes: number
  kind: 'unity' | 'legacy' | 'viewer' | string
}

export interface Report {
  summary: Summary
  timings_ms: { read: number; compute: number; write: number }
  files: OutputFile[]
}

export interface Job {
  id: string
  name: string
  mode: Mode
  status: Status
  progress: number
  stage: string
  error: string | null
  created_at: string
  started_at: string | null
  finished_at: string | null
  spec?: { name: string; mode: Mode; params: Params; legacy_intermediate: boolean }
  report?: Report | DockReport
}

export interface DockReport {
  summary: DockSummary
  timings_ms: { read: number; compute: number; write: number }
  files: OutputFile[]
}

export interface JobEvent {
  status: Status
  progress: number
  stage: string
  error?: string
}

export interface Sample {
  name: string
  has_stl: boolean
}

export type Capri = 'incorrect' | 'acceptable' | 'medium' | 'high'

export interface Metrics {
  fnat: number
  fnonnat: number
  irmsd: number
  lrmsd: number
  dockq: number
  capri: Capri
}

export interface RigidT {
  rot: number[][]
  trans: number[]
}

export interface DockPose {
  rank: number
  transform: RigidT
  score: number
  cluster_size: number
  components: Record<string, number>
  metrics?: Metrics
}

export interface DockSummary {
  kind: 'dock'
  surface: 'atoms' | 'ses'
  receptor_atoms: number
  ligand_atoms: number
  randomize_seed: number | null
  stats: {
    receptor_points: number
    ligand_points: number
    pairs: number
    candidates: number
    seeds: number
    seconds_descriptors: number
    seconds_matching: number
    seconds_refinement: number
    seconds_total: number
  }
  poses: DockPose[]
  native: null | {
    pose: RigidT
    first_acceptable: number | null
    first_medium: number | null
    first_high: number | null
    best_dockq: number | null
  }
}

export interface Complex {
  name: string
  has_native: boolean
}

export interface Config {
  version: string
  defaults: Params
}

const TOKEN_KEY = 'cshape.token'

export function getToken(): string {
  try {
    return localStorage.getItem(TOKEN_KEY) ?? ''
  } catch {
    return ''
  }
}

export function setToken(token: string) {
  try {
    localStorage.setItem(TOKEN_KEY, token)
  } catch {
    // Storage unavailable (private mode): the token lasts for this page only.
  }
  memoryToken = token
}

let memoryToken = getToken()

export class UnauthorizedError extends Error {}

export class ApiError extends Error {
  status: number
  constructor(status: number, message: string) {
    super(message)
    this.status = status
  }
}

let onUnauthorized: () => void = () => {}
export function setUnauthorizedHandler(fn: () => void) {
  onUnauthorized = fn
}

async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
  const headers = new Headers(init.headers)
  if (memoryToken) headers.set('Authorization', `Bearer ${memoryToken}`)
  const res = await fetch(`/api${path}`, { ...init, headers })
  if (res.status === 401) {
    onUnauthorized()
    throw new UnauthorizedError('unauthorized')
  }
  if (!res.ok) {
    let message = `${res.status} ${res.statusText}`
    try {
      const body = await res.json()
      if (body?.error) message = body.error
    } catch {
      // keep the status text
    }
    throw new ApiError(res.status, message)
  }
  if (res.status === 204 || res.status === 202) return undefined as T
  const type = res.headers.get('content-type') ?? ''
  return (type.includes('application/json') ? res.json() : res.arrayBuffer()) as Promise<T>
}

/** URL for links and EventSource, which cannot send headers. */
export function authUrl(path: string, extra = ''): string {
  const params = new URLSearchParams(extra)
  if (memoryToken) params.set('token', memoryToken)
  const q = params.toString()
  return `/api${path}${q ? `?${q}` : ''}`
}

export const api = {
  config: () => request<Config>('/config'),
  jobs: () => request<Job[]>('/jobs'),
  job: (id: string) => request<Job>(`/jobs/${id}`),
  samples: () => request<Sample[]>('/samples'),
  createJob: (form: FormData) => request<Job>('/jobs', { method: 'POST', body: form }),
  createSampleJob: (name: string, body: object) =>
    request<Job>(`/samples/${encodeURIComponent(name)}/jobs`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    }),
  cancel: (id: string) => request<void>(`/jobs/${id}/cancel`, { method: 'POST' }),
  remove: (id: string) => request<void>(`/jobs/${id}`, { method: 'DELETE' }),
  viewer: (id: string) => request<ArrayBuffer>(`/jobs/${id}/files/viewer.bin`),
  dockViewer: (id: string) => request<ArrayBuffer>(`/jobs/${id}/files/dock_viewer.bin`),
  complexes: () => request<Complex[]>('/complexes'),
  createDock: (form: FormData) => request<Job>('/dock', { method: 'POST', body: form }),
  createComplexDock: (name: string, body: object) =>
    request<Job>(`/complexes/${encodeURIComponent(name)}/dock`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    }),
}

/** Streams progress events until the job finishes. Returns a function that stops listening. */
export function watchJob(id: string, onEvent: (e: JobEvent) => void): () => void {
  const source = new EventSource(authUrl(`/jobs/${id}/events`))
  source.onmessage = (msg) => {
    const ev = JSON.parse(msg.data) as JobEvent
    onEvent(ev)
    if (isTerminal(ev.status)) source.close()
  }
  source.onerror = () => {
    // The server closes the stream after the final event; do not reconnect forever.
    if (source.readyState === EventSource.CLOSED) return
    source.close()
  }
  return () => source.close()
}

export function isTerminal(s: Status): boolean {
  return s === 'done' || s === 'failed' || s === 'cancelled'
}
