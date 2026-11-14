<script lang="ts">
  // 게시 준비 탭: 장소·날씨, 글 편집, AI로 다듬기, 카드, 올라갈 모습 미리보기, 복사·게시 완료. 게시는 사용자가 직접 한다.
  import { Button } from '$lib/components/ui/button'
  import { Checkbox } from '$lib/components/ui/checkbox'
  import { Input } from '$lib/components/ui/input'
  import { Label } from '$lib/components/ui/label'
  import { Textarea } from '$lib/components/ui/textarea'
  import { api } from '$lib/api'
  import type { PostStatus } from '$lib/bindings/PostStatus'
  import type { PostView } from '$lib/bindings/PostView'
  import type { RefineDto } from '$lib/bindings/RefineDto'
  import type { RefineStatusDto } from '$lib/bindings/RefineStatusDto'
  import { ask } from '$lib/confirm.svelte'
  import { mergeEditor, parseRoutePoints, parseWeather, routeText, weatherText } from '$lib/enrich'
  import { parseHashtags } from '$lib/format'
  import RefinePreview from '../RefinePreview.svelte'
  import FeedPreview from './FeedPreview.svelte'

  let { id, onDirtyChange, onStatusChange }: {
    id: string; onDirtyChange: (dirty: boolean) => void; onStatusChange: (s: PostStatus) => void
  } = $props()

  let post = $state<PostView | null>(null)
  let body = $state('')
  let tags = $state('')
  let message = $state('')
  let error = $state('')
  let busy = $state(false)
  let placeInput = $state('')
  let routeInput = $state('')
  let weatherInput = $state('')
  let includePhotos = $state(false)
  let refined = $state<RefineDto | null>(null)
  let ai = $state<RefineStatusDto | null>(null)
  let cacheBust = $state(0)
  $effect(() => { api.refineStatus().then(s => (ai = s)).catch(() => undefined) })

  const hashtags = $derived(parseHashtags(tags))
  const dirty = $derived(!!post && (body !== post.body || hashtags.join(' ') !== post.hashtags.join(' ')))
  const fullText = $derived(hashtags.length ? `${body}\n\n${hashtags.join(' ')}` : body)
  $effect(() => onDirtyChange(dirty))

  /** force: 사용자가 일부러 글을 바꾼 경우(다시 쓰기·적용)라 화면 글도 서버 값으로 바꾼다. */
  function load(p: PostView, force = false) {
    const merged = mergeEditor(p, { body, tags }, dirty && !force)
    const statusChanged = post !== null && post.status !== p.status
    post = p
    body = merged.body
    tags = merged.tags
    placeInput = p.place?.name ?? ''
    routeInput = routeText(p.routePoints)
    weatherInput = weatherText(p.weather)
    if (p.enrichError) error = `일부를 채우지 못했어요: ${p.enrichError}`
    if (statusChanged) onStatusChange(p.status)
  }
  $effect(() => { api.postView(id).then(p => load(p)).catch(e => (error = String(e))) })

  async function act(label: string, f: () => Promise<PostView | void>, force = false) {
    busy = true; error = ''; message = ''
    try {
      const r = await f()
      if (r) load(r, force)
      message = label
    } catch (e) {
      error = String(e)
    } finally {
      busy = false
    }
  }
  const save = () => act('저장했어요', () => api.savePost(id, body, hashtags), true)
  const copy = () => act('글을 복사했어요. 스레드에 붙여 넣으면 돼요', () => api.copyText(fullText))
  async function regenerate() {
    if (await ask({ title: '템플릿으로 다시 쓸까요?', description: '직접 고친 내용은 지워져요.', action: '다시 쓰기', destructive: true }))
      act('템플릿으로 다시 썼어요', () => api.regeneratePost(id), true)
  }
  const publish = () => act('게시 완료로 표시했어요', async () => {
    if (dirty) await api.savePost(id, body, hashtags)
    return api.markPublished(id)
  })
  // 장소·경로·날씨: 칸을 벗어나면 저장, 비우면 자동 값으로 되돌림.
  const savePlace = () => act('장소를 저장했어요', () => api.setPlace(id, placeInput.trim() || null))
  const saveRoute = () => act('경로를 저장했어요', () => api.setRoutePoints(id, parseRoutePoints(routeInput)))
  function saveWeather() {
    const w = parseWeather(weatherInput)
    if (weatherInput.trim() && !w) { error = '날씨는 "맑음 17도"처럼 적어 주세요'; return }
    act('날씨를 저장했어요', () => api.setWeather(id, w))
  }
  const enrich = () => act('장소·날씨를 다시 가져왔어요', () => api.enrichActivity(id))
  const refine = () => act('', async () => { refined = await api.refinePost(id, body, hashtags, includePhotos) })
  const applyRefined = () => {
    const r = refined!
    refined = null
    act('다듬은 글을 적용했어요', () => api.savePost(id, r.body, r.hashtags), true)
  }
  const regenerateCards = () => act('카드를 다시 만들었어요', async () => { await api.regenerateCards(id); cacheBust++ })
</script>

