<script lang="ts">
  // 상단 한 줄: 달 이동, 목표 막대(누르면 목표 바꾸기), 종목별 합계, 테마·설정·가져오기.
  import ChevronDown from '@lucide/svelte/icons/chevron-down'
  import ChevronLeft from '@lucide/svelte/icons/chevron-left'
  import ChevronRight from '@lucide/svelte/icons/chevron-right'
  import ThemeToggle from '$lib/components/ThemeToggle.svelte'
  import { Button } from '$lib/components/ui/button'
  import * as Dialog from '$lib/components/ui/dialog'
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu'
  import { Input } from '$lib/components/ui/input'
  import { Label } from '$lib/components/ui/label'
  import { backupStatusText } from '$lib/backup'
  import type { BackupStatusDto } from '$lib/bindings/BackupStatusDto'
  import type { MonthView } from '$lib/bindings/MonthView'
  import { duration, km } from '$lib/format'
  import { goalProgress } from '$lib/workbench'

  let { year, month, view, backup, unmatched, onShift, onSaveGoal, onImport, onUnmatched, onSettings }: {
    year: number; month: number; view: MonthView | null; backup: BackupStatusDto | null; unmatched: number
    onShift: (delta: number) => void; onSaveGoal: (km: number | null) => void
    onImport: (kind: 'files' | 'folder') => void; onUnmatched: () => void; onSettings: () => void
  } = $props()

  const p = $derived(view ? goalProgress(view, new Date()) : null)
  let goalOpen = $state(false)
  let goalInput = $state('')

  function openGoal() {
    goalInput = view?.goalKm ? String(view.goalKm) : ''
    goalOpen = true
  }
  function saveGoal(e: SubmitEvent) {
    e.preventDefault()
    const n = Number(goalInput)
    onSaveGoal(goalInput.trim() && n > 0 ? n : null)
    goalOpen = false
  }
</script>

<header class="flex flex-wrap items-center gap-x-5 gap-y-2 border-b bg-card px-4 py-2.5">
  <div class="flex items-center">
    <Button variant="ghost" size="icon-sm" onclick={() => onShift(-1)} aria-label="이전 달"><ChevronLeft /></Button>
    <h1 class="mx-1 text-[17px] font-bold tracking-tight">{year}년 {month}월</h1>
    <Button variant="ghost" size="icon-sm" onclick={() => onShift(1)} aria-label="다음 달"><ChevronRight /></Button>
  </div>

  {#if view}
    <div class="flex min-w-0 flex-1 flex-wrap items-center gap-x-4 gap-y-1 max-md:order-3 max-md:basis-full">
      <button class="flex items-center gap-2.5 rounded-md px-1 py-0.5 hover:bg-muted" onclick={openGoal}
        title="목표 바꾸기. 세로선은 오늘까지 채워야 할 거리예요">
        <b class="whitespace-nowrap text-xl tracking-tight">{km(view.totalKm)}<small class="ml-0.5 text-xs font-semibold text-muted-foreground">{view.goalKm ? `/ ${view.goalKm}km` : 'km'}</small></b>
        {#if p && view.goalKm}
          <span class="relative flex h-1.5 w-40 overflow-hidden rounded-full bg-sunken max-sm:w-24" role="img"
            aria-label="목표 {view.goalKm}km 중 {Math.round((view.totalKm / view.goalKm) * 100)}%">
            <i class="block bg-ride" style:width="{p.rideRatio * 100}%"></i>
            <i class="block bg-run" style:width="{p.runRatio * 100}%"></i>
            {#if p.paceRatio !== null}<i class="absolute inset-y-0 w-0.5 bg-foreground" style:left="{p.paceRatio * 100}%"></i>{/if}
          </span>
          {#if p.perDayKm !== null}<span class="text-xs text-muted-foreground max-sm:hidden">하루 {km(p.perDayKm)}km면 목표</span>{/if}
        {:else}
          <span class="text-xs text-muted-foreground">목표 정하기</span>
        {/if}
      </button>
      <div class="flex gap-3.5 text-xs text-muted-foreground max-lg:hidden">
        <span><i class="mr-1 inline-block size-2 rounded-[2px] bg-ride"></i>라이딩 <b class="text-[13px] text-foreground">{view.cycling.count}회 {km(view.cycling.km)}km</b></span>
        <span><i class="mr-1 inline-block size-2 rounded-[2px] bg-run"></i>러닝 <b class="text-[13px] text-foreground">{view.running.count}회 {km(view.running.km)}km</b></span>
        <span>시간 <b class="text-[13px] text-foreground">{duration(view.totalMovingS)}</b></span>
        {#if view.avgHr}<span>심박 <b class="text-[13px] text-foreground">{Math.round(view.avgHr)}</b></span>{/if}
      </div>
    </div>
  {/if}

  <div class="ml-auto flex items-center gap-1.5">
    {#if backup?.configured}
      <button class="flex items-center gap-1.5 text-xs max-sm:hidden {backup.lastError || !backup.signedIn ? 'text-destructive' : 'text-muted-foreground'}"
        onclick={onSettings}>
        <i class="size-1.5 rounded-full {backup.lastError || !backup.signedIn ? 'bg-destructive' : 'bg-done'}"></i>{backupStatusText(backup, new Date())}
      </button>
    {/if}
    {#if unmatched > 0}<Button variant="outline" size="sm" onclick={onUnmatched}>안 붙은 사진 {unmatched}장</Button>{/if}
    <ThemeToggle />
    <Button variant="ghost" size="sm" onclick={onSettings}>설정</Button>
    <DropdownMenu.Root>
      <DropdownMenu.Trigger>
        {#snippet child({ props })}<Button {...props} size="sm">가져오기 <ChevronDown /></Button>{/snippet}
      </DropdownMenu.Trigger>
      <DropdownMenu.Content align="end">
        <DropdownMenu.Item onSelect={() => onImport('files')}>파일 가져오기</DropdownMenu.Item>
        <DropdownMenu.Item onSelect={() => onImport('folder')}>폴더 가져오기</DropdownMenu.Item>
      </DropdownMenu.Content>
    </DropdownMenu.Root>
  </div>
</header>

<Dialog.Root bind:open={goalOpen}>
  <Dialog.Content class="sm:max-w-xs">
    <Dialog.Header><Dialog.Title>월 목표</Dialog.Title>
      <Dialog.Description>비워 두면 목표 없이 거리만 보여 줘요.</Dialog.Description></Dialog.Header>
    <form class="grid gap-3" onsubmit={saveGoal}>
      <Label for="goal">거리 (km)</Label>
      <Input id="goal" type="number" min="0" step="10" placeholder="없음" bind:value={goalInput} />
      <Dialog.Footer><Button type="submit">목표 저장</Button></Dialog.Footer>
    </form>
  </Dialog.Content>
</Dialog.Root>
