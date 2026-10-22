export function bytes(n: number): string {
  if (n < 1024) return `${n} B`
  if (n < 1024 ** 2) return `${(n / 1024).toFixed(1)} KB`
  if (n < 1024 ** 3) return `${(n / 1024 ** 2).toFixed(1)} MB`
  return `${(n / 1024 ** 3).toFixed(2)} GB`
}

export function ms(n: number): string {
  return n < 1000 ? `${n} ms` : `${(n / 1000).toFixed(2)} s`
}

const rtf = new Intl.RelativeTimeFormat('es', { numeric: 'auto' })

export function ago(iso: string): string {
  const s = (Date.parse(iso) - Date.now()) / 1000
  const abs = Math.abs(s)
  if (abs < 60) return rtf.format(Math.round(s), 'second')
  if (abs < 3600) return rtf.format(Math.round(s / 60), 'minute')
  if (abs < 86400) return rtf.format(Math.round(s / 3600), 'hour')
  return rtf.format(Math.round(s / 86400), 'day')
}

export function duration(from: string | null, to: string | null): string | null {
  if (!from || !to) return null
  return ms(Date.parse(to) - Date.parse(from))
}

export const STATUS_LABEL: Record<string, string> = {
  queued: 'en cola',
  running: 'calculando',
  done: 'listo',
  failed: 'error',
  cancelled: 'cancelado',
}

export const STAGE_LABEL: Record<string, string> = {
  starting: 'iniciando',
  mesh: 'malla',
  centroids: 'centroides',
  critical_points: 'puntos críticos',
  rays: 'rayos',
  layers: 'capas',
  surfaces: 'superficies',
  descriptors: 'descriptores',
  matching: 'emparejamiento',
  refinement: 'refinamiento',
  clustering: 'agrupamiento',
  writing: 'escribiendo',
  done: 'escribiendo',
}

export const MODE_LABEL: Record<string, string> = {
  corrected: 'corregido',
  legacy: 'fiel',
  dock: 'docking',
}

export const CAPRI_LABEL: Record<string, string> = {
  incorrect: 'incorrecta',
  acceptable: 'aceptable',
  medium: 'media',
  high: 'alta',
}
