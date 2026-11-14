<script lang="ts">
  import { api } from '../lib/api'
  import type { ActivityDetail } from '../lib/bindings/ActivityDetail'
  import { km } from '../lib/format'
  import { toChartSeries } from '../lib/series'
  import MetricCharts from './MetricCharts.svelte'
  import RouteMap from './RouteMap.svelte'
  import SplitsTable from './SplitsTable.svelte'

  let { id, onBack, onPost }: { id: string; onBack: () => void; onPost: (id: string) => void } = $props()
  let detail = $state<ActivityDetail | null>(null)
  let error = $state('')
  let highlight = $state(-1)

  $effect(() => {
    detail = null
    api.activityDetail(id).then(d => (detail = d)).catch(e => (error = String(e)))
  })

  const metrics = $derived.by(() => {
    if (!detail) return []
    const m = detail.metrics
    return [
      ['거리', `${km(m.distanceKm)} km`],
      ['이동 시간', m.moving],
      ['경과 시간', m.elapsed],
      m.pace ? ['평균 페이스', `${m.pace}/km`] : null,
      m.speedKmh !== null ? ['평균 속도', `${m.speedKmh.toFixed(1)} km/h`] : null,
      m.avgHr !== null ? ['평균 심박', `${m.avgHr} bpm`] : null,
      m.maxHr !== null ? ['최대 심박', `${m.maxHr} bpm`] : null,
      m.ascentM !== null ? ['상승 고도', `${m.ascentM} m`] : null,
      m.avgCadence !== null ? ['평균 케이던스', `${m.avgCadence}`] : null,
    ].filter((x): x is string[] => x !== null)
  })
</script>

<nav class="bar">
  <button class="btn" onclick={onBack}>← 대시보드</button>
  {#if detail}<button class="btn primary" onclick={() => onPost(id)}>게시 준비</button>{/if}
</nav>

{#if error}
  <p class="error">운동을 불러오지 못했어요: {error}</p>
{:else if detail}
  <header>
    <p class="date">{detail.date}</p>
    <h1 class={detail.sport}>{detail.title}</h1>
  </header>
  <dl class="metrics">
    {#each metrics as [k, v]}<div><dt>{k}</dt><dd>{v}</dd></div>{/each}
  </dl>
  <RouteMap {detail} {highlight} />
  <div class="lower">
    <section><h2>구간 그래프</h2><MetricCharts series={toChartSeries(detail)} onHover={i => (highlight = i)} /></section>
    <section><h2>km 구간</h2><SplitsTable splits={detail.splits} /></section>
  </div>
{:else}
  <p class="muted">불러오는 중…</p>
{/if}

<style>
  .bar { display: flex; justify-content: space-between; margin-bottom: 18px; }
  .date { margin: 0; color: var(--muted-foreground); font-size: 14px; }
  h1 { margin: 2px 0 16px; font-size: 28px; font-weight: 700; letter-spacing: -0.02em; padding-left: 12px; border-left: 4px solid var(--muted-foreground); }
  h1.cycling { border-color: var(--ride); }
  h1.running { border-color: var(--run); }
  .metrics { display: flex; flex-wrap: wrap; gap: 14px 36px; margin: 0 0 20px; }
  .metrics dt { font-size: 13px; color: var(--muted-foreground); }
  .metrics dd { margin: 0; font-size: 20px; font-weight: 700; }
  .lower { display: grid; grid-template-columns: 2fr 1fr; gap: 28px; margin-top: 24px; }
  .lower section:last-child { max-height: 460px; overflow: auto; }
  h2 { font-size: 16px; margin: 0 0 8px; }
  .muted { color: var(--muted-foreground); }
  .error { color: #b42318; }
  @media (max-width: 860px) { .lower { grid-template-columns: 1fr; } }
</style>
