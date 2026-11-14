<script lang="ts">
  // 가져오기 결과: 요약, 실패 목록, 운동에 맞지 않은 사진을 직접 붙이기.
  import { api } from '../lib/api'
  import type { ImportReport } from '../lib/bindings/ImportReport'
  import type { MonthView } from '../lib/bindings/MonthView'
  import { importSummary, km } from '../lib/format'

  let { report, onClose, title = '가져오기 결과' }: { report: ImportReport; onClose: () => void; title?: string } = $props()
  let choices = $state<{ id: string; label: string }[]>([])
  let assigned = $state<Record<number, string>>({})
  let error = $state('')

  // 사진 촬영 달(없으면 이번 달)의 운동을 고를 수 있게 모은다.
  $effect(() => {
    const months = new Set(report.unmatchedPhotos.map(p => (p.takenAt ?? new Date().toISOString()).slice(0, 7)))
    Promise.all([...months].map(m => api.monthView(Number(m.slice(0, 4)), Number(m.slice(5, 7)))))
      .then((views: MonthView[]) => {
        choices = views.flatMap(v => v.days.flatMap(d => d.items.map(it => ({
          id: it.activityId, label: `${d.date} ${it.sport === 'running' ? '러닝' : it.sport === 'cycling' ? '라이딩' : '운동'} ${km(it.km)}km` }))))
      })
  })

  async function assign(photoId: number, activityId: string) {
    if (!activityId) return
    try {
      await api.assignPhoto(photoId, activityId)
      assigned = { ...assigned, [photoId]: activityId }
    } catch (e) {
      error = String(e)
    }
  }
</script>

<div class="backdrop" role="presentation" onclick={onClose}></div>
<div class="panel" role="dialog" aria-modal="true" aria-labelledby="import-title">
  <h2 id="import-title">{title}</h2>
  {#if title === '가져오기 결과'}<p class="summary">{importSummary(report)}</p>{/if}
  {#if report.failed.length}
    <h3>가져오지 못한 파일</h3>
    <ul>{#each report.failed as f}<li><b>{f.file}</b> {f.reason}</li>{/each}</ul>
  {/if}
  {#if report.unmatchedPhotos.length}
    <h3>운동 시간과 맞지 않는 사진</h3>
    <p class="muted">어느 운동의 사진인지 고르면 카드가 만들어져요.</p>
    <ul class="photos">
      {#each report.unmatchedPhotos as p}
        <li>
          <img src={api.fileUrl(p.path)} alt={p.fileName} />
          <span class="name">{p.fileName}</span>
          {#if assigned[p.id]}
            <span class="ok">붙였어요</span>
          {:else}
            <select aria-label="{p.fileName} 운동 선택" onchange={e => assign(p.id, (e.target as HTMLSelectElement).value)}>
              <option value="">운동 선택</option>
              {#each choices as c}<option value={c.id}>{c.label}</option>{/each}
            </select>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
  {#if error}<p class="error">{error}</p>{/if}
  <div class="foot"><button class="btn primary" onclick={onClose}>닫기</button></div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; background: rgba(27, 36, 48, .35); }
  .panel { position: fixed; top: 10vh; left: 50%; transform: translateX(-50%); width: min(560px, calc(100vw - 32px)); max-height: 80vh; overflow: auto;
    background: var(--surface); border-radius: 18px; padding: 24px; box-shadow: 0 20px 60px rgba(27, 36, 48, .25); }
  h2 { margin: 0 0 6px; font-size: 20px; }
  h3 { font-size: 15px; margin: 18px 0 6px; }
  .summary { margin: 0; font-weight: 600; }
  ul { padding-left: 18px; margin: 0; }
  .photos { list-style: none; padding: 0; display: grid; gap: 8px; }
  .photos li { display: flex; align-items: center; gap: 10px; }
  .photos img { width: 48px; height: 48px; object-fit: cover; border-radius: 8px; background: var(--track); }
  .name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13px; }
  select { border: 1px solid var(--line); border-radius: 8px; padding: 4px 8px; background: var(--surface); }
  .ok { color: #13704c; font-weight: 600; }
  .muted { color: var(--muted-foreground); margin: 0 0 8px; font-size: 14px; }
  .error { color: #b42318; }
  .foot { display: flex; justify-content: flex-end; margin-top: 20px; }
</style>
