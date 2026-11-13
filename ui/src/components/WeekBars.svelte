<script lang="ts">
  // 주별 거리 막대 (그 달과 겹치는 주, 월요일 시작).
  let { weeks, color }: { weeks: { label: string; km: number }[]; color: string } = $props()
  const max = $derived(Math.max(1, ...weeks.map(w => w.km)))
</script>

<div class="bars" role="img" aria-label={weeks.map(w => `${w.label} ${w.km.toFixed(1)}km`).join(', ')}>
  {#each weeks as w}
    <div class="col">
      <div class="track"><div class="bar" style:height="{(w.km / max) * 100}%" style:background={color}></div></div>
      <span class="val">{w.km > 0 ? w.km.toFixed(0) : ''}</span>
      <span class="lbl">{w.label}</span>
    </div>
  {/each}
</div>

<style>
  .bars { display: flex; gap: 10px; align-items: flex-end; height: 132px; }
  .col { flex: 1; display: flex; flex-direction: column; align-items: center; height: 100%; }
  .track { flex: 1; width: 100%; max-width: 30px; display: flex; align-items: flex-end; }
  .bar { width: 100%; min-height: 0; border-radius: 6px 6px 2px 2px; }
  .val { font-size: 12px; font-weight: 600; min-height: 18px; }
  .lbl { font-size: 12px; color: var(--muted); }
</style>
