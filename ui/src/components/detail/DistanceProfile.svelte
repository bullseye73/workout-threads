<script lang="ts">
  // 거리 축 그래프 3줄(고도, 심박, 속도/페이스) + 위쪽 사진 핀. 마우스 위치를 상위로 알려 지도와 연동한다.
  import { Chart, Filler, LinearScale, LineController, LineElement, PointElement, type Plugin } from 'chart.js'
  import { onDestroy } from 'svelte'
  import { api } from '$lib/api'
  import type { Pin } from '$lib/profile'
  import { nearestIndex, type ChartSeries } from '$lib/series'
  import { cssVar, onThemeApplied } from '$lib/theme'
  import { cn } from '$lib/utils'

  Chart.register(LineController, LineElement, PointElement, LinearScale, Filler)

  let { series, pins, highlight, activePin, onHover, onFocusPin }: {
    series: ChartSeries; pins: Pin[]; highlight: number; activePin: number | null
    onHover: (index: number) => void; onFocusPin: (id: number | null) => void
  } = $props()

  const pace = $derived(series.secondaryLabel.startsWith('페이스'))
  const rows = $derived([
    { label: '고도', unit: 'm', data: series.altitude, token: '--alt', reverse: false },
    { label: '심박', unit: 'bpm', data: series.hr, token: '--hr', reverse: false },
    { label: pace ? '페이스' : '속도', unit: pace ? '분/km' : 'km/h', data: series.secondary, token: pace ? '--run' : '--ride', reverse: pace },
  ])
  const maxX = $derived(series.x[series.x.length - 1] ?? 1)

  let canvases: HTMLCanvasElement[] = $state([])
  let charts: Chart[] = []
  let cursor = -1

  // 세 그래프에 같은 세로선을 그린다.
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
    const grid = cssVar('--border'), tick = cssVar('--muted-foreground')
    charts = rows.map((row, i) => {
      const color = cssVar(row.token)
      return new Chart(canvases[i], {
        type: 'line',
        data: { datasets: [{ data: series.x.map((x, j) => ({ x, y: row.data[j] })), borderColor: color, borderWidth: 1.5,
          pointRadius: 0, fill: true, backgroundColor: color + '24', spanGaps: true }] },
        options: {
          animation: false, maintainAspectRatio: false, parsing: false, events: [],
          layout: { padding: { right: 2 } },
          scales: {
            x: { type: 'linear', min: 0, max: maxX, display: i === rows.length - 1, grid: { display: false },
              ticks: { color: tick, callback: v => `${v}km`, maxTicksLimit: 6 },
              // 끝값(예: 48.2)은 앞 눈금과 겹치니 정수 눈금만.
              afterBuildTicks: axis => { axis.ticks = axis.ticks.filter(t => Number.isInteger(t.value)) } },
            y: { reverse: row.reverse, grid: { color: grid }, ticks: { color: tick, maxTicksLimit: 3 }, afterFit: s => { s.width = 40 } },
          },
        },
        plugins: [cursorLine],
      })
    })
  }

  // 처음 그리고, 테마 클래스가 실제로 바뀐 뒤마다 토큰 색으로 다시 그린다.
  $effect(() => {
    if (canvases.length === rows.length && canvases.every(Boolean)) build()
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
    const px = e.clientX - r.left
    onHover(nearestIndex(series.x, c.scales.x.getValueForPixel(px) ?? 0))
  }

  const fmt = (v: number | null | undefined, unit: string) => {
    if (v === null || v === undefined) return '—'
    if (unit === '분/km') { const m = Math.floor(v); return `${m}'${String(Math.round((v - m) * 60)).padStart(2, '0')}"` }
    return `${unit === 'km/h' ? v.toFixed(1) : Math.round(v)}${unit}`
  }
</script>

<section class="rounded-lg border bg-card px-3.5 pt-2.5 pb-2" aria-label="거리별 그래프">
  <div class="mb-1 flex flex-wrap items-baseline justify-between gap-2">
    <h3 class="m-0 text-[13px] font-bold">거리별 {rows.map(r => r.label).join('·')}</h3>
    <div class="flex gap-3 text-xs text-muted-foreground" aria-live="polite">
      {#if highlight >= 0}
        <b class="text-foreground">{series.x[highlight].toFixed(1)}km</b>
        {#each rows as r}<span>{r.label} <b class="text-foreground">{fmt(r.data[highlight], r.unit)}</b></span>{/each}
      {:else}
        <span>그래프 위에 마우스를 올리면 지도에 위치가 표시돼요</span>
      {/if}
    </div>
  </div>

  {#if pins.length}
    <div class="relative mr-0.5 ml-10 h-[62px]">
      {#each pins as p (p.id)}
        <button class="absolute bottom-0 flex -translate-x-1/2 flex-col items-center" style:left="{p.leftPct}%"
          onmouseenter={() => onFocusPin(p.id)} onmouseleave={() => onFocusPin(null)}
          onfocus={() => onFocusPin(p.id)} onblur={() => onFocusPin(null)}
          aria-label="{p.km.toFixed(1)}km 지점 사진">
          <img src={api.fileUrl(p.src)} alt="" loading="lazy"
            class={cn('h-[50px] w-[34px] rounded object-cover shadow-sm ring-2', activePin === p.id ? 'ring-run' : 'ring-card')} />
          <i class="block h-2 w-0.5 bg-run"></i>
        </button>
      {/each}
    </div>
  {/if}

  <div class="grid cursor-crosshair" role="group" aria-label="고도·심박·속도 그래프" onmousemove={move} onmouseleave={() => onHover(-1)}>
    {#each rows as r, i}
      <div class={cn('relative', i === rows.length - 1 ? 'h-[86px]' : 'h-[66px]', i > 0 && 'border-t border-dashed')}>
        <canvas bind:this={canvases[i]} aria-label="{r.label} 그래프"></canvas>
      </div>
    {/each}
  </div>
</section>
