# 2단계: Tauri 데스크톱 앱 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** FIT·사진을 가져와 로컬 라이브러리(SQLite + 파일)에 쌓고, 월간 대시보드·달력·운동 상세(2D 지도·그래프)·게시 준비 화면을 보여 주는 Tauri 2 앱을 만든다.

**Architecture:** 1단계 crate 위에 `workout-store`(SQLite)와 `workout-service`(가져오기·조회·게시 준비를 하는 `Library` 파사드, 화면용 DTO)를 얹는다. `src-tauri`는 `Library` 메서드를 Tauri 명령으로 노출하는 얇은 껍데기다. 화면은 `ui/`(Vite + Svelte 5 + TypeScript)이고, `api.ts`가 Tauri 호출과 브라우저용 가짜 데이터를 같은 인터페이스로 감싸 브라우저에서도 화면을 확인할 수 있다. DTO 타입은 `ts-rs`로 Rust에서 TypeScript로 생성한다.

**Tech Stack:** Rust: `tauri` 2.x(3.0 알파 쓰지 않음), `tauri-plugin-dialog`, `tauri-plugin-clipboard-manager`, `tauri-plugin-opener`, `rusqlite`(bundled), `serde`, `serde_json`, `ts-rs`. UI: Node 24, `@tauri-apps/cli` 2.x(npm devDependency, `cargo install` 안 함), `@tauri-apps/api` 2.x, Svelte 5, Vite, `leaflet` 1.9, `chart.js` 4, `vitest`.

**Spec:** `docs/superpowers/specs/2026-11-13-workout-threads-app-design.md` §6.1–6.5, §6.7 (로드맵: `docs/superpowers/plans/2026-11-13-roadmap.md`)

## Global Constraints

- 1단계 Global Constraints 그대로 유효 (3 OS 공통, OS 전용 API 금지, 글 규칙, 카드 규칙, 매칭 창 ±10분, 경로 최대 1,500점).
- 라이브러리 폴더 = Tauri `app_data_dir()`. 안에 `library.db`, `files/YYYYMMDD/`(원본 FIT·사진·카드). **DB에 저장하는 파일 경로는 라이브러리 기준 상대 경로** (4단계 드라이브 복원 대비).
- 원본 사용자 폴더(`sns_create/2026*`)에는 절대 쓰지 않는다. 가져오기는 복사.
- 대시보드는 FIT 운동 기록만 사용. 바디 배터리·스트레스·수면 칸 없음.
- 지도: Leaflet + OpenStreetMap 타일 `https://tile.openstreetmap.org/{z}/{x}/{y}.png`, 저작자 표시 필수. 타일 URL은 `settings` 테이블 `tile_url`로 바꿀 수 있게.
- 달력 표시: 🏃 러닝 / 🚲 라이딩 / 👟 기타 + 거리, 게시 상태 3단계(준비 안 됨 / 준비됨 / 게시함).
- 화면 문구는 한국어. UI 작업(Task 6–8)은 시작할 때 `frontend-design` 스킬을 먼저 불러 따른다 (사용자 전역 지침). 참고 화면: 사용자가 준 Garmin Connect 캡처(흰 카드, 원형 게이지, 파란·주황 강조).
- 커밋 메시지 끝: `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. **같은 폴더를 두 번 가져오기** — 운동·사진·카드가 중복 생기지 않고 결과에 "이미 있음"으로 표시. → Task 3 테스트.
2. **사진만 나중에 가져오기** — FIT는 어제 가져왔고 오늘 사진만 끌어다 놓으면, 같은 묶음이 아니라 **라이브러리의 기존 운동**과 매칭돼야 한다. → Task 3 테스트.
3. **글을 고친 뒤 다시 열기** — 고친 글이 유지되고, "템플릿으로 다시 쓰기"를 누를 때만 덮어쓴다. → Task 4 테스트.
4. **운동이 없는 달·목표 미설정** — 빈 대시보드, NaN·0으로 나누기·"페이스 /km" 같은 깨진 표시 없음. → Task 4(Rust), Task 6(vitest) 테스트.
5. **라이브러리 경로에 한글·공백 (Windows 사용자 폴더 등)** — 사진·카드 썸네일이 `convertFileSrc`로 뜨고, asset 프로토콜 범위가 라이브러리 폴더를 허용. → Task 5 설정 + Task 3 테스트(한글 폴더명 라이브러리).

---

## 파일 구조

```
workout-app/
├─ Cargo.toml                         members에 crates/store, crates/service, src-tauri 추가
├─ crates/
│  ├─ core/src/model.rs               (수정) Sport::to_fit
│  ├─ core/src/matching.rs            (수정) assign_photos
│  ├─ cli/src/prepare.rs              (수정) assign_photos 사용
│  ├─ store/src/
│  │  ├─ lib.rs                       Store, StoreError
│  │  ├─ schema.rs                    마이그레이션 (PRAGMA user_version)
│  │  └─ rows.rs                      ActivityRow, PhotoRow, PostRow, PublishRow
│  └─ service/src/
│     ├─ lib.rs                       Library, ServiceError
│     ├─ dto.rs                       화면 DTO (#[derive(Serialize, TS)])
│     ├─ import.rs                    import_paths
│     ├─ views.rs                     month_view, activity_detail
│     └─ posts.rs                     post_view, save_post, regenerate_post, mark_published, assign_photo
├─ src-tauri/
│  ├─ Cargo.toml, build.rs, tauri.conf.json, capabilities/default.json, icons/
│  └─ src/main.rs                     명령 등록, Library 상태
└─ ui/
   ├─ package.json, vite.config.ts, tsconfig.json, index.html
   └─ src/
      ├─ main.ts, App.svelte          화면 전환 (dashboard | activity | post)
      ├─ lib/bindings/*.ts            ts-rs 생성물 (커밋함)
      ├─ lib/api.ts                   Api 인터페이스, tauriApi, mockApi
      ├─ lib/mock.ts                  가짜 데이터 (서울 중심 합성 경로, 실제 좌표 아님)
      ├─ lib/format.ts                km·시간·페이스 표시
      ├─ lib/calendar.ts              buildMonthGrid
      ├─ lib/series.ts                toChartSeries, nearestIndex
      └─ components/
         Gauge.svelte, WeekBars.svelte, Calendar.svelte, Dashboard.svelte,
         RouteMap.svelte, MetricCharts.svelte, SplitsTable.svelte, ActivityDetail.svelte,
         PostPanel.svelte, ImportPanel.svelte
```

---

### Task 1: 코어 보강 — 사진 묶음 매칭 재사용

**Files:**
- Modify: `crates/core/src/model.rs`, `crates/core/src/matching.rs`, `crates/cli/src/prepare.rs`
- Test: `crates/core/tests/photo.rs`, `crates/core/tests/format.rs`

**Interfaces:**
- Produces:
```rust
impl Sport { pub fn to_fit(self) -> u8 }   // from_fit의 역함수, Other(c) → c
pub struct PhotoAssignment { pub photo: PathBuf, pub taken_at: Option<DateTime<Utc>>, pub matched: Option<PhotoMatch> }
/// 사진마다 메타를 읽고(fallback 오프셋 = 각 사진 날짜와 가장 가까운 운동의 오프셋, 운동이 없으면 0) 매칭한다.
pub fn assign_photos(activities: &[Activity], photos: &[PathBuf]) -> Vec<PhotoAssignment>
```
CLI `prepare`는 직접 루프 대신 `assign_photos` 결과를 쓴다 (동작 동일, 1단계 CLI 테스트가 그대로 통과해야 함).

- [ ] **Step 1: 실패하는 테스트**
```rust
#[test] fn sport_roundtrip() { for c in [1u8, 2, 11, 17, 5] { assert_eq!(Sport::from_fit(c).to_fit(), c); } }
#[test] fn assigns_batch() {
    let ride = parse_fit(&read(fit_20261025())).unwrap();
    let photos = vec![sample_path("20261025/78D7CE5C-F354-4728-8E97-063487771792_1_105_c.jpeg"),
                      sample_path("20261011/31CCE40E-34DE-405F-ACE0-A1B5DEFC07D4_4_5005_c.jpeg")];
    let r = assign_photos(&[ride.clone()], &photos);
    assert_eq!(r[0].matched.as_ref().unwrap().activity_id, ride.header.id);
    assert!(r[1].taken_at.is_some() && r[1].matched.is_none());
}
```
- [ ] **Step 2:** `cargo test -p workout-core` → 컴파일 실패.
- [ ] **Step 3:** 구현 + `prepare.rs`를 `assign_photos`로 교체.
- [ ] **Step 4:** `cargo test --workspace` → 전체 PASS (1단계 46개 + 새 테스트).
- [ ] **Step 5:** 커밋 `refactor(core): assign_photos shared by CLI and app`.

---

### Task 2: 저장소 `workout-store`

**Files:**
- Create: `crates/store/Cargo.toml`, `crates/store/src/{lib.rs,schema.rs,rows.rs}`
- Test: `crates/store/tests/store.rs`

**Interfaces:**
- Consumes: `ActivityHeader`, `LatLon`, `Sport::to_fit/from_fit`, `workout_post::{PlaceText, WeatherText, PublishMethod}`.
- Produces:
```rust
pub struct ActivityRow { pub header: ActivityHeader, pub fit_path: String /* 라이브러리 상대 */, pub route: Vec<LatLon> /* ≤1500 */ }
pub struct PhotoRow { pub id: i64 /* insert 시 무시 */, pub activity_id: Option<String>, pub path: String /* 상대 */,
                      pub taken_at: Option<DateTime<Utc>>, pub km_mark: Option<f64>, pub lat: Option<f64>, pub lon: Option<f64>,
                      pub card_path: Option<String> /* 상대 */ }
