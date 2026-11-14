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

/**
 * <html>의 dark 클래스가 실제로 바뀐 뒤에 부른다. mode-watcher는 다음 프레임에 클래스를 바꾸므로
 * 테마 값을 바로 읽으면 이전 색이 나온다. 그래프·지도처럼 색을 직접 칠하는 곳에서 쓴다.
 */
export function onThemeApplied(cb: () => void): () => void {
  const html = document.documentElement
  let dark = html.classList.contains('dark')
  const obs = new MutationObserver(() => {
    const now = html.classList.contains('dark')
    if (now !== dark) { dark = now; cb() }
  })
  obs.observe(html, { attributes: true, attributeFilter: ['class'] })
  return () => obs.disconnect()
}
