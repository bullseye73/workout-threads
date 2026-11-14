import { describe, expect, it } from 'vitest'
import { answer, ask, confirmState } from './confirm.svelte'

describe('ask', () => {
  it('opens with the question and resolves with the answer', async () => {
    const p = ask({ title: '나갈까요?', action: '나가기' })
    expect(confirmState.open).toBe(true)
    expect(confirmState.title).toBe('나갈까요?')
    expect(confirmState.action).toBe('나가기')
    answer(true)
    expect(await p).toBe(true)
    expect(confirmState.open).toBe(false)
  })
  it('a second question cancels the first', async () => {
    const first = ask({ title: 'a', action: 'x' })
    const second = ask({ title: 'b', action: 'y' })
    expect(await first).toBe(false)
    answer(true)
    expect(await second).toBe(true)
  })
})
