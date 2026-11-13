<script lang="ts">
  // 설정: Claude API 키(키 값은 다시 보여 주지 않음), 모델, 카드 경로선 가리기.
  import { api } from '../lib/api'
  import { backupStatusText } from '../lib/backup'
  import type { BackupStatusDto } from '../lib/bindings/BackupStatusDto'
  let { onClose }: { onClose: () => void } = $props()
  let hasKey = $state(false)
  let model = $state('')
  let keyInput = $state('')
  let trim = $state(false)
  let message = $state('')
  let error = $state('')
  let clientId = $state('')
  let clientSecret = $state('')
  let hasSecret = $state(false)
  let backup = $state<BackupStatusDto | null>(null)
  let working = $state('')

  $effect(() => {
    api.claudeStatus().then(s => { hasKey = s.hasKey; model = s.model }).catch(e => (error = String(e)))
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
  function restore() {
    if (confirm('드라이브의 백업으로 이 라이브러리를 채울까요? 비어 있는 라이브러리에서만 할 수 있어요.')) {
      run('복원 중…', () => api.restoreFromDrive())
    }
  }

  async function saveKey() {
    if (!keyInput.trim()) return
    try { await api.setClaudeKey(keyInput.trim()); keyInput = ''; hasKey = true; message = 'API 키를 저장했어요' } catch (e) { error = String(e) }
  }
  async function clearKey() {
    try { await api.setClaudeKey(null); hasKey = false; message = 'API 키를 지웠어요' } catch (e) { error = String(e) }
  }
  async function saveModel() {
    try { await api.setClaudeModel(model.trim() || null); message = '모델을 저장했어요' } catch (e) { error = String(e) }
  }
  async function saveTrim() {
    try { await api.setCardRouteTrim(trim); message = trim ? '새 카드부터 출발·도착 부근을 가려요' : '카드에 경로 전체를 그려요' } catch (e) { error = String(e) }
  }
</script>

<div class="backdrop" role="presentation" onclick={onClose}></div>
<div class="panel" role="dialog" aria-modal="true" aria-labelledby="settings-title">
  <h2 id="settings-title">설정</h2>

  <section>
    <h3>Claude로 다듬기</h3>
    <p class="muted">API 키는 이 컴퓨터의 키 보관함에만 저장되고, 저장한 뒤에는 다시 보여 주지 않아요.</p>
    <div class="row">
      <input type="password" placeholder={hasKey ? '저장된 키 있음' : 'sk-ant-…'} bind:value={keyInput} aria-label="Claude API 키" autocomplete="off" />
      <button class="btn primary" onclick={saveKey} disabled={!keyInput.trim()}>키 저장</button>
      {#if hasKey}<button class="btn" onclick={clearKey}>키 지우기</button>{/if}
    </div>
    <label class="field">모델
      <input bind:value={model} onchange={saveModel} placeholder="claude-opus-5-5" />
    </label>
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
      <button class="btn" onclick={saveClient} disabled={!!working}>클라이언트 저장</button>
      {#if backup?.configured && !backup.signedIn}<button class="btn primary" onclick={login} disabled={!!working}>구글로 로그인</button>{/if}
      {#if backup?.signedIn}
        <button class="btn primary" onclick={backupNow} disabled={!!working}>지금 백업</button>
        <button class="btn" onclick={logout} disabled={!!working}>로그아웃</button>
      {/if}
      {#if backup?.signedIn && backup.canRestore}<button class="btn" onclick={restore} disabled={!!working}>드라이브에서 복원</button>{/if}
    </div>
    {#if backup}
      <p class="status" class:bad={!!backup.lastError}>{backupStatusText(backup, new Date())}{#if backup.signedIn && backup.pending > 0} · 올릴 파일 {backup.pending}개{/if}</p>
    {/if}
    {#if working}<p class="muted" role="status">{working}</p>{/if}
  </section>

  {#if message}<p class="ok" role="status">{message}</p>{/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  <div class="foot"><button class="btn" onclick={onClose}>닫기</button></div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; background: rgba(27, 36, 48, .35); }
  .panel { position: fixed; top: 10vh; left: 50%; transform: translateX(-50%); width: min(520px, calc(100vw - 32px)); max-height: 80vh; overflow: auto;
    background: var(--surface); border-radius: 18px; padding: 24px; box-shadow: 0 20px 60px rgba(27, 36, 48, .25); }
  h2 { margin: 0 0 8px; font-size: 20px; }
  h3 { margin: 18px 0 4px; font-size: 15px; }
  .row { display: flex; gap: 8px; flex-wrap: wrap; }
  input:not([type=checkbox]) { flex: 1; min-width: 160px; border: 1px solid var(--line); border-radius: 8px; padding: 6px 10px; background: var(--surface); }
  .field { display: flex; align-items: center; gap: 8px; margin-top: 10px; color: var(--muted); font-size: 14px; }
  .check { display: flex; align-items: center; gap: 8px; }
  .muted { color: var(--muted); font-size: 13px; margin: 2px 0 8px; }
  .ok { color: #13704c; } .error { color: #b42318; }
  .status { margin: 8px 0 0; font-size: 14px; }
  .status.bad { color: #b42318; }
  .row { margin-top: 8px; }
  .foot { display: flex; justify-content: flex-end; margin-top: 18px; }
</style>