pub struct PostRow { pub activity_id: String, pub body: String, pub hashtags: Vec<String>, pub place: Option<PlaceText>,
                     pub route_points: Vec<String>, pub weather: Option<WeatherText>, pub edited: bool,
                     pub published_at: Option<DateTime<Utc>> }
pub struct PublishRow { pub activity_id: String, pub method: PublishMethod, pub at: DateTime<Utc>, pub external_id: Option<String> }
#[derive(thiserror::Error, Debug)] pub enum StoreError { Db(#[from] rusqlite::Error), Json(#[from] serde_json::Error) }
pub struct Store { /* rusqlite::Connection */ }
impl Store {
    pub fn open(path: &Path) -> Result<Store, StoreError>;          // 없으면 만들고 마이그레이션
    pub fn open_in_memory() -> Result<Store, StoreError>;
    pub fn has_activity(&self, id: &str) -> Result<bool, StoreError>;
    pub fn insert_activity(&self, row: &ActivityRow) -> Result<(), StoreError>;
    pub fn activity(&self, id: &str) -> Result<Option<ActivityRow>, StoreError>;
    pub fn activities_in_month(&self, year: i32, month: u32) -> Result<Vec<ActivityRow>, StoreError>; // 현지 시작 날짜 기준, 시작순
    pub fn activities_near(&self, at: DateTime<Utc>, hours: i64) -> Result<Vec<ActivityRow>, StoreError>; // 시작이 at±hours
    pub fn insert_photo(&self, p: &PhotoRow) -> Result<i64, StoreError>;
    pub fn photo(&self, id: i64) -> Result<Option<PhotoRow>, StoreError>;
    pub fn photo_by_path(&self, path: &str) -> Result<Option<PhotoRow>, StoreError>;
    pub fn photos_for(&self, activity_id: &str) -> Result<Vec<PhotoRow>, StoreError>;   // km_mark 순
    pub fn unmatched_photos(&self) -> Result<Vec<PhotoRow>, StoreError>;
    pub fn update_photo(&self, p: &PhotoRow) -> Result<(), StoreError>;
    pub fn post(&self, activity_id: &str) -> Result<Option<PostRow>, StoreError>;
    pub fn upsert_post(&self, p: &PostRow) -> Result<(), StoreError>;
    pub fn insert_publish(&self, r: &PublishRow) -> Result<(), StoreError>;
    pub fn setting(&self, key: &str) -> Result<Option<String>, StoreError>;
    pub fn set_setting(&self, key: &str, value: Option<&str>) -> Result<(), StoreError>;
}
```
스키마 v1 (`PRAGMA user_version = 1`): `activities(id TEXT PK, sport INT, start_utc INT, start_local TEXT 'YYYY-MM-DD HH:MM:SS', utc_offset_s INT, distance_m REAL, moving_s REAL, elapsed_s REAL, avg_hr INT, max_hr INT, avg_speed_mps REAL, ascent_m INT, avg_cadence INT, fit_path TEXT, route_json TEXT)`, `photos(id INTEGER PK, activity_id TEXT NULL REFERENCES activities, path TEXT UNIQUE, taken_at INT NULL, km_mark REAL NULL, lat REAL NULL, lon REAL NULL, card_path TEXT NULL)`, `posts(activity_id TEXT PK, body TEXT, hashtags_json TEXT, place_name TEXT NULL, place_estimated INT, route_points_json TEXT, weather_desc TEXT NULL, weather_temp REAL NULL, edited INT, published_at INT NULL)`, `publish_records(id INTEGER PK, activity_id TEXT, method TEXT, at INT, external_id TEXT NULL)`, `settings(key TEXT PK, value TEXT)`. `activities_in_month`는 `start_local LIKE 'YYYY-MM-%'`.

- [ ] **Step 1: 실패하는 테스트** (`open_in_memory`, 합성 header 사용)
```rust
#[test] fn activity_roundtrip() {
    let s = Store::open_in_memory().unwrap();
    let row = activity_row("a1", Sport::Running, "2026-11-01T22:00:00Z", 5000.0);
    s.insert_activity(&row).unwrap();
    assert!(s.has_activity("a1").unwrap());
    let back = s.activity("a1").unwrap().unwrap();
    assert_eq!(back.header.sport, Sport::Running);
    assert_eq!(back.header.start, row.header.start);
    assert_eq!(back.route, row.route);
}
#[test] fn month_query_uses_local_date() {
    let s = Store::open_in_memory().unwrap();
    s.insert_activity(&activity_row("nov", Sport::Running, "2026-11-01T22:00:00Z", 5000.0)).unwrap(); // KST 11-02
    s.insert_activity(&activity_row("oct", Sport::Cycling, "2026-10-31T14:30:00Z", 9000.0)).unwrap(); // KST 10-31 23:30
    assert_eq!(ids(s.activities_in_month(2026, 11).unwrap()), vec!["nov"]);
    assert_eq!(ids(s.activities_in_month(2026, 10).unwrap()), vec!["oct"]);
}
#[test] fn photos_and_unmatched() {
    let s = Store::open_in_memory().unwrap();
    s.insert_activity(&activity_row("a1", Sport::Cycling, "2026-10-25T05:41:29Z", 48000.0)).unwrap();
    let id = s.insert_photo(&photo_row(None, "files/20261025/p.jpeg")).unwrap();
    assert_eq!(s.unmatched_photos().unwrap().len(), 1);
    let mut p = s.photo(id).unwrap().unwrap(); p.activity_id = Some("a1".into()); p.km_mark = Some(12.3);
    s.update_photo(&p).unwrap();
    assert!(s.unmatched_photos().unwrap().is_empty());
    assert_eq!(s.photos_for("a1").unwrap()[0].km_mark, Some(12.3));
    assert!(s.insert_photo(&photo_row(None, "files/20261025/p.jpeg")).is_err()); // path UNIQUE
}
#[test] fn post_upsert_keeps_latest() {
    let s = Store::open_in_memory().unwrap();
    s.insert_activity(&activity_row("a1", Sport::Cycling, "2026-10-25T05:41:29Z", 48000.0)).unwrap();
    s.upsert_post(&post_row("a1", "첫 글", false)).unwrap();
    s.upsert_post(&post_row("a1", "고친 글", true)).unwrap();
    let p = s.post("a1").unwrap().unwrap();
    assert_eq!((p.body.as_str(), p.edited), ("고친 글", true));
}
#[test] fn settings_set_and_clear() {
    let s = Store::open_in_memory().unwrap();
    s.set_setting("monthly_goal_km", Some("300")).unwrap();
    assert_eq!(s.setting("monthly_goal_km").unwrap().as_deref(), Some("300"));
    s.set_setting("monthly_goal_km", None).unwrap();
    assert!(s.setting("monthly_goal_km").unwrap().is_none());
}
#[test] fn reopen_file_db_keeps_data() {
    let dir = tempfile::tempdir().unwrap(); let p = dir.path().join("library.db");
    Store::open(&p).unwrap().insert_activity(&activity_row("a1", Sport::Running, "2026-11-01T22:00:00Z", 5000.0)).unwrap();
    assert!(Store::open(&p).unwrap().has_activity("a1").unwrap());
}
```
(`activity_row`, `photo_row`, `post_row`, `ids`는 `tests/common.rs` 도우미.)
- [ ] **Step 2:** FAIL 확인 (`cargo test -p workout-store`).
- [ ] **Step 3:** 구현. 시각은 unix 초 INTEGER, 경로·해시태그·경로점은 JSON TEXT.
- [ ] **Step 4:** PASS 확인.
- [ ] **Step 5:** 커밋 `feat(store): SQLite library store`.

---

### Task 3: 서비스 — 가져오기 (`workout-service`)

**Files:**
- Create: `crates/service/Cargo.toml`, `crates/service/src/{lib.rs,dto.rs,import.rs}`
- Test: `crates/service/tests/import.rs`, `crates/service/tests/common.rs`

**Interfaces:**
- Consumes: `Store`(Task 2), `parse_fit`, `assign_photos`(Task 1), `simplify_route`, `card_data`, `render_card_to_file`, `TemplateWriter`.
- Produces (DTO는 모두 `#[derive(Serialize, Deserialize, TS)] #[ts(export)]`, serde `rename_all = "camelCase"`. 출력 위치는 `workout-app/.cargo/config.toml`의 `[env] TS_RS_EXPORT_DIR = { value = "ui/src/lib/bindings", relative = true }`로 정한다 — `export_to` 상대 경로는 crate 기준이라 쓰지 않음):
```rust
pub struct Library { /* root: PathBuf, store: Store */ }
#[derive(thiserror::Error, Debug)] pub enum ServiceError { Store(#[from] StoreError), Core(#[from] CoreError),
    Card(#[from] CardError), Post(#[from] PostError), Io(#[from] std::io::Error), NotFound(String) }
impl Library {
    pub fn open(root: &Path) -> Result<Library, ServiceError>;   // root/library.db, root/files/ 생성
    pub fn root(&self) -> &Path;
    pub fn import_paths(&mut self, paths: &[PathBuf]) -> Result<ImportReport, ServiceError>;
}
pub struct ImportIssue { pub file: String, pub reason: String }
pub struct PhotoDto { pub id: i64, pub file_name: String, pub path: String /* 절대 */, pub card_path: Option<String> /* 절대 */,
                      pub taken_at: Option<String> /* RFC3339 */, pub km_mark: Option<f64>, pub lat: Option<f64>, pub lon: Option<f64> }
pub struct ImportReport { pub added_activities: Vec<String> /* id */, pub duplicate_files: Vec<String> /* 파일명 */,
                          pub failed: Vec<ImportIssue>, pub cards_created: u32, pub unmatched_photos: Vec<PhotoDto> }
```
`import_paths` 동작:
1. 경로가 폴더면 그 안 파일(하위 폴더 제외), 파일이면 그대로. `.fit`과 `is_photo_input` 통과 사진만.
2. FIT: 파싱 실패 → `failed`. `has_activity(id)`면 `duplicate_files`. 새 운동은 `files/YYYYMMDD/`(현지 시작 날짜)에 원본 복사, `ActivityRow{route: simplify_route(…,1500)}` 저장, 템플릿 글을 `PostRow{edited:false}`로 저장(장소·날씨 비움).
3. 사진: 매칭 대상 = 이번 묶음의 새 운동 + `activities_near(사진 시각, 36시간)`로 찾은 **기존 라이브러리 운동** (Review Focus 2). 사진 시각이 없으면 묶음의 첫 운동 날짜 폴더, 그것도 없으면 `files/unsorted/`에 복사.
   매칭된 사진은 운동 날짜 폴더로 복사 → 카드 `<stem>_card.jpg` 생성 → `PhotoRow` 저장. 이미 같은 상대 경로가 있으면(`photo_by_path`) `duplicate_files`. 카드 실패는 `failed`에 넣고 계속.
   매칭 안 된 사진도 복사·저장하고 `unmatched_photos`에 담는다.
