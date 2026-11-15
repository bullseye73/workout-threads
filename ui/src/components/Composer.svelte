<script lang="ts">
  // 새 게시물: FIT·사진을 올리면 무엇이 만들어질지 먼저 보여 주고, 확인하면 앱 보관함에 저장해 운동마다 게시물을 만든다.
  import { untrack } from 'svelte'
  import { Button } from '$lib/components/ui/button'
  import * as Dialog from '$lib/components/ui/dialog'
  import { api } from '$lib/api'
  import type { GarminActivityDto } from '$lib/bindings/GarminActivityDto'
  import type { ImportIssue } from '$lib/bindings/ImportIssue'
  import type { UploadPlan } from '$lib/bindings/UploadPlan'
  import type { UploadResult } from '$lib/bindings/UploadResult'
  import { activityExclusions, createLabel } from '$lib/composer'
  import { km } from '$lib/format'

  let { incoming = null, preloadedGarmin = [], onClose, onOpenActivity, onCreated, onOpenSettings }: {
    /** 창이 열려 있을 때 끌어다 놓은 파일 (n이 바뀌면 추가). */
    incoming?: { paths: string[]; n: number } | null
    preloadedGarmin?: string[]
    onClose: () => void
    onOpenActivity: (id: string, date: string) => void
    onCreated: (r: UploadResult) => void
    onOpenSettings?: () => void
  } = $props()

  let paths = $state<string[]>([])
  let exclude = $state<string[]>([])
  let plan = $state<UploadPlan | null>(null)
  let planning = $state(false)
  let committing = $state(false)
  let error = $state('')
  let result = $state<UploadResult | null>(null)
  let seq = 0

  let garminConnected = $state(false)
  let garminPickerOpen = $state(false)
  let garminLoading = $state(false)
  let garminFetching = $state(false)
  let garminList = $state<GarminActivityDto[]>([])
  let garminSelected = $state<string[]>([])
  let garminFailed = $state<ImportIssue[]>([])
  let garminAutoPaths = $state<string[]>([])
  let garminAutoChecked = false

  function isoDate(d: Date): string {
    return d.toISOString().slice(0, 10)
  }

  $effect(() => {
    const pre = preloadedGarmin
    if (pre.length) {
      untrack(() => {
        garminAutoPaths = [...new Set([...garminAutoPaths, ...pre])]
        add(pre)
      })
    }
  })

  $effect(() => {
    api.settings().then(s => {
      garminConnected = s.garminConnected
      if (s.garminConnected && s.garminAutoCheck && !garminAutoChecked) {
        garminAutoChecked = true
        api.garminNewSinceLastCheck().then(res => {
          if (res.paths.length) {
            garminAutoPaths = [...new Set([...garminAutoPaths, ...res.paths])]
            add(res.paths)
          }
          if (res.failed.length) garminFailed = res.failed
        }).catch(() => undefined)
      }
    }).catch(() => undefined)
  })

  async function openGarminPicker() {
    garminPickerOpen = true
    garminLoading = true
    garminFailed = []
    error = ''
    const to = new Date()
    const from = new Date(to.getTime() - 6 * 86400_000)
    try {
      garminList = await api.garminActivities(isoDate(from), isoDate(to))
      garminSelected = garminList.filter(a => !a.alreadyImported).map(a => a.id)
    } catch (e) {
      error = `Garmin 활동을 불러오지 못했어요: ${e}`
    } finally {
      garminLoading = false
    }
  }

  function toggleGarmin(id: string) {
    garminSelected = garminSelected.includes(id)
      ? garminSelected.filter(x => x !== id)
      : [...garminSelected, id]
  }

  async function fetchSelectedGarmin() {
    if (!garminSelected.length) return
    garminFetching = true
    garminFailed = []
    error = ''
    try {
      const res = await api.fetchGarminFits(garminSelected)
      if (res.paths.length) add(res.paths)
      garminFailed = res.failed
      if (!res.failed.length) garminPickerOpen = false
    } catch (e) {
      error = `Garmin FIT를 받지 못했어요: ${e}`
    } finally {
      garminFetching = false
    }
  }

  function add(more: string[]) {
    const next = [...paths]
    for (const p of more) if (!next.includes(p)) next.push(p)
    paths = next
    exclude = exclude.filter(e => !more.includes(e))
  }
  $effect(() => {
    const inc = incoming
    if (inc?.paths.length) untrack(() => add(inc.paths))
  })

  // 올린 목록이 바뀔 때마다 다시 계산한다. 늦게 온 이전 결과는 버린다.
  $effect(() => {
    const p = paths, ex = exclude
    if (!p.length) { plan = null; return }
    const mine = ++seq
    untrack(() => {
      planning = true
      error = ''
      api.planUpload(p, ex)
        .then(r => { if (mine === seq) plan = r })
        .catch(e => { if (mine === seq) error = `파일을 읽지 못했어요: ${e}` })
        .finally(() => { if (mine === seq) planning = false })
    })
  })

  async function pick(kind: 'files' | 'folder') {
    add(await api.pickImport(kind))
  }
  const removeActivity = (source: string) => { if (plan) exclude = [...exclude, ...activityExclusions(plan, source)] }
  const removePhoto = (path: string) => { exclude = [...exclude, path] }

  const nothing = $derived(!plan || (plan.newActivities.length === 0 && !plan.existing.some(e => e.photos.length)))
  const additions = $derived(plan?.existing.filter(e => e.photos.length) ?? [])
  const reuploaded = $derived(plan?.existing.filter(e => e.uploadedFit && !e.photos.length) ?? [])
  const ICON = { running: '🏃', cycling: '🚲', walking: '👟', hiking: '👟', other: '👟' } as const

  async function create() {
    committing = true
    error = ''
    try {
      const r = await api.commitUpload(paths, exclude)
      result = r
      onCreated(r)
      // 하나만 만들었으면 바로 연다.
      const only = r.created.length === 1 && r.updated.length === 0 ? plan?.newActivities.find(a => a.activityId === r.created[0]) : null
      if (only) onOpenActivity(only.activityId, only.date)
    } catch (e) {
      error = `저장하지 못했어요: ${e}`
    } finally {
      committing = false
    }
  }
  const dateOf = (id: string) => plan?.newActivities.find(a => a.activityId === id)?.date ?? plan?.existing.find(e => e.activityId === id)?.date ?? ''
  const titleOf = (id: string) => plan?.newActivities.find(a => a.activityId === id)?.title ?? plan?.existing.find(e => e.activityId === id)?.title ?? id
  const md = (iso: string) => `${Number(iso.slice(5, 7))}월 ${Number(iso.slice(8, 10))}일`
