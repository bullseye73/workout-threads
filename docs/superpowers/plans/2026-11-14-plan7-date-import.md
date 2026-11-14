# 7단계: 날짜를 골라 운동 기록 등록하기 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 날짜를 고르고 그 날짜의 FIT·사진만 등록한다. 날짜가 다른 파일은 넣지 않고 이유를 보여 준다.

**Architecture:** `workout-service`에 `import_for_date`를 더한다. 파일을 펼친 뒤 FIT 시작일·사진 촬영일을 고른 날짜와 비교해 걸러 `ImportReport.skipped`에 사유를 담고, 남은 파일은 기존 가져오기 본체(새 비공개 함수 `import_files`, `import_paths`도 이것을 쓴다)로 보낸다. 끝난 뒤 그날 운동이 하나뿐이면 시간이 안 맞은 그날 사진을 그 운동에 붙인다. 화면은 사진 띠의 빈 날짜 칸·상세 머리 버튼으로 "기록 추가" 패널을 열고, 패널이 열려 있으면 끌어다 놓기도 그 날짜로 등록한다.

**Tech Stack:** Rust(workout-service, ts-rs), Tauri 2, Svelte 5, vitest.

**Spec:** `docs/superpowers/specs/2026-11-14-date-import-design.md` (승인됨)

## Global Constraints
- 1–6단계 제약 유지. 커밋은 noreply 작성자, 브랜치 `public-main`.
- 날짜 기준: FIT = `header.start_local().date()`, 사진 = EXIF 촬영 시각의 현지 날짜(오프셋: 그날 운동의 `utc_offset_s`, 없으면 이 PC 시간대).
- 사유 문구(정확히): FIT `"{M}월 {D}일 기록이에요"`, 사진 `"{M}월 {D}일에 찍었어요"`, 촬영 날짜 없음 `"촬영 날짜가 없어요"`. M·D는 앞 0 없이.
- 상단 "가져오기"(`import_paths`) 동작은 바꾸지 않는다.
- 색은 토큰만(`check:colors`).

## Review Focus
1. 자정을 넘긴 운동(10/24 23:50 시작) → 10/24로 판단, 10/25를 고르면 넣지 않음. → Task 1 테스트 `fit_from_other_day_is_skipped`가 시작일 기준임을 확인.
2. 이미 가져온 같은 사진을 날짜 지정으로 다시 넣음 → "이미 있음"으로 세고 `skipped`에는 넣지 않음. → Task 1.
3. 그날 운동이 둘인데 시간이 안 맞는 사진 → 붙이지 않고 "안 붙은 사진"으로 남김. 샘플로 같은 날 운동 둘을 만들 수 없어 테스트 대신 코드 검토로 확인(조건은 `== 1`). → Task 1.
4. 기록 추가 패널이 열린 채 다른 운동을 고름 → 패널이 닫히고, 그 뒤 끌어다 놓기는 날짜 없는 가져오기. → Task 3.
5. 날짜 지정 등록 결과에서 새 운동이 없음(전부 다른 날짜) → 결과 창에 빠진 파일 목록만, 패널은 그대로. → Task 3.

---

### Task 1: 날짜 지정 가져오기 (`workout-service`)

**Files:**
- Modify: `crates/service/src/import.rs` (본체를 `import_files`로 분리, `import_for_date` 추가), `crates/service/src/dto.rs:33-39` (`skipped` 필드)
- Test: `crates/service/tests/date_import.rs`, `crates/service/tests/common.rs` (도우미 추가)
- 생성물: `ui/src/lib/bindings/ImportReport.ts`

**Interfaces:**
- Produces:
```rust
// dto.rs
pub struct ImportReport { /* 기존 */ pub skipped: Vec<ImportIssue> }   // 날짜가 달라 넣지 않은 파일
// import.rs
impl Library {
    pub fn import_for_date(&mut self, date: NaiveDate, paths: &[PathBuf]) -> Result<ImportReport>;
    fn import_files(&mut self, fits: &[PathBuf], photos: &[PathBuf], report: &mut ImportReport) -> Result<()>; // import_paths 본문
}
fn md(d: NaiveDate) -> String   // "10월 11일"
```
- 테스트 도우미(common.rs): `pub fn real_jpeg_with_local_time(src: &str, dest: &Path, local: &str)` — 샘플 JPEG의 기존 EXIF(APP1 "Exif") 구간을 빼고 `jpeg_with_local_time`과 같은 최소 EXIF를 SOI 바로 뒤에 넣는다(카드가 만들어지는 실제 이미지 + 원하는 촬영 시각).

