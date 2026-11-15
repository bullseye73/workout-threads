<script lang="ts">
  import ImportPanel from './components/ImportPanel.svelte'
  import Settings from './components/Settings.svelte'
  import { ModeWatcher } from 'mode-watcher'
  import Workbench from './components/workbench/Workbench.svelte'
  import Composer from './components/Composer.svelte'
  import ConfirmHost from '$lib/components/ConfirmHost.svelte'
  import { installScrollReveal } from '$lib/scrollReveal'
  import { api, isTauri } from './lib/api'
  import type { ImportReport } from './lib/bindings/ImportReport'

  const now = new Date()
  let year = $state(now.getFullYear())
  let month = $state(now.getMonth() + 1)
  let reloadKey = $state(0)
  let report = $state<ImportReport | null>(null)
  let showSettings = $state(false)
  let enriching = $state('')

  // 게시물 작성 창: 열려 있을 때 끌어다 놓은 파일은 창으로 보낸다.
  let composing = $state(false)
  let incoming = $state<{ paths: string[]; n: number } | null>(null)
  function compose(paths: string[] = []) {
    incoming = paths.length ? { paths, n: (incoming?.n ?? 0) + 1 } : null
    composing = true
  }
  // 만든 게시물을 열 때: 그 달로 옮기고 그 운동의 게시 준비 탭을 연다.
  let focus = $state<{ id: string; tab: 'record' | 'post'; n: number } | null>(null)
  function openActivity(id: string, date: string) {
    composing = false
    if (date) { year = Number(date.slice(0, 4)); month = Number(date.slice(5, 7)) }
    focus = { id, tab: 'post', n: (focus?.n ?? 0) + 1 }
    reloadKey++
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
  let panelTitle = $state('운동에 안 붙은 사진')
  async function showUnmatched() {
    const photos = await api.unmatchedPhotos()
    panelTitle = '운동에 안 붙은 사진'
    report = { addedActivities: [], duplicateFiles: [], failed: [], cardsCreated: 0, unmatchedPhotos: photos, skipped: [] }
  }

  $effect(() => installScrollReveal())

  // 창에 파일·폴더를 끌어다 놓으면 게시물 작성 창으로 (Tauri 안에서만).
  $effect(() => {
    if (!isTauri()) return
    let unlisten: (() => void) | undefined
    import('@tauri-apps/api/webview').then(({ getCurrentWebview }) =>
      getCurrentWebview().onDragDropEvent(e => { if (e.payload.type === 'drop') compose(e.payload.paths) })
    ).then(u => (unlisten = u))
    return () => unlisten?.()
  })
</script>

<ModeWatcher />
<ConfirmHost />
<main>
  {#if enriching}
    <div class="fixed inset-x-0 bottom-0 z-40 flex flex-col gap-1 p-3">
      <p class="notice" role="status">{enriching}</p>
    </div>
  {/if}
  <Workbench {year} {month} {reloadKey} onMonth={(y, m) => { year = y; month = m }}
    {focus} onCompose={() => compose()}
    onUnmatched={showUnmatched} onSettings={() => (showSettings = true)} />
  {#if showSettings}<Settings onClose={() => { showSettings = false; reloadKey++ }} />{/if}
  {#if report}<ImportPanel {report} title={panelTitle} showSummary={false} onClose={() => { report = null; reloadKey++ }} />{/if}
  {#if composing}
    <Composer {incoming} onClose={() => { composing = false; incoming = null }} onOpenActivity={openActivity}
      onCreated={r => { reloadKey++; void enrichNew(r.created) }} />
  {/if}
</main>

<style>
  .notice { margin: 0; background: var(--card); border: 1px solid var(--border); border-left: 3px solid var(--ride); padding: 8px 12px; border-radius: 8px; box-shadow: 0 4px 16px -8px color-mix(in srgb, var(--foreground) 40%, transparent); }
</style>