4. 사진이 붙은 운동은 글을 다시 만든다 — 단 `edited == true`인 글은 건드리지 않는다.

- [ ] **Step 1: 실패하는 테스트** (라이브러리는 `tempdir()/"내 라이브러리"` — 한글·공백 경로, Review Focus 5)
```rust
#[test] fn imports_folder() {
    let (_t, mut lib) = temp_library();
    let r = lib.import_paths(&[sample_path("20261025")]).unwrap();
    assert_eq!(r.added_activities.len(), 1);
    assert_eq!(r.cards_created, 2);                       // 원본 사진 2장, 기존 _card.jpg는 무시
    assert!(r.failed.is_empty() && r.unmatched_photos.is_empty());
    let day = lib.root().join("files/20261025");
    assert!(day.join("24842957203_ACTIVITY.fit").exists());
    assert!(day.join("78D7CE5C-F354-4728-8E97-063487771792_1_105_c_card.jpg").exists());
}
#[test] fn second_import_is_duplicate() {   // Review Focus 1
    let (_t, mut lib) = temp_library();
    lib.import_paths(&[sample_path("20261025")]).unwrap();
    let r = lib.import_paths(&[sample_path("20261025")]).unwrap();
    assert!(r.added_activities.is_empty());
    assert_eq!(r.cards_created, 0);
    assert_eq!(r.duplicate_files.len(), 3);   // FIT 1 + 사진 2
}
#[test] fn photos_later_match_existing_activity() {   // Review Focus 2
    let (_t, mut lib) = temp_library();
    lib.import_paths(&[sample_path("20261025/24842957203_ACTIVITY.fit")]).unwrap();
    let r = lib.import_paths(&[sample_path("20261025/0723ACAA-C18C-412E-BDF8-0CB9253CB50F_1_102_a.jpeg")]).unwrap();
    assert_eq!(r.cards_created, 1);
    assert!(r.unmatched_photos.is_empty());
}
#[test] fn unmatched_photo_is_kept_for_manual_assignment() {
    let (_t, mut lib) = temp_library();
    let r = lib.import_paths(&[sample_path("20261025/24842957203_ACTIVITY.fit"),
                               sample_path("20261011/31CCE40E-34DE-405F-ACE0-A1B5DEFC07D4_4_5005_c.jpeg")]).unwrap();
    assert_eq!(r.unmatched_photos.len(), 1);
    assert!(std::path::Path::new(&r.unmatched_photos[0].path).exists());
}
#[test] fn broken_fit_reported() {
    let (t, mut lib) = temp_library();
    let bad = t.path().join("bad.fit"); std::fs::write(&bad, b"garbage").unwrap();
    let r = lib.import_paths(&[bad]).unwrap();
    assert_eq!(r.failed.len(), 1);
}
```
- [ ] **Step 2:** FAIL 확인.
- [ ] **Step 3:** 구현.
- [ ] **Step 4:** PASS. `cargo test -p workout-service` 실행 시 `ui/src/lib/bindings/`에 `ImportReport.ts` 등이 생기는지 확인.
- [ ] **Step 5:** 커밋 `feat(service): library import`.

