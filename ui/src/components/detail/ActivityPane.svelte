<script lang="ts">
  // 상세 패널: 머리(종목·제목·날짜·게시 상태·글 복사) + 탭(기록 / 게시 준비).
  import StatusBadge from '$lib/components/StatusBadge.svelte'
  import { Button } from '$lib/components/ui/button'
  import * as Tabs from '$lib/components/ui/tabs'
  import { api } from '$lib/api'
  import type { ActivityDetail } from '$lib/bindings/ActivityDetail'
  import type { PostStatus } from '$lib/bindings/PostStatus'
  import { photoPins } from '$lib/profile'
  import { nearestIndex, toChartSeries } from '$lib/series'
  import { cn } from '$lib/utils'
  import RouteMap from '../RouteMap.svelte'
  import SplitsTable from '../SplitsTable.svelte'
  import DistanceProfile from './DistanceProfile.svelte'
  import MetricStrip from './MetricStrip.svelte'
  import PhotoRail from './PhotoRail.svelte'
  import PostTab from './PostTab.svelte'
  import { ask } from '$lib/confirm.svelte'
  import { createPostEditor } from '$lib/postEditor.svelte'

  let { id, status, onChanged }: { id: string; status: PostStatus; onChanged: () => void } = $props()

  let detail = $state<ActivityDetail | null>(null)
  let error = $state('')
  let tab = $state<'record' | 'post'>('record')
  let highlight = $state(-1)
  let activePin = $state<number | null>(null)
  let note = $state('')
  // 게시 탭을 벗어나도 남는 편집 상태 (저장 안 한 수정 확인·글 복사가 화면 글을 본다).
  const editor = createPostEditor()

  /** 게시 탭에 저장 안 한 수정이 있으면 나가기 전에 묻는다. Workbench도 다른 운동을 고르기 전에 부른다. */
  export async function requestLeave(): Promise<boolean> {
    if (!editor.dirty) return true
    const ok = await ask({ title: '저장하지 않은 수정 내용이 있어요. 나갈까요?', description: '나가면 고친 글이 사라져요.', action: '나가기', cancel: '계속 고치기', destructive: true })
    if (ok) editor.discard()
    return ok
  }
  async function changeTab(v: string) {
    if (v === tab) return
    if (tab === 'post' && !(await requestLeave())) return
    tab = v as 'record' | 'post'
  }

  $effect(() => {
    detail = null
    error = ''
    api.activityDetail(id).then(d => (detail = d)).catch(e => (error = String(e)))
  })

  const series = $derived(detail ? toChartSeries(detail) : null)
  const pins = $derived(detail ? photoPins(detail.photos, detail.metrics.distanceKm) : [])

  // 사진에 마우스를 올리면 그 km 지점을 지도·그래프에 표시한다.
  function focusPin(pinId: number | null) {
    activePin = pinId
    const p = pins.find(x => x.id === pinId)
    highlight = p && series ? nearestIndex(series.x, p.km) : -1
  }

  async function copyPost() {
    try {
      // 게시 탭에서 고친 글이 있으면 화면 글을, 아니면 저장된 글을 복사한다.
      await api.copyText(editor.loaded ? editor.fullText : (await api.postView(id)).fullText)
      note = '글을 복사했어요. 스레드에 붙여 넣으면 돼요'
    } catch (e) {
      note = `복사하지 못했어요: ${e}`
    }
  }
</script>

{#if error}
  <p class="text-destructive" role="alert">운동을 불러오지 못했어요: {error}</p>
{:else if detail && series}
  <Tabs.Root class="flex-col gap-0" bind:value={() => tab, v => void changeTab(v)}>
    <header class="mb-3.5 flex flex-wrap items-center gap-x-3 gap-y-2">
      <h2 class="m-0 flex items-center gap-2 text-xl font-bold tracking-tight">
        <i class={cn('inline-block h-5 w-1 rounded-sm', detail.sport === 'running' ? 'bg-run' : detail.sport === 'cycling' ? 'bg-ride' : 'bg-faint')}></i>
        {detail.title}
      </h2>
      <span class="text-[13px] text-muted-foreground">{detail.date}</span>
      <StatusBadge {status} />
      <span class="flex-1"></span>
      <Tabs.List>
        <Tabs.Trigger value="record">기록</Tabs.Trigger>
        <Tabs.Trigger value="post">게시 준비</Tabs.Trigger>
      </Tabs.List>
      <Button size="sm" onclick={copyPost}>글 복사</Button>
    </header>
    {#if note}<p class="-mt-2 mb-3 text-sm text-done" role="status">{note}</p>{/if}

    <Tabs.Content value="record" class="grid gap-3.5">
      <MetricStrip m={detail.metrics} />
      <div class={cn('grid gap-3.5', pins.length && 'grid-cols-[minmax(0,1fr)_210px] max-md:grid-cols-1')}>
        <RouteMap {detail} {highlight} activePhoto={activePin} />
        <PhotoRail {pins} active={activePin} onFocusPin={focusPin} />
      </div>
      <DistanceProfile {series} {pins} {highlight} {activePin} onHover={i => { highlight = i; activePin = null }} onFocusPin={focusPin} />
      <div class="max-w-xl"><SplitsTable splits={detail.splits} totalKm={detail.metrics.distanceKm} sport={detail.sport} /></div>
    </Tabs.Content>

    <Tabs.Content value="post">
      <PostTab {id} {editor} onStatusChange={() => onChanged()} />
    </Tabs.Content>
  </Tabs.Root>
{:else}
  <p class="text-muted-foreground">불러오는 중…</p>
{/if}
