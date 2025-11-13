import { io, Socket } from 'socket.io-client'
import type { SocketJobUpdate } from '@/types'

const SOCKET_URL = import.meta.env.VITE_SOCKET_URL || 'http://localhost:8080'

class SocketService {
  private socket: Socket | null = null
  private connected = false

  connect(token: string) {
    if (this.connected) {
      console.log('Socket already connected')
      return
    }

    this.socket = io(SOCKET_URL, {
      auth: { token },
      transports: ['websocket'],
      reconnection: true,
      reconnectionAttempts: 5,
      reconnectionDelay: 1000,
    })

    this.socket.on('connect', () => {
      console.log('Socket connected')
      this.connected = true
    })

    this.socket.on('disconnect', () => {
      console.log('Socket disconnected')
      this.connected = false
    })

    this.socket.on('connect_error', (error) => {
      console.error('Socket connection error:', error)
      this.connected = false
    })
  }

  disconnect() {
    if (this.socket) {
      this.socket.disconnect()
      this.socket = null
      this.connected = false
    }
  }

  onJobUpdate(callback: (data: SocketJobUpdate) => void) {
    if (!this.socket) {
      console.warn('Socket not connected')
      return
    }

    this.socket.on('job_update', callback)
  }

  offJobUpdate() {
    if (this.socket) {
      this.socket.off('job_update')
    }
  }

  isConnected() {
    return this.connected
  }
}

export const socketService = new SocketService()
