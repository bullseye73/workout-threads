// 운동 상세 계산: 거리 축 위 사진 핀, km 구간 속도 막대.
import type { PhotoDto } from './bindings/PhotoDto'
import type { SplitDto } from './bindings/SplitDto'

export type Pin = { id: number; km: number; leftPct: number; src: string }
export type SplitBar = SplitDto & { sec: number; widthPct: number; best: boolean }

/** 찍은 km 지점에 사진을 꽂는다. km를 모르거나 파일이 없으면 뺀다. */
export function photoPins(photos: PhotoDto[], totalKm: number): Pin[] {
  return photos
    .filter(p => p.kmMark !== null && (p.cardPath || p.path))
    .map(p => ({
      id: p.id,
      km: p.kmMark!,
      leftPct: totalKm > 0 ? Math.min(100, Math.max(0, (p.kmMark! / totalKm) * 100)) : 0,
      src: (p.cardPath || p.path)!,
    }))
    .sort((a, b) => a.km - b.km)
}

/** "m:ss" | "h:mm:ss" → 초. */
export function durationSec(s: string): number {
  return s.split(':').reduce((t, part) => t * 60 + Number(part), 0)
}

/**
 * 구간 막대: km당 시간이 짧을수록 넓다. 구간 번호는 1부터이고 마지막 1km 미만 구간도
 * 다음 번호를 받으므로, 마지막 구간 길이는 전체 거리에서 구한다.
 */
export function splitBars(splits: SplitDto[], totalKm: number): SplitBar[] {
  const perKm = splits.map((s, i) => {
    const raw = durationSec(s.duration)
    const isLast = i === splits.length - 1
    const len = isLast && totalKm > i && totalKm - i < 1 ? totalKm - i : 1
    return raw > 0 ? raw / len : 0
  })
  const valid = perKm.filter(v => v > 0)
  const fastest = valid.length ? Math.min(...valid) : 0
  let bestDone = false
  return splits.map((s, i) => {
    const sec = perKm[i]
    const best = !bestDone && sec > 0 && sec === fastest
    if (best) bestDone = true
    return { ...s, sec, widthPct: sec > 0 ? (fastest / sec) * 100 : 0, best }
  })
}
