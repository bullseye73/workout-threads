// 드라이브 백업 상태를 한 줄로.
import type { BackupStatusDto } from './bindings/BackupStatusDto'

const pad = (n: number) => String(n).padStart(2, '0')
const dayKey = (d: Date) => `${d.getFullYear()}-${d.getMonth()}-${d.getDate()}`

export function backupStatusText(s: BackupStatusDto, now: Date): string {
  if (!s.configured) return '드라이브 백업이 꺼져 있어요'
  if (!s.signedIn) return '구글 로그인이 필요해요'
  if (s.lastError) return `백업 실패: ${s.lastError}`
  if (!s.lastSuccess) return '아직 백업 전이에요'
  const t = new Date(s.lastSuccess)
  const min = Math.floor((now.getTime() - t.getTime()) / 60000)
  if (min < 1) return '마지막 백업 방금'
  if (min < 60) return `마지막 백업 ${min}분 전`
  if (dayKey(t) === dayKey(now)) return `마지막 백업 ${Math.floor(min / 60)}시간 전`
  const time = `${pad(t.getHours())}:${pad(t.getMinutes())}`
  const yesterday = new Date(now)
  yesterday.setDate(now.getDate() - 1)
  if (dayKey(t) === dayKey(yesterday)) return `마지막 백업 어제 ${time}`
  return `마지막 백업 ${t.getMonth() + 1}월 ${t.getDate()}일 ${time}`
}
