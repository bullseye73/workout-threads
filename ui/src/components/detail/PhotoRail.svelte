<script lang="ts">
  // 지도 옆 사진 묶음. 마우스·포커스를 올리면 그 사진이 찍힌 지점이 지도와 그래프에 표시된다.
  import { api } from '$lib/api'
  import type { Pin } from '$lib/profile'
  import { cn } from '$lib/utils'

  let { pins, active, onFocusPin }: { pins: Pin[]; active: number | null; onFocusPin: (id: number | null) => void } = $props()
</script>

{#if pins.length}
  <div class="grid max-h-[300px] grid-cols-2 content-start gap-1.5 overflow-y-auto max-md:max-h-none max-md:grid-cols-4"
    aria-label="경로 위 사진" role="group">
    {#each pins as p (p.id)}
      <button class={cn('relative aspect-[9/16] overflow-hidden rounded-md bg-sunken',
          active === p.id && 'outline-2 outline-offset-1 outline-run')}
        onmouseenter={() => onFocusPin(p.id)} onmouseleave={() => onFocusPin(null)}
        onfocus={() => onFocusPin(p.id)} onblur={() => onFocusPin(null)}
        onclick={() => api.openFolder(p.src)} aria-label="{p.km.toFixed(1)}km 지점 사진 열기">
        <img src={api.fileUrl(p.src)} alt="" loading="lazy" class="size-full object-cover" />
        <span class="absolute bottom-1 left-1 rounded bg-foreground/70 px-1.5 text-[11px] font-bold text-background">{p.km.toFixed(1)}km</span>
      </button>
    {/each}
  </div>
{/if}