- [ ] **Step 1: 실패하는 테스트** `date_import.rs` (모두 `require_samples!`):
  - `same_day_files_are_imported`: `import_for_date(2026-10-25, [20261025])` → `added_activities.len()==1`, `cards_created==2`, `skipped` 빔.
  - `fit_from_other_day_is_skipped`: `import_for_date(2026-10-25, [20261011])` → 추가 0, `skipped`에 FIT `{file:"…ACTIVITY.fit", reason:"10월 11일 기록이에요"}`, 사진 4장 모두 `"10월 11일에 찍었어요"`.
  - `photo_without_date_is_skipped`: 10/25 FIT + `b"\xFF\xD8\xFF\xD9"` 파일 `plain.jpg` → `skipped`에 `("plain.jpg","촬영 날짜가 없어요")`.
  - `same_day_photo_outside_ride_joins_the_only_ride`: 10/25 FIT 먼저 날짜 지정으로 넣고, `real_jpeg_with_local_time(P_0723, …, "2026:10:25 20:00:00")`(운동 끝 뒤 몇 시간)을 날짜 지정으로 넣음 → `unmatched_photos` 빔, `cards_created==1`, 그 운동 `post_view.card_paths.len()==3`.
  - `same_day_photo_without_ride_stays_unmatched`: 10/25 FIT 없이 위 사진만 날짜 지정으로 넣음 → `unmatched_photos.len()==1`(붙일 운동 없음), `skipped` 빔.
  - `already_imported_photo_counts_as_duplicate`: 같은 날짜 지정 가져오기를 두 번 → 두 번째는 `duplicate_files.len()==3`(FIT+사진 2), `skipped` 빔.
  - 기존 `import_paths` 테스트는 그대로 통과해야 함.
- [ ] **Step 2:** `cargo test -p workout-service --test date_import` → 컴파일 실패(함수·필드 없음).
- [ ] **Step 3: 구현.**
  - `import_paths`의 FIT 루프~글 쓰기를 `import_files`로 옮기고 `import_paths = expand + import_files`.
  - `import_for_date`: `expand` → FIT마다 `parse_fit`(읽기 실패는 통과시켜 `import_files`가 실패로 보고) 시작일 ≠ date면 `skipped`. 사진 오프셋 = 통과한 FIT 또는 `activities_in_month`에서 그날 운동의 `utc_offset_s`, 없으면 `local_offset_at(now)`; `read_photo_meta`의 `taken_at` 없으면 "촬영 날짜가 없어요", `local_date(t, offset) ≠ date`면 "…에 찍었어요". 남은 것으로 `import_files`.
  - 붙이기: 그날 운동(`activities_in_month` 중 `start_local().date()==date`)이 정확히 하나면, 이번 `report.unmatched_photos` 중 촬영일 == date인 것을 `store.photo(id)` → `attach_photo(row, &activity, None)`; 성공하면 `unmatched_photos`에서 빼고 `cards_created += 1`, 끝에 `write_template_post`.
- [ ] **Step 4:** `cargo test -p workout-service` PASS, `ui/src/lib/bindings/ImportReport.ts`에 `skipped` 생성.
- [ ] **Step 5: 커밋** `feat(service): import files for a chosen date`

### Task 2: 명령·API·결과 창 (`src-tauri`, `ui`)

**Files:**
- Modify: `src-tauri/src/main.rs` (명령 `import_for_date(date: String, paths: Vec<String>)`, 날짜는 `YYYY-MM-DD`), `ui/src/lib/api.ts` (`importForDate(date, paths)`, 모의: 경로 수만큼 추가 1개 + `skipped` 예시 1개), `ui/src/lib/format.ts` (`importSummary`에 "다른 날짜 N" 추가 — N>0일 때만), `ui/src/components/ImportPanel.svelte` ("날짜가 달라 넣지 않은 파일" 목록), `ui/src/lib/mock.ts` 필요 시
- Test: `ui/src/lib/format.test.ts`, `ui/src/lib/api.test.ts`

