import { useCallback, useEffect, useState } from 'react'
import { api, setToken, setUnauthorizedHandler, UnauthorizedError, type Config, type Job, type Sample } from './api'
import { JobDetail } from './components/JobDetail'
import { JobList } from './components/JobList'
import { NewJob } from './components/NewJob'

function hashId(): string | null {
  const m = location.hash.match(/^#\/jobs\/([0-9a-f-]{36})$/)
  return m ? m[1] : null
}

export function App() {
  const [config, setConfig] = useState<Config | null>(null)
  const [samples, setSamples] = useState<Sample[]>([])
  const [jobs, setJobs] = useState<Job[]>([])
  const [selected, setSelected] = useState<string | null>(hashId)
  const [needToken, setNeedToken] = useState(false)

  useEffect(() => setUnauthorizedHandler(() => setNeedToken(true)), [])

  const refresh = useCallback(() => {
    api
      .jobs()
      .then(setJobs)
      .catch((e) => !(e instanceof UnauthorizedError) && console.error(e))
  }, [])

  const load = useCallback(() => {
    api.config().then(setConfig).catch(() => {})
    api.samples().then(setSamples).catch(() => {})
    refresh()
  }, [refresh])

  useEffect(load, [load])

  // Poll while anything is pending so the list shows live progress.
  const pending = jobs.some((j) => j.status === 'queued' || j.status === 'running')
  useEffect(() => {
    if (!pending) return
    const t = setInterval(refresh, 1500)
    return () => clearInterval(t)
  }, [pending, refresh])

  useEffect(() => {
    const onHash = () => setSelected(hashId())
    addEventListener('hashchange', onHash)
    return () => removeEventListener('hashchange', onHash)
  }, [])

  const select = (id: string | null) => {
    location.hash = id ? `#/jobs/${id}` : ''
    setSelected(id)
  }

  return (
    <div className="app">
      <header className="topbar">
        <a className="brand" href="#" onClick={() => select(null)}>
          <Logo />
          <span>
            cshape<em>context shapes</em>
          </span>
        </a>
        <span className="mono dim">{config ? `v${config.version}` : ''}</span>
      </header>

      <main className="layout">
        <aside className="sidebar">
          <NewJob
            defaults={config?.defaults ?? null}
            samples={samples}
            onCreated={(job) => {
              refresh()
              select(job.id)
            }}
          />
          <JobList jobs={jobs} selected={selected} onSelect={select} />
        </aside>
        <section className="content">
          {selected ? (
            <JobDetail
              key={selected}
              id={selected}
              onChanged={refresh}
              onDeleted={() => {
                refresh()
                select(null)
              }}
            />
          ) : (
            <Intro />
          )}
        </section>
      </main>

      {needToken && (
        <TokenDialog
          onSubmit={(t) => {
            setToken(t)
            setNeedToken(false)
            load()
          }}
        />
      )}
    </div>
  )
}

function Intro() {
  return (
    <div className="intro">
      <p className="eyebrow mono">MSMS → formas de contexto</p>
      <h1>
        La superficie de una proteína,
        <br />
        <i>vista desde cada punto crítico.</i>
      </h1>
      <p>
        Para cada punto crítico de la superficie excluida al solvente se lanza una esfera de rayos y cada muestra se
        clasifica en diez capas: la superficie (SES), cuatro capas interiores, cuatro exteriores y el volumen.
      </p>
      <ol className="steps">
        <li>
          <span>
          <b>Sube</b> los archivos <code>.vert</code> y <code>.face</code> de MSMS (y el <code>.stl</code> si usas el modo
          fiel).
          </span>
        </li>
        <li>
          <span>
          <b>Elige</b> el modo <i>corregido</i> (geometría exacta) o <i>fiel al original</i> (mismos números que los scripts
          de la tesis).
          </span>
        </li>
        <li>
          <span>
          <b>Explora</b> el resultado en 3D y descarga los archivos para Unity.
          </span>
        </li>
      </ol>
    </div>
  )
}

function TokenDialog({ onSubmit }: { onSubmit: (token: string) => void }) {
  const [value, setValue] = useState('')
  return (
    <div className="modal-backdrop">
      <form
        className="modal"
        onSubmit={(e) => {
          e.preventDefault()
          onSubmit(value.trim())
        }}
      >
        <h2>Acceso</h2>
        <p>Este servidor requiere un token de acceso.</p>
        <input autoFocus type="password" value={value} onChange={(e) => setValue(e.target.value)} placeholder="token" />
        <button className="primary" type="submit" disabled={!value.trim()}>
          Entrar
        </button>
      </form>
    </div>
  )
}

function Logo() {
  return (
    <svg viewBox="0 0 32 32" width="26" height="26" aria-hidden>
      <circle cx="16" cy="16" r="10" fill="none" stroke="var(--amber)" strokeWidth="1.6" />
      <g stroke="var(--in2)" strokeWidth="1.3" strokeLinecap="round">
        {[0, 60, 120, 180, 240, 300].map((a) => (
          <line
            key={a}
            x1="16"
            y1="16"
            x2={16 + 13 * Math.cos((a * Math.PI) / 180)}
            y2={16 + 13 * Math.sin((a * Math.PI) / 180)}
          />
        ))}
      </g>
      <circle cx="16" cy="16" r="2.6" fill="var(--amber)" />
    </svg>
  )
}
