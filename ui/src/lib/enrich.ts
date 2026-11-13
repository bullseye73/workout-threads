// 장소·경로·날씨 칸의 입력·표시 형식.
import type { WeatherDto } from './bindings/WeatherDto'

/** "반포 → 잠실, 뚝섬" → ["반포", "잠실", "뚝섬"] (→, 쉼표, 공백 구분, 중복 제거). */
export function parseRoutePoints(input: string): string[] {
  const out: string[] = []
  for (const w of input.split(/[→,\s]+/)) {
    const t = w.trim()
    if (t && !out.includes(t)) out.push(t)
  }
  return out
}

export function routeText(points: string[]): string {
  return points.join(' → ')
}

export function weatherText(w: WeatherDto | null): string {
  return w ? `${w.description} ${Math.round(w.tempC)}도` : ''
}

/** "맑음 17도" / "구름 조금, -3" → {설명, 기온}. 숫자가 없으면 null. */
export function parseWeather(input: string): WeatherDto | null {
  const m = input.trim().match(/^(.*?)[\s,]*(-?\d+(?:\.\d+)?)\s*(?:도|°C?)?$/)
  if (!m || !m[1].trim()) return null
  return { description: m[1].trim(), tempC: Number(m[2]) }
}

/** 서버에서 새 글을 받았을 때, 저장하지 않은 본문·해시태그 수정은 지킨다. */
export function mergeEditor(
  loaded: { body: string; hashtags: string[] },
  local: { body: string; tags: string },
  dirty: boolean,
): { body: string; tags: string } {
  return dirty ? { ...local } : { body: loaded.body, tags: loaded.hashtags.join(' ') }
}