{#if post}
  <div class="grid items-start gap-4 [grid-template-columns:minmax(0,1fr)_300px] max-lg:grid-cols-1">
    <section class="grid gap-3">
      <div class="flex flex-wrap items-center gap-1.5">
        {@render fact('place', '장소', '예: 한강', post.place?.estimated ? '추정' : '', post.placeUserSet, savePlace, () => { placeInput = ''; savePlace() })}
        {@render fact('route', '지나간 곳', '예: 반포 → 잠실', '', post.routeUserSet, saveRoute, () => { routeInput = ''; saveRoute() })}
        {@render fact('weather', '날씨', '예: 맑음 17도', '', post.weatherUserSet, saveWeather, () => { weatherInput = ''; saveWeather() })}
        <Button variant="ghost" size="sm" onclick={enrich} disabled={busy} title="출발·도착 500m 밖의 지점만 조회해요">장소·날씨 다시 가져오기</Button>
      </div>

      <div class="grid gap-1">
        <div class="flex justify-between text-xs text-muted-foreground">
          <Label for="body" class="text-xs font-normal text-muted-foreground">본문{#if post.edited} (직접 고침){/if}</Label>
          <span>{fullText.length}자 / 500</span>
        </div>
        <Textarea id="body" rows={8} bind:value={body} class="bg-card text-[15px] leading-relaxed" />
      </div>
      <div class="grid gap-1">
        <Label for="tags" class="text-xs font-normal text-muted-foreground">해시태그</Label>
        <Input id="tags" bind:value={tags} placeholder="#라이딩 #한강" class="bg-card text-ride" />
      </div>

      <div class="flex flex-wrap items-center gap-1.5">
        <Button variant="outline" size="sm" onclick={refine} disabled={busy}
          title={ai ? `${ai.label}로 다듬어요 (설정에서 바꿀 수 있어요)` : ''}>{ai?.label ?? 'AI'}로 다듬기</Button>
        <Label class="text-sm font-normal"><Checkbox bind:checked={includePhotos} /> 사진도 보내기</Label>
        <Button variant="ghost" size="sm" onclick={regenerate} disabled={busy}>템플릿으로 다시 쓰기</Button>
        <Button variant="ghost" size="sm" onclick={save} disabled={busy || !dirty}>저장</Button>
      </div>
      {#if includePhotos}<p class="m-0 text-xs text-muted-foreground">사진은 위치 정보를 지운 작은 사본으로 {ai?.company ?? 'AI'} 서버에 보내져요.</p>{/if}

      <div class="grid gap-1.5">
        <div class="flex flex-wrap items-center justify-between gap-2 text-xs text-muted-foreground">
          <span>카드 {post.cardPaths.length}장</span>
          <span class="flex gap-1">
            {#if post.cardPaths.length}<Button variant="ghost" size="xs" onclick={regenerateCards} disabled={busy}>카드 다시 만들기</Button>{/if}
            <Button variant="ghost" size="xs" onclick={() => api.openFolder(post!.folder)}>카드 폴더 열기</Button>
          </span>
        </div>
        {#if post.cardPaths.length === 0}
          <p class="m-0 text-sm text-muted-foreground">이 운동에 붙은 사진이 없어요. 사진을 가져오면 카드가 만들어져요.</p>
        {:else}
          <div class="flex flex-wrap gap-1.5">
            {#each post.cardPaths as c}<img src="{api.fileUrl(c)}{cacheBust ? `?v=${cacheBust}` : ''}" alt="카드" loading="lazy" class="aspect-[9/16] w-16 rounded-md bg-sunken object-cover" />{/each}
          </div>
        {/if}
      </div>
    </section>

    <aside class="grid gap-2.5 lg:sticky lg:top-0">
      <span class="text-xs text-muted-foreground">올라갈 모습</span>
      <FeedPreview {body} {hashtags} cards={post.cardPaths} {cacheBust} />
      <Button onclick={copy} disabled={busy}>글 복사</Button>
      <Button variant="outline" class="border-done text-done hover:text-done" onclick={publish} disabled={busy || post.status === 'published'}>
        {post.status === 'published' ? '게시함' : '게시 완료로 표시'}
      </Button>
      <p class="m-0 text-center text-[11px] text-muted-foreground">스레드에 직접 올린 뒤 완료로 표시하세요.</p>
      {#if busy}<p class="m-0 text-sm text-muted-foreground" role="status">처리 중…</p>{/if}
      {#if message}<p class="m-0 text-sm text-done" role="status">{message}</p>{/if}
      {#if error}<p class="m-0 text-sm text-destructive" role="alert">{error}</p>{/if}
    </aside>
  </div>
{:else if error}
  <p class="text-destructive" role="alert">{error}</p>
{:else}
  <p class="text-muted-foreground">불러오는 중…</p>
{/if}

{#snippet fact(key: 'place' | 'route' | 'weather', label: string, placeholder: string, tag: string, userSet: boolean, onSave: () => void, onAuto: () => void)}
  <span class="flex items-center gap-1.5 rounded-md border bg-card px-2 py-1 focus-within:ring-2 focus-within:ring-ring">
    <label for={key} class="text-[11px] text-muted-foreground">{label}</label>
    {#if key === 'place'}<input id={key} bind:value={placeInput} onchange={onSave} {placeholder} class="w-32 bg-transparent text-[13px] outline-none" />
    {:else if key === 'route'}<input id={key} bind:value={routeInput} onchange={onSave} {placeholder} class="w-32 bg-transparent text-[13px] outline-none" />
    {:else}<input id={key} bind:value={weatherInput} onchange={onSave} {placeholder} class="w-24 bg-transparent text-[13px] outline-none" />{/if}
    {#if tag}<span class="text-[11px] font-bold text-wait">{tag}</span>{/if}
    {#if userSet}<button class="text-[11px] text-ride" onclick={onAuto}>자동으로</button>{/if}
  </span>
{/snippet}

{#if refined && post}
  <RefinePreview current={fullText} {refined} label={ai?.label ?? 'AI'} onApply={applyRefined} onCancel={() => (refined = null)} />
{/if}
