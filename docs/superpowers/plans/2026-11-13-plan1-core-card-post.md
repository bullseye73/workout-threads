# 1단계: Rust 코어 · 카드 · 템플릿 글 · CLI 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** FIT + 사진 폴더를 받아 카드 이미지와 초안 A 템플릿 글을 만드는 크로스 플랫폼 Rust 라이브러리와 CLI를 만든다 (`tools/fit_card.py` + `card.swift` 대체).

**Architecture:** Cargo workspace. `workout-core`(모델·FIT·EXIF·매칭·통계·공용 포맷), `workout-card`(카드 렌더링), `workout-post`(템플릿 글·Publisher), `workout-cli`(묶음). 앞의 셋은 UI·네트워크 의존이 없어 2단계 Tauri 앱이 그대로 재사용한다.

**Tech Stack:** Rust stable (2021 edition), `fitparser`, `kamadak-exif`, `chrono`, `thiserror`, `image`, `tiny-skia`, `ab_glyph`, `clap`. 버전은 실행 시점에 `cargo add`로 최신 안정판.

**Spec:** `docs/superpowers/specs/2026-11-13-workout-threads-app-design.md` (로드맵: `docs/superpowers/plans/2026-11-13-roadmap.md`)

## Global Constraints

- mac / Windows / Linux 공통 동작. OS 전용 API(AppKit, Swift 등) 금지.
- 저장소 위치: `sns_create/workout-app/` (운동 날짜 폴더와 같은 상위 폴더 안) (새 git 저장소). 날짜 폴더(`2026*/`)의 사진·FIT는 저장소에 넣지 않는다 — 테스트는 경로로 참조.
- 지원 종목 코드: 러닝 1, 라이딩 2, 걷기 11, 하이킹 17. 그 외는 `Other`.
- 글: 반말체, 출발·도착 지점 이름 금지, 좌표 금지.
- 카드: 긴 변 최소 2160px, JPEG 품질 92, EXIF 회전 반영, 결과 파일에 EXIF(특히 GPS) 없음.
- 사진↔운동 매칭 창: 운동 시작 −10분 ~ 종료 +10분.
- 사진 확장자: `.jpg .jpeg .png` (대소문자 무시). 파일명이 `_card`로 끝나면 입력에서 제외.
- 화면용 경로는 최대 1,500점으로 줄인다.
- 한글 폰트: Pretendard (OFL), 바이너리에 내장.

## Review Focus

1. **EXIF 시각에 시간대가 없는 사진** — 대부분의 EXIF `DateTimeOriginal`은 현지 시각만 있다. `OffsetTimeOriginal`이 있으면 그것, 없으면 운동의 UTC 오프셋으로 해석해야 매칭이 9시간 어긋나지 않는다. → Task 3 테스트.
2. **GPS 없는 FIT (실내 러닝·트레드밀)** — 파싱 성공, 경로 빈 배열, 카드는 경로 선 없이 그려지고 패닉 없음. → Task 2, Task 5 테스트.
3. **한 폴더에 FIT 여러 개 (오전 러닝 + 오후 라이딩)** — 사진은 시간이 맞는 운동에만 붙고 글도 운동별로 따로. → Task 3, Task 7 테스트.
4. **어느 운동 시간에도 안 맞는 사진** — 카드를 만들지 않고 경고만 출력, 나머지는 계속. → Task 7 테스트.
5. **잘리거나 깨진 FIT 파일** — 패닉 없이 `CoreError::Fit` 반환, CLI는 그 파일만 건너뛰고 계속. → Task 2, Task 7 테스트.

---

## 파일 구조

```
workout-app/
├─ Cargo.toml                      workspace
├─ .gitignore                      target/
├─ crates/
│  ├─ core/src/
│  │  ├─ lib.rs                    pub mod 재노출, CoreError
│  │  ├─ model.rs                  Sport, LatLon, Sample, Summary, ActivityHeader, Activity
│  │  ├─ fit.rs                    parse_fit, activity_from_records
│  │  ├─ photo.rs                  PhotoMeta, read_photo_meta, resolve_taken_at
│  │  ├─ matching.rs               match_photo, PhotoMatch
│  │  ├─ route.rs                  haversine_m, simplify_route
│  │  ├─ splits.rs                 km_splits
│  │  ├─ stats.rs                  month_stats, MonthStats …
│  │  └─ format.rs                 hms, pace, weekday_ko, time_of_day_ko, sport_ko
│  ├─ card/
│  │  ├─ assets/fonts/             Pretendard-Medium/SemiBold/Bold.otf, OFL.txt
│  │  └─ src/ lib.rs, data.rs (CardData), render.rs (render_card), text.rs (글자 그리기)
│  ├─ post/src/
│  │  ├─ lib.rs
│  │  ├─ context.rs                PostContext, PlaceText, WeatherText, PostText
│  │  ├─ template.rs               DraftWriter, TemplateWriter
│  │  └─ publish.rs                PostDraft, Publisher, ManualExport, PublishReceipt
│  └─ cli/src/main.rs              workout-cli prepare <폴더>
└─ tests 는 각 crate의 tests/ 폴더
```

