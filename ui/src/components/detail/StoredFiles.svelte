<script lang="ts">
  // 보관된 자료: 이 운동의 FIT·원본 사진·카드가 앱 보관함 어디에 있는지 (기본 접힘).
  import ChevronRight from '@lucide/svelte/icons/chevron-right'
  import { Button } from '$lib/components/ui/button'
  import { api } from '$lib/api'
  import type { ActivityDetail } from '$lib/bindings/ActivityDetail'
  import { cn } from '$lib/utils'

  let { detail }: { detail: ActivityDetail } = $props()
  let open = $state(false)
  const fitName = $derived(detail.fitFile.split('/').pop() ?? '')
  const cards = $derived(detail.photos.filter(p => p.cardPath).length)
</script>

<section class="rounded-lg border bg-card">
  <button class="flex w-full items-center gap-2 px-3.5 py-2.5 text-left text-[13px] font-bold" aria-expanded={open}
    onclick={() => (open = !open)}>
    <ChevronRight class={cn('size-4 transition-transform', open && 'rotate-90')} />보관된 자료
    <span class="font-normal text-muted-foreground">FIT 1개, 사진 {detail.photos.length}장, 카드 {cards}장</span>
  </button>
  {#if open}
    <div class="grid gap-1.5 border-t px-3.5 py-2.5 text-sm">
      <div><span class="text-muted-foreground">FIT 파일</span> {fitName}</div>
      <div><span class="text-muted-foreground">원본 사진</span> {detail.photos.length}장 <span class="text-muted-foreground">카드</span> {cards}장</div>
      <div class="flex flex-wrap items-center gap-2">
        <code class="min-w-0 truncate rounded bg-muted px-1.5 py-0.5 text-xs" title={detail.folder}>{detail.folder}</code>
        <Button variant="outline" size="sm" onclick={() => api.openFolder(detail.folder)}>폴더 열기</Button>
      </div>
    </div>
  {/if}
</section>
