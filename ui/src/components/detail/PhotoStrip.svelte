<script lang="ts">
  // 지도 아래 사진 줄: 자리에 들어가는 만큼만 보여 주고(스크롤 없음), 넘치면 마지막 칸이 "+N".
  // 마우스·포커스를 올리면 지도와 그래프에 그 지점을 표시하고, 누르면 크게 본다.
  import { api } from '$lib/api'
  import { fitItems } from '$lib/heat'
  import type { Pin } from '$lib/profile'
  import { cn } from '$lib/utils'
  import PhotoViewer from './PhotoViewer.svelte'

  let { pins, active, onFocusPin }: { pins: Pin[]; active: number | null; onFocusPin: (id: number | null) => void } = $props()

  const TILE_W = 62, GAP = 6
  let width = $state(0)
  let viewing = $state<number | null>(null)
  const capacity = $derived(Math.max(1, Math.floor((width + GAP) / (TILE_W + GAP))))
  const fit = $derived(fitItems(pins, capacity))
</script>

{#if pins.length}
  <div class="flex gap-1.5 overflow-hidden" bind:clientWidth={width} role="group" aria-label="경로 위 사진 {pins.length}장">
    {#each fit.shown as p, i (p.id)}
      <button class={cn('relative aspect-[9/16] w-[62px] shrink-0 overflow-hidden rounded-md bg-sunken',
          active === p.id && 'outline-2 outline-offset-1 outline-run')}
        onmouseenter={() => onFocusPin(p.id)} onmouseleave={() => onFocusPin(null)}
        onfocus={() => onFocusPin(p.id)} onblur={() => onFocusPin(null)}
        onclick={() => (viewing = i)} aria-label="{p.km.toFixed(1)}km 지점 사진 크게 보기">
        <img src={api.fileUrl(p.src)} alt="" loading="lazy" class="size-full object-cover" />
        <span class="absolute bottom-1 left-1 rounded bg-foreground/70 px-1 text-[10px] font-bold text-background">{p.km.toFixed(1)}</span>
      </button>
    {/each}
    {#if fit.more}
      {@const next = fit.shown.length}
      <button class="relative aspect-[9/16] w-[62px] shrink-0 overflow-hidden rounded-md bg-sunken"
        onclick={() => (viewing = next)} aria-label="사진 {fit.more}장 더 보기">
        <img src={api.fileUrl(pins[next].src)} alt="" loading="lazy" class="size-full object-cover opacity-40" />
        <span class="absolute inset-0 grid place-items-center text-base font-bold">+{fit.more}</span>
      </button>
    {/if}
  </div>
{/if}

{#if viewing !== null}
  <PhotoViewer {pins} bind:index={viewing} onShow={onFocusPin} onClose={() => { viewing = null; onFocusPin(null) }} />
{/if}
