// 게시물 작성 창 계산: 운동 하나 빼기(제외할 경로), 만들기 버튼 문구.
import type { UploadPlan } from './bindings/UploadPlan'

/** 운동 하나를 뺄 때 제외할 경로: 그 FIT와 그 운동에 붙기로 한 사진 (폴더로 올렸어도 파일 단위로 뺀다). */
export function activityExclusions(plan: UploadPlan, source: string): string[] {
  const act = plan.newActivities.find(a => a.source === source)
  return [source, ...(act?.photos.map(p => p.path) ?? [])]
}

export function createLabel(plan: UploadPlan): string {
  const n = plan.newActivities.length
  if (n === 0 && plan.existing.some(e => e.photos.length > 0)) return '사진 보태기'
  return `게시물 ${n}개 만들기`
}
