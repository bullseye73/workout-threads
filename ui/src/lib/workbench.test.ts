import { describe, expect, it } from 'vitest'
import type { DayDto } from './bindings/DayDto'
import type { MonthView } from './bindings/MonthView'
import { dayItem } from './fixtures'
import { dayLabel, filterGroups, goalProgress, groupByWeek, itemSummary, pickAfterImport, pickSelection, ribbonDays } from './workbench'

const NOV: DayDto[] = [
  { date: '2026-11-02', items: [dayItem({ activityId: 'r2', sport: 'running', km: 8.1, postStatus: 'ready' })] },
  { date: '2026-11-03', items: [dayItem({ activityId: 'c3', km: 59.4, postStatus: 'published' })] },
  { date: '2026-11-08', items: [dayItem({ activityId: 'c8', km: 42.6, postStatus: 'ready' }), dayItem({ activityId: 'r8', sport: 'running', km: 3 })] },
  { date: '2026-11-13', items: [dayItem({ activityId: 'r13', sport: 'running', km: 5.2 })] },
]

describe('ribbonDays', () => {
  const r = ribbonDays(2026, 11, NOV, '2026-11-14')
  it('has one cell per day of the month', () => { expect(r).toHaveLength(30) })
  it('marks weekends (11/1 is a Sunday)', () => {
    expect(r[0].weekend).toBe(true)
    expect(r[1].weekend).toBe(false)
  })
  it('marks only today', () => { expect(r.filter(d => d.today).map(d => d.day)).toEqual([14]) })
  it('keeps every activity of a day', () => { expect(r[7].items.map(i => i.activityId)).toEqual(['c8', 'r8']) })
})

describe('groupByWeek', () => {
  const g = groupByWeek(NOV)
  it('groups by Monday weeks, newest first', () => {
    expect(g.map(w => w.label)).toEqual(['11/9–11/15', '11/2–11/8'])
    expect(g[0].weekStart).toBe('2026-11-09')
  })
  it('orders items newest first and sums km', () => {
    expect(g[0].items.map(i => i.date)).toEqual(['2026-11-13'])
    expect(g[1].items.map(i => i.activityId)).toEqual(['r8', 'c8', 'c3', 'r2'])
    expect(g[1].km).toBeCloseTo(8.1 + 59.4 + 42.6 + 3)
  })
})

describe('filterGroups', () => {
  it('drops weeks with nothing left and re-sums km', () => {
    const f = filterGroups(groupByWeek(NOV), 'ready')
    expect(f.map(w => w.label)).toEqual(['11/2–11/8'])
    expect(f[0].km).toBeCloseTo(8.1 + 42.6)
  })
  it('all keeps everything', () => { expect(filterGroups(groupByWeek(NOV), 'all')).toHaveLength(2) })
})

describe('pickSelection', () => {
  const g = groupByWeek(NOV)
  it('keeps a current selection that is still listed', () => { expect(pickSelection(g, 'c3')).toBe('c3') })
  it('falls back to the newest activity', () => { expect(pickSelection(g, 'gone')).toBe('r13') })
  it('is null for an empty month', () => { expect(pickSelection([], 'c3')).toBeNull() })
})

describe('goalProgress', () => {
  const sport = (km: number) => ({ count: 1, km, movingS: 0, avgPace: null, avgSpeedKmh: null })
  const view = (goalKm: number | null, month = 11): MonthView => ({ year: 2026, month, goalKm, totalKm: 110.1, totalMovingS: 0,
    running: sport(8.1), cycling: sport(102), otherCount: 0, avgHr: null, weekly: [], days: [] })
  const today = new Date(2026, 10, 14)
  it('splits the bar by sport and paces against days gone by', () => {
    const p = goalProgress(view(300), today)
    expect(p.rideRatio).toBeCloseTo(102 / 300)
    expect(p.runRatio).toBeCloseTo(8.1 / 300)
    expect(p.paceRatio).toBeCloseTo(13 / 30)
    expect(p.perDayKm).toBeCloseTo((300 - 110.1) / 17)
  })
  it('has no pace without a goal', () => {
    const p = goalProgress(view(null), today)
    expect([p.rideRatio, p.runRatio, p.paceRatio, p.perDayKm]).toEqual([0, 0, null, null])
  })
  it('a past month is fully paced and needs nothing per day', () => {
    const p = goalProgress(view(300, 10), today)
    expect(p.paceRatio).toBe(1)
    expect(p.perDayKm).toBeNull()
  })
  it('scales down when over the goal', () => {
    const p = goalProgress(view(100), today)
    expect(p.rideRatio + p.runRatio).toBeCloseTo(1)
    expect(p.perDayKm).toBeNull()
  })
})

describe('itemSummary', () => {
  it('shows moving time and speed for rides', () => {
    expect(itemSummary(dayItem({ movingS: 6500, speedKmh: 23.64 }))).toBe('1:48:20  23.6km/h')
  })
  it('shows moving time and pace for runs', () => {
    expect(itemSummary(dayItem({ sport: 'running', movingS: 1660, speedKmh: null, pace: "5'19\"" }))).toBe("27:40  5'19\"/km")
  })
  it('shows only time when neither is known', () => {
    expect(itemSummary(dayItem({ movingS: 59, speedKmh: null }))).toBe('0:59')
  })
})

describe('dayLabel', () => {
  it('reads as month and day without leading zeros', () => { expect(dayLabel('2026-11-04')).toBe('11월 4일') })
})

describe('pickAfterImport', () => {
  const report = (added: string[]) => ({ addedActivities: added, duplicateFiles: [], failed: [], cardsCreated: 0, unmatchedPhotos: [], skipped: [] })
  it('opens the new workout on that date', () => {
    expect(pickAfterImport(report(['x', 'c8']), NOV, '2026-11-08')).toBe('c8')
  })
  it('falls back to the first workout of that date', () => {
    expect(pickAfterImport(report([]), NOV, '2026-11-08')).toBe('c8')
  })
  it('is null when the date still has no workout', () => {
    expect(pickAfterImport(report([]), NOV, '2026-11-20')).toBeNull()
  })
})
