// @vitest-environment happy-dom
import { describe, expect, it } from 'vitest'
import { onThemeApplied } from './theme'

describe('onThemeApplied', () => {
  it('calls back only after the dark class has actually changed', async () => {
    const seen: boolean[] = []
    const off = onThemeApplied(() => seen.push(document.documentElement.classList.contains('dark')))
    document.documentElement.classList.add('dark')
    await new Promise(r => setTimeout(r, 0))
    document.documentElement.classList.add('other')
    await new Promise(r => setTimeout(r, 0))
    document.documentElement.classList.remove('dark')
    await new Promise(r => setTimeout(r, 0))
    off()
    document.documentElement.classList.add('dark')
    await new Promise(r => setTimeout(r, 0))
    expect(seen).toEqual([true, false])
  })
})
