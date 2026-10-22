import { lazy, Suspense, useEffect, useState } from 'react'
import { api, authUrl, type DockSummary, type Job } from '../api'
import { CAPRI_LABEL, ms } from '../format'
import { parseDockViewer, type DockViewerData } from '../dockData'
import { Metric } from './Metric'

const DockViewer = lazy(() => import('./DockViewer').then((m) => ({ default: m.DockViewer })))

interface Props {
  job: Job
  summary: DockSummary
  timings: { read: number; compute: number; write: number }
}

export function DockResults({ job, summary, timings }: Props) {
  const [data, setData] = useState<DockViewerData | null>(null)
  const [viewerError, setViewerError] = useState<string | null>(null)
  const [selected, setSelected] = useState(1)
  const poses = summary.poses
  const native = summary.native
  const pose = poses.find((p) => p.rank === selected) ?? poses[0]
  const files = new Set(job.report?.files.map((f) => f.name))

  useEffect(() => {
    let alive = true
    api
      .dockViewer(job.id)
      .then((buf) => alive && setData(parseDockViewer(buf)))
      .catch((e) => alive && setViewerError(e.message))
    return () => {
      alive = false
    }
  }, [job.id])

  const st = summary.stats
  const rank = (r: number | null) => (r ? `#${r}` : '—')

  return (
    <>
      <dl className="metrics">
        <Metric label="Poses" value={poses.length.toString()} />
        <Metric label="Átomos (rec. / lig.)" value={`${summary.receptor_atoms}/${summary.ligand_atoms}`} />
        <Metric label="Pares evaluados" value={`${(st.pairs / 1e6).toFixed(1)} M`} />
        <Metric label="Docking" value={ms(Math.round(st.seconds_total * 1000))} accent />
        <Metric label="Total" value={ms(timings.read + timings.compute + timings.write)} />
        <Metric label="Superficie" value={summary.surface === 'ses' ? 'SES' : 'átomos'} />
      </dl>

      {native && (
        <section className="block">
          <h3>Contra el complejo real</h3>
          <dl className="metrics">
            <Metric label="Primera aceptable" value={rank(native.first_acceptable)} accent={!!native.first_acceptable} />
            <Metric label="Primera media" value={rank(native.first_medium)} />
            <Metric label="Primera alta" value={rank(native.first_high)} />
            <Metric label="Mejor DockQ" value={native.best_dockq?.toFixed(3) ?? '—'} />
          </dl>
          <p className="note">
            Calidad CAPRI y DockQ de cada pose frente a la estructura cristalográfica del complejo
            {summary.randomize_seed !== null ? ', partiendo de una posición aleatoria del ligando.' : '.'}
          </p>
          {native.first_acceptable && native.first_acceptable !== pose?.rank && (
            <button className="ghost small" onClick={() => setSelected(native.first_acceptable!)}>
              Ver la primera pose aceptable (#{native.first_acceptable})
            </button>
          )}
        </section>
      )}

      <section className="block">
        <h3>Visor 3D</h3>
        {viewerError && <p className="error">{viewerError}</p>}
        {data && pose ? (
          <Suspense fallback={<div className="viewer-placeholder">Cargando visor…</div>}>
            <DockViewer
              data={data}
              pose={pose.transform}
              native={native?.pose ?? null}
              label={`pose ${pose.rank} · puntaje ${pose.score.toFixed(1)}${pose.metrics ? ` · DockQ ${pose.metrics.dockq.toFixed(3)}` : ''}`}
            />
          </Suspense>
        ) : (
          !viewerError && <div className="viewer-placeholder">Cargando superficies…</div>
        )}
      </section>

      <section className="block">
        <h3>Poses</h3>
        <div className="poses-wrap">
          <table className="files poses">
            <thead>
              <tr>
                <th>#</th>
                <th className="right">Puntaje</th>
                <th className="right">Cúmulo</th>
                {native && (
                  <>
                    <th>CAPRI</th>
                    <th className="right">DockQ</th>
                    <th className="right">LRMSD</th>
                    <th className="right">iRMSD</th>
                    <th className="right">fnat</th>
                  </>
                )}
                <th />
              </tr>
            </thead>
            <tbody>
              {poses.map((p) => {
                const file = `${job.name}_pose_${String(p.rank).padStart(3, '0')}.pdb`
                return (
                  <tr key={p.rank} className={p.rank === pose?.rank ? 'sel' : ''} onClick={() => setSelected(p.rank)}>
                    <td className="mono">{p.rank}</td>
                    <td className="mono right">{p.score.toFixed(1)}</td>
                    <td className="mono right dim">{p.cluster_size}</td>
                    {p.metrics && (
                      <>
                        <td>
                          <span className={`capri ${p.metrics.capri}`}>{CAPRI_LABEL[p.metrics.capri]}</span>
                        </td>
                        <td className="mono right">{p.metrics.dockq.toFixed(3)}</td>
                        <td className="mono right">{p.metrics.lrmsd.toFixed(1)}</td>
                        <td className="mono right">{p.metrics.irmsd.toFixed(1)}</td>
                        <td className="mono right">{p.metrics.fnat.toFixed(2)}</td>
                      </>
                    )}
                    <td className="right">
                      {files.has(file) && (
                        <a
                          href={authUrl(`/jobs/${job.id}/files/${encodeURIComponent(file)}`, 'download')}
                          onClick={(e) => e.stopPropagation()}
                        >
                          PDB
                        </a>
                      )}
                    </td>
                  </tr>
                )
              })}
            </tbody>
          </table>
        </div>
      </section>
    </>
  )
}
