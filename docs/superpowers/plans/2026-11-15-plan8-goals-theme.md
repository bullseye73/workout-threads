# 8단계: 종목별 목표 · 테마 설정 이동 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 라이딩·러닝 월 목표를 따로 정하고 보여 주며, 테마 선택을 상단 줄에서 설정 창으로 옮긴다.

**Architecture:** 설정 키 두 개(`goal_cycling_km`, `goal_running_km`)와 `MonthView`의 목표 필드 두 개. 예전 합계 목표(`monthly_goal_km`)는 처음 읽을 때 라이딩 목표로 옮기고 지운다. 화면은 종목마다 같은 계산(`sportGoal`)으로 막대를 그린다.

**Tech Stack:** Rust(workout-service, ts-rs), Tauri 2, Svelte 5, vitest.

**Spec:** `docs/superpowers/specs/2026-11-15-post-composer-design.md` §7.4, §7.5 (범위 메모 참고)

## Global Constraints
- 기존 제약 유지(noreply 커밋, `public-main`, 색은 토큰만).
- 목표 문구: 목표 창 제목 "월 목표", 칸 "라이딩 목표 (km)" / "러닝 목표 (km)", 설명 "비워 두면 그 종목은 목표 없이 거리만 보여 줘요.", 버튼 "목표 저장". 상단에서 목표가 하나도 없으면 "목표 정하기".
- 설정 창 맨 위 섹션 제목 "화면 테마", 선택은 기존 `ThemeToggle` 그대로(시스템/밝게/어둡게).

## Review Focus
1. 예전 합계 목표만 있는 사용자 → 첫 화면에서 라이딩 목표로 보이고, 러닝은 목표 없음. 이후 예전 키가 남아 다시 옮겨지지 않는다. → Task 1.
2. 한 종목만 목표 → 그 종목 막대 하나만, 다른 종목은 거리만. → Task 2.
3. 목표를 넘긴 종목 → 막대는 꽉 차고 "하루 N km면 목표"는 숨김. → Task 2.
4. 0이나 음수, 숫자 아닌 입력 → 그 종목 목표 없음으로 저장. → Task 1.

---

### Task 1: 종목별 목표 (`workout-service`, `src-tauri`)

**Files:** Modify `crates/service/src/views.rs`(목표 설정·조회), `crates/service/src/dto.rs`(`MonthView`), `src-tauri/src/main.rs`(명령 교체), 생성물 `ui/src/lib/bindings/MonthView.ts`; Test `crates/service/tests/views.rs`, `crates/service/tests/backup.rs`(호출 갱신)

**Interfaces — Produces:**
```rust
// MonthView: goal_km 제거, 대신
pub goal_cycling_km: Option<f64>,
pub goal_running_km: Option<f64>,
impl Library {
    pub fn set_goals(&mut self, cycling_km: Option<f64>, running_km: Option<f64>) -> Result<()>; // 0 이하·NaN은 None
}
// Tauri: set_goals(cyclingKm, runningKm) — set_goal_km 제거
```
- [ ] **Step 1: 실패하는 테스트** `views.rs`: `goals_are_reported_per_sport`(라이딩 300·러닝 50 저장 → 그대로, None으로 지우면 None), `old_total_goal_becomes_cycling_goal`(설정 `monthly_goal_km=200`을 `store_for_tests().set_setting`으로 넣고 `month_view` → 라이딩 200, 러닝 None, 다시 읽어도 같고 예전 키는 지워짐), `bad_goal_values_mean_no_goal`(0, -5, NaN → None). 기존 `goal_is_reported`·`empty_month_is_clean`의 `goal_km`은 새 필드로 바꾼다.
- [ ] **Step 2:** `cargo test -p workout-service --test views` FAIL(컴파일).
- [ ] **Step 3: 구현** — 예전 키 이전은 `month_view`가 목표를 읽을 때 한 번(`goal_cycling_km` 키가 없고 예전 키가 있으면 옮기고 예전 키 삭제). 읽기 전용 경로에서 쓰기가 생기므로 `goals(&self)`가 아니라 `month_view`에서 쓰기 가능한 내부 함수로(필요하면 `&self`로 store에 쓰기 — store는 내부 가변성 사용 중인지 확인 후 결정).
- [ ] **Step 4:** `cargo test -p workout-service` PASS, `cargo build -p workout-desktop` PASS, 바인딩 생성 확인.
- [ ] **Step 5: 커밋** `feat(service): separate riding and running goals`

### Task 2: 목표 막대·목표 창·테마 이동 (`ui`)

**Files:** Modify `ui/src/lib/workbench.ts`(+test), `ui/src/components/workbench/TopBar.svelte`, `ui/src/components/workbench/Workbench.svelte`(저장 호출), `ui/src/components/Settings.svelte`, `ui/src/lib/api.ts`, `ui/src/lib/mock.ts`

**Interfaces:**
- Consumes: Task 1 `goalCyclingKm`, `goalRunningKm`, `set_goals`.
- Produces:
```ts
export type SportGoal = { ratio: number; paceRatio: number; perDayKm: number | null }
export function sportGoal(doneKm: number, goalKm: number | null, year: number, month: number, today: Date): SportGoal | null // 목표 없으면 null
// goalProgress / GoalProgress 제거
api.setGoals(cyclingKm: number | null, runningKm: number | null): Promise<void>
```
- [ ] **Step 1: 실패하는 테스트** `sportGoal(102, 300, 2026, 11, 2026-11-14)` → ratio≈0.34, paceRatio≈13/30, perDayKm≈(300-102)/17; 목표 null → null; 넘긴 경우 ratio 1, perDayKm null; 지난 달 paceRatio 1·perDayKm null; 다음 달 paceRatio 0. (기존 `goalProgress` 테스트는 이 테스트로 바꾼다.)
- [ ] **Step 2:** FAIL → **Step 3:** 구현. TopBar: 목표가 있는 종목마다 `[종목 점] {거리}/{목표}km [막대 w-28] 하루 N km` 한 묶음, 없으면 합계 거리 + "목표 정하기"; 누르면 목표 창(두 칸). 종목 합계 텍스트(라이딩 N회 …)는 그대로. `ThemeToggle` 제거. Settings 맨 위에 "화면 테마" 섹션 + `ThemeToggle`. API·모의 교체.
- [ ] **Step 4:** `npm --prefix ui test`, `check`, `check:colors` PASS.
- [ ] **Step 5: 브라우저 확인**(모의): 목표 둘/하나/없음, 넘긴 종목, 설정 창 테마 전환, 375px 가로 넘침 없음.
- [ ] **Step 6: 커밋** `feat(ui): per-sport goal bars and theme in settings`
