import { describe, it, expect } from 'vitest'
import { km, duration } from './format'

describe('format', () => {
  it('km', () => { expect(km(107.54)).toBe('107.5'); expect(km(0)).toBe('0'); expect(km(5)).toBe('5.0') })
  it('duration', () => { expect(duration(9052)).toBe('2시간 30분'); expect(duration(1650)).toBe('27분'); expect(duration(0)).toBe('0분') })
})

import { parseHashtags, importSummary } from './format'
describe('post helpers', () => {
  it('parseHashtags', () => {
    expect(parseHashtags('#라이딩 자전거  #한강 #라이딩')).toEqual(['#라이딩', '#자전거', '#한강'])
    expect(parseHashtags('  ')).toEqual([])
    expect(parseHashtags('#라이딩,#한강')).toEqual(['#라이딩', '#한강'])
  })
  it('importSummary', () => {
    expect(importSummary({ addedActivities: ['a'], duplicateFiles: ['x', 'y', 'z'], failed: [], cardsCreated: 2, unmatchedPhotos: [], skipped: [] }))
      .toBe('운동 1개 추가 · 카드 2장 · 이미 있음 3 · 실패 0')
  })
})
