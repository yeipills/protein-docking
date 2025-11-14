import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { proteinsApi, getErrorMessage } from '@/services/api'
import { toast } from '@/utils/toast'

export const useProteins = () => {
  return useQuery({
    queryKey: ['proteins'],
    queryFn: proteinsApi.getAll,
  })
}

export const useProtein = (id: number) => {
  return useQuery({
    queryKey: ['proteins', id],
    queryFn: () => proteinsApi.getById(id),
    enabled: !!id,
  })
}

export const useUploadProtein = () => {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: ({
      name,
      stlFile,
      verticesFile,
      facesFile,
    }: {
      name: string
      stlFile: File
      verticesFile: File
      facesFile: File
    }) => proteinsApi.upload(name, stlFile, verticesFile, facesFile),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['proteins'] })
      toast.success('Protein uploaded successfully')
    },
    onError: (error) => {
      toast.error(getErrorMessage(error))
    },
  })
}

export const useDeleteProtein = () => {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: (id: number) => proteinsApi.delete(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['proteins'] })
      toast.success('Protein deleted')
    },
    onError: (error) => {
      toast.error(getErrorMessage(error))
    },
  })
}
