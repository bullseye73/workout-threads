<script lang="ts">
  // 날짜를 골라 기록 추가: 그 날짜의 FIT와 그 날짜에 찍은 사진만 넣는다.
  import CalendarPlus from '@lucide/svelte/icons/calendar-plus'
  import { Button } from '$lib/components/ui/button'
  import { dayLabel } from '$lib/workbench'

  let { date, busy = false, hasWorkout = false, onPick, onClose }: {
    date: string; busy?: boolean; hasWorkout?: boolean
    onPick: (kind: 'files' | 'folder') => void; onClose: () => void
  } = $props()
</script>

<section class="mx-auto mt-6 grid max-w-md gap-4 rounded-xl border bg-card p-6" aria-labelledby="day-import-title">
  <div class="flex items-center gap-3">
    <span class="grid size-10 place-items-center rounded-lg bg-selected text-ride"><CalendarPlus class="size-5" /></span>
    <div>
      <h2 id="day-import-title" class="m-0 text-lg font-bold tracking-tight">{dayLabel(date)} 기록 추가</h2>
      <p class="m-0 text-sm text-muted-foreground">{hasWorkout ? '이 날짜에 운동을 더 넣거나 사진을 보탤 수 있어요.' : '이 날짜에는 아직 운동이 없어요.'}</p>
    </div>
  </div>
  <p class="m-0 text-sm leading-relaxed">
    이 날짜의 FIT 파일과 이 날짜에 찍은 사진만 넣어요. 날짜가 다른 파일은 넣지 않고 알려 드려요.
  </p>
  <div class="flex flex-wrap gap-2">
    <Button onclick={() => onPick('files')} disabled={busy}>파일 고르기</Button>
    <Button variant="outline" onclick={() => onPick('folder')} disabled={busy}>폴더 고르기</Button>
    <span class="flex-1"></span>
    <Button variant="ghost" onclick={onClose} disabled={busy}>닫기</Button>
  </div>
  <p class="m-0 text-xs text-muted-foreground" role="status">
    {busy ? '가져오는 중…' : '창에 끌어다 놓아도 이 날짜로 등록돼요.'}
  </p>
</section>
