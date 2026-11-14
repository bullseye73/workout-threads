<script lang="ts">
  import type { CalCell } from '../lib/calendar'
  import { km } from '../lib/format'
  let { grid, onOpen }: { grid: CalCell[][]; onOpen: (activityId: string) => void } = $props()
  const ICON = { running: '🏃', cycling: '🚲', walking: '👟', hiking: '👟', other: '👟' } as const
  const NAME = { running: '러닝', cycling: '라이딩', walking: '걷기', hiking: '하이킹', other: '운동' } as const
  const STATUS = { none: '게시 준비 안 됨', ready: '게시 준비됨', published: '게시함' } as const
  const today = new Date().toISOString().slice(0, 10)
</script>

<div class="cal">
  {#each ['월', '화', '수', '목', '금', '토', '일'] as d}<div class="head">{d}</div>{/each}
  {#each grid as week}
    {#each week as c}
      <div class="cell" class:out={!c.inMonth} class:today={c.date === today}>
        <span class="num">{c.day}</span>
        {#each c.items as it}
          <button class="item {it.sport}" onclick={() => onOpen(it.activityId)} title={STATUS[it.postStatus]}
            aria-label="{c.date} {NAME[it.sport]} {km(it.km)}km, {STATUS[it.postStatus]}">
            <span>{ICON[it.sport]}</span><span class="dist">{km(it.km)}km</span>
            <i class="dot {it.postStatus}" aria-label={STATUS[it.postStatus]}></i>
          </button>
        {/each}
      </div>
    {/each}
  {/each}
</div>

<style>
  .cal { display: grid; grid-template-columns: repeat(7, 1fr); border-top: 1px solid var(--line); border-left: 1px solid var(--line); }
  .head { padding: 8px 10px; font-size: 13px; color: var(--muted-foreground); border-right: 1px solid var(--line); border-bottom: 1px solid var(--line); }
  .cell { min-height: 92px; padding: 6px 8px; border-right: 1px solid var(--line); border-bottom: 1px solid var(--line); display: flex; flex-direction: column; gap: 4px; background: var(--surface); }
  .cell.out { background: transparent; }
  .cell.out .num { color: #b5bdc7; }
  .num { font-size: 13px; color: var(--muted-foreground); }
  .today .num { color: var(--surface); background: var(--ink); border-radius: 999px; width: 22px; height: 22px; display: grid; place-items: center; }
  .item { display: flex; align-items: center; gap: 4px; border: 0; border-radius: var(--radius-sm); padding: 3px 6px; text-align: left; font-size: 13px; font-weight: 600; }
  .item.cycling { background: color-mix(in srgb, var(--ride) 12%, white); color: #1d4fb8; }
  .item.running { background: color-mix(in srgb, var(--run) 14%, white); color: #a8480f; }
  .item.walking, .item.hiking, .item.other { background: var(--track); }
  .item:hover { filter: brightness(.96); }
  .dist { flex: 1; }
  .dot { width: 8px; height: 8px; border-radius: 999px; background: #c3cad3; flex: none; }
  .dot.ready { background: var(--ride); }
  .dot.published { background: var(--done); }
  @media (max-width: 700px) { .cell { min-height: 64px; padding: 4px; } .item { font-size: 11px; padding: 2px 4px; } }
</style>