테스트용 실제 데이터 경로 상수 (각 crate `tests/common.rs`에 둠):
`concat!(env!("CARGO_MANIFEST_DIR"), "/../../../20261011")`, `.../20261025`.

기준값 (기존 Python 결과로 확인함):

| 폴더 | FIT | 시작(KST) | 종목 | 거리 | 이동시간 | 평속 | 평균심박 | 경로점 |
|---|---|---|---|---|---|---|---|---|
| 20261011 | `24680396566_ACTIVITY.fit` | 2026-10-11 09:25:57 | 라이딩 | 59.38 km (5938.106 m×10) | 2:30:52 | 23.6 km/h | 147 | 9071 기록 |
| 20261025 | `24842957203_ACTIVITY.fit` | 2026-10-25 14:41:29 | 라이딩 | 48.16 km | 1:58:22 | 24.4 km/h | 146 | 7135 기록 |

사진 촬영 시각(UTC): `20261011/31CCE40E-…_c.jpeg` 01:42:30, `20261025/0723ACAA-…_a.jpeg` 06:39:56, `20261025/78D7CE5C-…_c.jpeg` 06:24:53.

---

### Task 0: 개발 환경과 workspace

**Files:**
- Create: `workout-app/Cargo.toml`, `workout-app/.gitignore`, `workout-app/crates/{core,card,post,cli}/Cargo.toml` + 빈 `lib.rs`/`main.rs`

**Interfaces:**
- Produces: crate 이름 `workout-core`, `workout-card`, `workout-post`, `workout-cli`(바이너리 `workout-cli`).

- [ ] **Step 1:** Rust가 없으면 설치 (현재 이 Mac에 `rustc` 없음). 시스템 변경이므로 사용자에게 먼저 확인.
  Run: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y` → `rustc --version`이 버전 출력.
- [ ] **Step 2:** workspace와 4개 crate 생성, `cargo build` 성공 확인.
- [ ] **Step 3:** `git init` (workout-app 안에서), `.gitignore`에 `target/`. 커밋 `chore: cargo workspace`.

---

### Task 1: 모델과 공용 포맷 (`workout-core`)

**Files:**
- Create: `crates/core/src/model.rs`, `crates/core/src/format.rs`, `crates/core/src/lib.rs`
- Test: `crates/core/tests/format.rs`

**Interfaces:**
- Produces:
```rust
pub enum Sport { Running, Cycling, Walking, Hiking, Other(u8) }
impl Sport { pub fn from_fit(code: u8) -> Sport }
#[derive(Clone, Copy, PartialEq, Debug)] pub struct LatLon { pub lat: f64, pub lon: f64 }
pub struct Sample { pub time: DateTime<Utc>, pub pos: Option<LatLon>, pub distance_m: Option<f64>,
                    pub heart_rate: Option<u8>, pub speed_mps: Option<f64>,
                    pub altitude_m: Option<f64>, pub cadence: Option<u8> }
pub struct Summary { pub distance_m: f64, pub moving_s: f64, pub elapsed_s: f64,
                     pub avg_hr: Option<u8>, pub max_hr: Option<u8>, pub avg_speed_mps: f64,
                     pub ascent_m: Option<u16>, pub avg_cadence: Option<u8> }
pub struct ActivityHeader { pub id: String, pub sport: Sport, pub start: DateTime<Utc>,
                            pub utc_offset_s: i32, pub summary: Summary }
impl ActivityHeader { pub fn start_local(&self) -> NaiveDateTime; pub fn end(&self) -> DateTime<Utc> /* start + elapsed */ }
pub struct Activity { pub header: ActivityHeader, pub samples: Vec<Sample> }
#[derive(thiserror::Error, Debug)] pub enum CoreError { Fit(String), Photo(String), Io(#[from] std::io::Error) }
// format.rs
pub fn hms(seconds: f64) -> String          // 9052.377 → "2:30:52", 593.0 → "9:53"
pub fn pace_per_km(moving_s: f64, distance_m: f64) -> String   // 1800, 6000 → "5'00\""
pub fn speed_kmh(moving_s: f64, distance_m: f64) -> f64
pub fn weekday_ko(d: NaiveDate) -> &'static str  // "일요일"
pub fn time_of_day_ko(hour: u32) -> &'static str // <5 새벽, <8 이른아침, <12 오전, <17 오후, <20 저녁, 그 외 밤
pub fn sport_ko(s: Sport) -> &'static str        // 러닝, 라이딩, 걷기, 하이킹, 운동
```
`id` 형식: `"{device_serial}-{start_unix}"` (시리얼이 없으면 `"0-{start_unix}"`).

- [ ] **Step 1: 실패하는 테스트** `format.rs`:
```rust
#[test] fn hms_formats() { assert_eq!(hms(9052.377), "2:30:52"); assert_eq!(hms(593.0), "9:53"); }
#[test] fn pace_formats() { assert_eq!(pace_per_km(1800.0, 6000.0), "5'00\""); }
#[test] fn speed() { assert!((speed_kmh(9052.377, 59381.06) - 23.6).abs() < 0.05); }
#[test] fn time_of_day_bounds() { assert_eq!(time_of_day_ko(4), "새벽"); assert_eq!(time_of_day_ko(9), "오전");
    assert_eq!(time_of_day_ko(14), "오후"); assert_eq!(time_of_day_ko(20), "밤"); }
