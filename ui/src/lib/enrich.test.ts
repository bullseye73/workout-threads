import { describe, it, expect } from 'vitest'
import { parseRoutePoints, parseWeather, routeText, weatherText } from './enrich'
import { mockApi } from './api'

describe('enrich helpers', () => {
  it('parseRoutePoints', () => {
    expect(parseRoutePoints('반포 → 잠실, 뚝섬  잠실')).toEqual(['반포', '잠실', '뚝섬'])
    expect(parseRoutePoints(' ')).toEqual([])
  })
  it('routeText', () => {
    expect(routeText(['반포', '잠실'])).toBe('반포 → 잠실')
    expect(routeText([])).toBe('')
  })
  it('weather text roundtrip', () => {
    expect(weatherText({ description: '맑음', tempC: 17.4 })).toBe('맑음 17도')
    expect(weatherText(null)).toBe('')
    expect(parseWeather('맑음 17도')).toEqual({ description: '맑음', tempC: 17 })
    expect(parseWeather('구름 조금, -3')).toEqual({ description: '구름 조금', tempC: -3 })
    expect(parseWeather('맑음')).toBeNull()
  })
  it('mock enrich fills place', async () => {
    expect((await mockApi.enrichActivity('m3')).place?.name).toBe('가짜공원')
  })
  it('mock refine returns preview', async () => {
    expect((await mockApi.refinePost('m3', '내 글 다들', ['#a'], false)).body).toContain('내 글')
  })
})

import { mergeEditor } from './enrich'
describe('mergeEditor (리뷰 I3)', () => {
  const loaded = { body: '서버 글', hashtags: ['#서버'] }
  it('keeps unsaved local edits', () => {
    expect(mergeEditor(loaded, { body: '내가 쓰는 중', tags: '#나' }, true)).toEqual({ body: '내가 쓰는 중', tags: '#나' })
  })
  it('takes server text when nothing unsaved', () => {
    expect(mergeEditor(loaded, { body: '옛 글', tags: '#옛' }, false)).toEqual({ body: '서버 글', tags: '#서버' })
  })
})
