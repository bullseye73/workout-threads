<script lang="ts">
  // 작업대: 상단 줄 + 한 달 사진 띠 + 운동 목록 + 상세 패널을 한 화면에.
  import ActivityPane from '../detail/ActivityPane.svelte'
  import { api } from '$lib/api'
  import type { BackupStatusDto } from '$lib/bindings/BackupStatusDto'
  import type { MonthView } from '$lib/bindings/MonthView'
  import { filterGroups, groupByWeek, pickSelection, ribbonDays, type ListFilter } from '$lib/workbench'
  import ActivityList from './ActivityList.svelte'
  import MonthRibbon from './MonthRibbon.svelte'
  import TopBar from './TopBar.svelte'

  let { year, month, reloadKey = 0, focus = null, onMonth, onCompose, onUnmatched, onSettings }: {
    year: number; month: number; reloadKey?: number
    /** 이 운동을 고르고 그 탭을 연다 (게시물 작성 후). n이 바뀔 때마다. */
    focus?: { id: string; tab: 'record' | 'post'; n: number } | null
    onMonth: (y: number, m: number) => void
    onCompose: () => void
    onUnmatched: () => void; onSettings: () => void
  } = $props()

  let view = $state<MonthView | null>(null)
  let error = $state('')
  let unmatched = $state(0)
  let backup = $state<BackupStatusDto | null>(null)
  let filter = $state<ListFilter>('all')
  let selected = $state<string | null>(null)
  let pane = $state<{ requestLeave: () => Promise<boolean> } | null>(null)
  let paneTab = $state<'record' | 'post'>('record')
  let handledFocus = 0
  /** 게시물 작성에서 연 운동은 이미 골라져 있어도 패널을 새로 열어 요청한 탭으로. */
  let paneOpenN = $state(0)

  $effect(() => {
    let off: (() => void) | undefined
    api.onBackupStatus(s => (backup = s)).then(f => (off = f))
    return () => off?.()
  })

  $effect(() => {
    void reloadKey
    error = ''
    api.monthView(year, month).then(v => (view = v)).catch(e => (error = String(e)))
    api.unmatchedPhotos().then(p => (unmatched = p.length)).catch(() => (unmatched = 0))
    api.backupStatus().then(s => (backup = s)).catch(() => undefined)
  })

  const todayIso = new Date().toLocaleDateString('sv-SE')
  const allGroups = $derived(view ? groupByWeek(view.days) : [])
  const groups = $derived(filterGroups(allGroups, filter))
  const ribbon = $derived(view ? ribbonDays(year, month, view.days, todayIso) : [])
  const selectedItem = $derived(allGroups.flatMap(g => g.items).find(i => i.activityId === selected))
  const readyCount = $derived(allGroups.reduce((t, g) => t + g.items.filter(i => i.postStatus === 'ready').length, 0))

  // 달을 바꾸거나 목록이 바뀌면 남아 있는 선택을 지키고, 없으면 가장 최근 운동.
  $effect(() => {
    const next = pickSelection(allGroups, selected)
    if (next !== selected) select(next)
  })

  function select(id: string | null) {
    selected = id
  }
  /** 사용자가 고를 때만: 게시 탭의 저장 안 한 수정이 있으면 먼저 묻는다. */
  async function choose(id: string) {
    if (id === selected || (pane && !(await pane.requestLeave()))) return
    paneTab = 'record'
    select(id)
  }
  // 게시물 작성 후 연 운동: 목록에 나타나면 고르고 그 탭으로.
  $effect(() => {
    const f = focus
    if (!f || f.n === handledFocus) return
    if (allGroups.some(g => g.items.some(i => i.activityId === f.id))) {
      handledFocus = f.n
      void openFocused(f)
    }
  })
  /** 게시 탭에 저장 안 한 수정이 있으면 먼저 묻고, 그 운동의 요청한 탭으로 연다. */
  async function openFocused(f: { id: string; tab: 'record' | 'post' }) {
    if (pane && !(await pane.requestLeave())) return
    paneTab = f.tab
    paneOpenN++
    select(f.id)
  }
  async function shift(delta: number) {
    if (pane && !(await pane.requestLeave())) return
    const d = new Date(year, month - 1 + delta, 1)
    onMonth(d.getFullYear(), d.getMonth() + 1)
  }
  async function saveGoal(cyclingKm: number | null, runningKm: number | null) {
    await api.setGoals(cyclingKm, runningKm)
    view = await api.monthView(year, month)
  }
  async function refresh() {
    view = await api.monthView(year, month)
  }
</script>

<div class="grid h-dvh grid-rows-[auto_auto_1fr] max-md:h-auto">
  <TopBar {year} {month} {view} {backup} {unmatched} onShift={shift} onSaveGoal={saveGoal}
    {onCompose} {onUnmatched} {onSettings} />
  <MonthRibbon days={ribbon} {month} {selected} onSelect={choose} />

  {#if error}
    <p class="p-4 text-destructive" role="alert">기록을 불러오지 못했어요: {error}</p>
  {:else}
    <div class="grid min-h-0 grid-cols-[330px_1fr] max-md:grid-cols-1">
      <aside class="overflow-y-auto border-r bg-card max-md:border-r-0 max-md:border-b" aria-label="운동 목록">
        <ActivityList {groups} {filter} {readyCount} {selected} onFilter={f => (filter = f)} onSelect={choose} />
      </aside>
      <main class="min-w-0 overflow-y-auto p-5 max-sm:p-4">
        {#if selected}
          {#key `${selected}:${paneOpenN}`}
            <ActivityPane bind:this={pane} id={selected} status={selectedItem?.postStatus ?? 'none'} initialTab={paneTab} {reloadKey}
              onChanged={() => void refresh()} />
          {/key}
        {:else if view}
          <p class="text-muted-foreground">{view.days.length ? '왼쪽에서 운동을 고르세요.' : '이번 달 기록이 없어요. 게시물 작성에서 FIT와 사진을 올려 보세요.'}</p>
        {/if}
      </main>
    </div>
  {/if}
</div>