#[test] fn weekday() { assert_eq!(weekday_ko(NaiveDate::from_ymd_opt(2026,10,11).unwrap()), "일요일"); }
#[test] fn sport_codes() { assert_eq!(Sport::from_fit(1), Sport::Running); assert_eq!(Sport::from_fit(2), Sport::Cycling);
    assert_eq!(Sport::from_fit(5), Sport::Other(5)); }
```
- [ ] **Step 2:** `cargo test -p workout-core` → 컴파일 실패(미정의).
- [ ] **Step 3:** 위 시그니처 구현.
- [ ] **Step 4:** `cargo test -p workout-core` → PASS.
- [ ] **Step 5:** 커밋 `feat(core): model and formatting helpers`.

---

### Task 2: FIT 해석

**Files:**
- Create: `crates/core/src/fit.rs`
- Test: `crates/core/tests/fit.rs`, `crates/core/tests/common.rs`

**Interfaces:**
- Consumes: Task 1 모델.
- Produces:
```rust
pub fn parse_fit(bytes: &[u8]) -> Result<Activity, CoreError>
/// fitparser와 무관한 중간 표현 — 테스트에서 합성 데이터로 엣지 케이스를 검증하기 위함
pub struct RawSession { pub sport: u8, pub start: DateTime<Utc>, pub distance_m: f64, pub timer_s: f64,
                        pub elapsed_s: f64, pub avg_hr: Option<u8>, pub max_hr: Option<u8>,
                        pub avg_speed_mps: Option<f64>, pub ascent_m: Option<u16>, pub avg_cadence: Option<u8> }
pub fn activity_from_records(session: RawSession, samples: Vec<Sample>, serial: Option<u32>,
                             local_offset_s: Option<i32>) -> Activity
```
`parse_fit`: `fitparser::from_bytes`로 메시지를 읽어 `file_id.serial_number`, `session`, `record`, `activity.local_timestamp`를 뽑고 `activity_from_records`에 넘긴다. 좌표는 semicircle → 도(`v * 180 / 2^31`). `local_offset_s = local_timestamp − timestamp`(activity 메시지), 없으면 `None` → 시스템 시간대 오프셋. `avg_speed_mps`가 없으면 `distance_m / timer_s`. fitparser 오류는 `CoreError::Fit(메시지)`로 감싸고 절대 패닉하지 않는다.

- [ ] **Step 1: 실패하는 테스트**
```rust
#[test] fn parses_ride_20261011() {
    let a = parse_fit(&std::fs::read(fit_20261011()).unwrap()).unwrap();
    assert_eq!(a.header.sport, Sport::Cycling);
    assert_eq!(a.header.utc_offset_s, 9 * 3600);
    assert_eq!(a.header.start_local().to_string(), "2026-10-11 09:25:57");
    assert!((a.header.summary.distance_m - 59381.06).abs() < 1.0);
    assert_eq!(hms(a.header.summary.moving_s), "2:30:52");
    assert_eq!(a.header.summary.avg_hr, Some(147));
    assert_eq!(a.samples.len(), 9071);
    assert!(a.samples.iter().all(|s| s.pos.is_some()));
    assert!(a.header.id.ends_with(&format!("-{}", a.header.start.timestamp())));
}
#[test] fn truncated_fit_is_error_not_panic() {
    let b = std::fs::read(fit_20261011()).unwrap();
    assert!(matches!(parse_fit(&b[..b.len() / 3]), Err(CoreError::Fit(_))));
    assert!(matches!(parse_fit(b"not a fit"), Err(CoreError::Fit(_))));
}
#[test] fn no_gps_activity_has_empty_positions() {   // Review Focus 2
    let a = activity_from_records(raw_session(1 /*running*/), samples_without_pos(600), None, Some(32400));
    assert_eq!(a.header.sport, Sport::Running);
    assert!(a.samples.iter().all(|s| s.pos.is_none()));
    assert_eq!(a.header.id, format!("0-{}", a.header.start.timestamp()));
}
```
(`raw_session`, `samples_without_pos`는 `common.rs`의 테스트 도우미.)
- [ ] **Step 2:** `cargo test -p workout-core --test fit` → FAIL.
- [ ] **Step 3:** 구현.
- [ ] **Step 4:** PASS 확인. 20261025 파일로도 같은 형태 테스트 1개 추가(48.16 km, 1:58:22, 146, 7135점) 후 PASS.
- [ ] **Step 5:** 커밋 `feat(core): FIT parsing`.

---

### Task 3: 사진 메타데이터와 사진↔운동 매칭

**Files:**
- Create: `crates/core/src/photo.rs`, `crates/core/src/matching.rs`
- Test: `crates/core/tests/photo.rs`

**Interfaces:**
- Consumes: `Activity`, `ActivityHeader::end()`.
- Produces:
```rust
pub struct PhotoMeta { pub path: PathBuf, pub taken_at: Option<DateTime<Utc>>, pub pos: Option<LatLon>,
                       pub orientation: u16 /* EXIF 1..8, 없으면 1 */ }
