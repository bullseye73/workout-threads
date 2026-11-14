// 테마와 게시 상태의 이름, 그래프·지도가 토큰 색을 읽는 도우미.
import type { PostStatus } from './bindings/PostStatus'

export type ThemePref = 'system' | 'light' | 'dark'

export const THEME_OPTIONS: { value: ThemePref; label: string }[] = [
  { value: 'system', label: '시스템' },
  { value: 'light', label: '밝게' },
  { value: 'dark', label: '어둡게' },
]

export const STATUS_LABEL: Record<PostStatus, string> = { none: '글 없음', ready: '게시 대기', published: '게시함' }

/** 토큰 값 (예: cssVar('--ride')). 캔버스·지도처럼 CSS 변수를 직접 못 쓰는 곳에서. */
export function cssVar(name: string, el: Element = document.documentElement): string {
  return getComputedStyle(el).getPropertyValue(name).trim()
}
