<script lang="ts">
  // 고리를 종목 비율로 나눠 칠한다. ratio(0..1)는 목표 대비 채움 정도, 0이면 회색 고리만.
  type Seg = { value: number; color: string }
  let { segments, ratio, size = 220, stroke = 16, children }: {
    segments: Seg[]; ratio: number; size?: number; stroke?: number; children?: import('svelte').Snippet
  } = $props()
  const r = $derived((size - stroke) / 2)
  const c = $derived(2 * Math.PI * r)
  const total = $derived(segments.reduce((t, s) => t + s.value, 0))
  const arcs = $derived.by(() => {
    let offset = 0
    return segments.filter(s => s.value > 0 && total > 0).map(s => {
      const len = (s.value / total) * ratio * c
      const a = { color: s.color, len, offset }
      offset += len
      return a
    })
  })
</script>

<div class="gauge" style:width="{size}px" style:height="{size}px">
  <svg viewBox="0 0 {size} {size}" aria-hidden="true">
    <circle cx={size / 2} cy={size / 2} {r} fill="none" stroke="var(--track)" stroke-width={stroke} />
    {#each arcs as a}
      <circle cx={size / 2} cy={size / 2} {r} fill="none" stroke={a.color} stroke-width={stroke}
        stroke-dasharray="{a.len} {c - a.len}" stroke-dashoffset={-a.offset}
        transform="rotate(-90 {size / 2} {size / 2})" />
    {/each}
  </svg>
  <div class="center">{@render children?.()}</div>
</div>

<style>
  .gauge { position: relative; flex: none; }
  svg { width: 100%; height: 100%; }
  .center { position: absolute; inset: 0; display: grid; place-items: center; text-align: center; }
</style>