---

### Task 4: 서비스 — 대시보드·상세·게시 준비 조회

**Files:**
- Create: `crates/service/src/{views.rs,posts.rs}`, `dto.rs`에 DTO 추가
- Test: `crates/service/tests/views.rs`, `crates/service/tests/posts.rs`

**Interfaces:**
- Consumes: Task 2·3, `month_stats`, `km_splits`, `format::*`, `card_data`, `ManualExport`.
- Produces:
```rust
#[serde(rename_all = "lowercase")] pub enum SportKind { Running, Cycling, Walking, Hiking, Other }
#[serde(rename_all = "lowercase")] pub enum PostStatus { None, Ready, Published }
pub struct SportSummaryDto { pub count: u32, pub km: f64, pub moving_s: f64, pub avg_pace: Option<String> /* 러닝만, "5'30\"" */, pub avg_speed_kmh: Option<f64> /* 라이딩만 */ }
pub struct WeekDto { pub week_start: String /* YYYY-MM-DD */, pub running_km: f64, pub cycling_km: f64 }
pub struct DayItemDto { pub activity_id: String, pub sport: SportKind, pub km: f64, pub post_status: PostStatus }
pub struct DayDto { pub date: String, pub items: Vec<DayItemDto> }
pub struct MonthView { pub year: i32, pub month: u32, pub goal_km: Option<f64>, pub total_km: f64, pub total_moving_s: f64,
                       pub running: SportSummaryDto, pub cycling: SportSummaryDto, pub other_count: u32, pub avg_hr: Option<f64>,
                       pub weekly: Vec<WeekDto>, pub days: Vec<DayDto> /* 운동 있는 날만, 날짜순 */ }
pub struct MetricsDto { pub distance_km: f64, pub moving: String, pub elapsed: String, pub pace: Option<String>, pub speed_kmh: Option<f64>,
                        pub avg_hr: Option<u8>, pub max_hr: Option<u8>, pub ascent_m: Option<u16>, pub avg_cadence: Option<u8> }
pub struct SeriesDto { pub distance_km: Vec<f64>, pub hr: Vec<Option<u8>>, pub speed_kmh: Vec<Option<f64>>,
                       pub altitude_m: Vec<Option<f64>>, pub lat_lon: Vec<Option<[f64; 2]>> }   // 최대 1000점, 같은 길이
pub struct SplitDto { pub km: u32, pub duration: String, pub avg_hr: Option<u8> }
pub struct ActivityDetail { pub id: String, pub sport: SportKind, pub title: String /* card_data().title */, pub date: String /* YYYY.MM.DD */,
                            pub metrics: MetricsDto, pub route: Vec<[f64; 2]> /* [lat, lon] */, pub start: Option<[f64; 2]>, pub end: Option<[f64; 2]>,
                            pub photos: Vec<PhotoDto>, pub series: SeriesDto, pub splits: Vec<SplitDto> }
pub struct PlaceDto { pub name: String, pub estimated: bool }
pub struct PostView { pub activity_id: String, pub body: String, pub hashtags: Vec<String>, pub full_text: String,
                      pub place: Option<PlaceDto>, pub card_paths: Vec<String> /* 절대 */, pub folder: String /* 절대 */,
                      pub status: PostStatus, pub edited: bool }
impl Library {
    pub fn month_view(&self, year: i32, month: u32) -> Result<MonthView, ServiceError>;
    pub fn activity_detail(&self, id: &str) -> Result<ActivityDetail, ServiceError>;   // 원본 FIT를 다시 읽어 시계열 계산
    pub fn post_view(&self, activity_id: &str) -> Result<PostView, ServiceError>;
    pub fn save_post(&mut self, activity_id: &str, body: &str, hashtags: &[String]) -> Result<PostView, ServiceError>; // edited = true
    pub fn regenerate_post(&mut self, activity_id: &str) -> Result<PostView, ServiceError>; // 템플릿으로 덮어씀, edited = false
    pub fn mark_published(&mut self, activity_id: &str) -> Result<PostView, ServiceError>; // ManualExport{운동 날짜 폴더} + PublishRow + published_at
    pub fn assign_photo(&mut self, photo_id: i64, activity_id: &str) -> Result<PhotoDto, ServiceError>; // 운동 폴더로 이동, 카드 생성, km_mark None
    pub fn set_goal_km(&mut self, km: Option<f64>) -> Result<(), ServiceError>;   // settings "monthly_goal_km"
}
```
게시 상태: `published_at`이 있으면 Published, 카드가 1장 이상이고 글이 있으면 Ready, 그 외 None.
`avg_pace`는 그 달 러닝 이동시간/거리, 러닝이 없거나 거리 0이면 `None`. `avg_speed_kmh`도 같은 규칙.
시계열 줄이기: record 수가 1000을 넘으면 균등 간격으로 1000개.

