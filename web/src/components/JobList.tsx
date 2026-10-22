import type { Job } from '../api'
import { MODE_LABEL, STATUS_LABEL, ago } from '../format'

interface Props {
  jobs: Job[]
  selected: string | null
  onSelect: (id: string) => void
}

export function JobList({ jobs, selected, onSelect }: Props) {
  return (
    <section className="panel job-list">
      <header className="panel-head">
        <h2>Cálculos</h2>
        <span className="count mono">{jobs.length}</span>
      </header>
      {jobs.length === 0 ? (
        <p className="empty">Todavía no hay cálculos. Sube un archivo o prueba con una muestra.</p>
      ) : (
        <ul>
          {jobs.map((j, i) => (
            <li key={j.id} style={{ animationDelay: `${Math.min(i, 12) * 30}ms` }}>
              <button className={`job-row ${j.id === selected ? 'sel' : ''}`} onClick={() => onSelect(j.id)}>
                <span className={`dot ${j.status}`} aria-hidden />
                <span className="job-name">{j.name}</span>
                <span className={`tag ${j.mode}`}>{MODE_LABEL[j.mode]}</span>
                <span className="job-meta mono">
                  {j.status === 'running' ? `${Math.round(j.progress * 100)}%` : STATUS_LABEL[j.status]} · {ago(j.created_at)}
                </span>
                {(j.status === 'running' || j.status === 'queued') && (
                  <span className="bar">
                    <span style={{ transform: `scaleX(${j.progress})` }} />
                  </span>
                )}
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  )
}
