<script lang="ts">
  import { api } from '../lib/api'
  import type { MonthView } from '../lib/bindings/MonthView'
  import { backupStatusText } from '../lib/backup'
  import type { BackupStatusDto } from '../lib/bindings/BackupStatusDto'
  import { buildMonthGrid } from '../lib/calendar'
  import { duration, gaugeRatio, km } from '../lib/format'
  import Calendar from './Calendar.svelte'
  import Gauge from './Gauge.svelte'
  import WeekBars from './WeekBars.svelte'

  let { year, month, onMonth, onOpen, onImport, onUnmatched, onSettings, reloadKey = 0 }: {
    year: number; month: number; onMonth: (y: number, m: number) => void
    onOpen: (id: string) => void; onImport: (kind: 'files' | 'folder') => void; onUnmatched: () => void; onSettings: () => void; reloadKey?: number
  } = $props()

  let view = $state<MonthView | null>(null)
  let error = $state('')
  let goalInput = $state('')
  let unmatched = $state(0)
  let backup = $state<BackupStatusDto | null>(null)

  $effect(() => {
    api.backupStatus().then(s => (backup = s)).catch(() => undefined)
    let off: (() => void) | undefined
    api.onBackupStatus(s => (backup = s)).then(f => (off = f))
    return () => off?.()
  })

  $effect(() => {
    void reloadKey
    error = ''
    api.monthView(year, month)
      .then(v => { view = v; goalInput = v.goalKm ? String(v.goalKm) : '' })
      .catch(e => (error = String(e)))
    api.unmatchedPhotos().then(p => (unmatched = p.length)).catch(() => (unmatched = 0))
    api.backupStatus().then(s => (backup = s)).catch(() => undefined)
  })

  function shift(delta: number) {
    const d = new Date(year, month - 1 + delta, 1)
    onMonth(d.getFullYear(), d.getMonth() + 1)
  }

  async function saveGoal() {
    const n = Number(goalInput)
    await api.setGoalKm(goalInput.trim() && n > 0 ? n : null)
    view = await api.monthView(year, month)
  }

  const weekLabel = (iso: string) => `${Number(iso.slice(5, 7))}/${Number(iso.slice(8, 10))}`
  const ratio = $derived(view ? (view.goalKm ? gaugeRatio(view.totalKm, view.goalKm) : 0) : 0)
  const grid = $derived(view ? buildMonthGrid(year, month, view.days) : [])
</script>

