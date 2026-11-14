// @vitest-environment happy-dom
// 실제 기록처럼 차트가 만들어지는 순간 선 색(segment.borderColor)을 바로 묻는 경우에도
// 화면이 무한 갱신에 빠지지 않아야 한다.
import { flushSync, mount, unmount } from 'svelte'
import { describe, expect, it, vi } from 'vitest'

vi.mock('chart.js', () => {
  class Chart {
    static register() {}
    scales = { x: { getPixelForValue: () => 0, getValueForPixel: () => 0 } }
    chartArea = { top: 0, bottom: 1 }
    constructor(_c: unknown, cfg: { data: { datasets: { data: { y: number | null }[]; segment?: { borderColor: (c: unknown) => string } }[] } }) {
      const ds = cfg.data.datasets[0]
      ds.data.slice(0, 3).forEach(p => ds.segment?.borderColor({ p0: { parsed: { y: p.y } } }))
    }
    draw() {}
    destroy() {}
  }
  return { Chart, Filler: {}, LinearScale: {}, LineController: {}, LineElement: {}, PointElement: {} }
})

import DistanceProfile from './DistanceProfile.svelte'

describe('DistanceProfile', () => {
  it('renders real-shaped data without an update loop', () => {
    const n = 20
    const series = {
      x: Array.from({ length: n }, (_, i) => i), hr: Array.from({ length: n }, (_, i) => 120 + i),
      secondary: Array.from({ length: n }, (_, i) => 20 + (i % 5)), secondaryLabel: '속도 (km/h)',
      altitude: Array.from({ length: n }, () => 30),
    }
    const errors: unknown[] = []
    const onErr = (e: ErrorEvent) => errors.push(e.error)
    window.addEventListener('error', onErr)
    let app: ReturnType<typeof mount> | undefined
    expect(() => {
      app = mount(DistanceProfile, { target: document.body,
        props: { series, pins: [], highlight: -1, activePin: null, onHover: () => {}, onFocusPin: () => {} } })
      flushSync()
    }).not.toThrow()
    expect(errors).toEqual([])
    expect(document.body.textContent).toContain('심박 평균')
    if (app) unmount(app)
    window.removeEventListener('error', onErr)
  })
})
