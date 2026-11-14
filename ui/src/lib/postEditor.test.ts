import { describe, expect, it } from 'vitest'
import { createPostEditor } from './postEditor.svelte'

const saved = { body: '저장된 글', hashtags: ['#라이딩'] }

describe('post editor', () => {
  it('is dirty after an edit and clean again after discard', () => {
    const e = createPostEditor()
    e.load(saved)
    expect(e.dirty).toBe(false)
    e.body = '고친 글'
    expect(e.dirty).toBe(true)
    e.discard()
    expect(e.dirty).toBe(false)
    expect(e.body).toBe('저장된 글')
  })
  it('stays dirty until discarded, however often it is asked', () => {
    const e = createPostEditor()
    e.load(saved)
    e.tags = '#라이딩 #한강'
    expect(e.dirty).toBe(true)
    expect(e.dirty).toBe(true)
  })
  it('full text is what is on screen, not what was saved', () => {
    const e = createPostEditor()
    e.load(saved)
    e.body = '고친 글'
    expect(e.fullText).toBe('고친 글\n\n#라이딩')
  })
  it('keeps unsaved edits when the server sends the post again', () => {
    const e = createPostEditor()
    e.load(saved)
    e.body = '고친 글'
    e.load({ body: '서버 글', hashtags: [] })
    expect(e.body).toBe('고친 글')
    e.load({ body: '서버 글', hashtags: [] }, true)
    expect(e.body).toBe('서버 글')
  })
  it('is never dirty before a post is loaded', () => {
    expect(createPostEditor().dirty).toBe(false)
  })
})