/// fallback_offset_s: EXIF에 오프셋 태그가 없을 때 쓸 UTC 오프셋 (보통 운동의 utc_offset_s)
pub fn read_photo_meta(path: &Path, fallback_offset_s: i32) -> Result<PhotoMeta, CoreError>
pub fn resolve_taken_at(local: NaiveDateTime, offset_tag: Option<&str> /* "+09:00" */, fallback_offset_s: i32) -> DateTime<Utc>
pub fn is_photo_input(path: &Path) -> bool   // 확장자 jpg/jpeg/png(대소문자 무시) && stem이 "_card"로 끝나지 않음
pub struct PhotoMatch { pub activity_id: String, pub km_mark: f64 }
pub fn match_photo(activities: &[Activity], taken_at: DateTime<Utc>) -> Option<PhotoMatch>
```
`match_photo`: 시작−10분 ≤ taken_at ≤ 종료+10분인 첫 운동. `km_mark`는 시각이 가장 가까운 sample의 `distance_m / 1000`(시작 전 10분 구간이면 0, 종료 후 10분 구간이면 마지막 sample의 거리), 소수 1자리 반올림. EXIF 읽기 실패나 EXIF 없음은 오류가 아니라 `taken_at: None`.

- [ ] **Step 1: 실패하는 테스트**
```rust
#[test] fn offset_tag_wins() {   // Review Focus 1
    let t = resolve_taken_at(naive("2026-10-25 15:39:56"), Some("+09:00"), 0);
    assert_eq!(t.to_rfc3339(), "2026-10-25T06:39:56+00:00");
}
#[test] fn fallback_offset_when_no_tag() {
    let t = resolve_taken_at(naive("2026-10-25 15:39:56"), None, 9 * 3600);
    assert_eq!(t.to_rfc3339(), "2026-10-25T06:39:56+00:00");
}
#[test] fn reads_real_photo() {
    let m = read_photo_meta(&photo("20261025/0723ACAA-C18C-412E-BDF8-0CB9253CB50F_1_102_a.jpeg"), 9*3600).unwrap();
    assert_eq!(m.taken_at.unwrap().to_rfc3339(), "2026-10-25T06:39:56+00:00");
    assert!(m.pos.is_some());   // GPS가 읽힘 (좌표 값은 공개 저장소에 적지 않음)
}
#[test] fn filters_inputs() {
    assert!(is_photo_input(Path::new("a.JPEG"))); assert!(!is_photo_input(Path::new("a_card.jpg")));
    assert!(!is_photo_input(Path::new("x.fit")));
}
#[test] fn matches_photo_to_right_activity() {   // Review Focus 3
    let ride = parse_fit(&read(fit_20261025())).unwrap();
    let run = synthetic_run_on("2026-10-25T00:00:00Z", 1800);   // 같은 날 오전 러닝
    let m = match_photo(&[run, ride.clone()], utc("2026-10-25T06:39:56Z")).unwrap();
    assert_eq!(m.activity_id, ride.header.id);
    assert!(m.km_mark > 0.0 && m.km_mark < 48.2);
}
#[test] fn unmatched_photo_is_none() {   // Review Focus 4
    let ride = parse_fit(&read(fit_20261025())).unwrap();
    assert!(match_photo(&[ride], utc("2026-10-25T12:00:00Z")).is_none());
}
```
- [ ] **Step 2:** FAIL 확인.
- [ ] **Step 3:** `kamadak-exif`로 `DateTimeOriginal`, `OffsetTimeOriginal`, GPS, `Orientation` 읽어 구현.
- [ ] **Step 4:** PASS 확인. `reads_real_photo` 시각이 다르면 `exiftool`이나 macOS `mdls`로 원본 EXIF 값을 확인하고 기대값을 고친 이유를 커밋 메시지에 남긴다.
- [ ] **Step 5:** 커밋 `feat(core): photo EXIF and activity matching`.

---

### Task 4: 경로 줄이기, km 구간, 월간 통계

**Files:**
- Create: `crates/core/src/route.rs`, `crates/core/src/splits.rs`, `crates/core/src/stats.rs`
- Test: `crates/core/tests/route_stats.rs`

**Interfaces:**
- Produces:
```rust
pub fn haversine_m(a: LatLon, b: LatLon) -> f64
pub fn simplify_route(samples: &[Sample], max_points: usize) -> Vec<LatLon>  // pos 있는 것만, 균등 간격 추림, 첫·끝점 보존
pub struct Split { pub km: u32, pub duration_s: f64, pub avg_hr: Option<u8> }
pub fn km_splits(samples: &[Sample]) -> Vec<Split>   // distance_m 기준 1km마다, 마지막 미만 구간 포함
pub struct SportStats { pub count: u32, pub distance_m: f64, pub moving_s: f64 }
pub struct WeekBucket { pub week_start: NaiveDate /* 월요일 */, pub running_m: f64, pub cycling_m: f64 }
pub struct DayItem { pub activity_id: String, pub sport: Sport, pub distance_m: f64 }
pub struct MonthStats { pub year: i32, pub month: u32, pub total_distance_m: f64, pub total_moving_s: f64,
                        pub running: SportStats, pub cycling: SportStats, pub other: SportStats,
                        pub avg_hr: Option<f64> /* 이동시간 가중 평균 */,
                        pub weekly: Vec<WeekBucket>, pub days: BTreeMap<NaiveDate, Vec<DayItem>> }
