import { useEffect } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { socketService } from '@/services/socket'
import { useAuthStore } from '@/store/authStore'
import type { SocketJobUpdate } from '@/types'
import { toast } from '@/utils/toast'

export const useSocket = () => {
  const queryClient = useQueryClient()
  const accessToken = useAuthStore((state) => state.accessToken)
  const isAuthenticated = useAuthStore((state) => state.isAuthenticated)

  useEffect(() => {
    if (isAuthenticated && accessToken && !socketService.isConnected()) {
      socketService.connect(accessToken)

      socketService.onJobUpdate((data: SocketJobUpdate) => {
        // Invalidate jobs query to refetch
        queryClient.invalidateQueries({ queryKey: ['jobs'] })
        queryClient.invalidateQueries({ queryKey: ['jobs', data.job_id] })

        // Show notification
        if (data.status === 'completed') {
          toast.success(`Job #${data.job_id} completed!`)
        } else if (data.status === 'failed') {
          toast.error(`Job #${data.job_id} failed: ${data.message || 'Unknown error'}`)
        }
      })
    }

    return () => {
      socketService.offJobUpdate()
    }
  }, [isAuthenticated, accessToken, queryClient])
}
