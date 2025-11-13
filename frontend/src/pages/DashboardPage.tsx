import { Link } from 'react-router-dom'
import { Plus } from 'lucide-react'
import { MainLayout } from '@/components/layout/MainLayout'
import { Button } from '@/components/ui/Button'
import { JobList } from '@/components/JobList'
import { useJobs } from '@/hooks/useJobs'

export function DashboardPage() {
  const { data: jobs, isLoading, error } = useJobs()

  return (
    <MainLayout>
      <div className="space-y-6">
        {/* Header */}
        <div className="flex justify-between items-center">
          <div>
            <h1 className="text-3xl font-bold text-gray-900">
              Dashboard
            </h1>
            <p className="text-gray-600 mt-1">
              Gestiona tus trabajos de protein docking
            </p>
          </div>
          <Link to="/upload">
            <Button variant="primary" size="lg">
              <Plus className="h-5 w-5 mr-2" />
              Nueva Proteína
            </Button>
          </Link>
        </div>

        {/* Stats */}
        {jobs && jobs.length > 0 && (
          <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
            <StatCard
              label="Total"
              value={jobs.length}
              variant="default"
            />
            <StatCard
              label="En Proceso"
              value={jobs.filter(j => j.status === 'processing').length}
              variant="info"
            />
            <StatCard
              label="Completados"
              value={jobs.filter(j => j.status === 'completed').length}
              variant="success"
            />
            <StatCard
              label="Fallidos"
              value={jobs.filter(j => j.status === 'failed').length}
              variant="danger"
            />
          </div>
        )}

        {/* Job List */}
        {isLoading ? (
          <div className="flex justify-center items-center py-12">
            <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600"></div>
          </div>
        ) : error ? (
          <div className="bg-red-50 border border-red-200 rounded-lg p-4 text-center">
            <p className="text-red-800">Error al cargar los trabajos</p>
            <p className="text-red-600 text-sm mt-1">
              {error instanceof Error ? error.message : 'Error desconocido'}
            </p>
          </div>
        ) : (
          <JobList />
        )}
      </div>
    </MainLayout>
  )
}

interface StatCardProps {
  label: string
  value: number
  variant: 'default' | 'info' | 'success' | 'danger'
}

function StatCard({ label, value, variant }: StatCardProps) {
  const colors = {
    default: 'bg-gray-50 border-gray-200 text-gray-900',
    info: 'bg-blue-50 border-blue-200 text-blue-900',
    success: 'bg-green-50 border-green-200 text-green-900',
    danger: 'bg-red-50 border-red-200 text-red-900',
  }

  return (
    <div className={`border rounded-lg p-6 ${colors[variant]}`}>
      <p className="text-sm font-medium opacity-80">{label}</p>
      <p className="text-3xl font-bold mt-2">{value}</p>
    </div>
  )
}