pub fn month_stats(headers: &[ActivityHeader], year: i32, month: u32) -> MonthStats
```
월·날짜 귀속은 `start_local()`의 날짜 기준. 총 거리·시간은 모든 종목 합(설계 §8-1). 걷기·하이킹·기타는 `other`.

- [ ] **Step 1: 실패하는 테스트**
```rust
#[test] fn simplify_caps_points_and_keeps_ends() {
    let a = parse_fit(&read(fit_20261011())).unwrap();
    let r = simplify_route(&a.samples, 1500);
    assert!(r.len() <= 1500 && r.len() > 1000);
    assert_eq!(r[0], a.samples[0].pos.unwrap()); assert_eq!(*r.last().unwrap(), a.samples.last().unwrap().pos.unwrap());
}
#[test] fn simplify_without_gps_is_empty() { assert!(simplify_route(&samples_without_pos(100), 1500).is_empty()); }
#[test] fn splits_count() { let a = parse_fit(&read(fit_20261025())).unwrap(); assert_eq!(km_splits(&a.samples).len(), 49); }
#[test] fn month_stats_sums() {
    let h1 = parse_fit(&read(fit_20261011())).unwrap().header;
    let h2 = parse_fit(&read(fit_20261025())).unwrap().header;
    let run_nov = header(Sport::Running, "2026-11-01T22:00:00Z" /* KST 11-02 07:00 */, 5000.0, 1800.0);
    let s = month_stats(&[h1, h2, run_nov], 2026, 10);
    assert_eq!(s.cycling.count, 2); assert_eq!(s.running.count, 0);
    assert!((s.total_distance_m - 107_542.0).abs() < 20.0);
    assert_eq!(s.days.len(), 2);
    assert!(s.days.contains_key(&NaiveDate::from_ymd_opt(2026,10,25).unwrap()));
    assert_eq!(month_stats(&[run_nov], 2026, 11).running.count, 1);   // KST 날짜로 귀속
}
```
- [ ] **Step 2:** FAIL 확인.
- [ ] **Step 3:** 구현.
- [ ] **Step 4:** PASS 확인.
- [ ] **Step 5:** 커밋 `feat(core): route simplification, km splits, month stats`.

---

### Task 5: 카드 이미지 렌더러 (`workout-card`)

**Files:**
- Create: `crates/card/src/{lib.rs,data.rs,render.rs,text.rs}`, `crates/card/assets/fonts/{Pretendard-Medium.otf,Pretendard-SemiBold.otf,Pretendard-Bold.otf,OFL.txt}`
- Test: `crates/card/tests/card.rs`

**Interfaces:**
- Consumes: `Activity`, `format::*`, `simplify_route`, `PhotoMeta.orientation`.
- Produces:
```rust
pub struct CardData { pub date: String, pub title: String, pub distance: String, pub sub: String,
                      pub route: Vec<[f64; 2]> /* [x=lon*cos(lat0), y=lat] */ }
