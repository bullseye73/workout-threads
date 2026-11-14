import { describe, expect, it } from 'vitest'
import type { PhotoDto } from './bindings/PhotoDto'
import { durationSec, photoPins, splitBars } from './profile'

const photo = (id: number, kmMark: number | null, cardPath: string | null = `c${id}.jpg`, path = `p${id}.jpg`): PhotoDto =>
  ({ id, fileName: path, path, cardPath, takenAt: null, kmMark, lat: null, lon: null })

describe('photoPins', () => {
  it('places photos by km, skips unknown km, clamps past the end', () => {
    const pins = photoPins([photo(1, 6.2), photo(2, null), photo(3, 50)], 42.6)
    expect(pins.map(p => p.id)).toEqual([1, 3])
    expect(pins[0].leftPct).toBeCloseTo((6.2 / 42.6) * 100)
    expect(pins[1].leftPct).toBe(100)
  })
  it('uses the card, else the original photo, else skips', () => {
    const pins = photoPins([photo(1, 1, null), photo(2, 2, null, '')], 10)
    expect(pins.map(p => p.src)).toEqual(['p1.jpg'])
  })
  it('sorts by km', () => {
    expect(photoPins([photo(1, 9), photo(2, 3)], 10).map(p => p.id)).toEqual([2, 1])
  })
})

describe('durationSec', () => {
  it('reads m:ss and h:mm:ss', () => {
    expect(durationSec('4:05')).toBe(245)
    expect(durationSec('1:02:03')).toBe(3723)
  })
})

describe('splitBars', () => {
  const splits = [{ km: 1, duration: '2:30', avgHr: 140 }, { km: 2, duration: '2:00', avgHr: 150 }, { km: 3, duration: '0:50', avgHr: null }]
  it('widens faster splits, marks the fastest, prorates the last partial km', () => {
    const b = splitBars(splits, 2.4)
    expect(b.map(s => s.best)).toEqual([false, true, false])
    expect(b.map(s => Math.round(s.widthPct))).toEqual([80, 100, 96])
    expect(b[2].sec).toBeCloseTo(125)
  })
  it('a zero-length split gets no bar and is never best', () => {
    const b = splitBars([{ km: 1, duration: '0:00', avgHr: null }, { km: 2, duration: '3:00', avgHr: null }], 2)
    expect(b[0].widthPct).toBe(0)
    expect(b[0].best).toBe(false)
    expect(b[1].best).toBe(true)
  })
  it('empty in, empty out', () => { expect(splitBars([], 0)).toEqual([]) })
})
