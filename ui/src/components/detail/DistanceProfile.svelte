<script lang="ts">
  // 거리 축 그래프 3줄(고도, 심박, 속도/페이스). 줄마다 왼쪽에 이름·값·평균/최고를 크게 두고,
  // 심박·속도 선은 값에 따라 낮음(차가운 색) → 높음(뜨거운 색)으로 칠한다. 위쪽에 사진 핀.
  import { Chart, Filler, LinearScale, LineController, LineElement, PointElement, type Plugin } from 'chart.js'
  import { onDestroy, untrack } from 'svelte'
  import { api } from '$lib/api'
  import { heatT, mixHex, seriesStats, valueRange } from '$lib/heat'
  import type { Pin } from '$lib/profile'
  import { nearestIndex, type ChartSeries } from '$lib/series'
  import { cssVar, heatStops, onThemeApplied } from '$lib/theme'
  import { cn } from '$lib/utils'

  Chart.register(LineController, LineElement, PointElement, LinearScale, Filler)

  let { series, pins, highlight, activePin, onHover, onFocusPin }: {
    series: ChartSeries; pins: Pin[]; highlight: number; activePin: number | null
    onHover: (index: number) => void; onFocusPin: (id: number | null) => void
  } = $props()

  type Row = { key: string; label: string; unit: string; data: (number | null)[]; heat: boolean; invert: boolean; reverse: boolean }
  const pace = $derived(series.secondaryLabel.startsWith('페이스'))
  const rows = $derived<Row[]>([
    { key: 'alt', label: '고도', unit: 'm', data: series.altitude, heat: false, invert: false, reverse: false },
    { key: 'hr', label: '심박', unit: 'bpm', data: series.hr, heat: true, invert: false, reverse: false },
    // 페이스는 값이 클수록 느리므로 색을 뒤집고, 그래프도 위가 빠르게 뒤집는다.
    { key: 'spd', label: pace ? '페이스' : '속도', unit: pace ? '/km' : 'km/h', data: series.secondary, heat: true, invert: pace, reverse: pace },
  ])
  const ranges = $derived(rows.map(r => valueRange(r.data)))
  const stats = $derived(rows.map(r => seriesStats(r.data)))
  const maxX = $derived(series.x[series.x.length - 1] ?? 1)

  let stops = $state<string[]>([])
  let canvases: HTMLCanvasElement[] = $state([])
  let charts: Chart[] = []
  let cursor = -1

  /** 값의 색: 심박·속도는 열 색, 고도는 글자색. */
  function colorOf(i: number, v: number | null | undefined): string {
    const r = ranges[i]
    if (!rows[i].heat || v === null || v === undefined || !r || !stops.length) return 'var(--foreground)'
    const t = heatT(v, r)
    return mixHex(stops, rows[i].invert ? 1 - t : t)
  }

  const cursorLine: Plugin<'line'> = {
    id: 'cursorLine',
    afterDatasetsDraw(chart) {
      if (cursor < 0) return
      const x = chart.scales.x.getPixelForValue(series.x[cursor])
      const { top, bottom } = chart.chartArea
      const ctx = chart.ctx
      ctx.save()
      ctx.strokeStyle = cssVar('--foreground')
      ctx.lineWidth = 1
      ctx.beginPath(); ctx.moveTo(x, top); ctx.lineTo(x, bottom); ctx.stroke()
      ctx.restore()
    },
  }

  function build() {
    charts.forEach(c => c.destroy())
    stops = heatStops()
    const grid = cssVar('--border'), tick = cssVar('--muted-foreground'), alt = cssVar('--alt')
    charts = rows.map((row, i) => {
      const r = ranges[i]
      const pad = r ? (r.hi - r.lo) * 0.25 || 1 : 0
      return new Chart(canvases[i], {
        type: 'line',
        data: { datasets: [{
          data: series.x.map((x, j) => ({ x, y: row.data[j] })),
          borderColor: row.heat ? tick : alt, borderWidth: row.heat ? 2 : 1.5, pointRadius: 0, spanGaps: true,
          fill: true, backgroundColor: (row.heat ? tick : alt) + (row.heat ? '14' : '30'),
          segment: row.heat ? { borderColor: c => colorOf(i, c.p0.parsed.y) } : undefined,
        }] },
        options: {
          animation: false, maintainAspectRatio: false, parsing: false, events: [],
          layout: { padding: { right: 4, top: 4 } },
          scales: {
            x: { type: 'linear', min: 0, max: maxX, display: i === rows.length - 1, grid: { display: false },
              ticks: { color: tick, callback: v => `${v}km`, maxTicksLimit: 8 },
              afterBuildTicks: axis => { axis.ticks = axis.ticks.filter(t => Number.isInteger(t.value)) } },
            // 열 색 줄은 튀는 값을 잘라 변화가 잘 보이게 범위를 좁힌다.
            y: { reverse: row.reverse, border: { display: false },
              min: r && row.heat ? r.lo - pad : undefined, max: r && row.heat ? r.hi + pad : undefined,
              grid: { color: grid }, ticks: { display: false, maxTicksLimit: 3 } },
          },
        },
        plugins: [cursorLine],
      })
    })
  }

  // build()는 색(stops)을 쓰고 차트를 만들며 그 색을 바로 읽는다. 추적하면 스스로를 다시 부르므로
  // 캔버스·줄 구성이 바뀔 때만 다시 그리고, build 안의 읽기·쓰기는 추적하지 않는다.
  $effect(() => {
    if (canvases.length === rows.length && canvases.every(Boolean)) untrack(build)
  })
  $effect(() => onThemeApplied(build))
  $effect(() => {
    cursor = highlight
    charts.forEach(c => c.draw())
  })
  onDestroy(() => charts.forEach(c => c.destroy()))

  function move(e: MouseEvent) {
    const c = charts[0]
    if (!c) return
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect()
    onHover(nearestIndex(series.x, c.scales.x.getValueForPixel(e.clientX - r.left) ?? 0))
  }

  function fmt(row: Row, v: number | null | undefined): string {
    if (v === null || v === undefined) return '—'
    if (row.key === 'spd' && pace) { const m = Math.floor(v); return `${m}'${String(Math.round((v - m) * 60)).padStart(2, '0')}"` }
    return row.key === 'spd' ? v.toFixed(1) : String(Math.round(v))
  }
  // 페이스는 "최고"가 가장 작은 값이다.
  const best = (i: number) => (rows[i].invert ? stats[i]?.min : stats[i]?.max)
