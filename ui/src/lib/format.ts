import type { ImportReport } from './bindings/ImportReport'

// 화면 표시 형식.

/** km 소수 1자리, 0은 "0". */
export function km(n: number): string {
  return n === 0 ? '0' : n.toFixed(1)
}

/** 초 → "2시간 30분" / "27분". */
export function duration(sec: number): string {
  const m = Math.floor(sec / 60)
  const h = Math.floor(m / 60)
  return h > 0 ? `${h}시간 ${m % 60}분` : `${m}분`
}

/** 입력 문자열 → 해시태그 목록 ("#" 자동 추가, 공백·쉼표 구분, 중복 제거). */
export function parseHashtags(input: string): string[] {
  const out: string[] = []
  for (const w of input.split(/[\s,]+/)) {
    const word = w.replace(/^#+/, '')
    if (!word) continue
    const tag = `#${word}`
    if (!out.includes(tag)) out.push(tag)
  }
  return out
}

/** 가져오기 결과 한 줄 요약. */
export function importSummary(r: ImportReport): string {
  return `운동 ${r.addedActivities.length}개 추가 · 카드 ${r.cardsCreated}장 · 이미 있음 ${r.duplicateFiles.length} · 실패 ${r.failed.length}`
}
