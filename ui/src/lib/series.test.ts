import { describe, it, expect } from 'vitest'
import type { ActivityDetail } from './bindings/ActivityDetail'
import type { SportKind } from './bindings/SportKind'
import { nearestIndex, toChartSeries } from './series'

function detail(sport: SportKind, o: { speedKmh: (number | null)[] }): ActivityDetail {
  const n = o.speedKmh.length
  return {
    id: 'x', sport, title: '', date: '', metrics: { distanceKm: 1, moving: '', elapsed: '', pace: null, speedKmh: null,
      avgHr: null, maxHr: null, ascentM: null, avgCadence: null },
    route: [], start: null, end: null, photos: [], splits: [],
    series: { distanceKm: Array.from({ length: n }, (_, i) => i), hr: Array(n).fill(140), speedKmh: o.speedKmh,
      altitudeM: Array(n).fill(10), latLon: Array(n).fill(null) },
  }
}

describe('series', () => {
  it('running uses pace minutes', () => {
    const s = toChartSeries(detail('running', { speedKmh: [12, 0, null] }))
    expect(s.secondaryLabel).toBe('페이스 (분/km)')
    expect(s.secondary).toEqual([5, null, null])
  })
  it('cycling uses speed', () => {
    const s = toChartSeries(detail('cycling', { speedKmh: [24.4] }))
    expect(s.secondaryLabel).toBe('속도 (km/h)')
    expect(s.secondary).toEqual([24.4])
  })
  it('nearestIndex', () => {
    expect(nearestIndex([0, 1, 2, 3], 1.6)).toBe(2)
    expect(nearestIndex([0, 1, 2, 3], -5)).toBe(0)
    expect(nearestIndex([0, 1, 2, 3], 99)).toBe(3)
    expect(nearestIndex([], 1)).toBe(-1)
  })
})