pub fn card_data(a: &Activity) -> CardData
pub fn render_card(photo: &Path, data: &CardData) -> Result<Vec<u8> /* JPEG */, CardError>
pub fn render_card_to_file(photo: &Path, data: &CardData, out: &Path) -> Result<(), CardError>
```
`card_data` 규칙 (기존 `fit_card.py`와 같음): `date` = `YYYY.MM.DD`(현지), `title` = `"{요일} {시간대} {종목}"`, `distance` = km 소수 2자리, `sub` = `[hms(moving), (라이딩이면 "{:.1} km/h", 아니면 거리>0일 때 페이스 "M'SS\"/km"), (심박 있으면 "{hr} bpm")]`을 `"  ·  "`로 연결, `route` = `simplify_route(…,1500)`를 x=lon·cos(lat0) 투영.

렌더 규칙 (기존 `card.swift`와 같은 디자인, 좌표는 위가 0인 화면 좌표로 환산):
- 사진을 EXIF 방향대로 회전 → 긴 변이 2160 미만이면 2160이 되게 확대. `u = min(W,H)/100`, 여백 `m = 5u`.
- 음영: 위쪽 띠 높이 26u, 위 끝 검정 α0.45 → 0. 아래쪽 띠 높이 52u, 아래 끝 α0.62 → 0.
- 글자(흰색, 그림자 검정 α0.45를 0.15u 아래로 한 번 더 그려 대체 — tiny-skia에 블러 없음):
  위: 날짜 3.3u Medium α0.85 자간 0.25u, 그 아래 0.6u 간격으로 제목 4.6u SemiBold.
  아래(밑에서 위로): 보조 기록 3.3u Medium α0.9 (바닥 여백 m), 그 위 0.2u 간격으로 거리 숫자 10u Bold 자간 −0.2u + `" km"` 4.2u SemiBold α0.9.
  경로: 거리 숫자 위 1.5u, 왼쪽 m+0.5u에서 시작, 가로 26u·세로 20u 상자에 비율 유지 맞춤, 흰 선 두께 0.55u, 둥근 이음·끝. 점이 2개 미만이면 생략.
- JPEG 품질 92로 새로 인코딩 (EXIF 복사 안 함 → GPS 없음).
- 폰트는 `include_bytes!`로 내장. Pretendard OTF는 GitHub `orioncactus/pretendard` 릴리스에서 받는다 — 다운로드 전에 사용자 확인.

- [ ] **Step 1: 실패하는 테스트**
```rust
#[test] fn card_data_matches_python() {
    let d = card_data(&parse_fit(&read(fit_20261011())).unwrap());
    assert_eq!(d.date, "2026.10.11"); assert_eq!(d.title, "일요일 오전 라이딩");
    assert_eq!(d.distance, "59.38"); assert_eq!(d.sub, "2:30:52  ·  23.6 km/h  ·  147 bpm");
    let d2 = card_data(&parse_fit(&read(fit_20261025())).unwrap());
    assert_eq!(d2.title, "일요일 오후 라이딩"); assert_eq!(d2.sub, "1:58:22  ·  24.4 km/h  ·  146 bpm");
}
#[test] fn renders_small_photo_upscaled_without_exif() {
    let d = card_data(&parse_fit(&read(fit_20261011())).unwrap());
    let jpg = render_card(&photo("20261011/31CCE40E-34DE-405F-ACE0-A1B5DEFC07D4_4_5005_c.jpeg"), &d).unwrap();
    let img = image::load_from_memory(&jpg).unwrap();
    assert_eq!(img.width().max(img.height()), 2160);
    assert!(img.height() > img.width());                    // 310×552 세로 사진 유지
    assert!(exif::Reader::new().read_from_container(&mut std::io::Cursor::new(&jpg)).is_err()); // EXIF 없음
}
#[test] fn renders_without_route() {   // Review Focus 2
    let mut d = card_data(&parse_fit(&read(fit_20261025())).unwrap()); d.route.clear();
    assert!(render_card(&photo("20261025/78D7CE5C-F354-4728-8E97-063487771792_1_105_c.jpeg"), &d).is_ok());
}
```
- [ ] **Step 2:** FAIL 확인.
- [ ] **Step 3:** 폰트 받기(사용자 확인 후), `image`(디코드·리사이즈 Lanczos3·JPEG 인코드) + `tiny-skia`(그라데이션·경로) + `ab_glyph`(글리프 외곽선 커버리지를 픽스맵에 알파 합성)로 구현.
- [ ] **Step 4:** PASS 확인.
- [ ] **Step 5: 눈으로 비교.** `cargo run -p workout-cli` 전 단계이므로 테스트 안에서 `target/card-preview/`에 20261025 사진 2장 카드를 써 두는 `#[ignore]` 테스트를 만들고 `cargo test -p workout-card -- --ignored` 실행 → 기존 `20261025/*_card.jpg`와 나란히 열어 배치·크기가 같은지 사용자에게 보여 준다.
- [ ] **Step 6:** 커밋 `feat(card): cross-platform card renderer`.

---

### Task 6: 템플릿 글과 게시 구조 (`workout-post`)

**Files:**
- Create: `crates/post/src/{lib.rs,context.rs,template.rs,publish.rs}`
- Test: `crates/post/tests/post.rs`