- [ ] **Step 1: 실패하는 테스트**
```rust
// views.rs
#[test] fn month_view_october() {
    let (_t, mut lib) = temp_library();
    lib.import_paths(&[sample_path("20261011"), sample_path("20261025")]).unwrap();
    let v = lib.month_view(2026, 10).unwrap();
    assert_eq!(v.cycling.count, 2);
    assert!((v.total_km - 107.54).abs() < 0.05);
    assert!(v.cycling.avg_speed_kmh.unwrap() > 23.0);
    assert!(v.running.avg_pace.is_none());
    assert_eq!(v.days.len(), 2);
    assert_eq!(v.days[0].items[0].post_status, PostStatus::Ready);
    assert_eq!(v.weekly.len(), 5);
}
#[test] fn empty_month_is_clean() {   // Review Focus 4
    let (_t, lib) = temp_library();
    let v = lib.month_view(2026, 12).unwrap();
    assert_eq!(v.total_km, 0.0); assert!(v.goal_km.is_none()); assert!(v.avg_hr.is_none());
    assert!(v.running.avg_pace.is_none() && v.cycling.avg_speed_kmh.is_none());
    assert!(v.days.is_empty());
    assert!(serde_json::to_string(&v).unwrap().find("NaN").is_none());
}
#[test] fn goal_is_reported() {
    let (_t, mut lib) = temp_library();
    lib.set_goal_km(Some(300.0)).unwrap();
    assert_eq!(lib.month_view(2026, 10).unwrap().goal_km, Some(300.0));
}
#[test] fn activity_detail_ride() {
    let (_t, mut lib) = temp_library();
    let id = lib.import_paths(&[sample_path("20261025")]).unwrap().added_activities[0].clone();
    let d = lib.activity_detail(&id).unwrap();
    assert_eq!(d.title, "일요일 오후 라이딩");
    assert_eq!(d.metrics.moving, "1:58:22");
    assert!(d.metrics.pace.is_none() && d.metrics.speed_kmh.is_some());
    assert!(d.route.len() <= 1500 && d.start.is_some() && d.end.is_some());
    assert_eq!(d.series.distance_km.len(), 1000);
    assert_eq!(d.series.hr.len(), 1000);
    assert_eq!(d.splits.len(), 49);
    assert_eq!(d.photos.len(), 2);
}
#[test] fn unknown_activity_is_not_found() {
    let (_t, lib) = temp_library();
    assert!(matches!(lib.activity_detail("nope"), Err(ServiceError::NotFound(_))));
}
// posts.rs
#[test] fn edited_post_survives_reopen() {   // Review Focus 3
    let (_t, mut lib) = temp_library();
    let id = lib.import_paths(&[sample_path("20261025")]).unwrap().added_activities[0].clone();
    lib.save_post(&id, "직접 쓴 글", &["#라이딩".into()]).unwrap();
    lib.import_paths(&[sample_path("20261025")]).unwrap();     // 재가져오기도 덮어쓰지 않음
    let p = lib.post_view(&id).unwrap();
    assert_eq!(p.body, "직접 쓴 글"); assert!(p.edited);
    assert_eq!(p.full_text, "직접 쓴 글\n\n#라이딩");
    let r = lib.regenerate_post(&id).unwrap();
    assert!(r.body.starts_with("일요일 오후 48km 🚲")); assert!(!r.edited);
}
#[test] fn mark_published_exports_and_records() {
    let (_t, mut lib) = temp_library();
    let id = lib.import_paths(&[sample_path("20261025")]).unwrap().added_activities[0].clone();
    let p = lib.mark_published(&id).unwrap();
    assert_eq!(p.status, PostStatus::Published);
    assert!(std::path::Path::new(&p.folder).join("post.txt").exists());
    assert_eq!(lib.month_view(2026, 10).unwrap().days[0].items[0].post_status, PostStatus::Published);
}
#[test] fn assign_unmatched_photo_creates_card() {
    let (_t, mut lib) = temp_library();
    let r = lib.import_paths(&[sample_path("20261025/24842957203_ACTIVITY.fit"),
                               sample_path("20261011/31CCE40E-34DE-405F-ACE0-A1B5DEFC07D4_4_5005_c.jpeg")]).unwrap();
    let ph = lib.assign_photo(r.unmatched_photos[0].id, &r.added_activities[0]).unwrap();
    assert!(std::path::Path::new(ph.card_path.as_ref().unwrap()).exists());
    assert_eq!(lib.post_view(&r.added_activities[0]).unwrap().card_paths.len(), 1);
}
```
- [ ] **Step 2:** FAIL 확인.
- [ ] **Step 3:** 구현.
- [ ] **Step 4:** `cargo test --workspace` PASS, `ui/src/lib/bindings/`에 새 타입 생성 확인.
- [ ] **Step 5:** 커밋 `feat(service): month view, activity detail, post workflow`.