**Interfaces:**
- Consumes: Task 1 `import_for_date`, `ImportReport.skipped`.
- Produces: `api.importForDate(date: string, paths: string[]): Promise<ImportReport>`.

- [ ] **Step 1: 실패하는 테스트**: `importSummary`가 `skipped` 2개면 문자열 끝에 `" · 다른 날짜 2"`, 0개면 기존 문자열 그대로. `mockApi.importForDate('2026-11-14', ['a.fit'])`가 `skipped` 배열을 가진 보고서를 돌려줌.
- [ ] **Step 2:** FAIL 확인 → **Step 3:** 구현 → **Step 4:** `npm --prefix ui test`, `check`, `cargo build -p workout-desktop` PASS.
- [ ] **Step 5: 커밋** `feat: import_for_date command and skipped files in the import result`

### Task 3: 날짜를 골라 기록 추가하는 화면 (`ui`)

**Files:**
- Create: `ui/src/components/workbench/DayImport.svelte`
- Modify: `ui/src/lib/workbench.ts` (+test), `MonthRibbon.svelte` (빈 칸을 버튼으로, `onPickDate`), `Workbench.svelte` (`adding` 날짜 상태, 패널 표시, 가져온 뒤 선택), `ActivityPane.svelte` (머리에 "이 날짜에 기록 추가", `onAddForDate`), `App.svelte` (`runImport(paths, date?)`, 끌어다 놓기 날짜 라우팅)

**Interfaces:**
- Consumes: Task 2 `api.importForDate`.
- Produces:
```ts
// workbench.ts
export function dayLabel(iso: string): string                                  // "2026-11-04" → "11월 4일"
export function pickAfterImport(report: ImportReport, days: DayDto[], date: string): string | null
  // 새 운동 중 그 날짜인 것 → 없으면 그날 첫 운동 → 없으면 null
```
  - `DayImport` props `{ date: string; busy: boolean; onPick: (kind: 'files' | 'folder') => void; onClose: () => void }`. 문구: 제목 `"{dayLabel} 기록 추가"`, 설명 `"이 날짜의 FIT 파일과 이 날짜에 찍은 사진만 넣어요. 날짜가 다른 파일은 넣지 않고 알려 드려요."`, 버튼 `"파일 고르기"`/`"폴더 고르기"`, 안내 `"창에 끌어다 놓아도 이 날짜로 등록돼요."`, 닫기 `"닫기"`.
  - Workbench props 추가: `onImportForDate: (date: string, kind: 'files' | 'folder') => Promise<ImportReport | null>`, `onDropTarget: (date: string | null) => void`. 패널이 열리면 `onDropTarget(date)`, 닫히거나 운동을 고르면 `onDropTarget(null)`.
  - App: `dropDate` 상태; 끌어다 놓기 → `dropDate ? runImport(paths, dropDate) : runImport(paths)`; `runImport`는 date가 있으면 `importForDate`. 결과는 기존 `ImportPanel`.
- [ ] **Step 1: 실패하는 테스트** `workbench.test.ts`: `dayLabel('2026-11-04')=='11월 4일'`; `pickAfterImport`: 새 운동 id가 그 날짜 항목에 있으면 그 id, 새 운동이 없으면 그날 첫 항목 id, 그날 항목도 없으면 null.
- [ ] **Step 2:** FAIL → **Step 3:** 구현 → **Step 4:** `npm --prefix ui test`, `check`, `check:colors`.
- [ ] **Step 5: 브라우저 확인**(모의): 빈 날짜 칸 클릭 → 패널, 파일 고르기 → 결과 창에 "다른 날짜" 목록, 운동 선택 시 패널 닫힘, 운동 있는 날 머리 버튼으로 패널 열림. 밝게·어둡게, 375px 가로 넘침 없음.
- [ ] **Step 6: 커밋** `feat(ui): add workouts for a chosen date`
