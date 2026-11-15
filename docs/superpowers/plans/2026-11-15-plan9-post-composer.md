# 9단계: 게시물 작성 창 (자료를 올리면 운동마다 게시물 하나) 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 상단 "가져오기"와 날짜 지정 추가를 없애고, "게시물 작성" 창에서 FIT·사진을 올리면 미리보기 후 앱 보관함에 저장해 FIT마다 운동·게시물을 하나씩 만든다.

**Architecture:** 새 모듈 `crates/service/src/upload.rs`에 저장 없는 미리보기 `plan_upload`와 저장 `commit_upload`를 둔다. 둘 다 같은 내부 함수 `build_plan`이 정한 계획을 쓰고, 저장은 그 계획을 그대로 실행한다(미리보기 = 결과). 사진 붙이기 규칙(시간 일치 → 보관함 운동 시간 일치 → 그날 운동 하나 → 제외)은 `build_plan` 한 곳에만 있다. 화면은 `Composer` 창이 경로 목록을 들고 바뀔 때마다 미리보기를 다시 받는다.

**Tech Stack:** Rust(workout-service, ts-rs), Tauri 2, Svelte 5(shadcn-svelte Dialog), vitest.

**Spec:** `docs/superpowers/specs/2026-11-15-post-composer-design.md` §2 1–5, §4, §6, §7.1–7.3 (§7.4·7.5는 8단계에서 끝남)

## Global Constraints
- 기존 제약 유지(noreply 커밋, `public-main`, 색은 토큰만, 원본 파일은 옮기거나 지우지 않음).
- 사유 문구(정확히): 촬영 날짜 없음 `"촬영 날짜가 없어요"`, 붙일 곳 없음 `"붙일 운동이 없어 제외"`, 읽을 수 없는 FIT `"읽을 수 없는 파일"`, 잠긴 글 `"글은 이미 고쳤거나 게시해서 그대로 둬요"`.
- 창 문구: 제목 "새 게시물", 놓기 안내 "여기에 FIT와 사진을 끌어다 놓거나", 버튼 "파일 고르기" / "폴더 고르기" / "취소" / "게시물 {N}개 만들기"(N=0이고 보탤 사진이 있으면 "사진 보태기"), 안내 "올린 파일은 앱 보관함에 복사돼요. 원본은 그대로 둬요.", 묶음 제목 "만들 게시물 {N}개" / "이미 있는 운동 {N}개" / "제외 {N}개", 결과 "게시물 {N}개를 만들었어요" / "사진을 보탠 운동", 상단 버튼 "게시물 작성".
- 폴더는 하위 3단계까지, 이름이 `.`으로 시작하는 폴더는 건너뜀.
- "그날 운동 하나" = 그 날짜(사진 촬영 현지 날짜)의 보관함 운동(종목 무관) + 이번 새 운동. 화면에서 뺀 파일은 경로 목록에서 빠지므로 자연히 세지 않는다.

## Review Focus
1. 같은 FIT 두 개(이름만 다름)를 함께 올림 → 운동 하나, 다른 하나는 이미 있는 것과 같은 취급(중복). → Task 1 `same_fit_twice_makes_one_activity`.
2. 미리보기 후 사용자가 사진 하나를 뺐는데 그 사진만 그날 다른 운동을 "하나뿐"으로 만들던 경우 → 다시 계산된 계획대로. 저장은 항상 받은 경로로 다시 계산하므로 미리보기와 같다. → Task 2 `commit_matches_plan`.
3. 보관함에 이미 붙어 있는 사진을 다시 올림 → "이미 있음", 다시 복사하지 않음. → Task 1.
4. 운동 상세가 열린 채 그 운동에 사진을 보탬 → 만들기 후 그 운동의 사진·카드가 갱신되어 보인다. → Task 4 브라우저 확인.
5. 창이 닫혀 있을 때 앱 창에 끌어다 놓음 → 창이 열리고 그 파일로 미리보기. → Task 4.

---

### Task 1: 미리보기 `plan_upload` (`workout-service`)

**Files:** Create `crates/service/src/upload.rs`; Modify `crates/service/src/lib.rs`(mod), `crates/service/src/import.rs`(쓰는 도우미를 `pub(crate)`로, `expand`에 하위 폴더), `crates/service/src/dto.rs`(DTO); Test `crates/service/tests/upload.rs`

