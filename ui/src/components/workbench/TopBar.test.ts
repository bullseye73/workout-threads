// @vitest-environment happy-dom
// 목표 막대는 색만이 아니라 종목 이름으로도 구분되고, 목표 없는 종목도 거리가 보여야 한다.
import { flushSync, mount, unmount } from 'svelte'
import { describe, expect, it } from 'vitest'
import type { MonthView } from '$lib/bindings/MonthView'
import TopBar from './TopBar.svelte'

const sport = (count: number, km: number) => ({ count, km, movingS: 0, avgPace: null, avgSpeedKmh: null })
const view = (goalCyclingKm: number | null, goalRunningKm: number | null): MonthView => ({
  year: 2026, month: 11, goalCyclingKm, goalRunningKm, totalKm: 110.1, totalMovingS: 0,
  cycling: sport(2, 102), running: sport(1, 8.1), otherCount: 0, avgHr: null, weekly: [], days: [],
})
function goalText(v: MonthView): string {
  const app = mount(TopBar, { target: document.body, props: {
    year: 2026, month: 11, view: v, backup: null, unmatched: 0,
    onShift: () => {}, onSaveGoal: () => {}, onImport: () => {}, onUnmatched: () => {}, onSettings: () => {} } })
  flushSync()
  const btn = [...document.querySelectorAll('header button')].find(b => b.getAttribute('title')?.startsWith('목표 바꾸기'))
  const text = btn?.textContent?.replace(/\s+/g, ' ').trim() ?? ''
  unmount(app)
  return text
}

describe('TopBar goals', () => {
  it('names each sport on its bar', () => {
    const t = goalText(view(300, 50))
    expect(t).toContain('라이딩')
    expect(t).toContain('러닝')
  })
  it('still shows the distance of a sport without a goal', () => {
    const t = goalText(view(300, null))
    expect(t).toContain('러닝 8.1km')
  })
})
