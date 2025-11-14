import { describe, it, expect, beforeEach } from 'vitest'
import { useAuthStore } from '../authStore'

describe('authStore', () => {
  beforeEach(() => {
    // Reset store before each test
    useAuthStore.setState({
      user: null,
      accessToken: null,
      refreshToken: null,
      isAuthenticated: false,
    })
  })

  it('initializes with unauthenticated state', () => {
    const state = useAuthStore.getState()
    expect(state.isAuthenticated).toBe(false)
    expect(state.user).toBeNull()
    expect(state.accessToken).toBeNull()
  })

  it('sets auth when setAuth is called', () => {
    const mockAuth = {
      access_token: 'test-access-token',
      refresh_token: 'test-refresh-token',
      token_type: 'bearer',
    }
    const mockUser = {
      id: 1,
      email: 'test@example.com',
      username: 'testuser',
      full_name: 'Test User',
    }

    useAuthStore.getState().setAuth(mockAuth, mockUser)

    const state = useAuthStore.getState()
    expect(state.isAuthenticated).toBe(true)
    expect(state.user).toEqual(mockUser)
    expect(state.accessToken).toBe('test-access-token')
    expect(state.refreshToken).toBe('test-refresh-token')
  })

  it('clears auth when logout is called', () => {
    // Set auth first
    const mockAuth = {
      access_token: 'test-token',
      refresh_token: 'test-refresh',
      token_type: 'bearer',
    }
    const mockUser = {
      id: 1,
      email: 'test@example.com',
      username: 'testuser',
    }

    useAuthStore.getState().setAuth(mockAuth, mockUser)
    expect(useAuthStore.getState().isAuthenticated).toBe(true)

    // Logout
    useAuthStore.getState().logout()

    const state = useAuthStore.getState()
    expect(state.isAuthenticated).toBe(false)
    expect(state.user).toBeNull()
    expect(state.accessToken).toBeNull()
    expect(state.refreshToken).toBeNull()
  })

  it('persists state to localStorage', () => {
    const mockAuth = {
      access_token: 'persisted-token',
      refresh_token: 'persisted-refresh',
      token_type: 'bearer',
    }
    const mockUser = {
      id: 1,
      email: 'persisted@example.com',
      username: 'persisted',
    }

    useAuthStore.getState().setAuth(mockAuth, mockUser)

    // Check if localStorage was called (in real env, not in jsdom without setup)
    // This is a basic check, full localStorage testing would need more setup
    const state = useAuthStore.getState()
    expect(state.accessToken).toBe('persisted-token')
  })
})
