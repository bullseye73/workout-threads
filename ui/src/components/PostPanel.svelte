<script lang="ts">
  // 게시 준비: 글 편집, 복사, 카드 확인, 게시 완료 표시. 게시는 사용자가 직접 한다.
  import { api } from '../lib/api'
  import type { PostView } from '../lib/bindings/PostView'
  import type { RefineDto } from '../lib/bindings/RefineDto'
  import type { RefineStatusDto } from '../lib/bindings/RefineStatusDto'
  import { mergeEditor, parseRoutePoints, parseWeather, routeText, weatherText } from '../lib/enrich'
  import { parseHashtags } from '../lib/format'
  import RefinePreview from './RefinePreview.svelte'

  let { id, onBack }: { id: string; onBack: () => void } = $props()
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
  $effect(() => { api.refineStatus().then(s => (ai = s)).catch(() => undefined) })
  let cacheBust = $state(0)

  const dirty = $derived(!!post && (body !== post.body || parseHashtags(tags).join(' ') !== post.hashtags.join(' ')))
  const fullText = $derived(parseHashtags(tags).length ? `${body}\n\n${parseHashtags(tags).join(' ')}` : body)

  /** force: 사용자가 일부러 글을 바꾼 경우(다시 쓰기·적용)라 화면 글도 서버 값으로 바꾼다. */
  function load(p: PostView, force = false) {
    const merged = mergeEditor(p, { body, tags }, dirty && !force)
    post = p
    body = merged.body
    tags = merged.tags
    placeInput = p.place?.name ?? ''
    routeInput = routeText(p.routePoints)
    weatherInput = weatherText(p.weather)
    if (p.enrichError) error = `일부를 채우지 못했어요: ${p.enrichError}`
  }
  $effect(() => { api.postView(id).then(load).catch(e => (error = String(e))) })

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
  const save = () => act('저장했어요', () => api.savePost(id, body, parseHashtags(tags)), true)
  const copy = () => act('글을 복사했어요. 스레드에 붙여 넣으면 돼요', () => api.copyText(fullText))
  function regenerate() {
    if (confirm('직접 고친 내용을 지우고 템플릿으로 다시 쓸까요?')) act('템플릿으로 다시 썼어요', () => api.regeneratePost(id), true)
  }
  const publish = () => act('게시 완료로 표시했어요', async () => {
    if (dirty) await api.savePost(id, body, parseHashtags(tags))
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
  const refine = () => act('', async () => { refined = await api.refinePost(id, body, parseHashtags(tags), includePhotos) })
  const applyRefined = () => {
    const r = refined!
    refined = null
    act('다듬은 글을 적용했어요', () => api.savePost(id, r.body, r.hashtags), true)
  }
  const regenerateCards = () => act('카드를 다시 만들었어요', async () => { await api.regenerateCards(id); cacheBust++ })

  function back() {
    if (!dirty || confirm('저장하지 않은 수정 내용이 있어요. 나갈까요?')) onBack()
  }
  const STATUS = { none: '게시 준비 안 됨', ready: '게시 준비됨', published: '게시함' } as const
</script>

<nav class="bar"><button class="btn" onclick={back}>← 운동 상세</button></nav>

{#if post}
  <div class="layout">
    <section class="editor">
      <header>
        <h1>게시 준비</h1>
        <span class="status {post.status}">{STATUS[post.status]}</span>
        {#if post.edited}<span class="tag">직접 고침</span>{/if}
      </header>
      <div class="facts">
        <label for="place">장소{#if post.place?.estimated}<span class="tag">추정</span>{/if}</label>
        <div class="fact">
          <input id="place" bind:value={placeInput} onchange={savePlace} placeholder="예: 한강" />
          {#if post.placeUserSet}<button class="link" onclick={() => { placeInput = ''; savePlace() }}>자동으로</button>{/if}
        </div>
        <label for="route">지나간 곳</label>
        <div class="fact">
          <input id="route" bind:value={routeInput} onchange={saveRoute} placeholder="예: 반포 → 잠실" />
          {#if post.routeUserSet}<button class="link" onclick={() => { routeInput = ''; saveRoute() }}>자동으로</button>{/if}
        </div>
        <label for="weather">날씨</label>
        <div class="fact">
          <input id="weather" bind:value={weatherInput} onchange={saveWeather} placeholder="예: 맑음 17도" />
          {#if post.weatherUserSet}<button class="link" onclick={() => { weatherInput = ''; saveWeather() }}>자동으로</button>{/if}
        </div>
        <div class="fact-actions">
          <button class="btn" onclick={enrich} disabled={busy}>장소·날씨 다시 가져오기</button>
          <span class="hint">출발·도착 500m 밖의 지점만 조회해요</span>
        </div>
      </div>
      <label for="body">본문</label>
      <textarea id="body" rows="9" bind:value={body}></textarea>
      <label for="tags">해시태그</label>
      <input id="tags" bind:value={tags} placeholder="#라이딩 #한강" />
      <div class="actions">
        <button class="btn primary" onclick={copy} disabled={busy}>글 복사</button>
        <button class="btn" onclick={save} disabled={busy || !dirty}>저장</button>
        <button class="btn" onclick={regenerate} disabled={busy}>템플릿으로 다시 쓰기</button>
        <button class="btn" onclick={refine} disabled={busy} title={ai ? `${ai.label}로 다듬어요 (설정에서 바꿀 수 있어요)` : ''}>{ai?.label ?? 'AI'}로 다듬기</button>
        <label class="check"><input type="checkbox" bind:checked={includePhotos} /> 사진도 보내기</label>
        <span class="spacer"></span>
        <button class="btn done" onclick={publish} disabled={busy || post.status === 'published'}>
          {post.status === 'published' ? '게시함' : '게시 완료'}
        </button>
      </div>
      {#if includePhotos}<p class="hint">사진은 위치 정보를 지운 작은 사본으로 {ai?.company ?? 'AI'} 서버에 보내져요.</p>{/if}
      {#if busy}<p class="hint" role="status">처리 중…</p>{/if}
      {#if message}<p class="msg" role="status">{message}</p>{/if}
      {#if error}<p class="error" role="alert">{error}</p>{/if}
    </section>
    <aside>
      <div class="cards-head">
        <h2>카드 {post.cardPaths.length}장</h2>
        <div class="card-actions">
          {#if post.cardPaths.length}<button class="btn" onclick={regenerateCards} disabled={busy}>카드 다시 만들기</button>{/if}
          <button class="btn" onclick={() => api.openFolder(post!.folder)}>카드 폴더 열기</button>
        </div>
      </div>
      {#if post.cardPaths.length === 0}
        <p class="muted">이 운동에 붙은 사진이 없어요. 사진을 가져오면 카드가 만들어져요.</p>
      {/if}
      <div class="cards">
        {#each post.cardPaths as c}<img src="{api.fileUrl(c)}{cacheBust ? `?v=${cacheBust}` : ''}" alt="카드 미리보기" loading="lazy" />{/each}
      </div>
    </aside>
  </div>
{:else if error}
  <p class="error">{error}</p>
{/if}

{#if refined && post}
  <RefinePreview current={fullText} {refined} label={ai?.label ?? 'AI'} onApply={applyRefined} onCancel={() => (refined = null)} />
{/if}

<style>
  .bar { margin-bottom: 18px; }
  .layout { display: grid; grid-template-columns: minmax(0, 3fr) minmax(0, 2fr); gap: 32px; }
  header { display: flex; align-items: center; gap: 10px; margin-bottom: 12px; }
  h1 { font-size: 26px; margin: 0; letter-spacing: -0.02em; }
  h2 { font-size: 16px; margin: 0; }
  .status { font-size: 13px; font-weight: 600; padding: 2px 10px; border-radius: 999px; background: var(--track); }
  .status.ready { background: color-mix(in srgb, var(--ride) 14%, white); color: #1d4fb8; }
  .status.published { background: color-mix(in srgb, var(--done) 16%, white); color: #13704c; }
  .tag { font-size: 12px; padding: 1px 8px; border-radius: 999px; background: var(--track); color: var(--muted-foreground); margin-left: 6px; }
  .facts { display: grid; grid-template-columns: auto 1fr; gap: 8px 12px; align-items: center; padding: 14px 16px; border-radius: 12px; background: var(--surface); margin-bottom: 6px; }
  .facts label { margin: 0; display: flex; align-items: center; }
  .fact { display: flex; gap: 8px; align-items: center; }
  .fact input { padding: 6px 10px; }
  .fact-actions { grid-column: 1 / -1; display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
  .link { border: 0; background: none; color: var(--ride); font-size: 13px; padding: 0; white-space: nowrap; }
  .hint { color: var(--muted-foreground); font-size: 13px; margin: 6px 0 0; }
  .check { display: flex; align-items: center; gap: 4px; font-size: 14px; margin: 0; color: var(--ink); }
  .check input { width: auto; }
  .card-actions { display: flex; gap: 6px; }
  label { display: block; font-size: 13px; color: var(--muted-foreground); margin: 10px 0 4px; }
  textarea, input { width: 100%; border: 1px solid var(--line); border-radius: 10px; padding: 12px 14px; background: var(--surface); line-height: 1.6; }
  textarea { resize: vertical; font-size: 16px; }
  .actions { display: flex; gap: 8px; flex-wrap: wrap; margin-top: 16px; align-items: center; }
  .spacer { flex: 1; }
  .btn.done { border-color: var(--done); color: #13704c; }
  .msg { color: #13704c; margin: 10px 0 0; }
  .error { color: #b42318; }
  .muted { color: var(--muted-foreground); }
  .cards-head { display: flex; justify-content: space-between; align-items: center; margin-bottom: 10px; }
  .cards { display: grid; grid-template-columns: repeat(auto-fill, minmax(120px, 1fr)); gap: 10px; }
  .cards img { width: 100%; border-radius: 10px; display: block; background: var(--track); }
  @media (max-width: 860px) { .layout { grid-template-columns: 1fr; } }
</style>
