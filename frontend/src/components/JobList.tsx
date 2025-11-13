import { useJobs } from '@/hooks/useJobs'
import { JobCard } from './JobCard'
import { Loader2, Inbox } from 'lucide-react'

export function JobList() {
  const { data: jobs, isLoading, error } = useJobs()

  if (isLoading) {
    return (
      <div className="flex flex-col items-center justify-center py-12">
        <Loader2 className="h-8 w-8 text-primary-600 animate-spin mb-4" />
        <p className="text-gray-600">Cargando trabajos...</p>
      </div>
    )
  }

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center py-12">
        <p className="text-red-600">Error al cargar trabajos</p>
        <p className="text-sm text-gray-500 mt-2">{error.message}</p>
      </div>
    )
  }

  if (!jobs || jobs.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-12 bg-white rounded-lg border-2 border-dashed border-gray-300">
        <Inbox className="h-12 w-12 text-gray-400 mb-4" />
        <p className="text-gray-600 font-medium">No tienes trabajos aún</p>
        <p className="text-sm text-gray-500 mt-1">
          Sube una proteína para empezar a procesar
        </p>
      </div>
    )
  }

  return (
    <div className="grid gap-4">
      {jobs.map((job) => (
        <JobCard key={job.id} job={job} />
      ))}
    </div>
  )
}
