// 화면이 쓰는 데이터 접근 계층. Tauri 안에서는 Rust 명령을, 브라우저에서는 가짜 데이터를 쓴다.
import type { ActivityDetail } from './bindings/ActivityDetail'
import type { ImportReport } from './bindings/ImportReport'
import type { MonthView } from './bindings/MonthView'
import type { PhotoDto } from './bindings/PhotoDto'
import type { PostView } from './bindings/PostView'
import type { RefineStatusDto } from './bindings/RefineStatusDto'
import type { RefineDto } from './bindings/RefineDto'
import type { SettingsDto } from './bindings/SettingsDto'
import type { WeatherDto } from './bindings/WeatherDto'
import type { BackupStatusDto } from './bindings/BackupStatusDto'
import type { GoogleClientDto } from './bindings/GoogleClientDto'
import { mockDetail, mockGoal, mockMonth, mockPost, mockPosts } from './mock'

export interface Api {
  importPaths(paths: string[]): Promise<ImportReport>
  importForDate(date: string, paths: string[]): Promise<ImportReport>
  monthView(year: number, month: number): Promise<MonthView>
  activityDetail(id: string): Promise<ActivityDetail>
  postView(id: string): Promise<PostView>
  savePost(id: string, body: string, hashtags: string[]): Promise<PostView>
  regeneratePost(id: string): Promise<PostView>
  markPublished(id: string): Promise<PostView>
  assignPhoto(photoId: number, activityId: string): Promise<PhotoDto>
  unmatchedPhotos(): Promise<PhotoDto[]>
  enrichActivity(id: string): Promise<PostView>
  setPlace(id: string, name: string | null): Promise<PostView>
  setRoutePoints(id: string, points: string[]): Promise<PostView>
  setWeather(id: string, weather: WeatherDto | null): Promise<PostView>
  /** 화면에 있는 글(저장 전 수정 포함)을 다듬는다. 저장하지 않는다. */
  refinePost(id: string, body: string, hashtags: string[], includePhotos: boolean): Promise<RefineDto>
  /** 글 다듬기 AI 상태 (키 값은 돌려주지 않음). */
  refineStatus(): Promise<RefineStatusDto>
  setRefineProvider(provider: string): Promise<void>
  setRefineKey(provider: string, key: string | null): Promise<void>
  setRefineModel(provider: string, model: string | null): Promise<void>
  setCardRouteTrim(on: boolean): Promise<void>
  settings(): Promise<SettingsDto>
  regenerateCards(id: string): Promise<number>
  setGoogleClient(clientId: string | null, clientSecret: string | null): Promise<void>
  googleClient(): Promise<GoogleClientDto | null>
  /** 시스템 브라우저로 구글 로그인을 열고 돌아올 때까지 기다린다 (최대 5분). */
  googleLogin(): Promise<BackupStatusDto>
  googleLogout(): Promise<void>
  backupStatus(): Promise<BackupStatusDto>
  backupNow(): Promise<BackupStatusDto>
  restoreFromDrive(): Promise<BackupStatusDto>
  /** 백그라운드 백업이 끝날 때마다 불린다. 해제 함수를 돌려준다. */
  onBackupStatus(cb: (s: BackupStatusDto) => void): Promise<() => void>
  setGoalKm(km: number | null): Promise<void>
  fileUrl(absPath: string): string
  copyText(text: string): Promise<void>
  openFolder(absPath: string): Promise<void>
  /** 파일 여러 개 또는 폴더 여러 개를 고른다 (macOS는 한 창에서 둘을 함께 고를 수 없다). */
  pickImport(kind: 'files' | 'folder'): Promise<string[]>
}

export function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke<T>(cmd, args)
}

