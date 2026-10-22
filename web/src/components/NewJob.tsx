import { useEffect, useRef, useState } from 'react'
import { api, ApiError, type Job, type Mode, type Params, type Sample } from '../api'

interface Props {
  defaults: Params | null
  samples: Sample[]
  onCreated: (job: Job) => void
}

interface Files {
  vert?: File
  face?: File
  stl?: File
}

function assign(files: FileList | File[], prev: Files): Files {
  const next = { ...prev }
  for (const f of Array.from(files)) {
    const ext = f.name.toLowerCase().split('.').pop()
    if (ext === 'vert') next.vert = f
    else if (ext === 'face') next.face = f
    else if (ext === 'stl') next.stl = f
  }
  return next
}

export function NewJob({ defaults, samples, onCreated }: Props) {
  const [mode, setMode] = useState<Mode>('corrected')
  const [files, setFiles] = useState<Files>({})
  const [name, setName] = useState('')
  const [params, setParams] = useState<Params | null>(null)
  const [edges, setEdges] = useState('')
  const [intermediate, setIntermediate] = useState(false)
  const [advanced, setAdvanced] = useState(false)
  const [dragging, setDragging] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const input = useRef<HTMLInputElement>(null)

  // Defaults arrive from /api/config after the first render.
  useEffect(() => {
    if (defaults && !params) {
      setParams(defaults)
      setEdges(defaults.layer_edges.join(', '))
    }
  }, [defaults, params])

  const effectiveParams = (): Params | null => {
    if (!params) return null
    const parsed = edges.split(/[,\s]+/).filter(Boolean).map(Number)
    if (parsed.length !== 9 || parsed.some((x) => !Number.isFinite(x))) {
      throw new Error('Los bordes de capa deben ser 9 números en Å')
    }
    return { ...params, layer_edges: parsed }
  }

  const ready = files.vert && files.face && (mode === 'corrected' || files.stl)

  async function submit(e: React.FormEvent) {
    e.preventDefault()
    if (!ready) return
    setBusy(true)
    setError(null)
    try {
      const form = new FormData()
      form.set('vert', files.vert!)
      form.set('face', files.face!)
      if (mode === 'legacy' && files.stl) form.set('stl', files.stl)
      form.set('mode', mode)
      if (name.trim()) form.set('name', name.trim())
      if (mode === 'corrected') {
        const p = effectiveParams()
        if (p) form.set('params', JSON.stringify(p))
      } else if (intermediate) {
        form.set('legacy_intermediate', 'true')
      }
      const job = await api.createJob(form)
      setFiles({})
      setName('')
      onCreated(job)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setBusy(false)
    }
  }

  async function runSample(s: Sample) {
    setBusy(true)
    setError(null)
    try {
      const body =
        mode === 'corrected' ? { mode, params: effectiveParams() ?? undefined } : { mode, legacy_intermediate: intermediate }
      onCreated(await api.createSampleJob(s.name, body))
    } catch (err) {
      setError(err instanceof ApiError || err instanceof Error ? err.message : String(err))
    } finally {
      setBusy(false)
    }
  }

  const num = (key: keyof Params, label: string, unit: string, step: number, min: number) => (
    <label className="field">
      <span>{label}</span>
      <div className="input-unit">
        <input
          type="number"
          step={step}
          min={min}
          value={params ? (params[key] as number) : ''}
          onChange={(e) => params && setParams({ ...params, [key]: Number(e.target.value) })}
        />
        <em>{unit}</em>
      </div>
    </label>
  )

  return (
    <form className="panel new-job" onSubmit={submit}>
      <header className="panel-head">
        <h2>Nuevo cálculo</h2>
      </header>

      <div className="segmented" role="tablist">
        <button type="button" className={mode === 'corrected' ? 'on' : ''} onClick={() => setMode('corrected')}>
          Corregido
        </button>
        <button type="button" className={mode === 'legacy' ? 'on' : ''} onClick={() => setMode('legacy')}>
          Fiel al original
        </button>
      </div>
      <p className="note">
        {mode === 'corrected'
          ? 'Distancias exactas a la superficie, rayos uniformes e índices MSMS correctos.'
          : 'Reproduce byte a byte los scripts originales, incluidos sus errores. Requiere el STL.'}
      </p>

      <div
        className={`drop ${dragging ? 'over' : ''}`}
        onDragOver={(e) => {
          e.preventDefault()
          setDragging(true)
        }}
        onDragLeave={() => setDragging(false)}
        onDrop={(e) => {
          e.preventDefault()
          setDragging(false)
          setFiles((f) => assign(e.dataTransfer.files, f))
        }}
        onClick={() => input.current?.click()}
        role="button"
        tabIndex={0}
        onKeyDown={(e) => (e.key === 'Enter' || e.key === ' ') && input.current?.click()}
      >
        <input
          ref={input}
          type="file"
          multiple
          accept=".vert,.face,.stl"
          hidden
          onChange={(e) => e.target.files && setFiles((f) => assign(e.target.files!, f))}
        />
        <strong>Suelta aquí los archivos MSMS</strong>
        <span>o haz clic para elegirlos</span>
        <ul className="file-slots mono">
          <FileSlot ext=".vert" file={files.vert} />
          <FileSlot ext=".face" file={files.face} />
          <FileSlot ext=".stl" file={files.stl} optional={mode === 'corrected'} />
        </ul>
      </div>

      <label className="field">
        <span>Nombre</span>
        <input
          type="text"
          placeholder={files.vert?.name.replace(/\.vert$/i, '') ?? 'proteína'}
          value={name}
          onChange={(e) => setName(e.target.value)}
        />
      </label>

      {mode === 'corrected' && params && (
        <>
          <div className="grid2">
            {num('min_spacing', 'Espaciado mínimo', 'Å', 0.5, 0.1)}
            {num('radius', 'Radio', 'Å', 0.5, 0.1)}
            {num('rays', 'Rayos por forma', '', 10, 1)}
            {num('samples', 'Muestras por rayo', '', 1, 1)}
          </div>
          <button type="button" className="link" onClick={() => setAdvanced((a) => !a)}>
            {advanced ? '− Ocultar avanzado' : '+ Avanzado'}
          </button>
          {advanced && (
            <div className="advanced">
              <label className="field">
                <span>Bordes de capa (in4 … out4, Å)</span>
                <input type="text" className="mono" value={edges} onChange={(e) => setEdges(e.target.value)} />
              </label>
              <label className="field">
                <span>Media anchura SES (vacío = media separación entre muestras)</span>
                <div className="input-unit">
                  <input
                    type="number"
                    step={0.01}
                    min={0}
                    value={params.ses_half_width ?? ''}
                    onChange={(e) =>
                      setParams({ ...params, ses_half_width: e.target.value === '' ? null : Number(e.target.value) })
                    }
                  />
                  <em>Å</em>
                </div>
              </label>
              <label className="field">
                <span>Tipos de cara MSMS excluidos</span>
                <input
                  type="text"
                  className="mono"
                  value={params.skip_face_types.join(', ')}
                  onChange={(e) =>
                    setParams({
                      ...params,
                      skip_face_types: e.target.value
                        .split(/[,\s]+/)
                        .filter(Boolean)
                        .map(Number)
                        .filter(Number.isInteger),
                    })
                  }
                />
              </label>
            </div>
          )}
        </>
      )}
      {mode === 'legacy' && (
        <label className="check">
          <input type="checkbox" checked={intermediate} onChange={(e) => setIntermediate(e.target.checked)} />
          Guardar también los 10 archivos intermedios de Script04 (grandes)
        </label>
      )}

      {error && <p className="error">{error}</p>}

      <button className="primary" type="submit" disabled={!ready || busy}>
        {busy ? 'Enviando…' : 'Calcular'}
      </button>

      {samples.length > 0 && (
        <div className="samples">
          <span>Probar con una muestra:</span>
          {samples.map((s) => (
            <button
              key={s.name}
              type="button"
              className="ghost small mono"
              disabled={busy || (mode === 'legacy' && !s.has_stl)}
              onClick={() => runSample(s)}
            >
              {s.name}
            </button>
          ))}
        </div>
      )}
    </form>
  )
}

function FileSlot({ ext, file, optional }: { ext: string; file?: File; optional?: boolean }) {
  return (
    <li className={file ? 'ok' : optional ? 'opt' : ''}>
      <b>{ext}</b>
      <span>{file ? file.name : optional ? 'opcional' : 'falta'}</span>
    </li>
  )
}
