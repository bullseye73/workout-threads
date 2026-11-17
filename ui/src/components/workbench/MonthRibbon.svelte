<script lang="ts">
  // 한 달 사진 띠: 하루 한 칸. 운동한 날은 대표 카드, 아래 종목 띠, 오른쪽 위 게시 상태.
  import { api } from '$lib/api'
  import { STATUS_LABEL } from '$lib/theme'
  import type { RibbonDay } from '$lib/workbench'
  import { cn } from '$lib/utils'

  let { days, month, selected, onSelect }: {
    days: RibbonDay[]; month: number; selected: string | null; onSelect: (activityId: string) => void
  } = $props()

  let popoverDate = $state<string | null>(null)

  const DOT = { none: 'shadow-[inset_0_0_0_1.5px_var(--on-photo)]', ready: 'bg-wait', published: 'bg-done' } as const

  function handleDayClick(d: RibbonDay) {
    if (!d.items.length) return
    if (d.items.length === 1) {
      popoverDate = null
      onSelect(d.items[0].activityId)
      return
    }
    // 다중 운동인 경우: 팝오버 토글 및 다음 운동 순환
    const curIdx = d.items.findIndex(i => i.activityId === selected)
    if (curIdx >= 0) {
      const nextIdx = (curIdx + 1) % d.items.length
      onSelect(d.items[nextIdx].activityId)
    } else {
      onSelect(d.items[0].activityId)
    }
    popoverDate = popoverDate === d.date ? null : d.date
  }
</script>

{#if popoverDate}
  <button type="button" class="fixed inset-0 z-40 size-full cursor-default border-0 bg-transparent" onclick={() => popoverDate = null} aria-label="메뉴 닫기"></button>
{/if}

<nav class="relative grid auto-cols-[minmax(26px,1fr)] grid-flow-col gap-[3px] overflow-x-auto border-b bg-card px-4 pt-2 pb-2.5 max-md:auto-cols-[30px]"
  aria-label="{month}월 날짜별 운동">
  {#each days as d (d.date)}
    {@const first = d.items[0]}
    {#if first}
      {@const isSel = d.items.some(i => i.activityId === selected)}
      <div class="relative">
        <button class={cn('relative h-[54px] w-full overflow-hidden rounded-[5px] text-left cursor-pointer transition-shadow',
            first.coverCard ? 'bg-sunken' : first.sport === 'running' ? 'bg-run/20' : 'bg-ride/20',
            d.today && 'shadow-[inset_0_0_0_1.5px_var(--foreground)]',
            isSel && 'outline-2 outline-offset-1 outline-ride')}
          onclick={() => handleDayClick(d)} aria-current={isSel}
          aria-label="{month}월 {d.day}일 {first.title} {first.km.toFixed(1)}km {STATUS_LABEL[first.postStatus]}{d.items.length > 1 ? ` 외 ${d.items.length - 1}개 (클릭해 전환)` : ''}">
          {#if first.coverCard}
            <img src={api.fileUrl(first.coverCard)} alt="" loading="lazy" class="absolute inset-0 size-full object-cover" />
          {/if}
          <span class={cn('absolute top-0.5 left-1 z-10 text-[11px] font-semibold', first.coverCard ? 'text-on-photo [text-shadow:0_0_3px_rgb(0_0_0/.8)]' : 'text-foreground')}>{d.day}</span>
          <i class={cn('absolute top-[3px] right-[3px] z-10 size-[7px] rounded-full', DOT[first.postStatus])}></i>
          {#if d.items.length > 1}
            <span class="absolute right-0.5 bottom-1.5 z-10 rounded-sm bg-foreground/90 px-1 text-[9px] font-bold text-background shadow-xs">{d.items.length}</span>
          {/if}
          <span class={cn('absolute inset-x-0 bottom-0 z-10 h-[3px]', first.sport === 'running' ? 'bg-run' : first.sport === 'cycling' ? 'bg-ride' : 'bg-faint')}></span>
        </button>

        {#if popoverDate === d.date && d.items.length > 1}
          <div class="absolute top-[60px] left-1/2 z-50 min-w-[170px] -translate-x-1/2 rounded-lg border bg-popover p-1 shadow-lg backdrop-blur-md animate-in fade-in zoom-in-95">
            <div class="px-2 py-1 text-[11px] font-semibold text-muted-foreground border-b mb-1">{month}월 {d.day}일 운동 ({d.items.length})</div>
            {#each d.items as item}
              <button class={cn('flex w-full items-center justify-between gap-2 rounded-md px-2 py-1.5 text-left text-xs transition-colors hover:bg-accent cursor-pointer',
                  item.activityId === selected && 'bg-accent/80 font-medium text-ride')}
                onclick={() => { onSelect(item.activityId); popoverDate = null }}>
                <span class="truncate">{item.sport === 'running' ? '🏃' : '🚴'} {item.title}</span>
                <span class="shrink-0 text-[11px] font-mono text-muted-foreground">{item.km.toFixed(1)}km</span>
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {:else}
      <div class={cn('relative h-[54px] rounded-[5px] bg-muted', d.today && 'shadow-[inset_0_0_0_1.5px_var(--foreground)]')}
        aria-hidden="true">
        <span class={cn('absolute top-0.5 left-1 text-[11px] font-semibold', d.today ? 'text-foreground' : d.weekend ? 'text-muted-foreground' : 'text-muted-foreground/60')}>{d.day}</span>
      </div>
    {/if}
  {/each}
</nav>
