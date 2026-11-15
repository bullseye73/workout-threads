// 게시물 작성 창 계산: 운동 하나 빼기, 만들기 버튼 문구.
import type { UploadPlan } from './bindings/UploadPlan'

/** 그 FIT와, 그 운동에 붙기로 한 사진의 경로를 뺀다. 폴더로 올린 경로는 남는다(다시 읽어 계획). */
export function removeActivityPaths(plan: UploadPlan, paths: string[], source: string): string[] {
  const act = plan.newActivities.find(a => a.source === source)
  const drop = new Set([source, ...(act?.photos.map(p => p.path) ?? [])])
  return paths.filter(p => !drop.has(p))
}

export function createLabel(plan: UploadPlan): string {
  const n = plan.newActivities.length
  if (n === 0 && plan.existing.some(e => e.photos.length > 0)) return '사진 보태기'
  return `게시물 ${n}개 만들기`
}
