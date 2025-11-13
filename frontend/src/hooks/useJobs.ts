import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { jobsApi, getErrorMessage } from '@/services/api'
import type { CreateJobRequest } from '@/types'
import { toast } from '@/utils/toast'

export const useJobs = () => {
  return useQuery({
    queryKey: ['jobs'],
    queryFn: jobsApi.getAll,
    refetchInterval: 5000, // Auto-refetch every 5 seconds
  })
}

export const useJob = (id: number) => {
  return useQuery({
    queryKey: ['jobs', id],
    queryFn: () => jobsApi.getById(id),
    enabled: !!id,
  })
}

export const useCreateJob = () => {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: (data: CreateJobRequest) => jobsApi.create(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['jobs'] })
      toast.success('Job created successfully')
    },
    onError: (error) => {
      toast.error(getErrorMessage(error))
    },
  })
}

export const useCancelJob = () => {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: (id: number) => jobsApi.cancel(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['jobs'] })
      toast.success('Job cancelled')
    },
    onError: (error) => {
      toast.error(getErrorMessage(error))
    },
  })
}
