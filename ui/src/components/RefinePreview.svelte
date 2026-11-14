<script lang="ts">
  // AI가 다듬은 글을 지금 글과 나란히 보여 주고, 적용할지 고르게 한다.
  import type { RefineDto } from '../lib/bindings/RefineDto'
  let { current, refined, label = 'AI', onApply, onCancel }: {
    current: string; refined: RefineDto; label?: string; onApply: () => void; onCancel: () => void
  } = $props()
</script>

<div class="backdrop" role="presentation" onclick={onCancel}></div>
<div class="panel" role="dialog" aria-modal="true" aria-labelledby="refine-title">
  <h2 id="refine-title">{label}가 다듬은 글</h2>
  <div class="cols">
    <section><h3>지금 글</h3><pre>{current}</pre></section>
    <section class="new"><h3>다듬은 글</h3><pre>{refined.fullText}</pre></section>
  </div>
  <p class="note">적용하면 본문과 해시태그가 바뀌고 '직접 고침'으로 저장돼요.</p>
  <div class="foot">
    <button class="btn" onclick={onCancel}>그대로 두기</button>
    <button class="btn primary" onclick={onApply}>다듬은 글 적용</button>
  </div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; background: rgba(27, 36, 48, .35); }
  .panel { position: fixed; top: 8vh; left: 50%; transform: translateX(-50%); width: min(860px, calc(100vw - 32px)); max-height: 84vh; overflow: auto;
    background: var(--surface); border-radius: 18px; padding: 24px; box-shadow: 0 20px 60px rgba(27, 36, 48, .25); }
  h2 { margin: 0 0 14px; font-size: 20px; }
  h3 { margin: 0 0 6px; font-size: 14px; color: var(--muted-foreground); font-weight: 600; }
  .cols { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
  pre { white-space: pre-wrap; font: inherit; line-height: 1.6; margin: 0; padding: 14px; border-radius: 10px; background: var(--paper); min-height: 160px; }
  .new pre { background: color-mix(in srgb, var(--ride) 7%, white); }
  .note { color: var(--muted-foreground); font-size: 13px; margin: 12px 0 0; }
  .foot { display: flex; justify-content: flex-end; gap: 8px; margin-top: 16px; }
  @media (max-width: 700px) { .cols { grid-template-columns: 1fr; } }
</style>
