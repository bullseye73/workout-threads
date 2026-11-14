<script lang="ts">
  // 2D 경로 지도: 경로를 속도 또는 심박에 따라 낮음(차가운 색) → 높음(뜨거운 색)으로 칠한다.
  // 출발·도착 핀, 사진 지점, 그래프와 연동되는 강조 점. 색은 테마 토큰.
  import 'leaflet/dist/leaflet.css'
  import L from 'leaflet'
  import { onDestroy, onMount } from 'svelte'
  import * as ToggleGroup from '$lib/components/ui/toggle-group'
  import { api } from '$lib/api'
  import type { ActivityDetail } from '$lib/bindings/ActivityDetail'
  import { colorRuns, mixHex, valueRange } from '$lib/heat'
  import { cssVar, heatStops, onThemeApplied } from '$lib/theme'
  import { cn } from '$lib/utils'

  let { detail, highlight = -1, activePhoto = null, class: className = 'h-[300px]' }: {
    detail: ActivityDetail; highlight?: number; activePhoto?: number | null; class?: string
  } = $props()

  type Metric = 'speed' | 'hr'
  const BUCKETS = 10
  const hasSpeed = $derived(detail.series.speedKmh.some(v => v !== null))
  const hasHr = $derived(detail.series.hr.some(v => v !== null))
  let metric = $state<Metric>('speed')

  let el = $state<HTMLDivElement>()
  let map: L.Map | null = null
  let line: L.Polyline | null = null
  let heat: L.LayerGroup | null = null
  let runs: { bucket: number; poly: L.Polyline }[] = []
  let start: L.CircleMarker | null = null
  let end: L.CircleMarker | null = null
  let marker: L.CircleMarker | null = null
  const photoMarkers = new Map<number, L.CircleMarker>()
  const TILE = 'https://tile.openstreetmap.org/{z}/{x}/{y}.png'

  /** 고른 지표로 경로를 색 구간 선으로 다시 나눈다. 값이 없으면 한 가지 색 선만 남긴다. */
  function drawHeat() {
    if (!map) return
    heat?.remove()
    runs = []
    const values = metric === 'hr' ? detail.series.hr : detail.series.speedKmh
    const range = valueRange(values)
    if (range) {
      heat = L.layerGroup().addTo(map)
      for (const r of colorRuns(detail.series.latLon, values, range, BUCKETS))
        runs.push({ bucket: r.bucket, poly: L.polyline(r.latlngs, { weight: 4.5, opacity: 0.95, lineCap: 'round' }).addTo(heat) })
    }
    paint()
  }

  /** 테마나 강조가 바뀌면 색만 다시 칠한다 (타일은 CSS 필터). */
  function paint() {
    const ring = cssVar('--card')
    const stops = heatStops()
    line?.setStyle({ color: cssVar('--ride'), opacity: runs.length ? 0 : 0.9 })
    for (const r of runs) r.poly.setStyle({ color: mixHex(stops, (r.bucket + 0.5) / BUCKETS) })
    start?.setStyle({ color: ring, fillColor: cssVar('--done') })
    end?.setStyle({ color: ring, fillColor: cssVar('--foreground') })
    for (const [id, m] of photoMarkers) m.setStyle({ color: ring, fillColor: cssVar('--run'), radius: id === activePhoto ? 8 : 5 })
    marker?.setStyle({ color: cssVar('--foreground'), fillColor: ring })
    start?.bringToFront(); end?.bringToFront()
    photoMarkers.forEach(m => m.bringToFront())
    marker?.bringToFront()
  }

  onMount(() => {
    if (detail.route.length < 2 || !el) return
    if (!hasSpeed && hasHr) metric = 'hr'
    // 페이지 스크롤과 겹치지 않게, 지도를 클릭한 뒤에만 휠 확대를 켠다.
    map = L.map(el, { zoomControl: true, attributionControl: true, scrollWheelZoom: false })
    map.on('click', () => map?.scrollWheelZoom.enable())
    map.on('mouseout', () => map?.scrollWheelZoom.disable())
    L.tileLayer(TILE, { maxZoom: 19, attribution: '&copy; OpenStreetMap contributors' }).addTo(map)
    line = L.polyline(detail.route, { weight: 4 }).addTo(map)
    if (detail.start) start = L.circleMarker(detail.start, { radius: 7, weight: 2, fillOpacity: 1 }).bindTooltip('출발').addTo(map)
    if (detail.end) end = L.circleMarker(detail.end, { radius: 7, weight: 2, fillOpacity: 1 }).bindTooltip('도착').addTo(map)
    for (const p of detail.photos) {
      if (p.lat === null || p.lon === null) continue
      const img = p.cardPath || p.path
      const label = p.kmMark !== null ? `${p.kmMark.toFixed(1)}km 지점` : '사진'
      const m = L.circleMarker([p.lat, p.lon], { radius: 5, weight: 2, fillOpacity: 1 })
        .bindPopup(img ? `<img src="${api.fileUrl(img)}" alt="" style="width:160px;border-radius:6px;display:block"><div>${label}</div>` : label)
        .addTo(map)
      photoMarkers.set(p.id, m)
    }
    marker = L.circleMarker(detail.route[0], { radius: 6, weight: 2, opacity: 0, fillOpacity: 0 }).addTo(map)
    map.fitBounds(line.getBounds(), { padding: [24, 24] })
    drawHeat()
  })

  $effect(() => {
    void metric
    drawHeat()
  })
  $effect(() => {
    void activePhoto
    paint()
  })
  // 테마는 클래스가 실제로 바뀐 뒤에 칠한다.
  $effect(() => onThemeApplied(paint))

  $effect(() => {
    const p = highlight >= 0 ? detail.series.latLon[highlight] : null
    if (!marker) return
    if (p) marker.setLatLng(p).setStyle({ opacity: 1, fillOpacity: 1 })
    else marker.setStyle({ opacity: 0, fillOpacity: 0 })
  })

  onDestroy(() => map?.remove())
