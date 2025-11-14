import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { BrowserRouter } from 'react-router-dom'
import { useLogin, useRegister, useLogout } from '../useAuth'

// Mock dependencies
vi.mock('@/services/api', () => ({
  authApi: {
    login: vi.fn(),
    register: vi.fn(),
  },
  getErrorMessage: vi.fn((error) => error.message),
}))

vi.mock('@/services/socket', () => ({
  socketService: {
    connect: vi.fn(),
    disconnect: vi.fn(),
  },
}))

vi.mock('@/utils/toast', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}))

vi.mock('@/store/authStore', () => ({
  useAuthStore: vi.fn((selector) => {
    const store = {
      setAuth: vi.fn(),
      logout: vi.fn(),
    }
    return selector(store)
  }),
}))

// Wrapper for providers
const createWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  })
  
  return ({ children }: { children: React.ReactNode }) => (
    <BrowserRouter>
      <QueryClientProvider client={queryClient}>
        {children}
      </QueryClientProvider>
    </BrowserRouter>
  )
}

describe('useAuth hooks', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('useLogin', () => {
    it('should be defined', () => {
      const { result } = renderHook(() => useLogin(), {
        wrapper: createWrapper(),
      })
      expect(result.current).toBeDefined()
    })

    it('should have mutate function', () => {
      const { result } = renderHook(() => useLogin(), {
        wrapper: createWrapper(),
      })
      expect(typeof result.current.mutate).toBe('function')
    })
  })

  describe('useRegister', () => {
    it('should be defined', () => {
      const { result } = renderHook(() => useRegister(), {
        wrapper: createWrapper(),
      })
      expect(result.current).toBeDefined()
    })

    it('should have mutate function', () => {
      const { result } = renderHook(() => useRegister(), {
        wrapper: createWrapper(),
      })
      expect(typeof result.current.mutate).toBe('function')
    })
  })

  describe('useLogout', () => {
    it('should be defined', () => {
      const { result } = renderHook(() => useLogout())
      expect(result.current).toBeDefined()
    })

    it('should return a function', () => {
      const { result } = renderHook(() => useLogout())
      expect(typeof result.current).toBe('function')
    })
  })
})
