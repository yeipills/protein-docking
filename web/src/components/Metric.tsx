export function Metric({ label, value, accent }: { label: string; value: string; accent?: boolean }) {
  return (
    <div className={accent ? 'metric accent' : 'metric'}>
      <dt>{label}</dt>
      <dd className="mono">{value}</dd>
    </div>
  )
}
