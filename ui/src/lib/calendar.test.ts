import { describe, it, expect } from 'vitest'
import { buildMonthGrid } from './calendar'
import { dayItem } from './fixtures'

describe('buildMonthGrid', () => {
  it('november 2026 grid', () => {
    const g = buildMonthGrid(2026, 11, [{ date: '2026-11-02', items: [] }])
    expect(g.length).toBe(6)
    expect(g[0][0].date).toBe('2026-10-26'); expect(g[0][0].inMonth).toBe(false)
    expect(g[5][0].date).toBe('2026-11-30')
    expect(g.flat().find(c => c.date === '2026-11-02')!.inMonth).toBe(true)
    expect(g.every(w => w.length === 7)).toBe(true)
  })
  it('october 2026 grid has 5 weeks', () => { expect(buildMonthGrid(2026, 10, []).length).toBe(5) })
  it('attaches day items', () => {
    const items = [dayItem({ km: 48, postStatus: 'ready' })]
    const g = buildMonthGrid(2026, 10, [{ date: '2026-10-25', items }])
    expect(g.flat().find(c => c.date === '2026-10-25')!.items).toEqual(items)
  })
})
