import React, { Component, ErrorInfo, ReactNode } from 'react'
import { Button } from './ui/Button'

interface Props {
  children: ReactNode
  fallback?: ReactNode
}

interface State {
  hasError: boolean
  error: Error | null
  errorInfo: ErrorInfo | null
}

/**
 * Error Boundary Component
 *
 * Catches JavaScript errors anywhere in the child component tree,
 * logs those errors, and displays a fallback UI instead of crashing the whole app.
 *
 * Usage:
 *   <ErrorBoundary>
 *     <YourComponent />
 *   </ErrorBoundary>
 */
export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props)
    this.state = {
      hasError: false,
      error: null,
      errorInfo: null,
    }
  }

  static getDerivedStateFromError(error: Error): Partial<State> {
    // Update state so the next render will show the fallback UI
    return { hasError: true, error }
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo): void {
    // Log error to console in development
    if (import.meta.env.DEV) {
      console.error('ErrorBoundary caught an error:', error)
      console.error('Error info:', errorInfo)
      console.error('Component stack:', errorInfo.componentStack)
    }

    // In production, you might want to log to an error reporting service
    // Example: logErrorToService(error, errorInfo)

    this.setState({
      error,
      errorInfo,
    })
  }

  handleReset = (): void => {
    this.setState({
      hasError: false,
      error: null,
      errorInfo: null,
    })
  }

  handleReload = (): void => {
    window.location.reload()
  }

  render(): ReactNode {
    const { hasError, error, errorInfo } = this.state
    const { children, fallback } = this.props

    if (hasError) {
      // Custom fallback UI provided
      if (fallback) {
        return fallback
      }

      // Default fallback UI
      return (
        <div className="min-h-screen flex items-center justify-center bg-gray-50 px-4">
          <div className="max-w-2xl w-full bg-white rounded-lg shadow-lg p-8">
            {/* Error Icon */}
            <div className="flex justify-center mb-6">
              <div className="w-16 h-16 bg-red-100 rounded-full flex items-center justify-center">
                <svg
                  className="w-8 h-8 text-red-600"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
                  />
                </svg>
              </div>
            </div>

            {/* Error Title */}
            <h1 className="text-3xl font-bold text-gray-900 text-center mb-4">
              Oops! Algo salió mal
            </h1>

            {/* Error Message */}
            <p className="text-gray-600 text-center mb-8">
              Ha ocurrido un error inesperado. Puedes intentar recargar la página o contactar
              soporte si el problema persiste.
            </p>

            {/* Error Details (Development Only) */}
            {import.meta.env.DEV && error && (
              <details className="mb-6 bg-gray-50 rounded-lg p-4">
                <summary className="cursor-pointer font-semibold text-gray-700 hover:text-gray-900">
                  Detalles del error (solo en desarrollo)
                </summary>
                <div className="mt-4 space-y-4">
                  <div>
                    <h3 className="font-semibold text-red-600 mb-2">Error:</h3>
                    <pre className="bg-red-50 text-red-800 p-3 rounded text-sm overflow-x-auto">
                      {error.toString()}
                    </pre>
                  </div>
                  {errorInfo && (
                    <div>
                      <h3 className="font-semibold text-red-600 mb-2">Stack trace:</h3>
                      <pre className="bg-red-50 text-red-800 p-3 rounded text-sm overflow-x-auto max-h-64 overflow-y-auto">
                        {errorInfo.componentStack}
                      </pre>
                    </div>
                  )}
                </div>
              </details>
            )}

            {/* Action Buttons */}
            <div className="flex flex-col sm:flex-row gap-4 justify-center">
              <Button
                onClick={this.handleReset}
                variant="outline"
                className="w-full sm:w-auto"
              >
                Intentar de nuevo
              </Button>
              <Button onClick={this.handleReload} className="w-full sm:w-auto">
                Recargar página
              </Button>
            </div>

            {/* Help Text */}
            <div className="mt-8 pt-6 border-t border-gray-200">
              <p className="text-sm text-gray-500 text-center">
                Si el problema continúa, por favor contacta a{' '}
                <a
                  href="mailto:support@proteindocking.com"
                  className="text-blue-600 hover:text-blue-800 underline"
                >
                  soporte técnico
                </a>
              </p>
            </div>
          </div>
        </div>
      )
    }

    return children
  }
}

/**
 * Hook-based error boundary wrapper for functional components
 *
 * Usage:
 *   <ErrorBoundaryWrapper>
 *     <YourFunctionalComponent />
 *   </ErrorBoundaryWrapper>
 */
export const ErrorBoundaryWrapper: React.FC<{ children: ReactNode; fallback?: ReactNode }> = ({
  children,
  fallback,
}) => {
  return <ErrorBoundary fallback={fallback}>{children}</ErrorBoundary>
}
