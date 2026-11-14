<script lang="ts">
  // 사진 크게 보기: 좌우 버튼·화살표 키로 넘기고, 보고 있는 사진의 지점을 지도에 표시한다.
  import ChevronLeft from '@lucide/svelte/icons/chevron-left'
  import ChevronRight from '@lucide/svelte/icons/chevron-right'
  import { Button } from '$lib/components/ui/button'
  import * as Dialog from '$lib/components/ui/dialog'
  import { api } from '$lib/api'
  import type { Pin } from '$lib/profile'

  let { pins, index = $bindable(), onClose, onShow }: {
    pins: Pin[]; index: number; onClose: () => void; onShow: (id: number) => void
  } = $props()

  const p = $derived(pins[index])
  $effect(() => { if (p) onShow(p.id) })

  function go(d: number) {
    index = (index + d + pins.length) % pins.length
  }
  function key(e: KeyboardEvent) {
    if (e.key === 'ArrowLeft') { e.preventDefault(); go(-1) }
    if (e.key === 'ArrowRight') { e.preventDefault(); go(1) }
  }
</script>

<Dialog.Root open onOpenChange={o => { if (!o) onClose() }}>
  <Dialog.Content class="gap-3 sm:max-w-[min(92vw,640px)]" onkeydown={key}>
    <Dialog.Header>
      <Dialog.Title>{p ? `${p.km.toFixed(1)}km 지점` : '사진'}</Dialog.Title>
      <Dialog.Description>{index + 1} / {pins.length}</Dialog.Description>
    </Dialog.Header>
    {#if p}
      <div class="relative grid place-items-center">
        <img src={api.fileUrl(p.src)} alt="{p.km.toFixed(1)}km 지점 사진" class="max-h-[70vh] w-auto rounded-lg object-contain" />
        {#if pins.length > 1}
          <Button variant="secondary" size="icon" class="absolute top-1/2 left-1 -translate-y-1/2 opacity-90" onclick={() => go(-1)} aria-label="이전 사진"><ChevronLeft /></Button>
          <Button variant="secondary" size="icon" class="absolute top-1/2 right-1 -translate-y-1/2 opacity-90" onclick={() => go(1)} aria-label="다음 사진"><ChevronRight /></Button>
        {/if}
      </div>
      <Dialog.Footer>
        <Button variant="outline" size="sm" onclick={() => api.openFolder(p.src)}>파일로 열기</Button>
      </Dialog.Footer>
    {/if}
  </Dialog.Content>
</Dialog.Root>
