import { describe, expect, it } from 'vitest'
import type { UploadPlan } from './bindings/UploadPlan'
import { activityExclusions, createLabel } from './composer'

const photo = (path: string) => ({ path, fileName: path.split('/').pop()!, kmMark: null })
const act = (source: string, photos: string[], sources = [source]) => ({ source, sources, activityId: source, date: '2026-10-25', title: 't', sport: 'cycling' as const,
  km: 10, photos: photos.map(photo), libraryPhotos: 0 })
const plan = (over: Partial<UploadPlan>): UploadPlan => ({ newActivities: [], existing: [], excluded: [], duplicates: [], ...over })

describe('activityExclusions', () => {
  it('is the FIT plus the photos planned for that workout', () => {
    const p = plan({ newActivities: [act('/f/a.fit', ['/f/a1.jpg', '/f/a2.jpg']), act('/f/b.fit', ['/f/b1.jpg'])] })
    expect(activityExclusions(p, '/f/a.fit')).toEqual(['/f/a.fit', '/f/a1.jpg', '/f/a2.jpg'])
  })
  it('excludes every copy of the same FIT', () => {
    const p = plan({ newActivities: [act('/x/a.fit', ['/x/p.jpg'], ['/x/a.fit', '/y/b.fit'])] })
    expect(activityExclusions(p, '/x/a.fit')).toEqual(['/x/a.fit', '/y/b.fit', '/x/p.jpg'])
  })
  it('unknown source excludes just that path', () => {
    expect(activityExclusions(plan({}), '/x.fit')).toEqual(['/x.fit'])
  })
})

describe('createLabel', () => {
  it('counts new posts', () => {
    expect(createLabel(plan({ newActivities: [act('/a.fit', []), act('/b.fit', [])] }))).toBe('게시물 2개 만들기')
  })
  it('adding photos only says so', () => {
    const existing = [{ activityId: 'x', date: '2026-10-25', title: 't', sport: 'cycling' as const, km: 1, uploadedFit: false, photos: [photo('/p.jpg')], postLocked: false }]
    expect(createLabel(plan({ existing }))).toBe('사진 보태기')
  })
  it('nothing to do', () => { expect(createLabel(plan({}))).toBe('게시물 0개 만들기') })
})
