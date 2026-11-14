import { describe, expect, it } from 'vitest'
import { colorRuns, fitItems, heatT, mixHex, seriesStats, splitRate, valueRange } from './heat'

describe('valueRange', () => {
  it('ignores missing values and trims outliers to the 5th–95th percentile', () => {
    const v = [null, ...Array.from({ length: 101 }, (_, i) => i)]
    expect(valueRange(v)).toEqual({ lo: 5, hi: 95 })
  })
  it('is null when nothing is known', () => { expect(valueRange([null, null])).toBeNull() })
})

describe('heatT', () => {
  it('maps into 0..1 and clamps', () => {
    const r = { lo: 10, hi: 20 }
    expect([heatT(5, r), heatT(15, r), heatT(30, r)]).toEqual([0, 0.5, 1])
  })
  it('a flat range sits in the middle', () => { expect(heatT(3, { lo: 3, hi: 3 })).toBe(0.5) })
})

describe('mixHex', () => {
  const stops = ['#000000', '#ff0000', '#ffffff']
  it('returns the stops at the ends and in the middle', () => {
    expect([mixHex(stops, 0), mixHex(stops, 0.5), mixHex(stops, 1)]).toEqual(['#000000', '#ff0000', '#ffffff'])
  })
  it('blends between stops', () => { expect(mixHex(stops, 0.25)).toBe('#800000') })
})

describe('seriesStats', () => {
  it('min, average, max over known values', () => {
    expect(seriesStats([null, 2, 4, 9])).toEqual({ min: 2, avg: 5, max: 9 })
  })
  it('null when empty', () => { expect(seriesStats([null])).toBeNull() })
})

describe('colorRuns', () => {
  const pts: ([number, number] | null)[] = [[0, 0], [0, 1], [0, 2], [0, 3], null, [0, 5], [0, 6]]
  it('joins neighbours of the same colour bucket and shares the joint point', () => {
    const runs = colorRuns(pts, [10, 10, 20, 20, 20, 20, 20], { lo: 10, hi: 20 }, 4)
    expect(runs.map(r => r.bucket)).toEqual([0, 3, 3])
    expect(runs[0].latlngs).toEqual([[0, 0], [0, 1], [0, 2]])
    expect(runs[1].latlngs).toEqual([[0, 2], [0, 3]])
    expect(runs[2].latlngs).toEqual([[0, 5], [0, 6]])
  })
  it('missing values break the line too', () => {
    const runs = colorRuns([[0, 0], [0, 1], [0, 2]], [10, null, 10], { lo: 0, hi: 20 }, 4)
    expect(runs).toEqual([])
  })
})

describe('fitItems', () => {
  it('shows everything that fits', () => { expect(fitItems([1, 2, 3], 4)).toEqual({ shown: [1, 2, 3], more: 0 }) })
  it('keeps the last slot for a "+N" tile', () => { expect(fitItems([1, 2, 3, 4, 5, 6], 4)).toEqual({ shown: [1, 2, 3], more: 3 }) })
})

describe('splitRate', () => {
  it('rides show km/h', () => { expect(splitRate(120, 'cycling')).toEqual({ value: 30, label: '30.0km/h' }) })
  it('runs show pace per km, faster is a bigger value', () => {
    const r = splitRate(330, 'running')
    expect(r.label).toBe("5'30\"")
    expect(r.value).toBeCloseTo(3600 / 330)
  })
  it('an empty split has no rate', () => { expect(splitRate(0, 'cycling')).toEqual({ value: 0, label: '—' }) })
})
