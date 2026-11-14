<script lang="ts">
  // km 구간표: 빠를수록 긴 막대, 가장 빠른 구간 강조.
  import type { SplitDto } from '$lib/bindings/SplitDto'
  import { splitBars } from '$lib/profile'
  import { cn } from '$lib/utils'

  let { splits, totalKm, sport }: { splits: SplitDto[]; totalKm: number; sport: string } = $props()
  const bars = $derived(splitBars(splits, totalKm))
</script>

<section class="rounded-lg border bg-card px-3.5 py-2.5">
  <h3 class="m-0 mb-1.5 text-[13px] font-bold">km 구간</h3>
  {#if bars.length === 0}
    <p class="m-0 text-sm text-muted-foreground">거리 기록이 없어 구간을 나누지 못했어요.</p>
  {:else}
    <table class="w-full border-collapse text-xs">
      <thead><tr class="text-left text-muted-foreground"><th class="px-1.5 py-1 font-semibold">km</th><th class="px-1.5 py-1 font-semibold">시간</th><th class="w-1/2 px-1.5 py-1"><span class="sr-only">빠르기</span></th><th class="px-1.5 py-1 font-semibold">심박</th></tr></thead>
      <tbody>
        {#each bars as b (b.km)}
          <tr class={cn('border-t', b.best && 'font-bold text-ride')}>
            <td class="px-1.5 py-1">{b.km}</td>
            <td class="px-1.5 py-1">{b.duration}</td>
            <td class="px-1.5 py-1"><i class={cn('block h-2 rounded-sm', sport === 'running' ? 'bg-run' : 'bg-ride')}
              style:width="{b.widthPct}%" style:opacity={0.35 + 0.65 * (b.widthPct / 100) ** 3}></i></td>
            <td class="px-1.5 py-1">{b.avgHr ?? '—'}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</section>
