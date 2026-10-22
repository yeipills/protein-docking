import { lazy, Suspense, useEffect, useMemo, useState } from 'react'
import { api, authUrl, isTerminal, watchJob, type DockSummary, type Job, type LayerName, type Report } from '../api'
import { LAYER_BY_NAME, LAYERS } from '../layers'
import { MODE_LABEL, STAGE_LABEL, STATUS_LABEL, bytes, duration, ms } from '../format'
import { parseViewer, type ViewerData } from '../viewerData'
import { DockResults } from './DockResults'
import { Metric } from './Metric'

const Viewer = lazy(() => import('./Viewer').then((m) => ({ default: m.Viewer })))

interface Props {
  id: string
  onChanged: () => void
  onDeleted: () => void
}

export function JobDetail({ id, onChanged, onDeleted }: Props) {
  const [job, setJob] = useState<Job | null>(null)
  const [error, setError] = useState<string | null>(null)

  // Load the job, then follow its progress until it finishes.
  useEffect(() => {
    let stop = () => {}
    let alive = true
    setJob(null)
    setError(null)
    api
      .job(id)
      .then((j) => {
        if (!alive) return
        setJob(j)
        if (isTerminal(j.status)) return
        stop = watchJob(id, (ev) => {
          setJob((prev) => (prev ? { ...prev, ...ev, error: ev.error ?? prev.error } : prev))
          if (isTerminal(ev.status)) {
            api.job(id).then((full) => alive && setJob(full))
            onChanged()
          }
        })
      })
      .catch((e) => alive && setError(e.message))
    return () => {
      alive = false
      stop()
    }
  }, [id, onChanged])

  if (error) return <div className="detail"><p className="error">{error}</p></div>
  if (!job) return <div className="detail loading">Cargando…</div>

  const active = job.status === 'running' || job.status === 'queued'
  const isDock = job.mode === 'dock'

  async function cancel() {
    await api.cancel(job!.id).catch((e) => setError(e.message))
  }
  async function remove() {
    if (!confirm(`¿Borrar el cálculo "${job!.name}" y sus archivos?`)) return
    await api.remove(job!.id)
    onDeleted()
  }

  return (
    <div className="detail">
      <header className="detail-head">
        <div>
          <p className="eyebrow mono">
            {MODE_LABEL[job.mode]} · {job.id.slice(0, 8)}
          </p>
          <h1>{job.name}</h1>
        </div>
        <div className="detail-actions">
          {active && (
            <button className="ghost" onClick={cancel}>
              Cancelar
            </button>
          )}
          {job.status === 'done' && (
            <a className="button primary" href={authUrl(`/jobs/${job.id}/archive`)}>
              Descargar todo (.zip)
            </a>
          )}
          {!active && (
            <button className="ghost danger" onClick={remove}>
              Borrar
            </button>
          )}
        </div>
      </header>

      {active && (
        <div className="progress">
          <div className="progress-label mono">
            <span>{job.status === 'queued' ? 'En cola' : STAGE_LABEL[job.stage] ?? job.stage}</span>
            <span>{Math.round(job.progress * 100)}%</span>
          </div>
          <div className="progress-track">
            <div className="progress-fill" style={{ transform: `scaleX(${job.progress})` }} />
          </div>
        </div>
      )}
      {job.status === 'failed' && <p className="error">{job.error ?? 'El cálculo falló'}</p>}
      {job.status === 'cancelled' && <p className="note">Cálculo cancelado.</p>}

      {job.status === 'done' && job.report && (
        isDock ? (
          <DockResults
            job={job}
            summary={job.report.summary as DockSummary}
            timings={job.report.timings_ms}
          />
        ) : (
          <ShapesResults job={job} report={job.report as Report} />
        )
      )}

      {job.report && (
        <section className="block">
          <h3>Archivos</h3>
          <table className="files">
            <tbody>
              {job.report.files.map((f) => (
                <tr key={f.name}>
                  <td className="mono">
                    <a href={authUrl(`/jobs/${job.id}/files/${encodeURIComponent(f.name)}`, 'download')}>{f.name}</a>
                  </td>
                  <td>
                    <span className={`tag ${f.kind}`}>{f.kind}</span>
                  </td>
                  <td className="mono dim right">{bytes(f.bytes)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      )}

      <footer className="detail-foot mono dim">
        {STATUS_LABEL[job.status]} · creado {new Date(job.created_at).toLocaleString('es')}
        {job.started_at && job.finished_at && ` · ${duration(job.started_at, job.finished_at)}`}
      </footer>
    </div>
  )
}

/** Results of a context-shape job: metrics, layer occupancy, 3D viewer, parameters. */
function ShapesResults({ job, report }: { job: Job; report: Report }) {
  const [viewer, setViewer] = useState<ViewerData | null>(null)
  const [viewerError, setViewerError] = useState<string | null>(null)
  const s = report.summary
  const t = report.timings_ms

  useEffect(() => {
    let alive = true
    api
      .viewer(job.id)
      .then((buf) => alive && setViewer(parseViewer(buf)))
      .catch((e) => alive && setViewerError(e.message))
    return () => {
      alive = false
    }
  }, [job.id])

  const occupancy = useMemo(() => {
    const o: Partial<Record<LayerName, number>> = {}
    for (const l of s.layer_occupancy) o[l.layer] = l.fraction
    return o
  }, [s])

  return (
    <>
      <dl className="metrics">
        <Metric label="Formas de contexto" value={s.context_shapes.toLocaleString('es')} />
        <Metric label="Puntos de superficie" value={s.surface_points.toLocaleString('es')} />
        <Metric label="Rayos × muestras" value={`${s.rays_per_shape} × ${s.samples_per_ray}`} />
        <Metric label="Radio" value={`${s.radius} Å`} />
        <Metric label="Cálculo" value={ms(t.compute)} accent />
        <Metric label="Total" value={duration(job.started_at, job.finished_at) ?? ms(t.read + t.compute + t.write)} />
      </dl>

      <section className="block">
        <h3>Ocupación por capa</h3>
        <div className="occupancy">
          {LAYERS.map((l) => {
            const f = occupancy[l.name] ?? 0
            return (
              <div key={l.name} className="occ-row">
                <span className="mono">{LAYER_BY_NAME[l.name].label}</span>
                <div className="occ-track">
                  <div style={{ width: `${(f * 100).toFixed(2)}%`, background: l.color }} />
                </div>
                <span className="mono dim">{(f * 100).toFixed(1)}%</span>
              </div>
            )
          })}
        </div>
      </section>

      <section className="block">
        <h3>Visor 3D</h3>
        {viewerError && <p className="error">{viewerError}</p>}
        {viewer ? (
          <Suspense fallback={<div className="viewer-placeholder">Cargando visor…</div>}>
            <Viewer data={viewer} mode={job.mode} occupancy={occupancy} />
          </Suspense>
        ) : (
          !viewerError && <div className="viewer-placeholder">Cargando geometría…</div>
        )}
      </section>

      {s.params && (
        <section className="block">
          <h3>Parámetros</h3>
          <pre className="params mono">{JSON.stringify(s.params, null, 2)}</pre>
        </section>
      )}
      <p className="note mono dim">
        {s.input.vertices.toLocaleString('es')} vértices, {s.input.faces.toLocaleString('es')} caras
        {s.input.atoms ? ` · ${s.input.atoms.toLocaleString('es')} átomos (superficie generada)` : ''}
      </p>
    </>
  )
}
