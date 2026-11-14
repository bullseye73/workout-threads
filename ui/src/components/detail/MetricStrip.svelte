<script lang="ts">
  // 핵심 수치 한 줄. 거리 칸을 크게.
  import type { MetricsDto } from '$lib/bindings/MetricsDto'
  import { km } from '$lib/format'

  let { m }: { m: MetricsDto } = $props()

  const cells = $derived(
    [
      ['이동 시간', m.moving, ''],
      ['경과 시간', m.elapsed, ''],
      m.pace ? ['평균 페이스', m.pace, '/km'] : null,
      m.speedKmh !== null ? ['평균 속도', m.speedKmh.toFixed(1), 'km/h'] : null,
      m.avgHr !== null ? ['평균 심박', String(m.avgHr), 'bpm'] : null,
      m.maxHr !== null ? ['최대 심박', String(m.maxHr), 'bpm'] : null,
      m.ascentM !== null ? ['상승 고도', String(m.ascentM), 'm'] : null,
      m.avgCadence !== null ? ['케이던스', String(m.avgCadence), ''] : null,
    ].filter((c): c is string[] => c !== null),
  )
</script>

<dl class="m-0 grid grid-cols-[repeat(auto-fit,minmax(96px,1fr))] overflow-hidden rounded-lg border bg-card">
  <div class="border-r px-3 py-2">
    <dt class="text-[11px] text-muted-foreground">거리</dt>
    <dd class="m-0 text-2xl font-bold tracking-tight">{km(m.distanceKm)}<small class="ml-0.5 text-[11px] font-semibold text-muted-foreground">km</small></dd>
  </div>
  {#each cells as [label, value, unit]}
    <div class="border-r px-3 py-2 last:border-r-0">
      <dt class="text-[11px] text-muted-foreground">{label}</dt>
      <dd class="m-0 text-lg font-bold tracking-tight">{value}{#if unit}<small class="ml-0.5 text-[11px] font-semibold text-muted-foreground">{unit}</small>{/if}</dd>
    </div>
  {/each}
</dl>
