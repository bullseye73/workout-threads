// 값에 따라 색을 바꾸는 계산: 범위, 0..1 위치, 색 섞기, 지도 경로 색 구간, 요약값, 공간에 맞춰 보여 줄 개수, 구간 빠르기.
export type Range = { lo: number; hi: number }
export type Stats = { min: number; avg: number; max: number }
export type Run = { bucket: number; latlngs: [number, number][] }

const known = (v: (number | null)[]) => v.filter((x): x is number => x !== null && Number.isFinite(x))

/** 아주 튀는 값에 색이 끌려가지 않게 5~95 백분위를 범위로 쓴다. */
export function valueRange(values: (number | null)[], lowPct = 5, highPct = 95): Range | null {
  const v = known(values).sort((a, b) => a - b)
  if (!v.length) return null
  const at = (p: number) => v[Math.round((p / 100) * (v.length - 1))]
  return { lo: at(lowPct), hi: at(highPct) }
}

export function heatT(v: number, r: Range): number {
  if (r.hi === r.lo) return 0.5
  return Math.min(1, Math.max(0, (v - r.lo) / (r.hi - r.lo)))
}

/** 색 구간(#rrggbb 목록)에서 t(0..1) 위치의 색. */
export function mixHex(stops: string[], t: number): string {
  const x = Math.min(1, Math.max(0, t)) * (stops.length - 1)
  const i = Math.min(stops.length - 2, Math.floor(x))
  const f = x - i
  const a = parseInt(stops[i].slice(1), 16), b = parseInt(stops[i + 1].slice(1), 16)
  const ch = (s: number) => Math.round(((a >> s) & 255) + (((b >> s) & 255) - ((a >> s) & 255)) * f)
  return '#' + [16, 8, 0].map(s => ch(s).toString(16).padStart(2, '0')).join('')
}

export function seriesStats(values: (number | null)[]): Stats | null {
  const v = known(values)
  if (!v.length) return null
  return { min: Math.min(...v), avg: v.reduce((t, x) => t + x, 0) / v.length, max: Math.max(...v) }
}

/**
 * 지도 경로를 색 구간별 선으로 나눈다. 한 구간의 색은 시작점 값으로 정하고,
 * 이웃 구간과 끝점을 나눠 선이 끊겨 보이지 않게 한다. 위치나 값이 없으면 선을 끊는다.
 */
export function colorRuns(points: ([number, number] | null)[], values: (number | null)[], r: Range, buckets: number): Run[] {
  const runs: Run[] = []
  let cur: Run | null = null
  for (let i = 0; i + 1 < points.length; i++) {
    const a = points[i], b = points[i + 1], va = values[i], vb = values[i + 1]
    if (!a || !b || va === null || vb === null) { cur = null; continue }
    const bucket = Math.min(buckets - 1, Math.floor(heatT(va, r) * buckets))
    if (cur && cur.bucket === bucket) cur.latlngs.push(b)
    else { cur = { bucket, latlngs: [a, b] }; runs.push(cur) }
  }
  return runs
}

/** 자리가 모자라면 마지막 칸은 "+N"으로 남긴다. */
export function fitItems<T>(items: T[], capacity: number): { shown: T[]; more: number } {
  if (items.length <= capacity) return { shown: items, more: 0 }
  const n = Math.max(0, capacity - 1)
  return { shown: items.slice(0, n), more: items.length - n }
}

/** 1km 구간의 빠르기. value는 클수록 빠름(km/h), 라이딩은 속도, 그 외는 페이스로 적는다. */
export function splitRate(secPerKm: number, sport: string): { value: number; label: string } {
  if (secPerKm <= 0) return { value: 0, label: '—' }
  const kmh = 3600 / secPerKm
  if (sport === 'cycling') return { value: kmh, label: `${kmh.toFixed(1)}km/h` }
  const s = Math.round(secPerKm)
  return { value: kmh, label: `${Math.floor(s / 60)}'${String(s % 60).padStart(2, '0')}"` }
}