**Interfaces — Produces:**
```rust
// dto.rs (TS export)
pub struct PlannedPhoto { pub path: String /*원본 절대 경로*/, pub file_name: String, pub km_mark: Option<f64> }
pub struct PlannedActivity {
    pub source: String,          // FIT 원본 경로
    pub activity_id: String, pub date: String /*YYYY-MM-DD*/, pub title: String, pub sport: SportKind, pub km: f64,
    pub photos: Vec<PlannedPhoto>,
    pub library_photos: u32,     // 보관함에 있던 안 붙은 사진 중 이 운동에 붙을 수
}
pub struct ExistingTarget {
    pub activity_id: String, pub date: String, pub title: String, pub sport: SportKind, pub km: f64,
    pub uploaded_fit: bool,      // 이 FIT를 다시 올렸음
    pub photos: Vec<PlannedPhoto>, // 이번에 보탤 사진
    pub post_locked: bool,       // 글이 고쳐졌거나 게시됨
}
pub struct UploadPlan {
    pub new_activities: Vec<PlannedActivity>, pub existing: Vec<ExistingTarget>,
    pub excluded: Vec<ImportIssue>, pub duplicates: Vec<String> /*이미 보관·붙은 사진 파일 이름*/,
}
// upload.rs
impl Library { pub fn plan_upload(&self, paths: &[PathBuf]) -> Result<UploadPlan>; }
pub(crate) struct Plan { /* build_plan 결과: UploadPlan + 저장에 필요한 원본 데이터(파싱된 Activity, 사진 메타·해시, 보관함 사진 id→운동 id) */ }
impl Library { pub(crate) fn build_plan(&self, paths: &[PathBuf]) -> Result<Plan>; }
```
- [ ] **Step 1: 실패하는 테스트** `tests/upload.rs` (샘플 필요한 것은 `require_samples!`, 임시 폴더 구성은 `copy_as`·`real_jpeg_with_local_time`):
  - `one_fit_with_photos_plans_one_post`: `[20261025]` → `new_activities.len()==1`, 그 `photos.len()==2`, `date=="2026-10-25"`, `title=="일요일 오후 라이딩"`, 아무것도 저장 안 됨(`month_view(2026,10).days` 빔).
  - `two_fits_plan_two_posts`: `[20261011, 20261025]` → 2개, 날짜 순.
  - `nested_folders_are_searched`: 임시 `root/a/b/` 아래에 10/25 FIT·사진 복사 후 `[root]` → 1개, 사진 2.
  - `same_fit_twice_makes_one_activity`: 같은 FIT를 다른 이름으로 두 번 → `new_activities.len()==1`.
  - `already_imported_fit_is_existing`: `import_paths([20261025])` 후 `[20261025]` 계획 → `new_activities` 빔, `existing[0].uploaded_fit==true`, 사진 2장은 `duplicates`.
  - `photo_without_date_is_excluded`: 10/25 FIT + `plain.jpg`(EXIF 없음) → `excluded==[("plain.jpg","촬영 날짜가 없어요")]`.
  - `photo_on_a_day_without_workout_is_excluded`: `real_jpeg_with_local_time(P_0723,…,"2026:10:20 12:00:00")`만 → `excluded` 사유 `"붙일 운동이 없어 제외"`.
  - `late_photo_joins_the_only_workout_that_day`: 10/25 FIT + 20:00에 찍은 사진 → 그 운동 `photos`에 포함, `km_mark==None`.
  - `photo_for_stored_workout_goes_to_existing`: `import_paths([10/25 FIT])` 후 20:00 사진만 계획 → `existing[0].photos.len()==1`, `uploaded_fit==false`.
  - `library_unmatched_photo_counts_for_new_workout`: `import_paths([20:00 사진])`(예전 흐름 → 안 붙음) 후 `[10/25 FIT]` 계획 → `new_activities[0].library_photos==1`.
  - `locked_post_is_flagged`: `import_paths([10/25 FIT])`, `save_post`로 글 수정, 20:00 사진 계획 → `existing[0].post_locked==true`.
- [ ] **Step 2:** `cargo test -p workout-service --test upload` FAIL(없는 함수).
- [ ] **Step 3: 구현.** `expand`: 하위 폴더 3단계까지(숨김 제외), 정렬해 결정적으로. `build_plan`: FIT 파싱→운동 ID로 중복 제거→보관함 존재 여부로 새/기존 분리(읽기 실패는 `excluded`). 사진: 해시가 보관함에 있으면 붙어 있으면 `duplicates`, 안 붙었으면 그 보관함 사진을 후보로; 아니면 새 사진. 사진마다 오프셋 = EXIF 태그 → 짝 후보 운동 오프셋 → PC 시간대(`import_photo`와 같은 2단계 추정). 붙이기 순서: ① 새 운동 `match_photo` ② `activities_near(t, NEAR_HOURS)`의 보관함 운동 `match_photo` ③ 촬영 현지 날짜의 보관함 운동 + 새 운동이 정확히 하나면 그 운동(km 없음) ④ 새 사진은 `excluded`, 보관함 사진은 그대로. 이어 `store.unmatched_photos()` 중 이번에 안 다룬 것도 ①·③으로 새 운동에만 붙여 `library_photos`를 센다. `post_locked`은 `store.post(id)`의 `edited || published_at.is_some()`.
- [ ] **Step 4:** `cargo test -p workout-service` PASS, 바인딩 생성(`UploadPlan.ts` 등).
- [ ] **Step 5: 커밋** `feat(service): plan an upload without storing anything`

