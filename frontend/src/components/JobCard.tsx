import { Job, JobStatus } from '@/types'
import { Card, CardContent, CardHeader, CardTitle } from './ui/Card'
import { Badge } from './ui/Badge'
import { Progress } from './ui/Progress'
import { Button } from './ui/Button'
import { formatRelativeTime, formatDuration } from '@/utils/format'
import { Clock, Calendar, AlertCircle, CheckCircle2, XCircle } from 'lucide-react'
import { useCancelJob } from '@/hooks/useJobs'

interface JobCardProps {
  job: Job
}

export function JobCard({ job }: JobCardProps) {
  const cancelJob = useCancelJob()

  const getStatusBadge = (status: JobStatus) => {
    switch (status) {
      case JobStatus.PENDING:
        return <Badge variant="warning">Pendiente</Badge>
      case JobStatus.PROCESSING:
        return <Badge variant="info">Procesando</Badge>
      case JobStatus.COMPLETED:
        return <Badge variant="success">Completado</Badge>
      case JobStatus.FAILED:
        return <Badge variant="error">Fallido</Badge>
      case JobStatus.CANCELLED:
        return <Badge variant="default">Cancelado</Badge>
    }
  }

  const getStatusIcon = (status: JobStatus) => {
    switch (status) {
      case JobStatus.COMPLETED:
        return <CheckCircle2 className="h-5 w-5 text-green-600" />
      case JobStatus.FAILED:
        return <XCircle className="h-5 w-5 text-red-600" />
      case JobStatus.PROCESSING:
        return <Clock className="h-5 w-5 text-blue-600 animate-spin" />
      default:
        return <Clock className="h-5 w-5 text-gray-400" />
    }
  }

  const canCancel = job.status === JobStatus.PENDING || job.status === JobStatus.PROCESSING

  return (
    <Card className="hover:shadow-md transition-shadow">
      <CardHeader>
        <div className="flex items-start justify-between">
          <div className="flex items-center space-x-3">
            {getStatusIcon(job.status)}
            <div>
              <CardTitle>Job #{job.id}</CardTitle>
              <p className="text-sm text-gray-500 mt-1">
                {job.job_type === 'part_one' ? 'Parte 1: Context Rays' : 'Parte 2: Layer Evaluation'}
              </p>
            </div>
          </div>
          {getStatusBadge(job.status)}
        </div>
      </CardHeader>

      <CardContent>
        {/* Progress */}
        {(job.status === JobStatus.PROCESSING || job.status === JobStatus.PENDING) && (
          <div className="mb-4">
            <Progress value={job.progress} variant={job.progress === 100 ? 'success' : 'default'} />
          </div>
        )}

        {/* Job Info */}
        <div className="grid grid-cols-2 gap-4 text-sm">
          <div className="flex items-center space-x-2 text-gray-600">
            <Calendar className="h-4 w-4" />
            <span>{formatRelativeTime(job.created_at)}</span>
          </div>

          {job.processing_time_seconds && (
            <div className="flex items-center space-x-2 text-gray-600">
              <Clock className="h-4 w-4" />
              <span>{formatDuration(job.processing_time_seconds)}</span>
            </div>
          )}

          {job.protein_id && (
            <div className="col-span-2">
              <span className="text-gray-600">Proteína ID: </span>
              <span className="font-medium">#{job.protein_id}</span>
            </div>
          )}
        </div>

        {/* Error Message */}
        {job.error_message && (
          <div className="mt-4 p-3 bg-red-50 border border-red-200 rounded-lg flex items-start space-x-2">
            <AlertCircle className="h-5 w-5 text-red-600 flex-shrink-0 mt-0.5" />
            <div>
              <p className="text-sm font-medium text-red-800">Error</p>
              <p className="text-sm text-red-700">{job.error_message}</p>
            </div>
          </div>
        )}

        {/* Actions */}
        {canCancel && (
          <div className="mt-4 pt-4 border-t border-gray-200">
            <Button
              variant="danger"
              size="sm"
              onClick={() => cancelJob.mutate(job.id)}
              isLoading={cancelJob.isPending}
            >
              Cancelar Job
            </Button>
          </div>
        )}

        {/* Download Results */}
        {job.status === JobStatus.COMPLETED && job.output_files.length > 0 && (
          <div className="mt-4 pt-4 border-t border-gray-200">
            <p className="text-sm font-medium text-gray-700 mb-2">
              Archivos de salida ({job.output_files.length})
            </p>
            <div className="flex flex-wrap gap-2">
              {job.output_files.slice(0, 3).map((file, index) => (
                <Badge key={index} variant="default">
                  {file.split('/').pop()}
                </Badge>
              ))}
              {job.output_files.length > 3 && (
                <Badge variant="default">+{job.output_files.length - 3} más</Badge>
              )}
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  )
}
