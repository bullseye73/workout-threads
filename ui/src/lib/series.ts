import type { ActivityDetail } from './bindings/ActivityDetail'

export type ChartSeries = {
  x: number[]
  hr: (number | null)[]
  secondary: (number | null)[]
  secondaryLabel: string
  altitude: (number | null)[]
}

/** 그래프용 시계열. 러닝은 페이스(분/km), 그 외는 속도(km/h). */
export function toChartSeries(d: ActivityDetail): ChartSeries {
  const s = d.series
  const running = d.sport === 'running'
  return {
    x: s.distanceKm,
    hr: s.hr,
    secondary: s.speedKmh.map(v => (v === null ? null : running ? (v > 0 ? 60 / v : null) : v)),
    secondaryLabel: running ? '페이스 (분/km)' : '속도 (km/h)',
    altitude: s.altitudeM,
  }
}

/** 오름차순 xs에서 x와 가장 가까운 인덱스 (이진 탐색). 빈 배열이면 -1. */
export function nearestIndex(xs: number[], x: number): number {
  if (xs.length === 0) return -1
  let lo = 0, hi = xs.length - 1
  while (lo < hi) {
    const mid = (lo + hi) >> 1
    if (xs[mid] < x) lo = mid + 1
    else hi = mid
  }
  return lo > 0 && Math.abs(xs[lo - 1] - x) <= Math.abs(xs[lo] - x) ? lo - 1 : lo
}
