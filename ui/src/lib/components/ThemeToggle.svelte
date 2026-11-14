<script lang="ts">
  // 시스템 / 밝게 / 어둡게. 좁은 화면에서는 아이콘만 보인다.
  import Monitor from '@lucide/svelte/icons/monitor'
  import Moon from '@lucide/svelte/icons/moon'
  import Sun from '@lucide/svelte/icons/sun'
  import { setMode, userPrefersMode } from 'mode-watcher'
  import * as ToggleGroup from '$lib/components/ui/toggle-group'
  import { THEME_OPTIONS, type ThemePref } from '$lib/theme'

  const ICON = { system: Monitor, light: Sun, dark: Moon } as const
</script>

<ToggleGroup.Root type="single" variant="outline" size="sm" aria-label="화면 테마"
  value={userPrefersMode.current}
  onValueChange={v => { if (v) setMode(v as ThemePref) }}>
  {#each THEME_OPTIONS as o (o.value)}
    {@const Icon = ICON[o.value]}
    <ToggleGroup.Item value={o.value} aria-label={o.label} title={o.label}>
      <Icon class="size-3.5" /><span class="max-sm:hidden">{o.label}</span>
    </ToggleGroup.Item>
  {/each}
</ToggleGroup.Root>