---

### Task 5: Tauri 껍데기와 UI 뼈대

**Files:**
- Create: `src-tauri/**`, `ui/package.json`, `ui/vite.config.ts`, `ui/tsconfig.json`, `ui/index.html`, `ui/src/main.ts`, `ui/src/App.svelte`, `ui/src/lib/api.ts`, `ui/src/lib/mock.ts`
- Modify: `Cargo.toml`(members), `.gitignore`(`ui/node_modules/`, `ui/dist/`)
- Test: `ui/src/lib/api.test.ts`

**Interfaces:**
- Consumes: `Library` 전체 메서드.
- Produces — Tauri 명령 (이름은 snake_case, 인자는 camelCase로 받음):
  `import_paths(paths: Vec<String>) -> ImportReport`, `month_view(year, month) -> MonthView`, `activity_detail(id) -> ActivityDetail`, `post_view(activityId)`, `save_post(activityId, body, hashtags)`, `regenerate_post(activityId)`, `mark_published(activityId)` → `PostView`, `assign_photo(photoId, activityId) -> PhotoDto`, `set_goal_km(km: Option<f64>)`. 오류는 `String`(한국어 메시지)으로 변환.
  상태: `tauri::State<Mutex<Library>>`, `setup`에서 `app.path().app_data_dir()`로 `Library::open`.
- UI `api.ts`:
```ts
export interface Api { importPaths(paths: string[]): Promise<ImportReport>; monthView(y: number, m: number): Promise<MonthView>;
  activityDetail(id: string): Promise<ActivityDetail>; postView(id: string): Promise<PostView>;
  savePost(id: string, body: string, hashtags: string[]): Promise<PostView>; regeneratePost(id: string): Promise<PostView>;
  markPublished(id: string): Promise<PostView>; assignPhoto(photoId: number, activityId: string): Promise<PhotoDto>;
  setGoalKm(km: number | null): Promise<void>; fileUrl(absPath: string): string;
  copyText(text: string): Promise<void>; openFolder(absPath: string): Promise<void>; pickImport(): Promise<string[]> }
export const api: Api = isTauri() ? tauriApi : mockApi;   // isTauri = '__TAURI_INTERNALS__' in window
```
`tauriApi.fileUrl` = `convertFileSrc`. `mockApi`는 `mock.ts`의 합성 데이터(여의도 근처 37.52,126.93을 중심으로 한 합성 원형 경로 — **사용자 실제 좌표 사용 금지**)를 돌려준다.
- `tauri.conf.json`: `build.frontendDist = "../ui/dist"`, `devUrl = "http://localhost:5173"`, `beforeDevCommand = "npm --prefix ../ui run dev"`, `beforeBuildCommand = "npm --prefix ../ui run build"`. `app.security.assetProtocol = { enable: true, scope: ["$APPDATA/**"] }`. CSP: `default-src 'self'; img-src 'self' asset: http://asset.localhost https://tile.openstreetmap.org data:; style-src 'self' 'unsafe-inline'`.
- `capabilities/default.json`: `core:default`, `dialog:allow-open`, `clipboard-manager:allow-write-text`, `opener:allow-open-path`(범위 `$APPDATA/**`).
- 창 파일 끌어다 놓기: `getCurrentWebview().onDragDropEvent`로 경로를 받아 `importPaths`.

