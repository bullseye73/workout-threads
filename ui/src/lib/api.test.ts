import { describe, it, expect } from 'vitest'
import { mockApi } from './api'

describe('mockApi', () => {
  it('returns a month with days', async () => {
    const v = await mockApi.monthView(2026, 10)
    expect(v.days.length).toBeGreaterThan(0)
  })
  it('detail has route and same-length series', async () => {
    const id = (await mockApi.monthView(2026, 10)).days[0].items[0].activityId
    const d = await mockApi.activityDetail(id)
    expect(d.route.length).toBeGreaterThan(10)
    expect(d.series.hr.length).toBe(d.series.distanceKm.length)
  })
  it('savePost marks edited', async () => {
    expect((await mockApi.savePost('m1', '글', [])).edited).toBe(true)
  })
  it('empty month has no days', async () => {
    expect((await mockApi.monthView(2026, 12)).days).toEqual([])
  })
})

describe('mockApi review fixes', () => {
  it('lists unmatched photos', async () => { expect(Array.isArray(await mockApi.unmatchedPhotos())).toBe(true) })
  it('can pick a folder', async () => { expect((await mockApi.pickImport('folder')).length).toBe(1) })
})

describe('mockApi backup', () => {
  it('configure, login, backup', async () => {
    await mockApi.setGoogleClient('cid', 'sec')
    expect((await mockApi.googleClient())?.clientId).toBe('cid')
    expect((await mockApi.googleLogin()).signedIn).toBe(true)
    const s = await mockApi.backupNow()
    expect(s.pending).toBe(0)
    expect(s.lastSuccess).not.toBeNull()
  })
})

describe('mockApi refine provider', () => {
  it('defaults to Gemini and switches to Claude', async () => {
    let s = await mockApi.refineStatus()
    expect(s.provider).toBe('gemini')
    expect(s.company).toBe('Google')
    await mockApi.setRefineKey('claude', 'k')
    await mockApi.setRefineProvider('claude')
    s = await mockApi.refineStatus()
    expect(s.provider).toBe('claude')
    expect(s.hasKey).toBe(true)
    expect(s.providers.find(p => p.id === 'gemini')?.hasKey).toBe(false)
  })
})

describe('mockApi.importForDate', () => {
  it('returns a report that lists skipped files', async () => {
    const r = await mockApi.importForDate('2026-11-14', ['a.fit'])
    expect(Array.isArray(r.skipped)).toBe(true)
    expect(r.skipped.length).toBeGreaterThan(0)
  })
})