**Interfaces:**
- Consumes: `ActivityHeader`, `format::*`.
- Produces:
```rust
pub struct PlaceText { pub name: String, pub estimated: bool }
pub struct WeatherText { pub description: String, pub temp_c: f64 }
pub struct PostContext { pub activity: ActivityHeader, pub place: Option<PlaceText>,
                         pub route_points: Vec<String>, pub weather: Option<WeatherText>,
                         pub photo_km_marks: Vec<f64> }
pub struct PostText { pub body: String, pub hashtags: Vec<String> }
impl PostText { pub fn full_text(&self) -> String }   // body + "\n\n" + hashtags.join(" ")
#[derive(thiserror::Error, Debug)] pub enum PostError { Io(#[from] std::io::Error), Writer(String) }
pub trait DraftWriter { fn write(&self, ctx: &PostContext) -> Result<PostText, PostError>; }
pub struct TemplateWriter;   // impl DraftWriter
pub struct PostDraft { pub activity_id: String, pub text: PostText, pub card_paths: Vec<PathBuf> }
pub enum PublishMethod { ManualExport, ThreadsApi }
pub struct PublishReceipt { pub method: PublishMethod, pub at: DateTime<Utc>, pub external_id: Option<String> }
pub trait Publisher { fn publish(&self, draft: &PostDraft) -> Result<PublishReceipt, PostError>; }
pub struct ManualExport { pub out_dir: PathBuf }   // impl Publisher: out_dir/post.txt 저장 + 카드 복사
```
템플릿 (줄 단위, 값이 없는 줄은 통째로 생략, 줄 사이 `\n`):
1. `"{요일} {시간대} {장소 }{거리}km {이모지}"` — 거리: 라이딩은 반올림 정수, 그 외 소수 1자리. 이모지: 러닝 🏃, 라이딩 🚲, 그 외 👟. 장소 없으면 장소와 공백 생략. 추정 여부는 글에 쓰지 않음(화면 표시용).
2. `route_points`가 1개 이상이면 `"{A} → {B} → … 찍고 복귀"`.
3. 라이딩 `"평속 {:.1}km/h / 이동 {hms} / 평균 심박 {hr}"`, 러닝 등 `"페이스 {pace}/km / 이동 {hms} / 평균 심박 {hr}"` — 심박 없으면 마지막 항목 생략.
4. 사진 마크가 있으면 `"사진은 {12.3}km, {25.0}km 지점에서 찍었음"` (소수 1자리, 쉼표 구분).
5. 날씨 있으면 `"날씨 {description}, {temp:.0}도"`.
6. 질문: 라이딩 `["다들 이번 주말 어디로 달렸음?", "라이딩 코스 추천 좀 해줘"]`, 러닝 `["다들 이번 주 몇 km 뛰었음?", "러닝 코스 추천해줄 사람?"]`, 그 외 `["다들 오늘 운동했음?"]` 중 `start_local`의 연중 일자 % 개수로 선택(결정적).
해시태그: 종목(러닝 `#러닝 #러닝기록`, 라이딩 `#라이딩 #자전거`, 그 외 `#운동기록`) + 장소 `#{공백 제거}` + 각 경로 지점 `#{공백 제거}`, 중복 제거, 순서 유지.

- [ ] **Step 1: 실패하는 테스트**
```rust
#[test] fn full_template() {
    let ctx = PostContext { activity: header_20261025(), place: Some(PlaceText{name:"한강".into(), estimated:true}),
        route_points: vec!["반포".into(), "잠실".into()], weather: Some(WeatherText{description:"맑음".into(), temp_c:17.4}),
        photo_km_marks: vec![12.34, 25.0] };
    let t = TemplateWriter.write(&ctx).unwrap();
    let lines: Vec<&str> = t.body.lines().collect();
    assert_eq!(lines[0], "일요일 오후 한강 48km 🚲");
    assert_eq!(lines[1], "반포 → 잠실 찍고 복귀");
    assert_eq!(lines[2], "평속 24.4km/h / 이동 1:58:22 / 평균 심박 146");
    assert_eq!(lines[3], "사진은 12.3km, 25.0km 지점에서 찍었음");
    assert_eq!(lines[4], "날씨 맑음, 17도");
    assert_eq!(lines.len(), 6);
    assert_eq!(t.hashtags, vec!["#라이딩", "#자전거", "#한강", "#반포", "#잠실"]);
    assert!(!t.full_text().contains("추정"));
}
#[test] fn minimal_template_drops_empty_lines() {
    let ctx = PostContext { activity: header_20261025(), place: None, route_points: vec![], weather: None, photo_km_marks: vec![] };
    let t = TemplateWriter.write(&ctx).unwrap();
    assert_eq!(t.body.lines().next().unwrap(), "일요일 오후 48km 🚲");
    assert_eq!(t.body.lines().count(), 3);
}
#[test] fn running_uses_pace() {
    let t = TemplateWriter.write(&ctx_for(run_header(5000.0, 1650.0, None))).unwrap();
    assert!(t.body.lines().nth(0).unwrap().contains("5.0km 🏃"));
    assert_eq!(t.body.lines().nth(1).unwrap(), "페이스 5'30\"/km / 이동 27:30");
}
#[test] fn manual_export_writes_files() {
    let dir = tempfile::tempdir().unwrap();
    let card = dir.path().join("c_card.jpg"); std::fs::write(&card, b"x").unwrap();
    let out = dir.path().join("out");
    let r = ManualExport{ out_dir: out.clone() }.publish(&PostDraft{ activity_id:"a".into(),
        text: PostText{ body:"본문".into(), hashtags: vec!["#러닝".into()] }, card_paths: vec![card] }).unwrap();
    assert!(matches!(r.method, PublishMethod::ManualExport));
    assert_eq!(std::fs::read_to_string(out.join("post.txt")).unwrap(), "본문\n\n#러닝");
    assert!(out.join("c_card.jpg").exists());
}
```
- [ ] **Step 2:** FAIL 확인.
- [ ] **Step 3:** 구현.
- [ ] **Step 4:** PASS 확인.
- [ ] **Step 5:** 커밋 `feat(post): template writer and manual export publisher`.

---

