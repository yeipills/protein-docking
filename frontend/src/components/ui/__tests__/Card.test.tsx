import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { Card } from '../Card'

describe('Card Component', () => {
  it('renders with children', () => {
    render(<Card>Card content</Card>)
    expect(screen.getByText('Card content')).toBeInTheDocument()
  })

  it('renders with title', () => {
    render(<Card title="Card Title">Content</Card>)
    expect(screen.getByText('Card Title')).toBeInTheDocument()
    expect(screen.getByText('Content')).toBeInTheDocument()
  })

  it('applies custom className', () => {
    const { container } = render(<Card className="custom-card">Content</Card>)
    expect(container.firstChild).toHaveClass('custom-card')
  })

  it('renders without title', () => {
    render(<Card>Just content</Card>)
    expect(screen.getByText('Just content')).toBeInTheDocument()
  })
})