### Task 2: 저장 `commit_upload`, 날짜 지정 가져오기 삭제, 보관 자료 정보 (`workout-service`, `src-tauri`)

**Files:** Modify `upload.rs`, `import.rs`(`import_for_date`·`attach_day_photos`·`activities_on` 삭제, 사진 저장 도우미 분리), `dto.rs`(`UploadResult`, `ActivityDetail`에 보관 자료), `views.rs`(상세에 채우기), `src-tauri/src/main.rs`(명령 `plan_upload`·`commit_upload` 추가, `import_paths`·`import_for_date` 명령 삭제); Delete `crates/service/tests/date_import.rs`, `date_import_tz.rs`; Test `tests/upload.rs`, `tests/views.rs`

**Interfaces:**
- Consumes: Task 1 `build_plan`, `UploadPlan`.
- Produces:
```rust
pub struct UploadResult {
    pub created: Vec<String>,   // 새 운동 id (날짜 순)
    pub updated: Vec<String>,   // 사진을 보탠 기존 운동 id
    pub cards_created: u32,
    pub failed: Vec<ImportIssue>, pub excluded: Vec<ImportIssue>,
}
impl Library { pub fn commit_upload(&mut self, paths: &[PathBuf]) -> Result<UploadResult>; }
// ActivityDetail 추가 필드
pub fit_file: String,        // 보관함 FIT 절대 경로
pub folder: String,          // 보관 폴더 절대 경로
// (사진·카드 경로는 기존 photos[].path / cardPath)
```
- [ ] **Step 1: 실패하는 테스트**:
  - `commit_matches_plan`: `[20261011, 20261025]`의 `plan_upload` 후 `commit_upload` → `created`가 계획의 `activity_id`들과 같고(순서 포함), 각 운동 `post_view.card_paths.len()`이 계획 `photos.len()`과 같음, `cards_created==6`.
  - `commit_attaches_late_photo_and_library_photo`: `import_paths([20:00 사진 A])` 후 `commit_upload([10/25 FIT, 20:05 사진 B])` → 그 운동 카드 2장(A·B), `unmatched_photos()` 빔.
  - `excluded_photos_are_not_stored`: 10/20 사진만 `commit_upload` → `excluded.len()==1`, 라이브러리 `files/` 아래 파일 수 변화 없음(`count_files`).
  - `existing_workout_gets_photos_and_is_updated`: `import_paths([10/25 FIT])` 후 20:00 사진 `commit_upload` → `updated==[id]`, `created` 빔.
  - `originals_are_untouched`: 커밋 뒤 원본 경로 파일들이 그대로 있음.
  - `views.rs` `activity_detail_lists_stored_files`: 10/25 가져온 뒤 `fit_file`이 라이브러리 `files/20261025/…ACTIVITY.fit`로 끝나고 존재, `folder`가 그 폴더.
  - 기존 `import_paths` 테스트는 그대로 통과(함수는 남김: 테스트·보관함 사진 시나리오가 씀).
- [ ] **Step 2:** FAIL → **Step 3:** 구현. 저장은 `build_plan`의 결과만 실행: 새 운동은 `import_fit`과 같은 복사·삽입, 사진은 원본 읽어 그 운동 날짜 폴더에 `unique_rel`로 복사 → `PhotoRow` 삽입(+`make_card`), 보관함 사진은 `attach_photo`. 끝에 새 운동·보탠 운동마다 `write_template_post`(잠긴 글은 건너뜀). 하나가 실패해도 계속하고 `failed`에 담는다.
- [ ] **Step 4:** `cargo test -p workout-service`, `cargo build -p workout-desktop`, `cargo clippy` PASS.
- [ ] **Step 5: 커밋** `feat(service): commit an upload as one post per workout; drop date-based import`

### Task 3: 화면 API (`ui/src/lib`)

**Files:** Modify `ui/src/lib/api.ts`(+test), `ui/src/lib/mock.ts`, `ui/src/lib/format.ts`(+test) 

