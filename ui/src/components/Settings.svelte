<script lang="ts">
  // 설정: 글 다듬기 AI(Gemini/Claude)와 키(키 값은 다시 보여 주지 않음)·모델, 카드 경로선, 드라이브 백업.
  import { Button } from '$lib/components/ui/button'
  import * as Dialog from '$lib/components/ui/dialog'
  import { api } from '$lib/api'
  import { backupStatusText } from '$lib/backup'
  import type { BackupStatusDto } from '$lib/bindings/BackupStatusDto'
  import type { RefineStatusDto } from '$lib/bindings/RefineStatusDto'
  import { ask } from '$lib/confirm.svelte'
  let { onClose }: { onClose: () => void } = $props()
  let refine = $state<RefineStatusDto | null>(null)
  let model = $state('')
  let keyInput = $state('')
  const KEY_HINT: Record<string, string> = { gemini: 'AIza…', claude: 'sk-ant-…' }
  let trim = $state(false)
  let message = $state('')
  let error = $state('')
  let clientId = $state('')
  let clientSecret = $state('')
  let hasSecret = $state(false)
  let backup = $state<BackupStatusDto | null>(null)
  let working = $state('')

  $effect(() => {
    loadRefine()
    api.settings().then(s => (trim = s.cardRouteTrim)).catch(e => (error = String(e)))
    api.googleClient().then(c => { clientId = c?.clientId ?? ''; hasSecret = !!c?.hasSecret }).catch(() => undefined)
    api.backupStatus().then(s => (backup = s)).catch(() => undefined)
  })

  async function run(label: string, f: () => Promise<BackupStatusDto | void>) {
    working = label; error = ''; message = ''
    try {
      const r = await f()
      backup = r ?? (await api.backupStatus())
    } catch (e) {
      error = String(e)
    } finally {
      working = ''
    }
  }
  const saveClient = () => run('저장 중…', async () => {
    // secret 칸이 비어 있으면 저장된 secret을 그대로 둔다.
    await api.setGoogleClient(clientId.trim() || null, clientSecret.trim() || null)
    if (clientSecret.trim()) hasSecret = true
    clientSecret = ''
    message = '구글 클라이언트를 저장했어요'
  })
  const login = () => run('브라우저에서 구글 로그인을 마쳐 주세요…', () => api.googleLogin())
  const logout = () => run('로그아웃 중…', () => api.googleLogout())
  const backupNow = () => run('백업 중…', () => api.backupNow())
  async function restore() {
    if (await ask({ title: '드라이브의 백업으로 이 라이브러리를 채울까요?', description: '비어 있는 라이브러리에서만 할 수 있어요.', action: '복원' })) {
      run('복원 중…', () => api.restoreFromDrive())
    }
  }

  async function loadRefine() {
    try { refine = await api.refineStatus(); model = refine.model; keyInput = '' } catch (e) { error = String(e) }
  }
  async function chooseProvider(p: string) {
    try { await api.setRefineProvider(p); await loadRefine(); message = `${refine?.label}로 다듬어요` } catch (e) { error = String(e) }
  }
  async function saveKey() {
    if (!keyInput.trim() || !refine) return
    try { await api.setRefineKey(refine.provider, keyInput.trim()); await loadRefine(); message = `${refine?.label} API 키를 저장했어요` } catch (e) { error = String(e) }
  }
  async function clearKey() {
    if (!refine) return
    try { await api.setRefineKey(refine.provider, null); await loadRefine(); message = 'API 키를 지웠어요' } catch (e) { error = String(e) }
  }
  async function saveModel() {
    if (!refine) return
    try { await api.setRefineModel(refine.provider, model.trim() || null); await loadRefine(); message = '모델을 저장했어요' } catch (e) { error = String(e) }
  }
  async function saveTrim() {
    try { await api.setCardRouteTrim(trim); message = trim ? '새 카드부터 출발·도착 부근을 가려요' : '카드에 경로 전체를 그려요' } catch (e) { error = String(e) }
  }
</script>

