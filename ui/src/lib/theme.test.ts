import { describe, expect, it } from 'vitest'
import { STATUS_LABEL, THEME_OPTIONS } from './theme'

describe('theme options', () => {
  it('lists system, light, dark in that order with Korean labels', () => {
    expect(THEME_OPTIONS.map(o => o.value)).toEqual(['system', 'light', 'dark'])
    expect(THEME_OPTIONS.map(o => o.label)).toEqual(['시스템', '밝게', '어둡게'])
  })
})

describe('post status labels', () => {
  it('names each status', () => {
    expect(STATUS_LABEL).toEqual({ none: '글 없음', ready: '게시 대기', published: '게시함' })
  })
})
