// 브라우저에서 화면을 확인하기 위한 가짜 데이터.
// 경로는 여의도 근처를 중심으로 합성한 원형 경로이며 실제 운동 좌표가 아니다.
import type { ActivityDetail } from './bindings/ActivityDetail'
import type { MonthView } from './bindings/MonthView'
import type { PostView } from './bindings/PostView'
import type { SportKind } from './bindings/SportKind'

type MockActivity = { id: string; date: string; sport: SportKind; km: number; movingS: number; hr: number | null; gps: boolean; hour: number }

export const mockActivities: MockActivity[] = [
  { id: 'm1', date: '2026-10-11', sport: 'cycling', km: 59.4, movingS: 9052, hr: 147, gps: true, hour: 9 },
  { id: 'm2', date: '2026-10-18', sport: 'running', km: 5.0, movingS: 1650, hr: 152, gps: false, hour: 7 },
  { id: 'm3', date: '2026-10-25', sport: 'cycling', km: 48.2, movingS: 7102, hr: 146, gps: true, hour: 14 },
  { id: 'm4', date: '2026-11-02', sport: 'running', km: 8.1, movingS: 2780, hr: 149, gps: true, hour: 7 },
]

const WEEKDAYS = ['일요일', '월요일', '화요일', '수요일', '목요일', '금요일', '토요일']

function hms(sec: number): string {
  const s = Math.round(sec)
  const h = Math.floor(s / 3600), m = Math.floor((s % 3600) / 60), r = s % 60
  return h > 0 ? `${h}:${String(m).padStart(2, '0')}:${String(r).padStart(2, '0')}` : `${m}:${String(r).padStart(2, '0')}`
}
function pace(sec: number, km: number): string {
  const p = Math.floor(sec / km)
  return `${Math.floor(p / 60)}'${String(p % 60).padStart(2, '0')}"`
}
function mockTitle(a: MockActivity): string {
  const tod = a.hour < 12 ? '오전' : a.hour < 17 ? '오후' : '저녁'
  return `${WEEKDAYS[new Date(a.date + 'T00:00:00Z').getUTCDay()]} ${tod} ${a.sport === 'running' ? '러닝' : '라이딩'}`
}
function mondayOf(d: Date): Date {
  const x = new Date(d)
  x.setUTCDate(x.getUTCDate() - ((x.getUTCDay() + 6) % 7))
  return x
}
const iso = (d: Date) => d.toISOString().slice(0, 10)

export const mockPosts = new Map<string, PostView>()

export function mockMonth(year: number, month: number): MonthView {
  const prefix = `${year}-${String(month).padStart(2, '0')}`
  const acts = mockActivities.filter(a => a.date.startsWith(prefix))
  const sum = (s: SportKind) => {
    const xs = acts.filter(a => a.sport === s)
    const km = xs.reduce((t, a) => t + a.km, 0), mv = xs.reduce((t, a) => t + a.movingS, 0)
    return { count: xs.length, km, movingS: mv,
      avgPace: s === 'running' && km > 0 ? pace(mv, km) : null,
      avgSpeedKmh: s === 'cycling' && mv > 0 ? km / mv * 3600 : null }
  }
  const first = new Date(Date.UTC(year, month - 1, 1)), last = new Date(Date.UTC(year, month, 0))
  const weekly = []
  for (let w = mondayOf(first); w <= last; w = new Date(w.getTime() + 7 * 86400000)) {
    const end = new Date(w.getTime() + 7 * 86400000)
    const inWeek = acts.filter(a => { const d = new Date(a.date + 'T00:00:00Z'); return d >= w && d < end })
    weekly.push({ weekStart: iso(w),
      runningKm: inWeek.filter(a => a.sport === 'running').reduce((t, a) => t + a.km, 0),
      cyclingKm: inWeek.filter(a => a.sport === 'cycling').reduce((t, a) => t + a.km, 0) })
  }
  const hrActs = acts.filter(a => a.hr !== null)
  const hrSecs = hrActs.reduce((t, a) => t + a.movingS, 0)
  return {
    year, month, goalCyclingKm: mockGoal.cycling, goalRunningKm: mockGoal.running,
    totalKm: acts.reduce((t, a) => t + a.km, 0), totalMovingS: acts.reduce((t, a) => t + a.movingS, 0),
    running: sum('running'), cycling: sum('cycling'), otherCount: 0,
    avgHr: hrSecs > 0 ? hrActs.reduce((t, a) => t + a.hr! * a.movingS, 0) / hrSecs : null,
    weekly,
    days: acts.map(a => ({ date: a.date, items: [{ activityId: a.id, sport: a.sport, km: a.km,
      postStatus: mockPosts.get(a.id)?.status ?? (a.id === 'm1' ? 'published' : a.gps ? 'ready' : 'none'),
      title: mockTitle(a), startTime: `${String(a.hour).padStart(2, '0')}:10`, movingS: a.movingS,
      speedKmh: a.sport === 'cycling' ? a.km / a.movingS * 3600 : null,
      pace: a.sport === 'cycling' ? null : pace(a.movingS, a.km),
      coverCard: null, cardCount: 0 }] })),
  }
}
export const mockGoal: { cycling: number | null; running: number | null } = { cycling: 300, running: 50 }

