// 작업대 화면 계산: 한 달 사진 띠, 주별 목록, 거르기, 선택 유지, 목표 진행.
import type { DayDto } from './bindings/DayDto'
import type { DayItemDto } from './bindings/DayItemDto'
import type { ImportReport } from './bindings/ImportReport'

export type Item = DayItemDto & { date: string }
export type RibbonDay = { day: number; date: string; weekend: boolean; today: boolean; items: Item[] }
export type WeekGroup = { weekStart: string; label: string; km: number; items: Item[] }
export type ListFilter = 'all' | 'ready' | 'published'
/** 날짜를 골라 등록한 결과 (n은 같은 결과를 다시 알리기 위한 일련번호). */
export type DateImported = { date: string; report: ImportReport; n: number }
export type SportGoal = { ratio: number; paceRatio: number; perDayKm: number | null }

const pad = (n: number) => String(n).padStart(2, '0')
const isoOf = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`
const daysIn = (year: number, month: number) => new Date(year, month, 0).getDate()
const sumKm = (items: Item[]) => items.reduce((t, i) => t + i.km, 0)

function withDate(day: DayDto): Item[] {
  return day.items.map(i => ({ ...i, date: day.date }))
}

export function ribbonDays(year: number, month: number, days: DayDto[], todayIso: string): RibbonDay[] {
  const byDate = new Map(days.map(d => [d.date, withDate(d)]))
  return Array.from({ length: daysIn(year, month) }, (_, i) => {
    const d = new Date(year, month - 1, i + 1)
    const date = isoOf(d)
    return { day: i + 1, date, weekend: d.getDay() === 0 || d.getDay() === 6, today: date === todayIso, items: byDate.get(date) ?? [] }
  })
}

/** 월요일 시작 주로 묶는다. 최근 주가 먼저, 주 안에서도 최근 운동이 먼저. */
export function groupByWeek(days: DayDto[]): WeekGroup[] {
  const groups = new Map<string, Item[]>()
  for (const day of days) {
    const d = new Date(day.date + 'T00:00:00')
    d.setDate(d.getDate() - ((d.getDay() + 6) % 7))
    const key = isoOf(d)
    groups.set(key, [...(groups.get(key) ?? []), ...withDate(day)])
  }
  return [...groups.entries()]
    .sort(([a], [b]) => b.localeCompare(a))
    .map(([weekStart, items]) => {
      const start = new Date(weekStart + 'T00:00:00')
      const end = new Date(start)
      end.setDate(end.getDate() + 6)
      const sorted = [...items].reverse().sort((a, b) => b.date.localeCompare(a.date))
      return { weekStart, label: `${start.getMonth() + 1}/${start.getDate()}–${end.getMonth() + 1}/${end.getDate()}`, km: sumKm(sorted), items: sorted }
    })
}

export function filterGroups(groups: WeekGroup[], f: ListFilter): WeekGroup[] {
  if (f === 'all') return groups
  return groups
    .map(g => {
      const items = g.items.filter(i => i.postStatus === f)
      return { ...g, items, km: sumKm(items) }
    })
    .filter(g => g.items.length > 0)
}

export function pickSelection(groups: WeekGroup[], current: string | null): string | null {
  const items = groups.flatMap(g => g.items)
  if (current && items.some(i => i.activityId === current)) return current
  return items[0]?.activityId ?? null
}

/** 한 종목의 월 목표 진행: 채운 비율, 오늘까지 기준선(어제까지 지난 날수/그 달 날수), 남은 날 하루 거리. 목표 없으면 null. */
export function sportGoal(doneKm: number, goalKm: number | null, year: number, month: number, today: Date): SportGoal | null {
  if (!goalKm) return null
  const total = daysIn(year, month)
  const monthKey = year * 12 + month
  const todayKey = today.getFullYear() * 12 + today.getMonth() + 1
  const gone = monthKey < todayKey ? total : monthKey > todayKey ? 0 : today.getDate() - 1
  const left = total - gone
  const remaining = goalKm - doneKm
  return {
    ratio: Math.min(1, Math.max(0, doneKm / goalKm)),
    paceRatio: gone / total,
    perDayKm: left > 0 && remaining > 0 ? remaining / left : null,
  }
}

/** 초 → "1:48:20" / "27:40" (카드·상세와 같은 형식). */
function hms(sec: number): string {
  const s = Math.round(sec)
  const h = Math.floor(s / 3600), m = Math.floor((s % 3600) / 60), r = s % 60
  return h > 0 ? `${h}:${pad(m)}:${pad(r)}` : `${m}:${pad(r)}`
}

/** 목록 한 줄 요약: 이동 시간 + 속도(라이딩) 또는 페이스. */
export function itemSummary(i: DayItemDto): string {
  const rate = i.speedKmh !== null ? `${i.speedKmh.toFixed(1)}km/h` : i.pace ? `${i.pace}/km` : ''
  return rate ? `${hms(i.movingS)}  ${rate}` : hms(i.movingS)
}

/** "2026-11-04" → "11월 4일". */
export function dayLabel(iso: string): string {
  return `${Number(iso.slice(5, 7))}월 ${Number(iso.slice(8, 10))}일`
}

/** 날짜를 골라 등록한 뒤 열 운동: 그날 새로 들어온 운동 → 그날 첫 운동 → 없음. */
export function pickAfterImport(report: ImportReport, days: DayDto[], date: string): string | null {
  const items = days.find(d => d.date === date)?.items ?? []
  return items.find(i => report.addedActivities.includes(i.activityId))?.activityId ?? items[0]?.activityId ?? null
}

/** 목표 입력 값 → km. 숫자 입력칸은 숫자(비우면 null)를 주므로 글자·숫자 모두 받는다. 0 이하·숫자 아님은 목표 없음. */
export function parseGoal(v: unknown): number | null {
  if (v === null || v === undefined || (typeof v === 'string' && !v.trim())) return null
  const n = Number(v)
  return Number.isFinite(n) && n > 0 ? n : null
}
