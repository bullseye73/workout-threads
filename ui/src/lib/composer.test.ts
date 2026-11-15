import { describe, expect, it } from 'vitest'
import type { UploadPlan } from './bindings/UploadPlan'
import { createLabel, removeActivityPaths } from './composer'

const photo = (path: string) => ({ path, fileName: path.split('/').pop()!, kmMark: null })
const act = (source: string, photos: string[]) => ({ source, activityId: source, date: '2026-10-25', title: 't', sport: 'cycling' as const,
  km: 10, photos: photos.map(photo), libraryPhotos: 0 })
const plan = (over: Partial<UploadPlan>): UploadPlan => ({ newActivities: [], existing: [], excluded: [], duplicates: [], ...over })

describe('removeActivityPaths', () => {
  it('drops the FIT and the photos planned for it, keeps the rest', () => {
    const p = plan({ newActivities: [act('/a.fit', ['/a1.jpg', '/a2.jpg']), act('/b.fit', ['/b1.jpg'])] })
    const paths = ['/a.fit', '/a1.jpg', '/a2.jpg', '/b.fit', '/b1.jpg', '/x.jpg']
    expect(removeActivityPaths(p, paths, '/a.fit')).toEqual(['/b.fit', '/b1.jpg', '/x.jpg'])
  })
  it('a folder that was added stays (its files are re-read and re-planned)', () => {
    const p = plan({ newActivities: [act('/f/a.fit', ['/f/a1.jpg'])] })
    expect(removeActivityPaths(p, ['/f'], '/f/a.fit')).toEqual(['/f'])
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
