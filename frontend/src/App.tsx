import { Routes, Route, Navigate } from 'react-router-dom'
import { lazy, Suspense } from 'react'
import { useAuthStore } from '@/store/authStore'
import { useSocket } from '@/hooks/useSocket'

// Eager load landing page (first paint)
import { LandingPage } from '@/pages/LandingPage'

// Lazy load other pages (code splitting)
const LoginPage = lazy(() => import('@/pages/LoginPage').then(m => ({ default: m.LoginPage })))
const RegisterPage = lazy(() => import('@/pages/RegisterPage').then(m => ({ default: m.RegisterPage })))
const DashboardPage = lazy(() => import('@/pages/DashboardPage').then(m => ({ default: m.DashboardPage })))
const UploadPage = lazy(() => import('@/pages/UploadPage').then(m => ({ default: m.UploadPage })))

// Loading fallback component
const PageLoader = () => (
  <div className="flex items-center justify-center min-h-screen">
    <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600" />
  </div>
)

function App() {
  const isAuthenticated = useAuthStore((state) => state.isAuthenticated)

  // Initialize WebSocket connection for real-time updates
  useSocket()

  return (
    <Suspense fallback={<PageLoader />}>
      <Routes>
        {/* Public Routes */}
        <Route path="/" element={<LandingPage />} />
        <Route
          path="/login"
          element={isAuthenticated ? <Navigate to="/dashboard" replace /> : <LoginPage />}
        />
        <Route
          path="/register"
          element={isAuthenticated ? <Navigate to="/dashboard" replace /> : <RegisterPage />}
        />

        {/* Protected Routes */}
        <Route
          path="/dashboard"
          element={isAuthenticated ? <DashboardPage /> : <Navigate to="/login" replace />}
        />
        <Route
          path="/upload"
          element={isAuthenticated ? <UploadPage /> : <Navigate to="/login" replace />}
        />

        {/* 404 - Redirect to landing */}
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </Suspense>
  )
}

export default App
