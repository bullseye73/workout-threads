<script lang="ts">
  // 2D 경로 지도: 경로 선, 시작(초록)·종료(빨강) 핀, 사진 지점, 그래프와 연동되는 강조 점.
  import 'leaflet/dist/leaflet.css'
  import L from 'leaflet'
  import { onDestroy, onMount } from 'svelte'
  import { api } from '../lib/api'
  import type { ActivityDetail } from '../lib/bindings/ActivityDetail'

  let { detail, highlight = -1 }: { detail: ActivityDetail; highlight?: number } = $props()
  let el = $state<HTMLDivElement>()
  let map: L.Map | null = null
  let marker: L.CircleMarker | null = null
  const TILE = 'https://tile.openstreetmap.org/{z}/{x}/{y}.png'

  onMount(() => {
    if (detail.route.length < 2) return
    if (!el) return
    // 페이지 스크롤과 겹치지 않게, 지도를 클릭한 뒤에만 휠 확대를 켠다.
    map = L.map(el, { zoomControl: true, attributionControl: true, scrollWheelZoom: false })
    map.on('click', () => map?.scrollWheelZoom.enable())
    map.on('mouseout', () => map?.scrollWheelZoom.disable())
    L.tileLayer(TILE, { maxZoom: 19, attribution: '&copy; OpenStreetMap contributors' }).addTo(map)
    const line = L.polyline(detail.route, { color: '#2f6feb', weight: 4, opacity: 0.9 }).addTo(map)
    if (detail.start) L.circleMarker(detail.start, { radius: 7, color: '#fff', weight: 2, fillColor: '#1f9d6b', fillOpacity: 1 }).bindTooltip('출발').addTo(map)
    if (detail.end) L.circleMarker(detail.end, { radius: 7, color: '#fff', weight: 2, fillColor: '#e5484d', fillOpacity: 1 }).bindTooltip('도착').addTo(map)
    for (const p of detail.photos) {
      if (p.lat === null || p.lon === null) continue
      const img = p.cardPath ?? p.path
      const label = p.kmMark !== null ? `${p.kmMark.toFixed(1)}km 지점` : '사진'
      L.circleMarker([p.lat, p.lon], { radius: 5, color: '#fff', weight: 2, fillColor: '#f07a2a', fillOpacity: 1 })
        .bindPopup(img ? `<img src="${api.fileUrl(img)}" alt="" style="width:160px;border-radius:6px;display:block"><div>${label}</div>` : label)
        .addTo(map)
    }
    marker = L.circleMarker(detail.route[0], { radius: 6, color: '#1b2430', weight: 2, fillColor: '#fff', opacity: 0, fillOpacity: 0 }).addTo(map)
    map.fitBounds(line.getBounds(), { padding: [24, 24] })
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
  <div class="nogps">GPS 기록이 없어요. 실내 운동이었거나 위치 기록이 꺼져 있었어요.</div>
{:else}
  <div class="map" bind:this={el}></div>
{/if}

<style>
  .map { height: 380px; border-radius: 14px; overflow: hidden; background: var(--track); }
  .nogps { height: 140px; display: grid; place-items: center; border-radius: 14px; background: var(--track); color: var(--muted-foreground); padding: 16px; text-align: center; }
</style>
