<script lang="ts">
  // 피드에 올라갔을 때 모습 (본문, 해시태그, 카드 가로 줄).
  import { api } from '$lib/api'

  let { body, hashtags, cards, cacheBust = 0 }: { body: string; hashtags: string[]; cards: string[]; cacheBust?: number } = $props()
</script>

<div class="rounded-xl border bg-card p-3">
  <div class="mb-1.5 flex items-center gap-2 text-[13px]"><span class="size-7 rounded-full bg-sunken"></span><b>내 계정</b></div>
  <p class="m-0 mb-1 text-sm whitespace-pre-line">{body || '본문이 비어 있어요.'}</p>
  {#if hashtags.length}<p class="m-0 text-[13px] text-ride">{hashtags.join(' ')}</p>{/if}
  {#if cards.length}
    <div class="mt-2 flex gap-1 overflow-x-auto">
      {#each cards as c}<img src="{api.fileUrl(c)}{cacheBust ? `?v=${cacheBust}` : ''}" alt="카드" loading="lazy" class="aspect-[9/16] h-[170px] shrink-0 rounded-md object-cover" />{/each}
    </div>
  {/if}
</div>