**Interfaces — Produces:**
```ts
api.planUpload(paths: string[]): Promise<UploadPlan>
api.commitUpload(paths: string[]): Promise<UploadResult>
// importPaths, importForDate 제거 (pickImport는 남김)
export function removeActivityPaths(plan: UploadPlan, paths: string[], source: string): string[]
  // 그 FIT 경로와, 그 운동에만 붙는 사진 경로를 뺀 목록
export function createLabel(plan: UploadPlan): string   // "게시물 2개 만들기" | "사진 보태기" | "게시물 0개 만들기"
```
- [ ] **Step 1: 실패하는 테스트**: `removeActivityPaths`가 FIT와 그 사진만 빼고 다른 경로는 남김; `createLabel` 세 경우; `mockApi.planUpload(['a.fit','b.jpg'])`가 새 운동 1개·제외 1개를 돌려줌; `mockApi.commitUpload`가 `created` 1개.
- [ ] **Step 2:** FAIL → **Step 3:** 구현(헬퍼는 `ui/src/lib/composer.ts` 새 파일) → **Step 4:** `npm --prefix ui test`, `check` PASS.
- [ ] **Step 5: 커밋** `feat(ui): upload plan/commit API and composer helpers`

### Task 4: 게시물 작성 창과 연결 (`ui`)

**Files:** Create `ui/src/components/Composer.svelte`; Modify `ui/src/App.svelte`(끌어다 놓기 → 창, 만든 뒤 열기, 장소·날씨 채우기), `ui/src/components/workbench/TopBar.svelte`(가져오기 메뉴 → "게시물 작성" 버튼), `Workbench.svelte`(날짜 추가 상태·`DayImport` 제거, 열 운동 `focus` prop), `MonthRibbon.svelte`(빈 칸 다시 비활성), `ActivityPane.svelte`("이 날짜에 기록 추가" 제거, 처음 탭 prop `initialTab`); Delete `DayImport.svelte`, `ImportPanel.svelte`의 날짜 관련 표시(`skipped` 목록은 남김: 안 붙은 사진 창에서 쓰지 않으면 삭제); `workbench.ts`의 `pickAfterImport`·`DateImported`·`dayLabel` 사용처 정리(+test)

**Interfaces:**
- Consumes: Task 3 API·헬퍼.
- Produces: `Composer` props `{ open: boolean; initialPaths?: string[]; onClose: () => void; onOpenActivity: (id: string, date: string) => void }`. Workbench prop `focus: { id: string; tab: 'record' | 'post'; n: number } | null` → 그 운동 선택, `ActivityPane initialTab`.
- [ ] **Step 1:** 구현. 창 배치는 설계 7.1 그림대로(만들 게시물/이미 있는 운동/보관함 사진 수/제외, 운동 줄마다 "빼기", 사진 썸네일 누르면 그 사진만 빼기, 잠긴 글 안내). 경로가 바뀔 때마다 `planUpload`(진행 중이면 "읽는 중…"). 만들기 → `commitUpload` → 결과 목록(만든 게시물 / 사진을 보탠 운동), 하나만 만들었으면 바로 열기. 열기 = App이 해당 달로 이동 + Workbench `focus`(`tab: 'post'`). 만든 운동은 `enrichNew`. 앱 창 끌어다 놓기: 창이 열려 있으면 경로 추가, 닫혀 있으면 그 경로로 연다.
- [ ] **Step 2:** `npm --prefix ui test`, `check`, `check:colors`, `cargo build -p workout-desktop` PASS. 남은 `importPaths`·`DayImport`·`onPickDate` 참조 없음(`grep`).
- [ ] **Step 3: 브라우저 확인**(모의): 창 열기·파일 고르기→미리보기·빼기·만들기→결과→열면 게시 준비 탭, 끌어다 놓기는 Tauri에서만이라 코드 확인. 밝게·어둡게, 375px.
- [ ] **Step 4: 커밋** `feat(ui): post composer replaces import`

### Task 5: 운동 상세 "보관된 자료" (`ui`)

**Files:** Create `ui/src/components/detail/StoredFiles.svelte`; Modify `ActivityPane.svelte`, `ui/src/lib/mock.ts`(상세에 `fitFile`, `folder`)

**Interfaces:** Consumes Task 2 `ActivityDetail.fitFile`, `folder`, `photos[].path/cardPath`.
- [ ] **Step 1:** 구현. 기록 탭 맨 아래 접히는 칸(기본 접힘) 제목 "보관된 자료", 줄: "FIT 파일 {이름}", "원본 사진 {N}장", "카드 {M}장", 폴더 경로(작은 글씨) + "폴더 열기"(`api.openFolder(folder)`).
- [ ] **Step 2:** `check`, `check:colors` PASS, 브라우저 확인.
- [ ] **Step 3: 커밋** `feat(ui): show where a workout's files are stored`