- [ ] **Step 1:** `ui/` 만들기: `npm create vite@latest ui -- --template svelte-ts`, `npm i @tauri-apps/api leaflet chart.js @tauri-apps/plugin-dialog @tauri-apps/plugin-clipboard-manager @tauri-apps/plugin-opener`, `npm i -D @tauri-apps/cli vitest @types/leaflet`. `src-tauri`는 `npx tauri init` 대신 위 설정으로 직접 작성 (workspace 멤버 이름 `workout-desktop`).
- [ ] **Step 2: 실패하는 테스트** `api.test.ts`:
```ts
import { describe, it, expect } from 'vitest'
import { mockApi } from './api'
describe('mockApi', () => {
  it('returns a month with days', async () => { const v = await mockApi.monthView(2026, 10); expect(v.days.length).toBeGreaterThan(0) })
  it('detail has route and same-length series', async () => {
    const d = await mockApi.activityDetail((await mockApi.monthView(2026, 10)).days[0].items[0].activityId)
    expect(d.route.length).toBeGreaterThan(10); expect(d.series.hr.length).toBe(d.series.distanceKm.length) })
  it('savePost marks edited', async () => { expect((await mockApi.savePost('m1', '글', [])).edited).toBe(true) })
})
```
- [ ] **Step 3:** `npx vitest run` → FAIL (api.ts 없음).
- [ ] **Step 4:** `api.ts`, `mock.ts`, `src-tauri` 구현. `App.svelte`는 "운동 기록" 제목과 현재 달 `monthView` JSON 길이만 표시하는 임시 화면.
- [ ] **Step 5:** `npx vitest run` PASS, `cargo build -p workout-desktop` 성공, `npm --prefix ui run build` 성공.
- [ ] **Step 6: 실제 실행.** `cd src-tauri && npx --prefix ../ui tauri dev` 실행 → 창이 뜨고 오류 없이 임시 화면 표시 (로그에 panic 없음). 확인 후 종료.
- [ ] **Step 7:** 커밋 `feat(desktop): tauri shell and ui skeleton`.

---

### Task 6: 대시보드와 달력 화면

**Files:**
- Create: `ui/src/lib/format.ts`, `ui/src/lib/calendar.ts`, `ui/src/components/{Gauge,WeekBars,Calendar,Dashboard}.svelte`
- Modify: `ui/src/App.svelte`
- Test: `ui/src/lib/format.test.ts`, `ui/src/lib/calendar.test.ts`

**Interfaces:**
- Consumes: `api.monthView`, `api.setGoalKm`, `api.importPaths`.
- Produces:
```ts
export function km(n: number): string            // 107.54 → "107.5", 0 → "0"
export function duration(sec: number): string    // 9052 → "2시간 30분", 1650 → "27분", 0 → "0분"
export function gaugeRatio(value: number, goal: number | null): number   // 0..1, goal null/0 → 0
export type CalCell = { date: string; inMonth: boolean; items: DayItemDto[] }
export function buildMonthGrid(year: number, month: number, days: DayDto[]): CalCell[][]   // 월요일 시작 주 배열, 각 주 7칸
```
화면 (설계 §6.2 스케치): 위 4개 게이지 카드 [총 거리(목표 대비, 목표 없으면 원 전체를 회색으로 두고 숫자만) · 총 시간 · 러닝 횟수 · 라이딩 횟수], 포커스 카드 2개 [러닝: 주별 막대 + 평균 페이스 / 라이딩: 주별 막대 + 평균 속도], 달력(날짜 칸에 🏃/🚲/👟 + km, 상태 점: 회색 준비 안 됨, 파랑 준비됨, 초록 게시함, 칸 클릭 → 그날 첫 운동 상세), 아래 요약 목록(러닝 n회 · 라이딩 n회 · 평균 심박). 상단 바: ◀ 2026년 11월 ▶, "가져오기" 버튼, 목표 km 입력.

- [ ] **Step 0:** `frontend-design` 스킬 불러오기.
- [ ] **Step 1: 실패하는 테스트**
```ts
it('km', () => { expect(km(107.54)).toBe('107.5'); expect(km(0)).toBe('0') })
it('duration', () => { expect(duration(9052)).toBe('2시간 30분'); expect(duration(1650)).toBe('27분'); expect(duration(0)).toBe('0분') })
it('gauge without goal', () => { expect(gaugeRatio(50, null)).toBe(0); expect(gaugeRatio(50, 0)).toBe(0); expect(gaugeRatio(450, 300)).toBe(1) })
it('november 2026 grid', () => {
  const g = buildMonthGrid(2026, 11, [{ date: '2026-11-02', items: [] }])
  expect(g.length).toBe(6); expect(g[0][0].date).toBe('2026-10-26'); expect(g[0][0].inMonth).toBe(false)
  expect(g[5][0].date).toBe('2026-11-30'); expect(g.flat().find(c => c.date === '2026-11-02')!.inMonth).toBe(true)
})
it('october 2026 grid has 5 weeks', () => { expect(buildMonthGrid(2026, 10, []).length).toBe(5) })
```
- [ ] **Step 2:** `npx vitest run` FAIL.
- [ ] **Step 3:** 구현.
- [ ] **Step 4:** vitest PASS.
- [ ] **Step 5: 브라우저 확인.** `npm --prefix ui run dev` 후 내장 브라우저로 `http://localhost:5173` 열기 → mock 데이터로 게이지·막대·달력이 보이는지 스크린샷. 데이터 없는 달(◀ 여러 번)로 가서 빈 상태가 깨지지 않는지 확인 (Review Focus 4).
- [ ] **Step 6:** 커밋 `feat(ui): monthly dashboard and calendar`.

---

### Task 7: 운동 상세 화면 (2D 지도 · 그래프 · 구간표)

**Files:**
- Create: `ui/src/lib/series.ts`, `ui/src/components/{RouteMap,MetricCharts,SplitsTable,ActivityDetail}.svelte`
- Test: `ui/src/lib/series.test.ts`

