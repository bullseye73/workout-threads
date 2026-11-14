<script lang="ts">
  import ImportPanel from './components/ImportPanel.svelte'
  import Settings from './components/Settings.svelte'
  import { ModeWatcher } from 'mode-watcher'
  import Workbench from './components/workbench/Workbench.svelte'
  import ConfirmHost from '$lib/components/ConfirmHost.svelte'
  import { installScrollReveal } from '$lib/scrollReveal'
  import { api, isTauri } from './lib/api'
  import type { ImportReport } from './lib/bindings/ImportReport'

  const now = new Date()
  let year = $state(now.getFullYear())
  let month = $state(now.getMonth() + 1)
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

  $effect(() => installScrollReveal())

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
<ConfirmHost />
<main>
  {#if importing || enriching || importError}
    <div class="fixed inset-x-0 bottom-0 z-40 flex flex-col gap-1 p-3">
      {#if importing}<p class="notice" role="status">가져오는 중… 사진이 많으면 카드를 만드는 데 시간이 걸려요.</p>{/if}
      {#if enriching}<p class="notice" role="status">{enriching}</p>{/if}
      {#if importError}<p class="notice error" role="alert">{importError}</p>{/if}
    </div>
  {/if}
  <Workbench {year} {month} {reloadKey} onMonth={(y, m) => { year = y; month = m }}
    onImport={async kind => runImport(await api.pickImport(kind))}
    onUnmatched={showUnmatched} onSettings={() => (showSettings = true)} />
  {#if showSettings}<Settings onClose={() => { showSettings = false; reloadKey++ }} />{/if}
  {#if report}<ImportPanel {report} title={panelTitle} onClose={() => { report = null; reloadKey++ }} />{/if}
</main>

<style>
  .notice { margin: 0; background: var(--card); border: 1px solid var(--border); border-left: 3px solid var(--ride); padding: 8px 12px; border-radius: 8px; box-shadow: 0 4px 16px -8px color-mix(in srgb, var(--foreground) 40%, transparent); }
  .notice.error { border-left-color: var(--destructive); color: var(--destructive); }
</style>
