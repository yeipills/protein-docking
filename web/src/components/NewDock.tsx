import { useRef, useState } from 'react'
import { api, type Complex, type Job } from '../api'

interface Props {
  complexes: Complex[]
  onCreated: (job: Job) => void
}

type Slot = 'receptor' | 'ligand' | 'native_receptor' | 'native_ligand'

const SLOTS: { key: Slot; label: string; optional?: boolean }[] = [
  { key: 'receptor', label: 'Receptor' },
  { key: 'ligand', label: 'Ligando' },
  { key: 'native_receptor', label: 'Receptor unido', optional: true },
  { key: 'native_ligand', label: 'Ligando unido', optional: true },
]

export function NewDock({ complexes, onCreated }: Props) {
  const [files, setFiles] = useState<Partial<Record<Slot, File>>>({})
  const [name, setName] = useState('')
  const [surface, setSurface] = useState<'atoms' | 'ses'>('atoms')
  const [randomize, setRandomize] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const nativeHalf = !!files.native_receptor !== !!files.native_ligand
  const ready = files.receptor && files.ligand && !nativeHalf

  async function submit(e: React.FormEvent) {
    e.preventDefault()
    if (!ready) return
    setBusy(true)
    setError(null)
    try {
      const form = new FormData()
      for (const s of SLOTS) if (files[s.key]) form.set(s.key, files[s.key]!)
      form.set('surface', surface)
      if (name.trim()) form.set('name', name.trim())
      if (randomize) form.set('randomize_seed', '1')
      const job = await api.createDock(form)
      setFiles({})
      setName('')
      onCreated(job)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setBusy(false)
    }
  }

  async function runComplex(c: Complex) {
    setBusy(true)
    setError(null)
    try {
      onCreated(await api.createComplexDock(c.name, { surface }))
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setBusy(false)
    }
  }

  return (
    <form className="panel new-job" onSubmit={submit}>
      <header className="panel-head">
        <h2>Nuevo docking</h2>
      </header>
      <p className="note">
        Acoplamiento rígido por complementariedad de formas de contexto. Con el complejo unido (opcional) cada pose se
        evalúa con CAPRI y DockQ.
      </p>
      <ul className="pdb-slots">
        {SLOTS.map((s) => (
          <PdbSlot
            key={s.key}
            label={s.label}
            optional={s.optional}
            file={files[s.key]}
            onFile={(f) => setFiles((prev) => ({ ...prev, [s.key]: f }))}
          />
        ))}
      </ul>
      {nativeHalf && <p className="error">El complejo unido necesita receptor y ligando.</p>}

      <label className="field">
        <span>Nombre</span>
        <input
          type="text"
          placeholder={files.receptor && files.ligand ? `${stem(files.receptor)}+${stem(files.ligand)}` : 'docking'}
          value={name}
          onChange={(e) => setName(e.target.value)}
        />
      </label>

      <div className="field">
        <span>Superficie</span>
        <div className="segmented small">
          <button type="button" className={surface === 'atoms' ? 'on' : ''} onClick={() => setSurface('atoms')}>
            Átomos (recomendada)
          </button>
          <button type="button" className={surface === 'ses' ? 'on' : ''} onClick={() => setSurface('ses')}>
            SES
          </button>
        </div>
      </div>
      <label className="check">
        <input type="checkbox" checked={randomize} onChange={(e) => setRandomize(e.target.checked)} />
        Partir de una posición aleatoria del ligando (si ya viene superpuesto al complejo)
      </label>

      {error && <p className="error">{error}</p>}

      <button className="primary" type="submit" disabled={!ready || busy}>
        {busy ? 'Enviando…' : 'Acoplar'}
      </button>

      {complexes.length > 0 && (
        <div className="samples">
          <span>Probar con un complejo:</span>
          {complexes.map((c) => (
            <button key={c.name} type="button" className="ghost small mono" disabled={busy} onClick={() => runComplex(c)}>
              {c.name}
            </button>
          ))}
        </div>
      )}
    </form>
  )
}

function stem(f: File) {
  return f.name.replace(/\.[^.]+$/, '')
}

function PdbSlot({
  label,
  optional,
  file,
  onFile,
}: {
  label: string
  optional?: boolean
  file?: File
  onFile: (f: File | undefined) => void
}) {
  const input = useRef<HTMLInputElement>(null)
  const [over, setOver] = useState(false)
  return (
    <li
      className={`pdb-slot ${file ? 'ok' : ''} ${over ? 'over' : ''}`}
      onClick={() => input.current?.click()}
      onDragOver={(e) => {
        e.preventDefault()
        setOver(true)
      }}
      onDragLeave={() => setOver(false)}
      onDrop={(e) => {
        e.preventDefault()
        setOver(false)
        onFile(e.dataTransfer.files[0])
      }}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => (e.key === 'Enter' || e.key === ' ') && input.current?.click()}
    >
      <input ref={input} type="file" accept=".pdb,.ent" hidden onChange={(e) => onFile(e.target.files?.[0])} />
      <b>{label}</b>
      <span className="mono">{file ? file.name : optional ? 'opcional' : 'arrastra un .pdb'}</span>
      {file && (
        <button
          type="button"
          className="link"
          aria-label={`Quitar ${label}`}
          onClick={(e) => {
            e.stopPropagation()
            onFile(undefined)
          }}
        >
          ×
        </button>
      )}
    </li>
  )
}
