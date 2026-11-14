<script lang="ts">
  // 2D 경로 지도: 경로 선, 출발·도착 핀, 사진 지점, 그래프와 연동되는 강조 점. 색은 테마 토큰.
  import 'leaflet/dist/leaflet.css'
  import L from 'leaflet'
  import { mode } from 'mode-watcher'
  import { onDestroy, onMount } from 'svelte'
  import { api } from '$lib/api'
  import type { ActivityDetail } from '$lib/bindings/ActivityDetail'
  import { cssVar } from '$lib/theme'

  let { detail, highlight = -1, activePhoto = null }: { detail: ActivityDetail; highlight?: number; activePhoto?: number | null } = $props()
  let el = $state<HTMLDivElement>()
  let map: L.Map | null = null
  let line: L.Polyline | null = null
  let start: L.CircleMarker | null = null
  let end: L.CircleMarker | null = null
  let marker: L.CircleMarker | null = null
  const photoMarkers = new Map<number, L.CircleMarker>()
  const TILE = 'https://tile.openstreetmap.org/{z}/{x}/{y}.png'

  /** 테마가 바뀌면 선·핀 색만 다시 칠한다 (타일은 CSS 필터). */
  function paint() {
    const ring = cssVar('--card')
    line?.setStyle({ color: cssVar('--ride') })
    start?.setStyle({ color: ring, fillColor: cssVar('--done') })
    end?.setStyle({ color: ring, fillColor: cssVar('--hr') })
    for (const [id, m] of photoMarkers) m.setStyle({ color: ring, fillColor: cssVar('--run'), radius: id === activePhoto ? 8 : 5 })
    marker?.setStyle({ color: cssVar('--foreground'), fillColor: ring })
  }

  onMount(() => {
    if (detail.route.length < 2 || !el) return
    // 페이지 스크롤과 겹치지 않게, 지도를 클릭한 뒤에만 휠 확대를 켠다.
    map = L.map(el, { zoomControl: true, attributionControl: true, scrollWheelZoom: false })
    map.on('click', () => map?.scrollWheelZoom.enable())
    map.on('mouseout', () => map?.scrollWheelZoom.disable())
    L.tileLayer(TILE, { maxZoom: 19, attribution: '&copy; OpenStreetMap contributors' }).addTo(map)
    line = L.polyline(detail.route, { weight: 4, opacity: 0.9 }).addTo(map)
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
    paint()
  })

  $effect(() => {
    void mode.current
    void activePhoto
    paint()
  })

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
  <div class="map h-[300px] overflow-hidden rounded-lg border bg-muted" bind:this={el}></div>
{/if}

<style>
  /* 어두운 테마: 같은 OSM 타일을 뒤집어 어둡게 (새 외부 서비스 없이). */
  :global(.dark) .map :global(.leaflet-tile) { filter: invert(1) hue-rotate(180deg) brightness(.85) contrast(.9); }
  .map:global(.leaflet-container) { background: var(--muted); }
</style>
