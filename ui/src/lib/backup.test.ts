import { describe, it, expect } from 'vitest'
import { backupStatusText } from './backup'
import type { BackupStatusDto } from './bindings/BackupStatusDto'

const base: BackupStatusDto = { configured: true, signedIn: true, lastSuccess: null, lastError: null, pending: 0, canRestore: false }
const now = new Date('2026-11-14T12:00:00+09:00')

describe('backupStatusText', () => {
  it('not configured', () => expect(backupStatusText({ ...base, configured: false, signedIn: false }, now)).toBe('드라이브 백업이 꺼져 있어요'))
  it('needs login', () => expect(backupStatusText({ ...base, signedIn: false }, now)).toBe('구글 로그인이 필요해요'))
  it('error first', () => expect(backupStatusText({ ...base, lastError: '연결 끊김', lastSuccess: '2026-11-14T02:57:00Z' }, now)).toBe('백업 실패: 연결 끊김'))
  it('never backed up', () => expect(backupStatusText(base, now)).toBe('아직 백업 전이에요'))
  it('minutes ago', () => expect(backupStatusText({ ...base, lastSuccess: '2026-11-14T02:57:00Z' }, now)).toBe('마지막 백업 3분 전'))
  it('just now', () => expect(backupStatusText({ ...base, lastSuccess: '2026-11-14T02:59:40Z' }, now)).toBe('마지막 백업 방금'))
  it('hours ago', () => expect(backupStatusText({ ...base, lastSuccess: '2026-11-14T00:00:00Z' }, now)).toBe('마지막 백업 3시간 전'))
  it('yesterday', () => expect(backupStatusText({ ...base, lastSuccess: '2026-11-13T12:40:00Z' }, now)).toBe('마지막 백업 어제 21:40'))
  it('older', () => expect(backupStatusText({ ...base, lastSuccess: '2026-11-01T01:05:00Z' }, now)).toBe('마지막 백업 11월 1일 10:05'))
})
