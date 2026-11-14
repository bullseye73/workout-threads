<script lang="ts">
  // 1km 구간 막대: 1km마다 얼마나 빨랐는지. 막대가 높고 뜨거운 색일수록 빠른 구간.
  // 가로로 모두 들어가게 그려 스크롤이 없다. 막대에 마우스를 올리면 그 구간 값과 지도 위치를 보여 준다.
  import { heatT, mixHex, splitRate, valueRange } from '$lib/heat'
  import type { SplitDto } from '$lib/bindings/SplitDto'
  import { splitBars } from '$lib/profile'
  import { heatStops, onThemeApplied } from '$lib/theme'
  import { cn } from '$lib/utils'

  let { splits, totalKm, sport, onFocusKm }: {
    splits: SplitDto[]; totalKm: number; sport: string; onFocusKm?: (km: number | null) => void
  } = $props()

  const ride = $derived(sport === 'cycling')
  const bars = $derived(splitBars(splits, totalKm).map(b => ({ ...b, rate: splitRate(b.sec, sport) })))
  // 짧은 마지막 구간 같은 튀는 값에 색이 끌려가지 않게 5~95 백분위로.
  const range = $derived(valueRange(bars.map(b => (b.sec > 0 ? b.rate.value : null))))
  const valid = $derived(bars.filter(b => b.sec > 0))
  const avg = $derived(valid.length ? valid.reduce((t, b) => t + b.rate.value, 0) / valid.length : 0)
  const fastest = $derived(bars.find(b => b.best))
  const labelEvery = $derived(bars.length <= 12 ? 1 : bars.length <= 30 ? 5 : 10)

  let stops = $state<string[]>([])
  $effect(() => { stops = heatStops(); return onThemeApplied(() => (stops = heatStops())) })
  let hover = $state<number | null>(null)
  const shown = $derived(hover !== null ? bars[hover] : null)

  // 막대 높이: 가장 느린 구간도 보이게 바닥 18%를 깔고 나머지를 빠르기로 채운다.
  const height = (v: number) => (range ? 18 + 82 * heatT(v, range) : 50)
  const color = (v: number) => (range && stops.length ? mixHex(stops, heatT(v, range)) : 'var(--ride)')
  const avgFmt = $derived(splitRate(avg > 0 ? 3600 / avg : 0, sport).label)

  function focus(i: number | null) {
    hover = i
    onFocusKm?.(i === null ? null : Math.max(0, bars[i].km - 0.5))
  }
</script>

<section class="rounded-lg border bg-card px-3.5 py-2.5" aria-label="1km 구간 {ride ? '속도' : '페이스'}">
  <div class="mb-1.5 flex flex-wrap items-baseline justify-between gap-x-3 gap-y-1">
    <h3 class="m-0 text-[13px] font-bold">1km 구간 {ride ? '속도' : '페이스'}</h3>
    <p class="m-0 text-[11px] text-muted-foreground" aria-live="polite">
      {#if shown}
        <b class="text-foreground">{shown.km}km</b> 구간 {shown.duration} 걸림, <b class="text-foreground">{shown.rate.label}</b>{shown.avgHr ? `, 심박 ${shown.avgHr}` : ''}
      {:else if fastest}
        막대가 높을수록 빨랐어요. 평균 {avgFmt}, 가장 빠른 구간 {fastest.km}km ({fastest.rate.label})
      {/if}
    </p>
  </div>

  {#if bars.length === 0}
    <p class="m-0 text-sm text-muted-foreground">거리 기록이 없어 구간을 나누지 못했어요.</p>
  {:else}
    <div class="relative flex h-[96px] items-end gap-px" role="group" aria-label="구간 막대" onmouseleave={() => focus(null)}>
      {#if range && avg > 0}
        <i class="pointer-events-none absolute inset-x-0 z-10 border-t border-dashed border-foreground/40" style:bottom="{height(avg)}%"
          title="평균 {avgFmt}"></i>
      {/if}
      {#each bars as b, i (b.km)}
        <button class="group relative h-full min-w-0 flex-1 rounded-t-[3px] outline-offset-1"
          onmouseenter={() => focus(i)} onfocus={() => focus(i)} onblur={() => focus(null)}
          aria-label="{b.km}km 구간 {b.duration}, {b.rate.label}{b.avgHr ? `, 심박 ${b.avgHr}` : ''}{b.best ? ', 가장 빠름' : ''}">
          <i class={cn('absolute inset-x-0 bottom-0 block rounded-t-[3px] transition-opacity', hover !== null && hover !== i && 'opacity-45')}
            style:height="{b.sec > 0 ? height(b.rate.value) : 2}%" style:background={b.sec > 0 ? color(b.rate.value) : 'var(--border)'}></i>
          {#if b.best}<span class="absolute inset-x-0 mb-0.5 text-center text-[9px] leading-none text-foreground" style:bottom="{height(b.rate.value)}%" aria-hidden="true">●</span>{/if}
        </button>
      {/each}
    </div>
    <div class="mt-1 flex gap-px text-[10px] text-muted-foreground" aria-hidden="true">
      {#each bars as b (b.km)}
        <span class="min-w-0 flex-1 text-center">{b.km % labelEvery === 0 || b.km === 1 ? b.km : ''}</span>
      {/each}
    </div>
  {/if}
</section>