export function mockDetail(id: string): ActivityDetail {
  const a = mockActivities.find(x => x.id === id) ?? mockActivities[0]
  const n = 400
  const center = [37.525, 126.93], r = a.sport === 'cycling' ? 0.03 : 0.008
  const pts: [number, number][] = Array.from({ length: n }, (_, i) => {
    const t = (i / (n - 1)) * Math.PI * 2
    return [center[0] + r * Math.sin(t) * 0.8, center[1] + r * (1 - Math.cos(t)) * 1.3]
  })
  const speed = a.km / a.movingS * 3600
  return {
    id: a.id, sport: a.sport,
    title: mockTitle(a),
    date: a.date.replaceAll('-', '.'),
    metrics: { distanceKm: a.km, moving: hms(a.movingS), elapsed: hms(a.movingS * 1.1),
      pace: a.sport === 'running' ? pace(a.movingS, a.km) : null,
      speedKmh: a.sport === 'cycling' ? speed : null,
      avgHr: a.hr, maxHr: a.hr ? a.hr + 25 : null, ascentM: 120, avgCadence: a.sport === 'running' ? 172 : null },
    route: a.gps ? pts : [], start: a.gps ? pts[0] : null, end: a.gps ? pts[n - 1] : null,
    photos: a.gps ? [{ id: 1, fileName: 'mock.jpg', path: '', cardPath: null, takenAt: null, kmMark: a.km / 3,
      lat: pts[130][0], lon: pts[130][1] }] : [],
    series: {
      distanceKm: pts.map((_, i) => a.km * i / (n - 1)),
      hr: pts.map((_, i) => Math.round((a.hr ?? 140) + 12 * Math.sin(i / 25))),
      speedKmh: pts.map((_, i) => speed + 3 * Math.sin(i / 17)),
      altitudeM: pts.map((_, i) => 20 + 15 * Math.sin(i / 60)),
      latLon: a.gps ? pts : pts.map(() => null),
    },
    splits: Array.from({ length: Math.ceil(a.km) }, (_, i) => ({ km: i + 1, duration: hms(a.movingS / a.km), avgHr: a.hr })),
    fitFile: `/mock/files/${a.date.replaceAll('-', '')}/activity.fit`, folder: `/mock/files/${a.date.replaceAll('-', '')}`,
  }
}

export function mockPost(id: string): PostView {
  const existing = mockPosts.get(id)
  if (existing) return existing
  const d = mockDetail(id)
  const body = `${d.title.replace(/ (러닝|라이딩)$/, '')} ${d.sport === 'cycling' ? Math.round(d.metrics.distanceKm) : d.metrics.distanceKm.toFixed(1)}km ${d.sport === 'cycling' ? '🚲' : '🏃'}\n` +
    (d.sport === 'cycling' ? `평속 ${d.metrics.speedKmh!.toFixed(1)}km/h` : `페이스 ${d.metrics.pace}/km`) +
    ` / 이동 ${d.metrics.moving} / 평균 심박 ${d.metrics.avgHr}\n다들 이번 주말 어디로 달렸음?`
  const hashtags = d.sport === 'cycling' ? ['#라이딩', '#자전거'] : ['#러닝', '#러닝기록']
  return { activityId: id, body, hashtags, fullText: `${body}\n\n${hashtags.join(' ')}`, place: null,
    cardPaths: [], folder: '/mock/files', status: id === 'm1' ? 'published' : 'ready', edited: false,
    routePoints: [], weather: null, placeUserSet: false, routeUserSet: false, weatherUserSet: false, enrichError: null }
}
