<script lang="ts">
  // 거리 축 그래프 3개(심박, 페이스/속도, 고도). 마우스 위치를 상위로 알려 지도와 연동한다.
  import { Chart, Filler, LinearScale, LineController, LineElement, PointElement, Tooltip } from 'chart.js'
  import { onDestroy, onMount } from 'svelte'
  import type { ChartSeries } from '../lib/series'
  import { nearestIndex } from '../lib/series'

  Chart.register(LineController, LineElement, PointElement, LinearScale, Filler, Tooltip)
  let { series, onHover }: { series: ChartSeries; onHover: (index: number) => void } = $props()

  const rows = $derived([
    { label: '심박 (bpm)', data: series.hr, color: '#e5484d', reverse: false },
    { label: series.secondaryLabel, data: series.secondary, color: series.secondaryLabel.startsWith('페이스') ? '#f07a2a' : '#2f6feb', reverse: series.secondaryLabel.startsWith('페이스') },
    { label: '고도 (m)', data: series.altitude, color: '#6b7684', reverse: false },
  ])
  let canvases: HTMLCanvasElement[] = []
  let charts: Chart[] = []

  onMount(() => {
    charts = rows.map((row, i) => new Chart(canvases[i], {
      type: 'line',
      data: { datasets: [{ data: series.x.map((x, j) => ({ x, y: row.data[j] })), borderColor: row.color, borderWidth: 1.5,
        pointRadius: 0, fill: true, backgroundColor: row.color + '1a', spanGaps: true }] },
      options: {
        animation: false, maintainAspectRatio: false, parsing: false,
        interaction: { mode: 'nearest', axis: 'x', intersect: false },
        plugins: { tooltip: { callbacks: { title: items => `${(items[0].parsed.x ?? 0).toFixed(2)} km` } } },
        scales: { x: { type: 'linear', min: 0, max: series.x[series.x.length - 1] ?? 1, ticks: { callback: v => `${v}km` }, grid: { display: false } },
          y: { reverse: row.reverse, grid: { color: '#eef1f4' } } },
        onHover: (e, _els, chart) => {
          if (e.x === null) return
          onHover(nearestIndex(series.x, chart.scales.x.getValueForPixel(e.x) ?? 0))
        },
      },
    }))
  })
  onDestroy(() => charts.forEach(c => c.destroy()))
</script>

<div class="charts" role="group" aria-label="운동 그래프" onmouseleave={() => onHover(-1)}>
  {#each rows as row, i}
    <figure>
      <figcaption>{row.label}</figcaption>
      <div class="box"><canvas bind:this={canvases[i]}></canvas></div>
    </figure>
  {/each}
</div>

<style>
  .charts { display: grid; gap: 12px; }
  figure { margin: 0; }
  figcaption { font-size: 13px; color: var(--muted-foreground); margin-bottom: 2px; }
  .box { height: 120px; position: relative; }
</style>
