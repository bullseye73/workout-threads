<script lang="ts">
  // 한 달 사진 띠: 하루 한 칸. 운동한 날은 대표 카드, 아래 종목 띠, 오른쪽 위 게시 상태.
  import { api } from '$lib/api'
  import { STATUS_LABEL } from '$lib/theme'
  import type { RibbonDay } from '$lib/workbench'
  import { cn } from '$lib/utils'

  let { days, month, selected, picked = null, onSelect, onPickDate }: {
    days: RibbonDay[]; month: number; selected: string | null
    /** 기록 추가 패널을 연 날짜. */
    picked?: string | null
    onSelect: (activityId: string) => void
    onPickDate: (date: string) => void
  } = $props()

  const DOT = { none: 'shadow-[inset_0_0_0_1.5px_var(--on-photo)]', ready: 'bg-wait', published: 'bg-done' } as const
</script>

<nav class="grid auto-cols-[minmax(26px,1fr)] grid-flow-col gap-[3px] overflow-x-auto border-b bg-card px-4 pt-2 pb-2.5 max-md:auto-cols-[30px]"
  aria-label="{month}월 날짜별 운동">
  {#each days as d (d.date)}
    {@const first = d.items[0]}
    {#if first}
      {@const isSel = d.items.some(i => i.activityId === selected)}
      <button class={cn('relative h-[54px] overflow-hidden rounded-[5px] text-left',
          first.coverCard ? 'bg-sunken' : first.sport === 'running' ? 'bg-run/20' : 'bg-ride/20',
          d.today && 'shadow-[inset_0_0_0_1.5px_var(--foreground)]',
          isSel && 'outline-2 outline-offset-1 outline-ride')}
        onclick={() => onSelect(first.activityId)} aria-current={isSel}
        aria-label="{month}월 {d.day}일 {first.title} {first.km.toFixed(1)}km {STATUS_LABEL[first.postStatus]}{d.items.length > 1 ? ` 외 ${d.items.length - 1}개` : ''}">
        {#if first.coverCard}
          <img src={api.fileUrl(first.coverCard)} alt="" loading="lazy" class="absolute inset-0 size-full object-cover" />        {/if}
        <span class={cn('absolute top-0.5 left-1 z-10 text-[10px] font-semibold', first.coverCard ? 'text-on-photo [text-shadow:0_0_3px_rgb(0_0_0/.6)]' : 'text-foreground')}>{d.day}</span>
        <i class={cn('absolute top-[3px] right-[3px] z-10 size-[7px] rounded-full', DOT[first.postStatus])}></i>
        {#if d.items.length > 1}
          <span class="absolute right-0.5 bottom-1.5 z-10 rounded-sm bg-foreground px-1 text-[9px] font-bold text-background">{d.items.length}</span>
        {/if}
        <span class={cn('absolute inset-x-0 bottom-0 z-10 h-[3px]', first.sport === 'running' ? 'bg-run' : first.sport === 'cycling' ? 'bg-ride' : 'bg-faint')}></span>
      </button>
    {:else}
      <!-- 운동 없는 날: 누르면 그 날짜에 기록 추가 -->
      <button class={cn('group relative h-[54px] rounded-[5px] bg-muted hover:bg-sunken', d.today && 'shadow-[inset_0_0_0_1.5px_var(--foreground)]',
          picked === d.date && 'outline-2 outline-offset-1 outline-ride')}
        onclick={() => onPickDate(d.date)} aria-current={picked === d.date} aria-label="{month}월 {d.day}일 기록 추가">
        <span class={cn('absolute top-0.5 left-1 text-[10px] font-semibold', d.today ? 'text-foreground' : d.weekend ? 'text-muted-foreground' : 'text-faint')}>{d.day}</span>
        <span class="absolute inset-0 grid place-items-center pt-2 text-base leading-none text-muted-foreground opacity-0 group-hover:opacity-100 group-focus-visible:opacity-100" aria-hidden="true">+</span>
      </button>
    {/if}
  {/each}
</nav>
