<script lang="ts">
  import type { SplitDto } from '../lib/bindings/SplitDto'
  let { splits }: { splits: SplitDto[] } = $props()
</script>

<table>
  <thead><tr><th>km</th><th>시간</th><th>평균 심박</th></tr></thead>
  <tbody>
    {#each splits as s}
      <tr><td>{s.km}</td><td>{s.duration}</td><td>{s.avgHr ?? '—'}</td></tr>
    {/each}
  </tbody>
</table>

<style>
  table { width: 100%; border-collapse: collapse; font-size: 14px; }
  th { text-align: left; font-weight: 600; color: var(--muted-foreground); font-size: 13px; padding: 6px 8px; border-bottom: 1px solid var(--line); }
  td { padding: 6px 8px; border-bottom: 1px solid var(--line); }
  tbody tr:last-child td { border-bottom: 0; }
</style>
