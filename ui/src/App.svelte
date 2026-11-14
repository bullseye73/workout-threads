<script lang="ts">
  import ActivityDetail from './components/ActivityDetail.svelte'
  import Dashboard from './components/Dashboard.svelte'
  import ImportPanel from './components/ImportPanel.svelte'
  import PostPanel from './components/PostPanel.svelte'
  import Settings from './components/Settings.svelte'
  import { ModeWatcher } from 'mode-watcher'
  import ThemeToggle from '$lib/components/ThemeToggle.svelte'
  import { api, isTauri } from './lib/api'
  import type { ImportReport } from './lib/bindings/ImportReport'

  type Screen = { name: 'dashboard' } | { name: 'activity'; id: string } | { name: 'post'; id: string }
  const now = new Date()
  let year = $state(now.getFullYear())
  let month = $state(now.getMonth() + 1)
  let screen = $state<Screen>({ name: 'dashboard' })
  let reloadKey = $state(0)
  let report = $state<ImportReport | null>(null)
  let importing = $state(false)
  let importError = $state('')
  let showSettings = $state(false)
  let enriching = $state('')

  async function runImport(paths: string[]) {
    if (!paths.length) return
    importing = true; importError = ''
    try {
      panelTitle = '가져오기 결과'
      report = await api.importPaths(paths)
      reloadKey++
      void enrichNew(report.addedActivities)
    } catch (e) {
      importError = `가져오지 못했어요: ${e}`
    } finally {
      importing = false
    }
  }

  // 새로 가져온 운동의 장소·날씨를 하나씩 채운다 (인터넷 조회, 실패해도 계속).
  async function enrichNew(ids: string[]) {
    for (const [i, id] of ids.entries()) {
      enriching = `장소·날씨 채우는 중 ${i + 1}/${ids.length}`
      await api.enrichActivity(id).catch(() => undefined)
    }
    enriching = ''
    if (ids.length) reloadKey++
  }

  // 예전에 가져와 아직 운동에 붙지 않은 사진을 다시 보여 준다.
  let panelTitle = $state('가져오기 결과')
  async function showUnmatched() {
    const photos = await api.unmatchedPhotos()
    panelTitle = '운동에 안 붙은 사진'
    report = { addedActivities: [], duplicateFiles: [], failed: [], cardsCreated: 0, unmatchedPhotos: photos }
  }

  // 창에 파일·폴더를 끌어다 놓으면 가져온다 (Tauri 안에서만).
  $effect(() => {
    if (!isTauri()) return
    let unlisten: (() => void) | undefined
    import('@tauri-apps/api/webview').then(({ getCurrentWebview }) =>
      getCurrentWebview().onDragDropEvent(e => { if (e.payload.type === 'drop') runImport(e.payload.paths) })
    ).then(u => (unlisten = u))
    return () => unlisten?.()
  })
</script>

<ModeWatcher />
<main>
  <div class="flex justify-end mb-2"><ThemeToggle /></div>
  {#if importing}<p class="notice" role="status">가져오는 중… 사진이 많으면 카드를 만드는 데 시간이 걸려요.</p>{/if}
  {#if enriching}<p class="notice" role="status">{enriching}</p>{/if}
  {#if importError}<p class="notice error" role="alert">{importError}</p>{/if}
  {#if screen.name === 'dashboard'}
    <Dashboard {year} {month} {reloadKey} onMonth={(y, m) => { year = y; month = m }}
      onOpen={id => (screen = { name: 'activity', id })} onImport={async kind => runImport(await api.pickImport(kind))}
      onUnmatched={showUnmatched} onSettings={() => (showSettings = true)} />
  {:else if screen.name === 'activity'}
    <ActivityDetail id={screen.id} onBack={() => (screen = { name: 'dashboard' })}
      onPost={id => (screen = { name: 'post', id })} />
  {:else}
    {@const id = screen.id}
    <PostPanel {id} onBack={() => { screen = { name: 'activity', id }; reloadKey++ }} />
  {/if}
  {#if showSettings}<Settings onClose={() => { showSettings = false; reloadKey++ }} />{/if}
  {#if report}<ImportPanel {report} title={panelTitle} onClose={() => { report = null; reloadKey++ }} />{/if}
</main>

<style>
  main { max-width: 1120px; margin: 0 auto; padding: 28px 24px 48px; }
  .notice { background: var(--surface); border-left: 3px solid var(--ride); padding: 8px 12px; border-radius: 6px; }
  .notice.error { border-color: #b42318; color: #b42318; }
  @media (max-width: 600px) { main { padding: 16px; } }
</style>
