import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { ErrorBoundary } from '../ErrorBoundary'

// Component that throws an error
const ThrowError = () => {
  throw new Error('Test error')
}

const SafeComponent = () => <div>Safe content</div>

describe('ErrorBoundary', () => {
  it('renders children when no error', () => {
    render(
      <ErrorBoundary>
        <SafeComponent />
      </ErrorBoundary>
    )
    expect(screen.getByText('Safe content')).toBeInTheDocument()
  })

  it('catches errors and displays fallback UI', () => {
    // Suppress console.error for this test
    const consoleError = console.error
    console.error = () => {}

    render(
      <ErrorBoundary>
        <ThrowError />
      </ErrorBoundary>
    )

    expect(screen.getByText(/something went wrong/i)).toBeInTheDocument()

    // Restore console.error
    console.error = consoleError
  })
})
