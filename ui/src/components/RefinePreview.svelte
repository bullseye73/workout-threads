<script lang="ts">
  // AI가 다듬은 글을 지금 글과 나란히 보여 주고, 적용할지 고르게 한다.
  import { Button } from '$lib/components/ui/button'
  import * as Dialog from '$lib/components/ui/dialog'
  import type { RefineDto } from '$lib/bindings/RefineDto'

  let { current, refined, label = 'AI', onApply, onCancel }: {
    current: string; refined: RefineDto; label?: string; onApply: () => void; onCancel: () => void
  } = $props()
</script>

<Dialog.Root open onOpenChange={o => { if (!o) onCancel() }}>
  <Dialog.Content class="max-h-[84vh] overflow-auto sm:max-w-3xl">
    <Dialog.Header>
      <Dialog.Title>{label}가 다듬은 글</Dialog.Title>
      <Dialog.Description>적용하면 본문과 해시태그가 바뀌고 '직접 고침'으로 저장돼요.</Dialog.Description>
    </Dialog.Header>
    <div class="grid grid-cols-2 gap-4 max-sm:grid-cols-1">
      <section><h3 class="mb-1.5 text-sm font-semibold text-muted-foreground">지금 글</h3>
        <pre class="m-0 min-h-40 rounded-lg bg-muted p-3.5 font-sans leading-relaxed whitespace-pre-wrap">{current}</pre></section>
      <section><h3 class="mb-1.5 text-sm font-semibold text-muted-foreground">다듬은 글</h3>
        <pre class="m-0 min-h-40 rounded-lg bg-selected p-3.5 font-sans leading-relaxed whitespace-pre-wrap">{refined.fullText}</pre></section>
    </div>
    <Dialog.Footer>
      <Button variant="outline" onclick={onCancel}>그대로 두기</Button>
      <Button onclick={onApply}>다듬은 글 적용</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