export const tauriApi: Api = {
  importPaths: paths => invoke('import_paths', { paths }),
  importForDate: (date, paths) => invoke('import_for_date', { date, paths }),
  monthView: (year, month) => invoke('month_view', { year, month }),
  activityDetail: id => invoke('activity_detail', { id }),
  postView: activityId => invoke('post_view', { activityId }),
  savePost: (activityId, body, hashtags) => invoke('save_post', { activityId, body, hashtags }),
  regeneratePost: activityId => invoke('regenerate_post', { activityId }),
  markPublished: activityId => invoke('mark_published', { activityId }),
  assignPhoto: (photoId, activityId) => invoke('assign_photo', { photoId, activityId }),
  unmatchedPhotos: () => invoke('unmatched_photos'),
  enrichActivity: id => invoke('enrich_activity', { id }),
  setPlace: (activityId, name) => invoke('set_place', { activityId, name }),
  setRoutePoints: (activityId, points) => invoke('set_route_points', { activityId, points }),
  setWeather: (activityId, weather) => invoke('set_weather', { activityId, weather }),
  refinePost: (activityId, body, hashtags, includePhotos) => invoke('refine_post', { activityId, body, hashtags, includePhotos }),
  refineStatus: () => invoke('refine_status'),
  setRefineProvider: provider => invoke('set_refine_provider', { provider }),
  setRefineKey: (provider, key) => invoke('set_refine_key', { provider, key }),
  setRefineModel: (provider, model) => invoke('set_refine_model', { provider, model }),
  setCardRouteTrim: on => invoke('set_card_route_trim', { on }),
  settings: () => invoke('settings'),
  regenerateCards: activityId => invoke('regenerate_cards', { activityId }),
  setGoogleClient: (clientId, clientSecret) => invoke('set_google_client', { clientId, clientSecret }),
  googleClient: () => invoke('google_client'),
  googleLogin: () => invoke('google_login'),
  googleLogout: () => invoke('google_logout'),
  backupStatus: () => invoke('backup_status'),
  backupNow: () => invoke('backup_now'),
  restoreFromDrive: () => invoke('restore_from_drive'),
  onBackupStatus: async cb => {
    const { listen } = await import('@tauri-apps/api/event')
    return listen<BackupStatusDto>('backup-status', e => cb(e.payload))
  },
  setGoalKm: km => invoke('set_goal_km', { km }),
  fileUrl: p => {
    // convertFileSrc는 동기 함수라 전역 내부 객체를 직접 쓴다 (@tauri-apps/api/core와 같은 구현).
    const w = window as unknown as { __TAURI_INTERNALS__: { convertFileSrc(p: string, proto: string): string } }
    return w.__TAURI_INTERNALS__.convertFileSrc(p, 'asset')
  },
  copyText: async text => (await import('@tauri-apps/plugin-clipboard-manager')).writeText(text),
  openFolder: async p => (await import('@tauri-apps/plugin-opener')).openPath(p),
  pickImport: async kind => {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const picked = kind === 'folder'
      ? await open({ multiple: true, directory: true })
      : await open({ multiple: true, directory: false,
          filters: [{ name: '운동 기록·사진', extensions: ['fit', 'FIT', 'jpg', 'jpeg', 'png', 'JPG', 'JPEG', 'PNG'] }] })
    return picked === null ? [] : Array.isArray(picked) ? picked : [picked]
  },
}

const mockBackup: { client: GoogleClientDto | null; status: BackupStatusDto } = {
  client: null,
  status: { configured: false, signedIn: false, lastSuccess: null, lastError: null, pending: 3, canRestore: false },
}
const mockRefine = {
  provider: 'gemini',
  providers: [
    { id: 'gemini', label: 'Gemini', company: 'Google', hasKey: false, model: 'gemini-3.8-flash' },
    { id: 'claude', label: 'Claude', company: 'Anthropic', hasKey: false, model: 'claude-opus-5-5' },
  ],
}
function mockRefineStatus(): RefineStatusDto {
  const cur = mockRefine.providers.find(p => p.id === mockRefine.provider)!
  return { provider: cur.id, label: cur.label, company: cur.company, hasKey: cur.hasKey, model: cur.model,
    providers: mockRefine.providers.map(p => ({ ...p })) }
}
const mockSettings = { cardRouteTrim: false }

const delay = <T>(v: T) => new Promise<T>(r => setTimeout(() => r(v), 50))

