import { describe, expect, it } from 'vitest'
import type { DayDto } from './bindings/DayDto'
import { dayItem } from './fixtures'
import { dayLabel, filterGroups, groupByWeek, parseGoal, sportGoal, itemSummary, pickAfterImport, pickSelection, ribbonDays } from './workbench'

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

describe('sportGoal', () => {
  const today = new Date(2026, 10, 14)
  it('fills toward the goal and paces against days gone by', () => {
    const g = sportGoal(102, 300, 2026, 11, today)!
    expect(g.ratio).toBeCloseTo(102 / 300)
    expect(g.paceRatio).toBeCloseTo(13 / 30)
    expect(g.perDayKm).toBeCloseTo((300 - 102) / 17)
  })
  it('is null without a goal', () => { expect(sportGoal(10, null, 2026, 11, today)).toBeNull() })
  it('caps at a full bar once the goal is passed', () => {
    const g = sportGoal(320, 300, 2026, 11, today)!
    expect(g.ratio).toBe(1)
    expect(g.perDayKm).toBeNull()
  })
  it('a past month is fully paced and needs nothing per day', () => {
    const g = sportGoal(100, 300, 2026, 10, today)!
    expect(g.paceRatio).toBe(1)
    expect(g.perDayKm).toBeNull()
  })
  it('a future month has not started', () => {
    expect(sportGoal(0, 300, 2026, 12, today)!.paceRatio).toBe(0)
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

describe('parseGoal', () => {
  it('reads what a number input gives (number, empty as null, or text)', () => {
    expect([parseGoal(300), parseGoal('50'), parseGoal(null), parseGoal(''), parseGoal(undefined)]).toEqual([300, 50, null, null, null])
  })
  it('zero, negative and not-a-number mean no goal', () => {
    expect([parseGoal(0), parseGoal(-3), parseGoal(NaN), parseGoal('abc')]).toEqual([null, null, null, null])
  })
})