### Task 7: CLI `workout-cli prepare <폴더>`

**Files:**
- Create: `crates/cli/src/main.rs`, `crates/cli/src/prepare.rs`
- Test: `crates/cli/tests/prepare.rs`

**Interfaces:**
- Consumes: `parse_fit`, `read_photo_meta`, `is_photo_input`, `match_photo`, `card_data`, `render_card_to_file`, `TemplateWriter`, `PostContext`.
- Produces:
```rust
pub struct PrepareReport { pub activities: usize, pub cards: Vec<PathBuf>, pub posts: Vec<PathBuf>,
                           pub skipped_fit: Vec<(PathBuf, String)>, pub unmatched_photos: Vec<PathBuf> }
pub fn prepare(folder: &Path) -> anyhow::Result<PrepareReport>
```
동작: 폴더의 `*.fit`(대소문자 무시) 전부 파싱 — 실패한 파일은 `skipped_fit`에 넣고 계속. 사진마다 첫 성공 운동의 오프셋을 fallback으로 메타 읽기 → `match_photo` → 맞으면 `<사진 stem>_card.jpg`를 같은 폴더에 생성, 안 맞으면 `unmatched_photos`. 운동마다 `post_{HHMM}.txt`(현지 시작 시각)에 `TemplateWriter` 결과의 `full_text()` 저장 (장소·경로·날씨는 비움 — 3단계에서 채움). 표준 출력에 기존 Python처럼 요약 줄 + `저장: …` 출력, 경고는 stderr. FIT가 하나도 없으면 오류 종료.

- [ ] **Step 1: 실패하는 테스트** (실제 폴더를 임시 폴더로 복사해서 실행)
```rust
#[test] fn prepares_20261025() {
    let dir = copy_to_temp(&["20261025/24842957203_ACTIVITY.fit",
        "20261025/0723ACAA-C18C-412E-BDF8-0CB9253CB50F_1_102_a.jpeg", "20261025/78D7CE5C-F354-4728-8E97-063487771792_1_105_c.jpeg"]);
    let r = prepare(dir.path()).unwrap();
    assert_eq!(r.activities, 1); assert_eq!(r.cards.len(), 2);
    assert!(dir.path().join("post_1441.txt").exists());
    assert!(std::fs::read_to_string(dir.path().join("post_1441.txt")).unwrap().contains("km 지점에서 찍었음"));
}
#[test] fn bad_fit_and_unmatched_photo_do_not_stop_run() {   // Review Focus 4, 5
    let dir = copy_to_temp(&["20261025/24842957203_ACTIVITY.fit", "20261011/31CCE40E-34DE-405F-ACE0-A1B5DEFC07D4_4_5005_c.jpeg"]);
    std::fs::write(dir.path().join("broken.fit"), b"garbage").unwrap();
    let r = prepare(dir.path()).unwrap();
    assert_eq!(r.skipped_fit.len(), 1); assert_eq!(r.unmatched_photos.len(), 1); assert!(r.cards.is_empty());
}
#[test] fn two_activities_get_their_own_posts() {   // Review Focus 3
    let dir = copy_to_temp(&["20261011/24680396566_ACTIVITY.fit", "20261025/24842957203_ACTIVITY.fit",
        "20261025/78D7CE5C-F354-4728-8E97-063487771792_1_105_c.jpeg"]);
    let r = prepare(dir.path()).unwrap();
    assert_eq!(r.posts.len(), 2); assert_eq!(r.cards.len(), 1);
}
#[test] fn ignores_existing_cards() {
    let dir = copy_to_temp(&["20261025/24842957203_ACTIVITY.fit", "20261025/78D7CE5C-F354-4728-8E97-063487771792_1_105_c_card.jpg"]);
    assert!(prepare(dir.path()).unwrap().cards.is_empty());
}
```
- [ ] **Step 2:** FAIL 확인.
- [ ] **Step 3:** `prepare` 구현, `main.rs`는 `clap`으로 `prepare <FOLDER>` 서브커맨드만.
- [ ] **Step 4:** `cargo test --workspace` 전체 PASS.
- [ ] **Step 5: 실제 실행.** 사용자 폴더를 건드리지 않게 복사본에서: `cp -R ../20261025 /tmp/... && cargo run -p workout-cli -- prepare <복사본>` → 카드 2장 + `post_1441.txt` 생성, 글 첫 줄이 `일요일 오후 48km 🚲`.
- [ ] **Step 6:** `cargo clippy --workspace -- -D warnings`, `cargo fmt --check` 통과.
- [ ] **Step 7:** 커밋 `feat(cli): prepare command`.

---

## 완료 기준

- `cargo test --workspace` 전체 PASS, clippy·fmt 통과.
- `workout-cli prepare`가 기존 Python 도구와 같은 카드(눈 비교)와 템플릿 글을 만든다.
- Windows·Linux 빌드 확인은 4단계(CI)에서. 이 단계에서는 OS 전용 API를 쓰지 않았는지 코드로 확인 (`grep -r "AppKit\|cocoa\|winapi" crates/` 결과 없음).