**Interfaces:**
- Consumes: `api.activityDetail`, `api.fileUrl`.
- Produces:
```ts
export type ChartSeries = { x: number[]; hr: (number | null)[]; secondary: (number | null)[]; secondaryLabel: string; altitude: (number | null)[] }
export function toChartSeries(d: ActivityDetail): ChartSeries   // 러닝: secondary = 페이스(분/km, 속도 0이면 null), 라이딩·기타: 속도 km/h
export function nearestIndex(xs: number[], x: number): number   // 이진 탐색, 빈 배열이면 -1
```
`RouteMap`: Leaflet 지도, `L.polyline(route)` 파란 선, 시작 `L.circleMarker` 초록·종료 빨강(기본 마커 아이콘 쓰지 않음 — 번들러 아이콘 경로 문제 회피), 사진 위치는 작은 원 + 클릭 시 썸네일 팝업(`api.fileUrl`), `fitBounds`. 경로가 비면 지도 대신 "GPS 기록 없음" 안내. 외부에서 `highlight(index)`를 받으면 `series.latLon[index]`에 점 표시.
`MetricCharts`: Chart.js 선 그래프 3개(심박·페이스/속도·고도, x = 거리 km), 마우스 이동 시 `nearestIndex` 결과를 상위로 알림 → 지도 점 이동.
`ActivityDetail`: 제목·날짜, 지표 카드(거리·이동시간·페이스/속도·평균/최대 심박·상승·케이던스 — 없는 값은 칸 생략), 지도, 그래프, 구간표, "게시 준비" 버튼, "← 대시보드".

- [ ] **Step 0:** `frontend-design` 스킬 불러오기 (Task 6과 같은 시각 언어 유지).
- [ ] **Step 1: 실패하는 테스트**
```ts
it('running uses pace minutes', () => {
  const s = toChartSeries(detail('running', { speedKmh: [12, 0, null] }))
  expect(s.secondaryLabel).toBe('페이스 (분/km)'); expect(s.secondary).toEqual([5, null, null]) })
it('cycling uses speed', () => {
  const s = toChartSeries(detail('cycling', { speedKmh: [24.4] })); expect(s.secondaryLabel).toBe('속도 (km/h)'); expect(s.secondary).toEqual([24.4]) })
it('nearestIndex', () => { expect(nearestIndex([0, 1, 2, 3], 1.6)).toBe(2); expect(nearestIndex([], 1)).toBe(-1) })
```
(`detail()`은 테스트 안의 최소 ActivityDetail 생성 도우미.)
- [ ] **Step 2:** FAIL 확인.
- [ ] **Step 3:** 구현.
- [ ] **Step 4:** vitest PASS.
- [ ] **Step 5: 브라우저 확인.** mock 상세 화면에서 지도 타일·경로·시작/종료 핀·그래프가 보이고, 그래프 위 마우스 이동 시 지도 점이 따라 움직이는지 확인. GPS 없는 mock 운동(러닝)에서 "GPS 기록 없음"이 뜨는지 확인.
- [ ] **Step 6:** 커밋 `feat(ui): activity detail with map and charts`.

---

### Task 8: 게시 준비 화면과 가져오기 흐름

**Files:**
- Create: `ui/src/components/{PostPanel,ImportPanel}.svelte`
- Modify: `ui/src/App.svelte`, `ui/src/lib/format.ts`
- Test: `ui/src/lib/format.test.ts` (추가)

**Interfaces:**
- Consumes: `api.postView/savePost/regeneratePost/markPublished/copyText/openFolder/fileUrl/pickImport/importPaths/assignPhoto`.
- Produces:
```ts
export function parseHashtags(input: string): string[]   // "#라이딩 자전거  #한강" → ["#라이딩", "#자전거", "#한강"], 중복 제거
export function importSummary(r: ImportReport): string   // "운동 1개 추가 · 카드 2장 · 이미 있음 3 · 실패 0"
```
`PostPanel`: 장소(추정이면 "추정" 배지 — 3단계 전까지는 비어 있음), 본문 textarea, 해시태그 입력, [복사](full_text → 클립보드, "복사됨" 표시) [저장] [템플릿으로 다시 쓰기](확인 대화) [카드 폴더 열기] [게시 완료] 버튼, 카드 썸네일 줄. 저장하지 않은 변경이 있으면 화면을 떠날 때 확인.
`ImportPanel`: "가져오기" 버튼(`pickImport` — 파일·폴더 다중 선택) 또는 창에 끌어다 놓기 → 진행 표시 → `importSummary` + 실패 목록 + 매칭 안 된 사진 목록(썸네일 + 그 달 운동 선택 드롭다운 → `assignPhoto`).

- [ ] **Step 0:** `frontend-design` 스킬 불러오기.
- [ ] **Step 1: 실패하는 테스트**
```ts
it('parseHashtags', () => { expect(parseHashtags('#라이딩 자전거  #한강 #라이딩')).toEqual(['#라이딩', '#자전거', '#한강']); expect(parseHashtags('  ')).toEqual([]) })
it('importSummary', () => { expect(importSummary({ addedActivities: ['a'], duplicateFiles: ['x', 'y', 'z'], failed: [], cardsCreated: 2, unmatchedPhotos: [] }))
  .toBe('운동 1개 추가 · 카드 2장 · 이미 있음 3 · 실패 0') })
```
- [ ] **Step 2:** FAIL 확인.
- [ ] **Step 3:** 구현.
- [ ] **Step 4:** vitest PASS, `cargo test --workspace` PASS, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`, `npm --prefix ui run build` 통과.
- [ ] **Step 5: 브라우저 확인 (mock).** 게시 준비 화면에서 글 수정 → 저장 → 다른 화면 갔다 오기 → 수정 유지 확인.
- [ ] **Step 6: 실제 앱 확인.** `tauri dev`로 앱 실행 → 가져오기로 `sns_create/20261011`, `sns_create/20261025` 선택(사용자 폴더는 복사만 됨) → 10월 대시보드에 라이딩 2회·107.5km, 달력 11일·25일 표시 → 25일 상세에서 지도·그래프 → 게시 준비에서 복사·카드 폴더 열기 동작. 같은 폴더를 다시 가져와 "이미 있음 3"이 뜨는지 확인. 스크린샷을 사용자에게 보여 준다.
- [ ] **Step 7:** 커밋 `feat(ui): post preparation and import flow`.

---

## 완료 기준

- `cargo test --workspace`, `npx vitest run`, clippy, fmt, `npm run build` 모두 통과.
- 실제 앱에서 Task 8 Step 6 시나리오가 끝까지 동작.
- 사용자 원본 폴더(`sns_create/2026*`)에 새 파일이 생기지 않음 (`git status`와 무관하므로 `ls`로 확인).
- Windows·Linux 빌드는 4단계 CI에서 확인.
