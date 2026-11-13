import type { DayDto } from './bindings/DayDto'
import type { DayItemDto } from './bindings/DayItemDto'

export type CalCell = { date: string; day: number; inMonth: boolean; items: DayItemDto[] }

const iso = (d: Date) => d.toISOString().slice(0, 10)

/** 월요일 시작 주 배열. 각 주는 7칸, 그 달과 겹치는 주만. */
export function buildMonthGrid(year: number, month: number, days: DayDto[]): CalCell[][] {
  const byDate = new Map(days.map(d => [d.date, d.items]))
  const first = new Date(Date.UTC(year, month - 1, 1))
  const last = new Date(Date.UTC(year, month, 0))
  const start = new Date(first)
  start.setUTCDate(1 - ((first.getUTCDay() + 6) % 7))
  const weeks: CalCell[][] = []
  for (let w = new Date(start); w <= last; ) {
    const week: CalCell[] = []
    for (let i = 0; i < 7; i++) {
      const date = iso(w)
      week.push({ date, day: w.getUTCDate(), inMonth: w.getUTCMonth() === month - 1, items: byDate.get(date) ?? [] })
      w = new Date(w.getTime() + 86400000)
    }
    weeks.push(week)
  }
  return weeks
}
