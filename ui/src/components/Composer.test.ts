// @vitest-environment happy-dom
import { flushSync, mount, unmount } from 'svelte'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { api } from '$lib/api'
import Composer from './Composer.svelte'

const tick = (ms = 80) => new Promise(r => setTimeout(r, ms))

describe('Composer Garmin flow', () => {
  afterEach(async () => {
    await api.setGarminKey(null)
    await api.setGarminAutoCheck(false)
    document.body.innerHTML = ''
    vi.restoreAllMocks()
  })

  it('shows link when garmin is disconnected and does not call auto-check when off', async () => {
    const spy = vi.spyOn(api, 'garminNewSinceLastCheck')
    const app = mount(Composer, {
      target: document.body,
      props: { onClose: () => {}, onOpenActivity: () => {}, onCreated: () => {}, onOpenSettings: () => {} },
    })
    await tick()
    flushSync()
    expect(document.body.textContent).toContain('설정에서 Garmin을 연결하세요')
    expect(spy).not.toHaveBeenCalled()
    unmount(app)
  })

  it('loads Garmin list, selects unimported activity, fetches FIT, and shows upload preview', async () => {
    await api.setGarminKey('personal-key')
    await api.setGarminAutoCheck(false)
    const autoSpy = vi.spyOn(api, 'garminNewSinceLastCheck')

    const app = mount(Composer, {
      target: document.body,
      props: { onClose: () => {}, onOpenActivity: () => {}, onCreated: () => {} },
    })
    await tick()
    flushSync()
    expect(autoSpy).not.toHaveBeenCalled()

    const garminBtn = [...document.querySelectorAll('button')].find(b => b.textContent?.includes('Garmin에서 가져오기'))
    expect(garminBtn).toBeDefined()
    garminBtn!.click()
    await tick(100)
    flushSync()

    expect(document.body.textContent).toContain('한강 아침 러닝')
    expect(document.body.textContent).toContain('이미 있음')

    const checkboxes = [...document.querySelectorAll('input[type="checkbox"]')] as HTMLInputElement[]
    expect(checkboxes.length).toBe(2)
    expect(checkboxes[0].checked).toBe(true)
    expect(checkboxes[1].checked).toBe(false)

    const fetchBtn = [...document.querySelectorAll('button')].find(b => b.textContent?.includes('선택한 1개 가져오기'))
    expect(fetchBtn).toBeDefined()
    fetchBtn!.click()
    await tick(150)
    flushSync()

    expect(document.body.textContent).toContain('만들 게시물 1개')
    expect(document.body.textContent).toContain('월요일 이른아침 러닝')
    unmount(app)
  })
})