</script>

<section class="min-w-0 rounded-lg border bg-card px-3.5 pt-2.5 pb-2" aria-label="거리별 그래프">
  <div class="mb-1 flex flex-wrap items-center justify-between gap-2">
    <h3 class="m-0 text-[13px] font-bold">거리별 기록</h3>
    <div class="flex items-center gap-2 text-[11px] text-muted-foreground">
      {highlight >= 0 ? `${series.x[highlight].toFixed(1)}km 지점` : '마우스를 올리면 그 지점 값을 보여 줘요'}
      <span class="flex items-center gap-1 max-sm:hidden" aria-hidden="true">
        낮음<i class="inline-block h-1.5 w-14 rounded-full bg-gradient-to-r from-heat-0 via-[var(--heat-2)] to-heat-4"></i>높음
      </span>
    </div>
  </div>

  {#if pins.length}
    <div class="relative mr-1 ml-[104px] h-[58px]">
      {#each pins as p (p.id)}
        <button class="absolute bottom-0 flex -translate-x-1/2 flex-col items-center" style:left="{p.leftPct}%"
          onmouseenter={() => onFocusPin(p.id)} onmouseleave={() => onFocusPin(null)}
          onfocus={() => onFocusPin(p.id)} onblur={() => onFocusPin(null)}
          aria-label="{p.km.toFixed(1)}km 지점 사진">
          <img src={api.fileUrl(p.src)} alt="" loading="lazy"
            class={cn('h-[46px] w-[32px] rounded object-cover shadow-sm ring-2', activePin === p.id ? 'ring-run' : 'ring-card')} />
          <i class="block h-2 w-0.5 bg-run"></i>
        </button>
      {/each}
    </div>
  {/if}

  <div class="grid" role="group" aria-label="고도·심박·속도 그래프">
    {#each rows as row, i (row.key)}
      {@const v = highlight >= 0 ? row.data[highlight] : stats[i]?.avg}
      <div class={cn('grid grid-cols-[104px_minmax(0,1fr)] items-stretch', i > 0 && 'border-t')}>
        <div class="flex flex-col justify-center gap-0.5 py-1.5 pr-2">
          <span class="text-[11px] text-muted-foreground">{row.label}{highlight >= 0 ? '' : ' 평균'}</span>
          <span class="text-xl leading-none font-bold tracking-tight" style:color={colorOf(i, v)}>
            {fmt(row, v)}<small class="ml-0.5 text-[11px] font-semibold text-muted-foreground">{row.unit}</small>
          </span>
          {#if stats[i]}
            <span class="text-[10.5px] text-muted-foreground">
              {row.key === 'alt' ? `${fmt(row, stats[i]!.min)}–${fmt(row, stats[i]!.max)}${row.unit}` : `최고 ${fmt(row, best(i))}`}
            </span>
          {/if}
        </div>
        <div class={cn('relative min-w-0 cursor-crosshair', i === rows.length - 1 ? 'h-[84px]' : 'h-[64px]')}
          role="presentation" onmousemove={move} onmouseleave={() => onHover(-1)}>
          <canvas bind:this={canvases[i]} aria-label="{row.label} 그래프"></canvas>
        </div>
      </div>
    {/each}
  </div>
</section>
