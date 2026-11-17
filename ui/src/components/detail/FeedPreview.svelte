<script lang="ts">
  // 피드에 올라갔을 때 모습 (본문, 해시태그, 카드 가로 줄).
  import { api } from '$lib/api'

  let { body, hashtags, cards, cacheBust = 0 }: { body: string; hashtags: string[]; cards: string[]; cacheBust?: number } = $props()

  let previewCard = $state<string | null>(null)
</script>

<div class="rounded-xl border bg-card p-3">
  <div class="mb-1.5 flex items-center gap-2 text-[13px]"><span class="size-7 rounded-full bg-sunken"></span><b>내 계정</b></div>
  <p class="m-0 mb-1 text-sm whitespace-pre-line">{body || '본문이 비어 있어요.'}</p>
  {#if hashtags.length}<p class="m-0 text-[13px] text-ride">{hashtags.join(' ')}</p>{/if}
  {#if cards.length}
    <div class="mt-2 flex gap-1 overflow-x-auto">
      {#each cards as c}
        <button type="button" class="cursor-pointer border-0 p-0 bg-transparent shrink-0 focus-visible:outline-2 focus-visible:outline-ride rounded-md transition-opacity hover:opacity-90"
          onclick={() => previewCard = c} aria-label="카드 크게 보기">
          <img src="{api.fileUrl(c)}{cacheBust ? `?v=${cacheBust}` : ''}" alt="카드" loading="lazy" class="aspect-[9/16] h-[170px] rounded-md object-cover pointer-events-none" />
        </button>
      {/each}
    </div>
  {/if}
</div>

{#if previewCard}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/80 p-4 backdrop-blur-xs animate-in fade-in"
    role="dialog" aria-modal="true" aria-label="카드 원본 미리보기" tabindex="-1">
    <button type="button" class="fixed inset-0 size-full cursor-default border-0 bg-transparent"
      onclick={() => previewCard = null} aria-label="배경 클릭해 닫기"></button>
    <div class="relative z-10 flex flex-col items-center max-h-[92vh] max-w-[90vw]">
      <button type="button" class="absolute -top-10 right-0 cursor-pointer rounded-full bg-white/20 p-1.5 text-white hover:bg-white/40 transition-colors"
        onclick={() => previewCard = null} aria-label="닫기">
        <svg class="size-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 6L6 18M6 6l12 12"/></svg>
      </button>
      <img src="{api.fileUrl(previewCard)}{cacheBust ? `?v=${cacheBust}` : ''}" alt="카드 원본"
        class="max-h-[85vh] w-auto aspect-[9/16] rounded-xl object-contain shadow-2xl border border-white/10" />
      <span class="mt-2 text-xs text-white/70">클릭하여 닫기</span>
    </div>
  </div>
{/if}