</script>

<Dialog.Root open onOpenChange={o => { if (!o && !committing) onClose() }}>
  <Dialog.Content class="max-h-[88vh] overflow-auto sm:max-w-2xl">
    <Dialog.Header>
      <Dialog.Title>새 게시물</Dialog.Title>
      <Dialog.Description>올린 파일은 앱 보관함에 복사돼요. 원본은 그대로 둬요.</Dialog.Description>
    </Dialog.Header>

    {#if result}
      <section class="grid gap-3" aria-live="polite">
        <h3 class="m-0 text-base font-bold">게시물 {result.created.length}개를 만들었어요</h3>
        {#if result.created.length}
          <ul class="m-0 grid list-none gap-1 p-0">
            {#each result.created as id (id)}
              <li><button class="w-full rounded-md px-2 py-1.5 text-left hover:bg-muted" onclick={() => onOpenActivity(id, dateOf(id))}>
                {md(dateOf(id))} {titleOf(id)} <span class="text-xs text-muted-foreground">게시 준비 열기</span></button></li>
            {/each}
          </ul>
        {/if}
        {#if result.updated.length}
          <h4 class="m-0 text-sm font-semibold text-muted-foreground">사진을 보탠 운동</h4>
          <ul class="m-0 grid list-none gap-1 p-0">
            {#each result.updated as id (id)}
              <li><button class="w-full rounded-md px-2 py-1.5 text-left hover:bg-muted" onclick={() => onOpenActivity(id, dateOf(id))}>{md(dateOf(id))} {titleOf(id)}</button></li>
            {/each}
          </ul>
        {/if}
        {#if result.failed.length}
          <ul class="m-0 pl-4 text-sm text-destructive">{#each result.failed as f}<li><b>{f.file}</b> {f.reason}</li>{/each}</ul>
        {/if}
        <Dialog.Footer><Button onclick={onClose}>닫기</Button></Dialog.Footer>
      </section>
    {:else}
      <div class="grid place-items-center gap-2 rounded-lg border-2 border-dashed p-5 text-center">
        <p class="m-0 text-sm text-muted-foreground">여기에 FIT와 사진을 끌어다 놓거나</p>
        <div class="flex flex-wrap justify-center gap-2">
          <Button size="sm" onclick={() => pick('files')} disabled={committing}>파일 고르기</Button>
          <Button size="sm" variant="outline" onclick={() => pick('folder')} disabled={committing}>폴더 고르기</Button>
          {#if garminConnected}
            <Button size="sm" variant="outline" onclick={openGarminPicker} disabled={committing || garminLoading}>Garmin에서 가져오기</Button>
          {:else if onOpenSettings}
            <Button size="sm" variant="ghost" onclick={onOpenSettings}>설정에서 Garmin을 연결하세요</Button>
          {/if}
        </div>
        {#if paths.length}<p class="m-0 text-xs text-muted-foreground">올린 항목 {paths.length}개{planning ? ' · 읽는 중…' : ''}</p>{/if}
      </div>

      {#if plan}
        <div class="grid gap-4">
          {#if plan.newActivities.length}
            <section class="grid gap-1.5">
              <h3 class="m-0 text-sm font-bold">만들 게시물 {plan.newActivities.length}개</h3>
              {#each plan.newActivities as a (a.activityId)}
                <div class="flex items-center gap-3 rounded-lg border bg-card px-3 py-2">
                  <span class="text-lg" aria-hidden="true">{ICON[a.sport]}</span>
                  <div class="min-w-0 flex-1">
                    <div class="truncate text-sm font-semibold">
                      {md(a.date)} {a.title}
                      <span class="font-normal text-muted-foreground">{km(a.km)}km</span>
                      {#if a.sources.some(s => garminAutoPaths.includes(s))}
                        <span class="ml-1 rounded bg-sunken px-1.5 py-0.5 text-[11px] font-normal text-muted-foreground">Garmin</span>
                      {/if}
                    </div>
                    <div class="mt-1 flex flex-wrap items-center gap-1">
                      {#each a.photos as p (p.path)}
                        <button class="rounded bg-sunken px-1.5 py-0.5 text-[11px] hover:line-through" title="이 사진 빼기"
                          onclick={() => removePhoto(p.path)} aria-label="{p.fileName} 빼기">{p.fileName}</button>
                      {:else}
                        <span class="text-xs text-muted-foreground">사진 없음</span>
                      {/each}
                      {#if a.libraryPhotos}<span class="text-xs text-muted-foreground">보관함에 있던 사진 {a.libraryPhotos}장이 이 운동에 붙어요</span>{/if}
                    </div>
                  </div>
                  <Button variant="ghost" size="sm" onclick={() => removeActivity(a.source)}>빼기</Button>
                </div>
              {/each}
            </section>
          {/if}

          {#if reuploaded.length || additions.length}
            <section class="grid gap-1.5">
              <h3 class="m-0 text-sm font-bold">이미 있는 운동 {reuploaded.length + additions.length}개</h3>
              {#each [...reuploaded, ...additions] as e (e.activityId)}
                <div class="flex items-center gap-3 rounded-lg border px-3 py-2">
                  <span class="text-lg" aria-hidden="true">{ICON[e.sport]}</span>
                  <div class="min-w-0 flex-1">
                    <div class="truncate text-sm">{md(e.date)} {e.title} <span class="text-muted-foreground">{km(e.km)}km</span></div>
                    {#if e.photos.length}
                      <div class="mt-1 flex flex-wrap items-center gap-1">
                        <span class="text-xs text-muted-foreground">보탤 사진</span>
                        {#each e.photos as p (p.path)}
                          <button class="rounded bg-sunken px-1.5 py-0.5 text-[11px] hover:line-through" title="이 사진 빼기"
                            onclick={() => removePhoto(p.path)} aria-label="{p.fileName} 빼기">{p.fileName}</button>
                        {/each}
                      </div>
                      {#if e.postLocked}<p class="m-0 mt-1 text-xs text-wait">글은 이미 고쳤거나 게시해서 그대로 둬요</p>{/if}
                    {/if}
                  </div>
                  <Button variant="ghost" size="sm" onclick={() => onOpenActivity(e.activityId, e.date)}>열기</Button>
                </div>
              {/each}
            </section>
          {/if}

          {#if plan.excluded.length || plan.duplicates.length}
            <section class="grid gap-1">
              <h3 class="m-0 text-sm font-bold">제외 {plan.excluded.length + plan.duplicates.length}개</h3>
              <ul class="m-0 grid list-none gap-0.5 p-0 text-xs">
                {#each plan.excluded as x}<li><b class="font-semibold">{x.file}</b> <span class="text-muted-foreground">{x.reason}</span></li>{/each}
                {#each plan.duplicates as d}<li><b class="font-semibold">{d}</b> <span class="text-muted-foreground">이미 있음</span></li>{/each}
              </ul>
            </section>
          {/if}
        </div>
      {/if}

      {#if error}<p class="m-0 text-sm text-destructive" role="alert">{error}</p>{/if}
      <Dialog.Footer>
        <Button variant="outline" onclick={onClose} disabled={committing}>취소</Button>
        <Button onclick={create} disabled={nothing || planning || committing}>
          {committing ? '만드는 중…' : plan ? createLabel(plan) : '게시물 0개 만들기'}
        </Button>
      </Dialog.Footer>
    {/if}
  </Dialog.Content>
</Dialog.Root>

{#if garminPickerOpen}
  <Dialog.Root open onOpenChange={o => { if (!o && !garminFetching) garminPickerOpen = false }}>
    <Dialog.Content class="max-h-[80vh] overflow-auto sm:max-w-lg">
      <Dialog.Header>
        <Dialog.Title>Garmin에서 가져오기</Dialog.Title>
        <Dialog.Description>최근 7일 활동 중 가져올 운동을 고르세요.</Dialog.Description>
      </Dialog.Header>
      {#if garminLoading}
        <p class="m-0 text-sm text-muted-foreground" role="status">활동 목록을 불러오는 중…</p>
      {:else if !garminList.length}
        <p class="m-0 text-sm text-muted-foreground">최근 7일간 기록된 활동이 없어요.</p>
      {:else}
        <ul class="m-0 grid list-none gap-1.5 p-0">
          {#each garminList as a (a.id)}
            <li class="flex items-center gap-2 rounded-md border px-3 py-2" class:opacity-50={a.alreadyImported}>
              <input type="checkbox" checked={garminSelected.includes(a.id)} onchange={() => toggleGarmin(a.id)} disabled={garminFetching} aria-label="{a.name} 선택" />
              <span aria-hidden="true">{ICON[a.sport]}</span>
              <div class="min-w-0 flex-1">
                <div class="truncate text-sm font-medium">{a.name || a.date}</div>
                <div class="text-xs text-muted-foreground">{a.date} {a.startTime} · {km(a.km)}km</div>
              </div>
              {#if a.alreadyImported}<span class="text-xs text-muted-foreground">이미 있음</span>{/if}
            </li>
          {/each}
        </ul>
      {/if}
      {#if garminFetching}<p class="m-0 text-sm text-muted-foreground" role="status">FIT 파일을 받는 중…</p>{/if}
      {#if garminFailed.length}
        <ul class="m-0 pl-4 text-sm text-destructive">{#each garminFailed as f}<li><b>{f.file}</b> {f.reason}</li>{/each}</ul>
      {/if}
      <Dialog.Footer>
        <Button variant="outline" onclick={() => (garminPickerOpen = false)} disabled={garminFetching}>닫기</Button>
        <Button onclick={fetchSelectedGarmin} disabled={garminLoading || garminFetching || !garminSelected.length}>
          {garminFetching ? '받는 중…' : `선택한 ${garminSelected.length}개 가져오기`}
        </Button>
      </Dialog.Footer>
    </Dialog.Content>
  </Dialog.Root>
{/if}