<Dialog.Root open onOpenChange={o => { if (!o) onClose() }}>
<Dialog.Content class="max-h-[84vh] overflow-auto sm:max-w-lg">
  <Dialog.Header><Dialog.Title>설정</Dialog.Title></Dialog.Header>

  <section>
    <h3>글 다듬기 AI</h3>
    {#if refine}
      <div class="choices" role="radiogroup" aria-label="글 다듬기 AI">
        {#each refine.providers as p}
          <label class="choice" class:on={p.id === refine.provider}>
            <input type="radio" name="refine-provider" checked={p.id === refine.provider} onchange={() => chooseProvider(p.id)} />
            {p.label}<span class="sub">{p.hasKey ? '키 있음' : '키 없음'}</span>
          </label>
        {/each}
      </div>
      <p class="muted">{refine.label} API 키는 이 컴퓨터의 키 보관함에만 저장되고, 저장한 뒤에는 다시 보여 주지 않아요. "사진도 보내기"를 켜면 위치 정보를 지운 작은 사진이 {refine.company} 서버로 가요.</p>
      <div class="row">
        <input type="password" placeholder={refine.hasKey ? '저장된 키 있음' : KEY_HINT[refine.provider]} bind:value={keyInput} aria-label="{refine.label} API 키" autocomplete="off" />
        <Button size="sm" onclick={saveKey} disabled={!keyInput.trim()}>키 저장</Button>
        {#if refine.hasKey}<Button size="sm" variant="outline" onclick={clearKey}>키 지우기</Button>{/if}
      </div>
      <label class="field">모델
        <input bind:value={model} onchange={saveModel} />
      </label>
    {/if}
  </section>

  <section>
    <h3>카드 경로선</h3>
    <label class="check">
      <input type="checkbox" bind:checked={trim} onchange={saveTrim} />
      출발·도착 500m를 카드에서 가리기
    </label>
    <p class="muted">집 근처에서 출발했다면 켜 두세요. 이미 만든 카드는 게시 준비 화면의 '카드 다시 만들기'로 바꿀 수 있어요.</p>
  </section>

  <section>
    <h3>구글 드라이브 백업</h3>
    <p class="muted">구글 Cloud에서 만든 "데스크톱 앱" OAuth 클라이언트의 ID와 secret을 넣어 주세요. 만드는 방법은 docs/setup-google-oauth.md에 있어요. 앱이 만든 파일만 드라이브의 WorkoutApp 폴더에 올리고, 어느 쪽 파일도 지우지 않아요.</p>
    <label class="field">클라이언트 ID <input bind:value={clientId} placeholder="….apps.googleusercontent.com" /></label>
    <label class="field">secret <input type="password" bind:value={clientSecret} placeholder={hasSecret ? '저장된 secret 있음' : ''} autocomplete="off" /></label>
    <div class="row">
      <Button size="sm" variant="outline" onclick={saveClient} disabled={!!working}>클라이언트 저장</Button>
      {#if backup?.configured && !backup.signedIn}<Button size="sm" onclick={login} disabled={!!working}>구글로 로그인</Button>{/if}
      {#if backup?.signedIn}
        <Button size="sm" onclick={backupNow} disabled={!!working}>지금 백업</Button>
        <Button size="sm" variant="outline" onclick={logout} disabled={!!working}>로그아웃</Button>
      {/if}
      {#if backup?.signedIn && backup.canRestore}<Button size="sm" variant="outline" onclick={restore} disabled={!!working}>드라이브에서 복원</Button>{/if}
    </div>
    {#if backup}
      <p class="status" class:bad={!!backup.lastError}>{backupStatusText(backup, new Date())}{#if backup.signedIn && backup.pending > 0} · 올릴 파일 {backup.pending}개{/if}</p>
    {/if}
    {#if working}<p class="muted" role="status">{working}</p>{/if}
  </section>

  {#if message}<p class="ok" role="status">{message}</p>{/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  <Dialog.Footer><Button variant="outline" onclick={onClose}>닫기</Button></Dialog.Footer>
</Dialog.Content>
</Dialog.Root>

<style>
  h3 { margin: 14px 0 4px; font-size: 15px; font-weight: 700; }
  .row { display: flex; gap: 8px; flex-wrap: wrap; margin-top: 8px; }
  input:not([type=checkbox]):not([type=radio]) { flex: 1; min-width: 160px; border: 1px solid var(--border); border-radius: 8px; padding: 6px 10px; background: var(--card); }
  .field { display: flex; align-items: center; gap: 8px; margin-top: 10px; color: var(--muted-foreground); font-size: 14px; }
  .check { display: flex; align-items: center; gap: 8px; }
  .muted { color: var(--muted-foreground); font-size: 13px; margin: 2px 0 8px; }
  .ok { color: var(--done); } .error { color: var(--destructive); }
  .status { margin: 8px 0 0; font-size: 14px; }
  .status.bad { color: var(--destructive); }
  .choices { display: flex; gap: 8px; margin: 6px 0 4px; }
  .choice { display: flex; align-items: center; gap: 6px; border: 1px solid var(--border); border-radius: 10px; padding: 6px 12px; cursor: pointer; font-weight: 600; }
  .choice.on { border-color: var(--ride); background: var(--selected); }
  .choice .sub { font-weight: 500; font-size: 12px; color: var(--muted-foreground); }
</style>