export const mockApi: Api = {
  importPaths: async paths => delay({ addedActivities: paths.length ? ['m3'] : [], duplicateFiles: [], failed: [],
    cardsCreated: paths.length ? 2 : 0, unmatchedPhotos: [], skipped: [] }),
  importForDate: async (_date, paths) => delay({ addedActivities: paths.length ? ['m4'] : [], duplicateFiles: [], failed: [],
    cardsCreated: paths.length ? 1 : 0, unmatchedPhotos: [], skipped: [{ file: 'IMG_0412.jpeg', reason: '11월 1일에 찍었어요' }] }),
  monthView: async (y, m) => delay(mockMonth(y, m)),
  activityDetail: async id => delay(mockDetail(id)),
  postView: async id => delay(mockPost(id)),
  savePost: async (id, body, hashtags) => {
    const p = { ...mockPost(id), body, hashtags, edited: true,
      fullText: hashtags.length ? `${body}\n\n${hashtags.join(' ')}` : body }
    mockPosts.set(id, p)
    return delay(p)
  },
  regeneratePost: async id => { mockPosts.delete(id); return delay(mockPost(id)) },
  markPublished: async id => { const p = { ...mockPost(id), status: 'published' as const }; mockPosts.set(id, p); return delay(p) },
  assignPhoto: async (photoId) => delay({ id: photoId, fileName: 'mock.jpg', path: '', cardPath: null, takenAt: null,
    kmMark: null, lat: null, lon: null }),
  setGoalKm: async km => { mockGoal.value = km; await delay(undefined) },
  fileUrl: p => p,
  copyText: async text => { await navigator.clipboard?.writeText(text).catch(() => undefined) },
  openFolder: async () => undefined,
  unmatchedPhotos: async () => delay([]),
  enrichActivity: async id => {
    const p = mockPost(id)
    const next = { ...p, place: p.place ?? { name: '가짜공원', estimated: true },
      routePoints: p.routeUserSet ? p.routePoints : ['가짜동'],
      weather: p.weather ?? { description: '맑음', tempC: 17 }, enrichError: null }
    mockPosts.set(id, next)
    return delay(next)
  },
  setPlace: async (id, name) => { const p = { ...mockPost(id), place: name ? { name, estimated: false } : null, placeUserSet: !!name }; mockPosts.set(id, p); return delay(p) },
  setRoutePoints: async (id, points) => { const p = { ...mockPost(id), routePoints: points, routeUserSet: points.length > 0 }; mockPosts.set(id, p); return delay(p) },
  setWeather: async (id, weather) => { const p = { ...mockPost(id), weather, weatherUserSet: !!weather }; mockPosts.set(id, p); return delay(p) },
  refinePost: async (_id, text, hashtags) => {
    const body = text.replace('다들', '오늘도 잘 달렸음. 다들')
    const tags = [...hashtags, '#가짜공원']
    return delay({ body, hashtags: tags, fullText: `${body}\n\n${tags.join(' ')}` })
  },
  refineStatus: async () => delay(mockRefineStatus()),
  setRefineProvider: async p => { mockRefine.provider = p; await delay(undefined) },
  setRefineKey: async (p, key) => { mockRefine.providers.find(x => x.id === p)!.hasKey = !!key; await delay(undefined) },
  setRefineModel: async (p, model) => {
    const x = mockRefine.providers.find(y => y.id === p)!
    x.model = model ?? (p === 'gemini' ? 'gemini-3.8-flash' : 'claude-opus-5-5')
    await delay(undefined)
  },
  setCardRouteTrim: async on => { mockSettings.cardRouteTrim = on; await delay(undefined) },
  settings: async () => delay({ ...mockSettings }),
  regenerateCards: async () => delay(2),
  setGoogleClient: async (id, secret) => { mockBackup.client = id ? { clientId: id, hasSecret: !!secret } : null; mockBackup.status.configured = !!(id && secret); await delay(undefined) },
  googleClient: async () => delay(mockBackup.client),
  googleLogin: async () => { mockBackup.status = { ...mockBackup.status, signedIn: true }; return delay({ ...mockBackup.status }) },
  googleLogout: async () => { mockBackup.status = { ...mockBackup.status, signedIn: false }; await delay(undefined) },
  backupStatus: async () => delay({ ...mockBackup.status }),
  backupNow: async () => { mockBackup.status = { ...mockBackup.status, lastSuccess: new Date().toISOString(), lastError: null, pending: 0 }; return delay({ ...mockBackup.status }) },
  restoreFromDrive: async () => delay({ ...mockBackup.status, canRestore: false }),
  onBackupStatus: async () => () => undefined,
  pickImport: async kind => (kind === 'folder' ? ['/mock/20261025'] : ['/mock/20261025/a.fit']),
}

export const api: Api = isTauri() ? tauriApi : mockApi
