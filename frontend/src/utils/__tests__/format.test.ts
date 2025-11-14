import { describe, it, expect } from 'vitest'
import { formatDate, formatFileSize, formatDuration } from '../format'

describe('format utilities', () => {
  describe('formatDate', () => {
    it('formats valid date', () => {
      const date = new Date('2025-01-15T10:30:00Z')
      const formatted = formatDate(date)
      expect(formatted).toBeDefined()
      expect(typeof formatted).toBe('string')
    })

    it('handles invalid date', () => {
      const result = formatDate('invalid')
      expect(result).toBe('Invalid Date')
    })
  })

  describe('formatFileSize', () => {
    it('formats bytes correctly', () => {
      expect(formatFileSize(0)).toBe('0 Bytes')
      expect(formatFileSize(1024)).toBe('1 KB')
      expect(formatFileSize(1048576)).toBe('1 MB')
      expect(formatFileSize(1073741824)).toBe('1 GB')
    })

    it('handles negative values', () => {
      expect(formatFileSize(-1024)).toBe('0 Bytes')
    })
  })

  describe('formatDuration', () => {
    it('formats seconds correctly', () => {
      expect(formatDuration(0)).toBe('0s')
      expect(formatDuration(30)).toBe('30s')
      expect(formatDuration(60)).toBe('1m 0s')
      expect(formatDuration(3661)).toBe('1h 1m 1s')
    })
  })
})
