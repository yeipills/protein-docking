import axios, { type AxiosError } from 'axios'
import type {
  User,
  LoginRequest,
  RegisterRequest,
  AuthResponse,
  Job,
  Protein,
  CreateJobRequest,
  ApiError,
} from '@/types'
import { useAuthStore } from '@/store/authStore'

const API_URL = import.meta.env.VITE_API_URL || '/api/v1'

// Create axios instance
export const api = axios.create({
  baseURL: API_URL,
  headers: {
    'Content-Type': 'application/json',
  },
})

// Request interceptor to add auth token
api.interceptors.request.use(
  (config) => {
    const token = useAuthStore.getState().accessToken
    if (token) {
      config.headers.Authorization = `Bearer ${token}`
    }
    return config
  },
  (error) => Promise.reject(error)
)

// Response interceptor to handle token refresh
api.interceptors.response.use(
  (response) => response,
  async (error: AxiosError<ApiError>) => {
    const originalRequest = error.config

    if (error.response?.status === 401 && originalRequest && !originalRequest.headers['X-Retry']) {
      // Try to refresh token
      const refreshToken = useAuthStore.getState().refreshToken

      if (refreshToken) {
        try {
          const response = await axios.post<AuthResponse>(
            `${API_URL}/auth/refresh`,
            {},
            {
              headers: {
                Authorization: `Bearer ${refreshToken}`,
              },
            }
          )

          const { access_token } = response.data
          const user = useAuthStore.getState().user

          if (user) {
            useAuthStore.getState().setAuth(response.data, user)
          }

          // Retry original request
          if (originalRequest.headers) {
            originalRequest.headers.Authorization = `Bearer ${access_token}`
            originalRequest.headers['X-Retry'] = 'true'
          }

          return api(originalRequest)
        } catch (refreshError) {
          // Refresh failed, logout user
          useAuthStore.getState().logout()
          window.location.href = '/login'
          return Promise.reject(refreshError)
        }
      }
    }

    return Promise.reject(error)
  }
)

// Auth endpoints
export const authApi = {
  login: async (credentials: LoginRequest): Promise<{ auth: AuthResponse; user: User }> => {
    const formData = new FormData()
    formData.append('username', credentials.username)
    formData.append('password', credentials.password)

    const response = await api.post<AuthResponse>('/auth/login', formData, {
      headers: { 'Content-Type': 'multipart/form-data' },
    })

    // Get user data
    const userResponse = await api.get<User>('/users/me', {
      headers: { Authorization: `Bearer ${response.data.access_token}` },
    })

    return { auth: response.data, user: userResponse.data }
  },

  register: async (data: RegisterRequest): Promise<User> => {
    const response = await api.post<User>('/auth/register', data)
    return response.data
  },

  getMe: async (): Promise<User> => {
    const response = await api.get<User>('/users/me')
    return response.data
  },
}

// Jobs endpoints
export const jobsApi = {
  getAll: async (): Promise<Job[]> => {
    const response = await api.get<Job[]>('/jobs/')
    return response.data
  },

  getById: async (id: number): Promise<Job> => {
    const response = await api.get<Job>(`/jobs/${id}`)
    return response.data
  },

  create: async (data: CreateJobRequest): Promise<Job> => {
    const response = await api.post<Job>('/jobs/', data)
    return response.data
  },

  cancel: async (id: number): Promise<Job> => {
    const response = await api.post<Job>(`/jobs/${id}/cancel`)
    return response.data
  },
}

// Proteins endpoints
export const proteinsApi = {
  getAll: async (): Promise<Protein[]> => {
    const response = await api.get<Protein[]>('/proteins/')
    return response.data
  },

  getById: async (id: number): Promise<Protein> => {
    const response = await api.get<Protein>(`/proteins/${id}`)
    return response.data
  },

  upload: async (
    name: string,
    stlFile: File,
    verticesFile: File,
    facesFile: File
  ): Promise<Protein> => {
    const formData = new FormData()
    formData.append('name', name)
    formData.append('stl_file', stlFile)
    formData.append('vertices_file', verticesFile)
    formData.append('faces_file', facesFile)

    const response = await api.post<Protein>('/proteins/', formData, {
      headers: { 'Content-Type': 'multipart/form-data' },
    })

    return response.data
  },

  delete: async (id: number): Promise<void> => {
    await api.delete(`/proteins/${id}`)
  },
}

// Helper to handle API errors
export const getErrorMessage = (error: unknown): string => {
  if (axios.isAxiosError(error)) {
    const axiosError = error as AxiosError<ApiError>
    return axiosError.response?.data?.detail || axiosError.message
  }
  return 'An unexpected error occurred'
}
