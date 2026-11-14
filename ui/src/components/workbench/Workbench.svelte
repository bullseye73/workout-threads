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

  let { year, month, reloadKey = 0, onMonth, onImport, onUnmatched, onSettings }: {
    year: number; month: number; reloadKey?: number; onMonth: (y: number, m: number) => void
    onImport: (kind: 'files' | 'folder') => void; onUnmatched: () => void; onSettings: () => void
  } = $props()

  let view = $state<MonthView | null>(null)
  let error = $state('')
  let unmatched = $state(0)
  let backup = $state<BackupStatusDto | null>(null)
  let filter = $state<ListFilter>('all')
  let selected = $state<string | null>(null)
  let paneKey = $state(0)

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
  function shift(delta: number) {
    const d = new Date(year, month - 1 + delta, 1)
    onMonth(d.getFullYear(), d.getMonth() + 1)
  }
  async function saveGoal(km: number | null) {
    await api.setGoalKm(km)
    view = await api.monthView(year, month)
  }
  async function refresh() {
    view = await api.monthView(year, month)
    paneKey++
  }
</script>

<div class="grid h-dvh grid-rows-[auto_auto_1fr] max-md:h-auto">
  <TopBar {year} {month} {view} {backup} {unmatched} onShift={shift} onSaveGoal={saveGoal}
    {onImport} {onUnmatched} {onSettings} />
  <MonthRibbon days={ribbon} {month} {selected} onSelect={select} />

  {#if error}
    <p class="p-4 text-destructive" role="alert">기록을 불러오지 못했어요: {error}</p>
  {:else}
    <div class="grid min-h-0 grid-cols-[330px_1fr] max-md:grid-cols-1">
      <aside class="overflow-y-auto border-r bg-card max-md:border-r-0 max-md:border-b" aria-label="운동 목록">
        <ActivityList {groups} {filter} {readyCount} {selected} onFilter={f => (filter = f)} onSelect={select} />
      </aside>
      <main class="min-w-0 overflow-y-auto p-5 max-sm:p-4">
        {#if selected}
          {#key `${selected}-${paneKey}`}
            <ActivityPane id={selected} status={selectedItem?.postStatus ?? 'none'} onChanged={() => void refresh()} />
          {/key}
        {:else if view}
          <p class="text-muted-foreground">{view.days.length ? '왼쪽에서 운동을 고르세요.' : '이번 달 기록이 없어요.'}</p>
        {/if}
      </main>
    </div>
  {/if}
</div>
