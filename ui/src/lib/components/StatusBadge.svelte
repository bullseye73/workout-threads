<script lang="ts">
  // 게시 상태: 글 없음(점선 원) / 게시 대기(호박색) / 게시함(초록). 종목 색과 겹치지 않게.
  import type { PostStatus } from '$lib/bindings/PostStatus'
  import { STATUS_LABEL } from '$lib/theme'
  import { cn } from '$lib/utils'

  let { status, size = 'sm', class: className = '' }: { status: PostStatus; size?: 'sm' | 'md'; class?: string } = $props()
</script>

<span class={cn('inline-flex items-center gap-1.5 font-semibold whitespace-nowrap',
  size === 'sm' ? 'text-xs' : 'text-sm',
  status === 'ready' ? 'text-wait' : status === 'published' ? 'text-done' : 'text-muted-foreground', className)}>
  <i class={cn('inline-block size-2 shrink-0 rounded-full',
    status === 'ready' ? 'bg-wait' : status === 'published' ? 'bg-done' : 'border-[1.5px] border-dashed border-faint')}></i>
  {STATUS_LABEL[status]}
</span>