</script>

{#if detail.route.length < 2}
  <div class="grid h-[140px] place-items-center rounded-lg bg-muted p-4 text-center text-muted-foreground">GPS 기록이 없어요. 실내 운동이었거나 위치 기록이 꺼져 있었어요.</div>
{:else}
  <div class={cn('relative', className)}>
    <div class="map isolate size-full overflow-hidden rounded-lg border bg-muted" bind:this={el}></div>
    {#if hasSpeed || hasHr}
      <div class="absolute top-2 right-2 z-10 flex items-center gap-2 rounded-md border bg-card/90 px-1.5 py-1 shadow-sm backdrop-blur-sm">
        <span class="pl-1 text-[11px] text-muted-foreground">경로 색</span>
        <ToggleGroup.Root type="single" size="sm" aria-label="경로 색 기준" bind:value={() => metric, v => { if (v) metric = v as Metric }}>
          <ToggleGroup.Item value="speed" disabled={!hasSpeed} class="h-6 px-2 text-xs">속도</ToggleGroup.Item>
          <ToggleGroup.Item value="hr" disabled={!hasHr} class="h-6 px-2 text-xs">심박</ToggleGroup.Item>
        </ToggleGroup.Root>
        <span class="flex items-center gap-1 pr-1 text-[10px] text-muted-foreground" aria-hidden="true">
          낮음<i class="inline-block h-1.5 w-10 rounded-full bg-gradient-to-r from-heat-0 via-[var(--heat-2)] to-heat-4"></i>높음
        </span>
      </div>
    {/if}
  </div>
{/if}

<style>
  /* 어두운 테마: 같은 OSM 타일을 뒤집어 어둡게 (새 외부 서비스 없이). */
  :global(.dark) .map :global(.leaflet-tile) { filter: invert(1) hue-rotate(180deg) brightness(.85) contrast(.9); }
  .map:global(.leaflet-container) { background: var(--muted); }
</style>