<header class="top">
  <div class="month">
    <button class="nav" onclick={() => shift(-1)} aria-label="이전 달">‹</button>
    <h1>{year}년 {month}월</h1>
    <button class="nav" onclick={() => shift(1)} aria-label="다음 달">›</button>
  </div>
  <div class="actions">
    {#if backup?.configured}
      <button class="backup" class:bad={!!backup.lastError || !backup.signedIn} onclick={onSettings}>{backupStatusText(backup, new Date())}</button>
    {/if}
    <label class="goal">월 목표
      <input type="number" min="0" step="10" placeholder="없음" bind:value={goalInput} onchange={saveGoal} />km
    </label>
    {#if unmatched > 0}<button class="btn" onclick={onUnmatched}>운동에 안 붙은 사진 {unmatched}장</button>{/if}
    <button class="btn" onclick={onSettings}>설정</button>
    <button class="btn" onclick={() => onImport('folder')}>폴더 가져오기</button>
    <button class="btn primary" onclick={() => onImport('files')}>파일 가져오기</button>
  </div>
</header>

{#if error}
  <p class="error">기록을 불러오지 못했어요: {error}</p>
{:else if view}
  <section class="hero">
    <Gauge ratio={view.goalKm ? ratio : 0}
      segments={[{ value: view.cycling.km, color: 'var(--ride)' }, { value: view.running.km, color: 'var(--run)' }]}>
      <div>
        <div class="total">{km(view.totalKm)}<small>km</small></div>
        <div class="goal-note">
          {#if view.goalKm}목표 {view.goalKm}km의 {Math.round(ratio * 100)}%{:else}목표를 정하면 고리가 채워져요{/if}
        </div>
      </div>
    </Gauge>
    <dl class="stats">
      <div><dt>운동 시간</dt><dd>{duration(view.totalMovingS)}</dd></div>
      <div><dt><i class="sw ride"></i>라이딩</dt><dd>{view.cycling.count}회 <span>{km(view.cycling.km)}km</span></dd></div>
      <div><dt><i class="sw run"></i>러닝</dt><dd>{view.running.count}회 <span>{km(view.running.km)}km</span></dd></div>
      <div><dt>평균 심박</dt><dd>{view.avgHr ? `${Math.round(view.avgHr)} bpm` : '—'}</dd></div>
    </dl>
    {#if view.days.length === 0}
      <p class="empty">이번 달 기록이 없어요. 파일 가져오기나 폴더 가져오기로 FIT 파일과 사진을 넣거나, 창에 끌어다 놓으면 여기에 쌓여요.</p>
    {/if}
  </section>

  <section class="focus">
    <article>
      <h2><i class="sw ride"></i>라이딩</h2>
      <p class="kpi">{view.cycling.avgSpeedKmh ? `평균 ${view.cycling.avgSpeedKmh.toFixed(1)} km/h` : '이번 달 라이딩 없음'}</p>
      <WeekBars color="var(--ride)" weeks={view.weekly.map(w => ({ label: weekLabel(w.weekStart), km: w.cyclingKm }))} />
    </article>
    <article>
      <h2><i class="sw run"></i>러닝</h2>
      <p class="kpi">{view.running.avgPace ? `평균 페이스 ${view.running.avgPace}/km` : '이번 달 러닝 없음'}</p>
      <WeekBars color="var(--run)" weeks={view.weekly.map(w => ({ label: weekLabel(w.weekStart), km: w.runningKm }))} />
    </article>
  </section>

  <section class="calendar">
    <h2>달력</h2>
    <Calendar {grid} {onOpen} />
    <p class="legend"><i class="dot"></i>게시 준비 안 됨 <i class="dot ready"></i>게시 준비됨 <i class="dot published"></i>게시함</p>
  </section>
{/if}

<style>
  .top { display: flex; justify-content: space-between; align-items: center; gap: 16px; flex-wrap: wrap; margin-bottom: 20px; }
  .month { display: flex; align-items: center; gap: 4px; }
  h1 { font-size: 30px; font-weight: 700; letter-spacing: -0.02em; margin: 0 6px; }
  .nav { border: 0; background: none; font-size: 28px; line-height: 1; width: 36px; height: 36px; border-radius: 999px; color: var(--muted-foreground); }
  .nav:hover { background: var(--track); color: var(--ink); }
  .actions { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .backup { border: 0; background: none; color: var(--muted-foreground); font-size: 13px; padding: 0; display: flex; align-items: center; gap: 6px; }
  .backup::before { content: ''; width: 8px; height: 8px; border-radius: 999px; background: var(--done); }
  .backup.bad { color: #b42318; }
  .backup.bad::before { background: #b42318; }
  .goal { display: flex; align-items: center; gap: 6px; color: var(--muted-foreground); font-size: 14px; }
  .goal input { width: 80px; border: 1px solid var(--line); border-radius: var(--radius-sm); padding: 5px 8px; background: var(--surface); text-align: right; }
  .hero { display: flex; align-items: center; gap: 48px; flex-wrap: wrap; background: var(--surface); border-radius: var(--radius-lg); padding: 28px 36px; }
  .total { font-size: 52px; font-weight: 700; letter-spacing: -0.03em; line-height: 1; }
  .total small { font-size: 18px; font-weight: 600; margin-left: 3px; color: var(--muted-foreground); }
  .goal-note { font-size: 13px; color: var(--muted-foreground); margin-top: 8px; max-width: 150px; }
  .stats { display: grid; grid-template-columns: repeat(2, minmax(150px, auto)); gap: 18px 48px; margin: 0; }
  .stats dt { font-size: 13px; color: var(--muted-foreground); display: flex; align-items: center; gap: 6px; }
  .stats dd { margin: 2px 0 0; font-size: 22px; font-weight: 700; }
  .stats dd span { font-size: 14px; font-weight: 600; color: var(--muted-foreground); margin-left: 4px; }
  .empty { flex-basis: 100%; margin: 0; color: var(--muted-foreground); }
  .sw { display: inline-block; width: 10px; height: 10px; border-radius: 3px; }
  .sw.ride { background: var(--ride); }
  .sw.run { background: var(--run); }
  .focus { display: grid; grid-template-columns: 1fr 1fr; gap: 20px; margin: 20px 0; }
  .focus article { padding: 4px 4px 0; }
  h2 { font-size: 17px; font-weight: 700; margin: 0 0 2px; display: flex; align-items: center; gap: 8px; }
  .kpi { margin: 0 0 14px; color: var(--muted-foreground); font-size: 14px; }
  .calendar h2 { margin-bottom: 10px; }
  .legend { display: flex; align-items: center; gap: 6px; font-size: 13px; color: var(--muted-foreground); margin-top: 10px; }
  .legend .dot { width: 8px; height: 8px; border-radius: 999px; background: #c3cad3; margin-left: 10px; }
  .legend .dot:first-child { margin-left: 0; }
  .legend .dot.ready { background: var(--ride); }
  .legend .dot.published { background: var(--done); }
  .error { color: #b42318; }
  @media (max-width: 760px) { .focus { grid-template-columns: 1fr; } .hero { padding: 20px; gap: 24px; } .stats { grid-template-columns: 1fr 1fr; gap: 14px 20px; } }
</style>
