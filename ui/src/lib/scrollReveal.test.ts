// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from 'vitest'
import { installScrollReveal } from './scrollReveal'

describe('installScrollReveal', () => {
  afterEach(() => vi.useRealTimers())
  it('marks the scrolled element while scrolling and clears it after a pause', () => {
    vi.useFakeTimers()
    const box = document.createElement('div')
    document.body.append(box)
    const off = installScrollReveal(document, 800)
    box.dispatchEvent(new Event('scroll'))
    expect(box.hasAttribute('data-scrolling')).toBe(true)
    vi.advanceTimersByTime(500)
    box.dispatchEvent(new Event('scroll'))
    vi.advanceTimersByTime(500)
    expect(box.hasAttribute('data-scrolling')).toBe(true)
    vi.advanceTimersByTime(400)
    expect(box.hasAttribute('data-scrolling')).toBe(false)
    off()
    box.dispatchEvent(new Event('scroll'))
    expect(box.hasAttribute('data-scrolling')).toBe(false)
  })
})
