<script lang="ts">
  // 운동 목록: 주별로 묶고, 한 운동이 한 줄 (썸네일, 날짜·제목·게시 상태, 거리·시간).
  import StatusBadge from '$lib/components/StatusBadge.svelte'
  import * as ToggleGroup from '$lib/components/ui/toggle-group'
  import { api } from '$lib/api'
  import { km } from '$lib/format'
  import { itemSummary, type ListFilter, type WeekGroup } from '$lib/workbench'
  import { cn } from '$lib/utils'

  let { groups, filter, readyCount, selected, onFilter, onSelect }: {
    groups: WeekGroup[]; filter: ListFilter; readyCount: number; selected: string | null
    onFilter: (f: ListFilter) => void; onSelect: (activityId: string) => void
  } = $props()

  const WEEKDAY = ['일', '월', '화', '수', '목', '금', '토']
  const dayLabel = (iso: string) => {
    const d = new Date(iso + 'T00:00:00')
    return `${d.getMonth() + 1}월 ${d.getDate()}일 ${WEEKDAY[d.getDay()]}`
  }
</script>

<div class="sticky top-0 z-10 border-b bg-card px-3 py-2">
  <ToggleGroup.Root type="single" size="sm" variant="outline" aria-label="목록 거르기" bind:value={() => filter, v => { if (v) onFilter(v as ListFilter) }}>
    <ToggleGroup.Item value="all">전체</ToggleGroup.Item>
    <ToggleGroup.Item value="ready">게시 대기 {readyCount}</ToggleGroup.Item>
    <ToggleGroup.Item value="published">게시함</ToggleGroup.Item>
  </ToggleGroup.Root>
</div>

{#if groups.length === 0}
  <p class="p-4 text-sm text-muted-foreground">
    {filter === 'all' ? '이번 달 기록이 없어요. 가져오기로 FIT 파일과 사진을 넣거나 창에 끌어다 놓으세요.' : '해당하는 운동이 없어요.'}
  </p>
{/if}

{#each groups as g (g.weekStart)}
  <div class="flex justify-between px-3.5 pt-2.5 pb-1 text-xs text-muted-foreground">
    <span>{g.label}</span><b class="font-semibold text-foreground">{km(g.km)}km</b>
  </div>
  {#each g.items as it (it.activityId)}
    {@const sel = it.activityId === selected}
    <button class={cn('grid w-full grid-cols-[40px_1fr_auto] items-center gap-2.5 border-l-[3px] px-3.5 py-1.5 text-left hover:bg-muted',
        sel ? 'border-l-ride bg-selected hover:bg-selected' : 'border-l-transparent')}
      aria-current={sel} onclick={() => onSelect(it.activityId)}>
      <span class="relative h-[54px] w-10 overflow-hidden rounded-[5px] bg-sunken">
        {#if it.coverCard}
          <img src={api.fileUrl(it.coverCard)} alt="" loading="lazy" class="size-full object-cover" />
        {:else}
          <span class="absolute inset-0 grid place-items-center text-center text-[10px] leading-tight text-faint">사진<br />없음</span>
        {/if}
      </span>
      <span class="min-w-0">
        <span class="flex items-center gap-1.5 text-xs text-muted-foreground">{dayLabel(it.date)} {it.startTime}
          <i class={cn('inline-block size-2 rounded-[2px]', it.sport === 'running' ? 'bg-run' : it.sport === 'cycling' ? 'bg-ride' : 'bg-faint')}></i></span>
        <span class="block truncate font-semibold">{it.title}</span>
        <StatusBadge status={it.postStatus} />
      </span>
      <span class="text-right">
        <b class="block text-[17px] leading-tight tracking-tight">{km(it.km)}<small class="ml-px text-[11px] text-muted-foreground">km</small></b>
        <span class="text-[11px] whitespace-pre text-muted-foreground">{itemSummary(it)}</span>
      </span>
    </button>
  {/each}
{/each}
