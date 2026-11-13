# 3단계: 장소·날씨 자동 입력과 Claude로 다듬기 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 게시 준비 화면에서 장소·경로·날씨가 자동으로 채워지고(직접 고칠 수 있음), 버튼 하나로 Claude가 초안 A 규칙에 맞게 글을 다듬어 미리보기로 보여 준다.

**Architecture:** 네트워크를 쓰는 기능은 새 crate `workout-enrich`에 모은다(Nominatim, Open-Meteo, Claude Messages API). HTTP는 `Http` 트레이트 뒤에 두어 테스트는 고정 JSON으로 오프라인 실행한다. 좌표 보호(시작·끝 500m 제외, 중간 지점 ≤5개, 날씨 좌표 소수 2자리)는 `workout-core`의 순수 함수다. `workout-service`가 이것들을 묶어 저장소(v3: 필드별 "직접 고침" 표시, 지오코딩 캐시)에 반영하고, Tauri·UI가 버튼과 편집 칸을 붙인다. `workout-post`는 계속 네트워크 없이 템플릿만 담당한다.

**Tech Stack:** `reqwest`(blocking, `rustls-tls`, `json`), `serde_json`, `keyring`(OS 키체인: macOS Keychain / Windows Credential Manager / Linux Secret Service), `base64`. Claude는 Rust 공식 SDK가 없으므로 **raw HTTP** `POST https://api.anthropic.com/v1/messages`(헤더 `x-api-key`, `anthropic-version: 2026-10-21`, `anthropic-beta: server-side-fallback-2026-07-01`).

**Spec:** `docs/superpowers/specs/2026-11-13-workout-threads-app-design.md` §8.1, §8.2 (로드맵: `docs/superpowers/plans/2026-11-13-roadmap.md`)

## Global Constraints

- 1·2단계 Global Constraints 그대로 유효.
- **좌표 보호(설계 §8.2):** 경로 시작·끝 각 **500m** 안쪽 좌표는 외부로 보내지 않는다. 장소·경로용으로 보내는 점은 거리 기준 균등 분할한 **중간 지점 최대 5개**. 날씨는 경로 중간 지점 1개를 **소수 2자리로 반올림**해 보낸다. 사진 GPS는 어떤 외부 서비스로도 보내지 않는다.
- **Claude로 보내는 것(설계 §8.1):** 템플릿 초안, 요약 수치, 장소·경로·날씨 텍스트, 사진 촬영 지점 km. **좌표 금지.** 사진은 "사진도 보내기"를 켰을 때만, EXIF 없이 다시 인코딩한 긴 변 1024px 축소본.
- Claude 요청: 모델 기본 `claude-opus-5-5`(설정 `claude_model`로 변경 가능), `output_config: {effort: "medium", format: {type: "json_schema", schema}}`, `thinking` 생략(이 모델은 항상 켜짐), `fallbacks: "default"` + 위 beta 헤더, `max_tokens: 16000`, 비스트리밍. `stop_reason`이 `refusal`이면 `stop_details.category`를 담은 오류, `max_tokens`면 "응답이 잘렸어요" 오류. 결과는 저장하지 않고 미리보기로만 돌려준다.
- API 키는 OS 키체인(`keyring`, 서비스 이름 `dev.sns-create.workout`, 계정 `anthropic_api_key`)에만 저장. DB·로그·화면에 키를 쓰지 않는다. 테스트는 키체인을 쓰지 않는다(`SecretStore` 트레이트의 메모리 구현).
- Nominatim 정책: 초당 1건 이하, `User-Agent: workout-app/0.1 (personal desktop app)`(개인 정보 없음), `accept-language=ko`, 결과는 DB 캐시(좌표 소수 4자리 키).
- 자동 값 vs 직접 고친 값: 장소·경로·날씨 각각 "직접 고침" 표시가 있으면 다시 가져와도 덮어쓰지 않는다. 자동 장소명은 `estimated = true`.
- 네트워크가 안 되면 그 칸만 비우고 오류 문구를 보여 준다. 가져오기(import)는 계속 오프라인으로 동작한다.
- 실제 외부 호출(Nominatim, Open-Meteo, Claude)은 **Task 7에서만**, 사용자 확인 후 실행한다(Claude는 비용 발생).

## Review Focus

1. **경로가 1km보다 짧거나 GPS가 없는 운동** — 시작·끝 500m를 빼면 남는 점이 없다. 외부 호출 없이 장소·경로·날씨를 비워 두고 오류 없이 끝나야 한다. → Task 1, Task 4 테스트.
2. **사용자가 장소를 고친 뒤 "다시 가져오기"** — 고친 장소는 그대로, 고치지 않은 경로·날씨만 새로 채워진다. 글이 직접 고친 상태면 글은 바뀌지 않는다. → Task 4 테스트.
3. **Claude가 거절·잘림·형식 오류 응답** — `refusal`, `max_tokens`, JSON이 아닌 텍스트, 해시태그에 `#` 없음. 앱이 멈추지 않고 이해할 수 있는 오류나 정리된 결과를 돌려준다. → Task 3 테스트.
4. **API 키가 없을 때 Claude 버튼** — 요청을 보내지 않고 "설정에서 API 키를 넣어 주세요"를 돌려준다. → Task 4 테스트.
5. **Nominatim이 이름 없는 결과나 오류를 줄 때**(바다, 빈 address, HTTP 429/5xx) — 그 점만 건너뛰고 나머지 점으로 장소·경로를 만든다. → Task 2, Task 4 테스트.

---

## 파일 구조

```
crates/core/src/privacy.rs          (신규) privacy_points, weather_point
crates/card/src/upload.rs           (신규) photo_for_upload (EXIF 없는 축소 JPEG)
crates/enrich/                      (신규 crate workout-enrich)
  src/lib.rs                        Http 트레이트, ReqwestHttp, EnrichError
  src/geocode.rs                    reverse_url, parse_reverse, area_name, summarize_places, RateLimiter
  src/weather.rs                    weather_url, parse_weather, wmo_ko
  src/claude.rs                     ClaudeRequest, build_request, parse_response, refine
  tests/fixtures/*.json             Nominatim·Open-Meteo·Claude 응답 예시(손으로 만든 것)
crates/store/src/schema.rs          (수정) v3
crates/store/src/rows.rs            (수정) PostRow 필드별 user_set
crates/store/src/lib.rs             (수정) geocode_cache get/put
crates/service/src/enrich.rs        (신규) enrich_activity, set_place/route/weather, refine_post
crates/service/src/secrets.rs       (신규) SecretStore, KeyringStore, MemoryStore
crates/service/src/lib.rs           (수정) Library::open_with(root, http, secrets)
src-tauri/src/main.rs               (수정) 새 명령
ui/src/components/PostPanel.svelte  (수정) 장소·경로·날씨 칸, 다시 가져오기, Claude 다듬기
ui/src/components/RefinePreview.svelte, Settings.svelte (신규)
ui/src/lib/enrich.ts                (신규) routeText, parseRoutePoints, weatherText
```

---

### Task 1: 좌표 보호 함수 (`workout-core`)

**Files:** Create `crates/core/src/privacy.rs`; Test `crates/core/tests/privacy.rs`

**Interfaces:**
- Consumes: `haversine_m`, `LatLon`.
- Produces:
```rust
pub const TRIM_M: f64 = 500.0;
pub const MAX_PLACE_POINTS: usize = 5;
/// 경로 누적 거리에서 앞·뒤 trim_m을 뺀 구간을 max개로 균등 분할한 점 (양 끝 제외). 남는 구간이 없으면 빈 Vec.
pub fn privacy_points(route: &[LatLon], trim_m: f64, max: usize) -> Vec<LatLon>
/// 보호 구간 가운데 지점을 소수 2자리로 반올림. 남는 구간이 없으면 None.
pub fn weather_point(route: &[LatLon], trim_m: f64) -> Option<LatLon>
```

- [ ] **Step 1: 실패하는 테스트**
```rust
#[test] fn points_avoid_both_ends() {
    let a = parse_fit(&read(fit_20261025())).unwrap();
    let route = simplify_route(&a.samples, 1500);
    let pts = privacy_points(&route, TRIM_M, MAX_PLACE_POINTS);
    assert_eq!(pts.len(), 5);
    let (s, e) = (route[0], *route.last().unwrap());
    assert!(pts.iter().all(|p| haversine_m(*p, s) > 500.0 && haversine_m(*p, e) > 500.0));
}
#[test] fn short_route_gives_nothing() {
    let route = vec![LatLon { lat: 37.5, lon: 127.0 }, LatLon { lat: 37.505, lon: 127.0 }]; // 약 556m
    assert!(privacy_points(&route, TRIM_M, 5).is_empty());
    assert!(weather_point(&route, TRIM_M).is_none());
    assert!(privacy_points(&[], TRIM_M, 5).is_empty());
}
#[test] fn weather_point_is_rounded() {
    let a = parse_fit(&read(fit_20261025())).unwrap();
    let p = weather_point(&simplify_route(&a.samples, 1500), TRIM_M).unwrap();
    assert_eq!((p.lat * 100.0).round() / 100.0, p.lat);
    assert_eq!((p.lon * 100.0).round() / 100.0, p.lon);
}
```
※ "양 끝 500m" 판정은 경로를 따라간 누적 거리 기준이다. 순환 경로에서 출발점 근처를 다시 지나가는 점은 직선거리로는 가까울 수 있으므로, 첫 테스트가 실패하면 **누적 거리 기준에 더해 출발·도착점과 직선거리 500m 안의 점도 제외**한다(집 근처를 다시 지나가는 경우까지 보호 — 설계 취지).
- [ ] **Step 2:** `cargo test -p workout-core --test privacy` → 컴파일 실패.
- [ ] **Step 3:** 구현.
- [ ] **Step 4:** PASS.
- [ ] **Step 5:** 커밋 `feat(core): privacy points for external lookups`.

---

### Task 2: 장소·날씨 조회 파싱 (`workout-enrich`)

**Files:** Create `crates/enrich/{Cargo.toml,src/lib.rs,src/geocode.rs,src/weather.rs}`, `crates/enrich/tests/fixtures/{nominatim_park.json,nominatim_dong.json,nominatim_sea.json,openmeteo_hourly.json}`; Test `crates/enrich/tests/{geocode.rs,weather.rs}`; Modify workspace `Cargo.toml` members.

**Interfaces:**
- Produces:
```rust
// lib.rs
pub struct HttpResponse { pub status: u16, pub body: Vec<u8> }
pub trait Http: Send + Sync {
    fn get(&self, url: &str, headers: &[(&str, &str)]) -> Result<HttpResponse, EnrichError>;
    fn post_json(&self, url: &str, headers: &[(&str, &str)], body: &serde_json::Value) -> Result<HttpResponse, EnrichError>;
}
pub struct ReqwestHttp;   // reqwest::blocking, timeout 30s (Claude는 120s)
#[derive(thiserror::Error, Debug)] pub enum EnrichError { Network(String), Status(u16, String), Parse(String), Refused(Option<String>), Truncated, NoKey }
pub const USER_AGENT: &str = "workout-app/0.1 (personal desktop app)";
// geocode.rs
pub fn reverse_url(p: LatLon) -> String   // https://nominatim.openstreetmap.org/reverse?format=jsonv2&lat={:.5}&lon={:.5}&zoom=16&accept-language=ko
pub struct ReverseResult { pub poi: Option<String>, pub area: Option<String>, pub district: Option<String> }
pub fn parse_reverse(json: &[u8]) -> Result<ReverseResult, EnrichError>
pub fn area_name(raw: &str) -> String       // 행정동 → 짧은 이름
pub struct PlaceSummary { pub place: Option<String>, pub route_points: Vec<String> }
pub fn summarize_places(results: &[ReverseResult]) -> PlaceSummary
pub struct RateLimiter { /* 마지막 호출 시각 */ }
impl RateLimiter { pub fn new(min_interval: Duration) -> Self; pub fn wait(&self) }
// weather.rs
pub fn weather_url(p: LatLon, local_date: NaiveDate, today: NaiveDate) -> String
pub fn parse_weather(json: &[u8], local_hour: NaiveDateTime) -> Result<(String, f64), EnrichError>  // (설명, 기온)
pub fn wmo_ko(code: u8) -> &'static str
```
규칙:
- `parse_reverse`: `poi` = 응답 `category`가 `leisure`·`natural`·`waterway`·`tourism`·`place`이고 `name`이 비어 있지 않을 때 그 `name`. `area` = `address`의 `quarter` → `suburb` → `neighbourhood` 순 첫 값에 `area_name` 적용. `district` = `city_district` → `borough` → `county`. `error` 키가 있으면 `poi/area/district` 모두 None.
- `area_name`: 끝 "동" 제거 → 끝 숫자 제거 → 끝 "본" 제거 → 끝 "N가" 제거. 예: `반포2동`→`반포`, `잠실본동`→`잠실`, `여의도동`→`여의도`, `성수1가1동`→`성수`, `망원1동`→`망원`, `서교동`→`서교`. 동이 아닌 이름(`Yeouido`)은 그대로.
- `summarize_places`: `place` = `poi` 중 가장 많이 나온 것(동률이면 먼저 나온 것), 없으면 `district` 최빈값. `route_points` = `area`를 순서대로 모아 연속 중복 제거, 전체 중복 제거, 최대 3개, `place`와 같은 이름은 뺀다.
- `weather_url`: `today - local_date ≤ 60일`이면 `https://api.open-meteo.com/v1/forecast`, 아니면 `https://archive-api.open-meteo.com/v1/archive`. 쿼리 `latitude={:.2}&longitude={:.2}&hourly=temperature_2m,weather_code&start_date=D&end_date=D&timezone=auto`.
- `parse_weather`: `hourly.time`(현지 "YYYY-MM-DDTHH:MM")에서 `local_hour`의 시(時)와 같은 칸, `temperature_2m`·`weather_code`가 null이면 Parse 오류.
- `wmo_ko`: 0 맑음, 1·2 구름 조금, 3 흐림, 45·48 안개, 51–57 이슬비, 61–67 비, 71–77 눈, 80–82 소나기, 85·86 소낙눈, 95–99 뇌우, 그 외 "날씨 정보 없음".

- [ ] **Step 1: 고정 JSON 만들기** (실제 응답 형식을 본뜬 손 작성 예시. 실제 좌표 쓰지 않음):
  `nominatim_park.json` `{"category":"leisure","type":"park","name":"반포한강공원","address":{"suburb":"반포2동","city_district":"서초구","city":"서울특별시"}}`,
  `nominatim_dong.json` `{"category":"highway","type":"residential","name":"","address":{"quarter":"잠실본동","city_district":"송파구"}}`,
  `nominatim_sea.json` `{"error":"Unable to geocode"}`,
  `openmeteo_hourly.json` `{"hourly":{"time":["2026-10-25T14:00","2026-10-25T15:00"],"temperature_2m":[17.4,16.9],"weather_code":[1,3]}}`.
- [ ] **Step 2: 실패하는 테스트**
```rust
#[test] fn area_names() { for (a, b) in [("반포2동","반포"),("잠실본동","잠실"),("여의도동","여의도"),("성수1가1동","성수"),("망원1동","망원"),("Yeouido","Yeouido")] { assert_eq!(area_name(a), b); } }
#[test] fn parses_park() { let r = parse_reverse(&fixture("nominatim_park.json")).unwrap();
    assert_eq!(r.poi.as_deref(), Some("반포한강공원")); assert_eq!(r.area.as_deref(), Some("반포")); assert_eq!(r.district.as_deref(), Some("서초구")); }
#[test] fn parses_error_as_empty() { let r = parse_reverse(&fixture("nominatim_sea.json")).unwrap(); assert!(r.poi.is_none() && r.area.is_none()); }
#[test] fn summarizes() {
    let park = parse_reverse(&fixture("nominatim_park.json")).unwrap();
    let dong = parse_reverse(&fixture("nominatim_dong.json")).unwrap();
    let s = summarize_places(&[park.clone(), dong.clone(), park.clone(), ReverseResult { poi: None, area: None, district: None }]);
    assert_eq!(s.place.as_deref(), Some("반포한강공원")); assert_eq!(s.route_points, vec!["반포", "잠실"]);
    assert!(summarize_places(&[]).place.is_none());
}
#[test] fn url_has_rounded_coords_and_korean() {
    let u = reverse_url(LatLon { lat: 37.123456, lon: 127.654321 });
    assert!(u.contains("lat=37.12346") && u.contains("accept-language=ko") && u.contains("zoom=16")); }
#[test] fn weather_endpoint_by_age() {
    let d = |s: &str| NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap();
    assert!(weather_url(LatLon { lat: 37.52, lon: 126.93 }, d("2026-10-25"), d("2026-11-14")).starts_with("https://api.open-meteo.com/v1/forecast"));
    assert!(weather_url(LatLon { lat: 37.52, lon: 126.93 }, d("2026-01-05"), d("2026-11-14")).starts_with("https://archive-api.open-meteo.com/v1/archive")); }
#[test] fn parses_weather_hour() {
    let t = NaiveDateTime::parse_from_str("2026-10-25 15:20:00", "%Y-%m-%d %H:%M:%S").unwrap();
    assert_eq!(parse_weather(&fixture("openmeteo_hourly.json"), t).unwrap(), ("흐림".to_string(), 16.9)); }
#[test] fn rate_limiter_spaces_calls() {
    let r = RateLimiter::new(Duration::from_millis(200)); let t0 = Instant::now(); r.wait(); r.wait(); r.wait();
    assert!(t0.elapsed() >= Duration::from_millis(400)); }
```
- [ ] **Step 3:** `cargo test -p workout-enrich` → FAIL.
- [ ] **Step 4:** 구현 (`ReqwestHttp`는 이 단계에서 만들되 테스트에서 쓰지 않는다).
- [ ] **Step 5:** PASS.
- [ ] **Step 6:** 커밋 `feat(enrich): reverse geocoding and weather parsing`.

---

### Task 3: Claude 다듬기 요청·응답 (`workout-enrich` + `workout-card`)

**Files:** Create `crates/enrich/src/claude.rs`, `crates/card/src/upload.rs`, `crates/enrich/tests/fixtures/{claude_ok.json,claude_refusal.json,claude_max_tokens.json,claude_text.json}`; Test `crates/enrich/tests/claude.rs`, `crates/card/tests/upload.rs`

**Interfaces:**
- Consumes: `Http`, `workout_post::{PostContext, PostText}`, `format::*`.
- Produces:
```rust
// card/upload.rs
/// EXIF 회전 반영, 긴 변 max_side 이하로 줄인 EXIF 없는 JPEG(품질 85).
pub fn photo_for_upload(path: &Path, max_side: u32) -> Result<Vec<u8>, CardError>
// enrich/claude.rs
pub const API_URL: &str = "https://api.anthropic.com/v1/messages";
pub const DEFAULT_MODEL: &str = "claude-opus-5-5";
pub struct RefineInput { pub ctx: PostContext, pub draft: PostText, pub photos_jpeg: Vec<Vec<u8>> }
pub fn system_prompt() -> &'static str
pub fn build_request(model: &str, input: &RefineInput) -> serde_json::Value
pub fn parse_response(json: &[u8]) -> Result<PostText, EnrichError>
pub fn refine(http: &dyn Http, api_key: &str, model: &str, input: &RefineInput) -> Result<PostText, EnrichError>
```
요청 본문(`build_request`): `{"model", "max_tokens": 16000, "system": system_prompt(), "fallbacks": "default", "output_config": {"effort": "medium", "format": {"type": "json_schema", "schema": {"type":"object","properties":{"body":{"type":"string"},"hashtags":{"type":"array","items":{"type":"string"}}},"required":["body","hashtags"],"additionalProperties":false}}}, "messages": [{"role":"user","content":[ ...사진마다 {"type":"image","source":{"type":"base64","media_type":"image/jpeg","data":…}}, {"type":"text","text": <사실 JSON + 초안>} ]}]}`. 사실 JSON 키: `sport`, `weekday`, `time_of_day`, `distance_km`, `moving`, `pace_or_speed`, `avg_hr`, `place`, `place_estimated`, `route_points`, `weather`, `photo_km_marks` — **좌표 키 없음**.
`system_prompt()` 내용(고정 문구, 한국어): 스레드용 운동 기록 글을 다듬는 역할 / 반말체("~했음", "~더라") / 7줄 구조(요일·시간대+장소+거리+이모지, 경로 한 줄, 기록 한 줄, 사진 지점 한두 줄, 날씨 한 줄, 답글 유도 질문, 해시태그는 `hashtags` 배열) / 주어진 숫자는 바꾸지 말 것 / 출발·도착 지점 이름과 좌표·주소를 쓰지 말 것 / 정보가 없는 줄은 지어내지 말고 빼기 / 사진이 있으면 보이는 장면을 한 줄로 / 해시태그 3–7개.
헤더: `x-api-key`, `anthropic-version: 2026-10-21`, `anthropic-beta: server-side-fallback-2026-07-01`, `content-type: application/json`.
`parse_response`: `stop_reason == "refusal"` → `Refused(stop_details.category)`; `"max_tokens"` → `Truncated`; 첫 `text` 블록을 JSON으로 파싱(실패 시 `Parse`); 해시태그는 공백 제거·`#` 없으면 붙이기·빈 값/중복 제거. HTTP 401 → `Status(401, "API 키를 확인해 주세요")`, 429·5xx → `Status(code, …)`.

- [ ] **Step 1: 고정 JSON**: `claude_ok.json` = `{"stop_reason":"end_turn","content":[{"type":"thinking","thinking":""},{"type":"text","text":"{\"body\":\"일요일 오후 한강 48km 🚲\\n…\",\"hashtags\":[\"#라이딩\",\"한강\",\"#라이딩\"]}"}]}`; `claude_refusal.json` = `{"stop_reason":"refusal","stop_details":{"type":"refusal","category":"general_harms"},"content":[]}`; `claude_max_tokens.json` = `{"stop_reason":"max_tokens","content":[{"type":"text","text":"{\"body\":\"일요"}]}`; `claude_text.json` = `{"stop_reason":"end_turn","content":[{"type":"text","text":"죄송"}]}`.
- [ ] **Step 2: 실패하는 테스트**
```rust
#[test] fn request_has_schema_fallback_and_no_coords() {
    let req = build_request(DEFAULT_MODEL, &input_with_route(vec![]));
    assert_eq!(req["model"], "claude-opus-5-5"); assert_eq!(req["fallbacks"], "default");
    assert_eq!(req["output_config"]["format"]["type"], "json_schema"); assert!(req.get("thinking").is_none());
    let s = req.to_string(); assert!(!s.contains("lat") && !s.contains("lon") && !s.contains("37.5"));
}
#[test] fn request_includes_photos_only_when_given() {
    assert_eq!(build_request(DEFAULT_MODEL, &input_with_route(vec![])) ["messages"][0]["content"].as_array().unwrap().len(), 1);
    assert_eq!(build_request(DEFAULT_MODEL, &input_with_route(vec![vec![0xFF, 0xD8]]))["messages"][0]["content"][0]["type"], "image");
}
#[test] fn parses_ok_and_cleans_hashtags() { let t = parse_response(&fixture("claude_ok.json")).unwrap();
    assert!(t.body.starts_with("일요일 오후")); assert_eq!(t.hashtags, vec!["#라이딩", "#한강"]); }
#[test] fn refusal_and_truncation() {
    assert!(matches!(parse_response(&fixture("claude_refusal.json")), Err(EnrichError::Refused(Some(c))) if c == "general_harms"));
    assert!(matches!(parse_response(&fixture("claude_max_tokens.json")), Err(EnrichError::Truncated)));
    assert!(matches!(parse_response(&fixture("claude_text.json")), Err(EnrichError::Parse(_)))); }
#[test] fn refine_sends_headers_and_maps_401() {
    let http = MockHttp::new(vec![HttpResponse { status: 401, body: b"{}".to_vec() }]);
    assert!(matches!(refine(&http, "k", DEFAULT_MODEL, &input_with_route(vec![])), Err(EnrichError::Status(401, _))));
    let h = http.last_headers(); assert!(h.contains(&("anthropic-version".into(), "2026-10-21".into())));
    assert!(h.contains(&("anthropic-beta".into(), "server-side-fallback-2026-07-01".into()))); }
// card/tests/upload.rs
#[test] fn upload_photo_is_small_and_has_no_exif() {
    let b = photo_for_upload(&sample("20261025/0723ACAA-C18C-412E-BDF8-0CB9253CB50F_1_102_a.jpeg"), 1024).unwrap();
    let img = image::load_from_memory(&b).unwrap(); assert_eq!(img.width().max(img.height()), 1024);
    assert!(exif::Reader::new().read_from_container(&mut std::io::Cursor::new(&b)).is_err()); }
```
(`MockHttp`·`input_with_route`·`fixture`는 `tests/common.rs` 도우미. `MockHttp`는 받은 URL·헤더·본문을 기록.)
- [ ] **Step 3:** FAIL 확인.
- [ ] **Step 4:** 구현. `card`의 기존 `load_oriented`를 `upload.rs`와 공유하도록 `pub(crate)`로 옮긴다(중복 금지).
- [ ] **Step 5:** PASS.
- [ ] **Step 6:** 커밋 `feat(enrich): Claude refine request and response handling`.

---

### Task 4: 서비스 — 자동 채우기·직접 고침·Claude 다듬기·API 키

**Files:** Modify `crates/store/src/{schema.rs,rows.rs,lib.rs}`, `crates/service/src/{lib.rs,dto.rs}`; Create `crates/service/src/{enrich.rs,secrets.rs}`; Test `crates/store/tests/store.rs`(추가), `crates/service/tests/enrich.rs`

**Interfaces:**
- Store v3: `posts`에 `place_user_set INT NOT NULL DEFAULT 0`, `route_user_set INT …`, `weather_user_set INT …` 추가. `geocode_cache(key TEXT PRIMARY KEY, json TEXT NOT NULL)` (key = `"{lat:.4},{lon:.4}"`).
```rust
// rows.rs PostRow에 추가
pub place_user_set: bool, pub route_user_set: bool, pub weather_user_set: bool,
// Store
pub fn geocode_cached(&self, key: &str) -> Result<Option<String>, StoreError>;
pub fn put_geocode(&self, key: &str, json: &str) -> Result<(), StoreError>;
// service secrets.rs
pub trait SecretStore: Send + Sync { fn get(&self) -> Option<String>; fn set(&self, v: Option<&str>) -> Result<(), ServiceError>; }
pub struct KeyringStore;  pub struct MemoryStore(Mutex<Option<String>>);
// service lib.rs
impl Library {
    pub fn open(root: &Path) -> Result<Library>;   // ReqwestHttp + KeyringStore
    pub fn open_with(root: &Path, http: Arc<dyn Http>, secrets: Arc<dyn SecretStore>) -> Result<Library>;
    pub fn enrich_activity(&mut self, id: &str) -> Result<PostView>;
    pub fn set_place(&mut self, id: &str, name: Option<String>) -> Result<PostView>;   // user_set = name.is_some(); None이면 자동으로 되돌림(user_set=false, 값 비움)
    pub fn set_route_points(&mut self, id: &str, points: Vec<String>) -> Result<PostView>;
    pub fn set_weather(&mut self, id: &str, weather: Option<WeatherDto>) -> Result<PostView>;
    pub fn refine_post(&self, id: &str, include_photos: bool) -> Result<RefineDto>;     // 저장 안 함
    pub fn set_claude_key(&mut self, key: Option<String>) -> Result<()>;
    pub fn claude_status(&self) -> Result<ClaudeStatusDto>;
    pub fn set_claude_model(&mut self, model: Option<String>) -> Result<()>;           // settings "claude_model"
}
// dto.rs (ts-rs)
pub struct WeatherDto { pub description: String, pub temp_c: f64 }
pub struct RefineDto { pub body: String, pub hashtags: Vec<String>, pub full_text: String }
pub struct ClaudeStatusDto { pub has_key: bool, pub model: String }
// PostView에 추가: route_points: Vec<String>, weather: Option<WeatherDto>, place_user_set, route_user_set, weather_user_set: bool, enrich_error: Option<String>
```
`enrich_activity` 순서: 경로(저장된 `route`) → `privacy_points` → 각 점을 캐시 확인 후 없으면 `RateLimiter(1초)` + `reverse_url` 호출·캐시 저장(실패한 점은 건너뜀) → `summarize_places` → `user_set`이 아닌 칸만 갱신(장소는 `estimated: true`) → `weather_point`가 있으면 Open-Meteo 1회(운동 중간 시각의 현지 시) → 글이 `edited`·`published`가 아니면 템플릿 다시 쓰기. 네트워크 오류는 `PostView.enrich_error`에 한국어 문구로 담고 `Ok`.
`refine_post`: 키 없으면 `ServiceError::Failed("설정에서 Claude API 키를 넣어 주세요")`, 요청 보내지 않음. 사진은 `include_photos`일 때 카드가 아닌 **원본 사진**을 `photo_for_upload(…, 1024)`로 최대 4장.

- [ ] **Step 1: 실패하는 테스트** (`MockHttp`: URL 접두사별 고정 응답, 호출 기록; `MemoryStore`)
```rust
#[test] fn enrich_fills_place_route_weather() {
    let (_t, mut lib, http) = temp_library_with_mock();   // nominatim → park/dong 번갈아, open-meteo → hourly
    let id = import_1025(&mut lib);
    let p = lib.enrich_activity(&id).unwrap();
    assert_eq!(p.place.as_ref().unwrap().name, "반포한강공원"); assert!(p.place.unwrap().estimated);
    assert_eq!(p.route_points, vec!["반포", "잠실"]); assert!(p.weather.is_some());
    assert!(p.body.contains("반포 → 잠실 찍고 복귀"));
    assert!(http.calls().iter().filter(|u| u.contains("nominatim")).count() <= 5);
}
#[test] fn lookups_never_include_ends_or_photo_gps() {
    let (_t, mut lib, http) = temp_library_with_mock(); let id = import_1025(&mut lib);
    lib.enrich_activity(&id).unwrap();
    let route = route_of(&lib, &id); let (s, e) = (route[0], *route.last().unwrap());
    for (lat, lon) in http.queried_coords() { let p = LatLon { lat, lon };
        assert!(haversine_m(p, s) > 450.0 && haversine_m(p, e) > 450.0); }   // 반올림 오차 여유
}
#[test] fn second_enrich_uses_cache() {
    let (_t, mut lib, http) = temp_library_with_mock(); let id = import_1025(&mut lib);
    lib.enrich_activity(&id).unwrap(); let n = http.count("nominatim");
    lib.enrich_activity(&id).unwrap(); assert_eq!(http.count("nominatim"), n); }
#[test] fn user_place_survives_reenrich() {   // Review Focus 2
    let (_t, mut lib, _h) = temp_library_with_mock(); let id = import_1025(&mut lib);
    lib.set_place(&id, Some("뚝섬".into())).unwrap();
    let p = lib.enrich_activity(&id).unwrap();
    assert_eq!(p.place.as_ref().unwrap().name, "뚝섬"); assert!(!p.place.unwrap().estimated); assert!(p.place_user_set);
    assert_eq!(p.route_points, vec!["반포", "잠실"]); }
#[test] fn no_gps_activity_skips_network() {   // Review Focus 1
    let (_t, mut lib, http) = temp_library_with_mock(); let id = insert_no_gps_activity(&mut lib);
    let p = lib.enrich_activity(&id).unwrap();
    assert!(p.place.is_none() && p.weather.is_none()); assert_eq!(http.calls().len(), 0); }
#[test] fn network_failure_is_reported_not_fatal() {   // Review Focus 5
    let (_t, mut lib, _h) = temp_library_with_failing_http(); let id = import_1025(&mut lib);
    let p = lib.enrich_activity(&id).unwrap(); assert!(p.enrich_error.is_some()); assert!(p.place.is_none()); }
#[test] fn refine_without_key_sends_nothing() {   // Review Focus 4
    let (_t, mut lib, http) = temp_library_with_mock(); let id = import_1025(&mut lib);
    assert!(lib.refine_post(&id, false).is_err()); assert_eq!(http.count("anthropic"), 0); }
#[test] fn refine_returns_preview_without_saving() {
    let (_t, mut lib, http) = temp_library_with_mock(); let id = import_1025(&mut lib);
    lib.set_claude_key(Some("test-key".into())).unwrap();
    let before = lib.post_view(&id).unwrap().body;
    let r = lib.refine_post(&id, true).unwrap();
    assert!(r.body.starts_with("일요일 오후")); assert_eq!(lib.post_view(&id).unwrap().body, before);
    let body = http.last_body("anthropic"); assert_eq!(body["messages"][0]["content"][0]["type"], "image");
    assert!(!body.to_string().contains("\"lat\"")); }
#[test] fn key_status_never_reveals_key() {
    let (_t, mut lib, _h) = temp_library_with_mock(); lib.set_claude_key(Some("sk-secret".into())).unwrap();
    let s = serde_json::to_string(&lib.claude_status().unwrap()).unwrap();
    assert!(s.contains("\"hasKey\":true") && !s.contains("sk-secret")); }
// store.rs
#[test] fn v3_flags_and_geocode_cache() { /* user_set 3개 왕복 + put/get geocode */ }
```
- [ ] **Step 2:** FAIL 확인.
- [ ] **Step 3:** 구현. 기존 v1/v2 DB가 v3로 올라가는지 `reopen_file_db_keeps_data`와 같은 방식의 테스트 1개 추가.
- [ ] **Step 4:** `cargo test --workspace` PASS, bindings 갱신 확인.
- [ ] **Step 5:** 커밋 `feat(service): place/weather enrichment, manual overrides, Claude refine`.

---

### Task 5: 카드 경로선 출발·도착 가리기 (설정)

**Files:** Modify `crates/card/src/data.rs`, `crates/service/src/{import.rs,views.rs}`; Test `crates/card/tests/card.rs`(추가), `crates/service/tests/enrich.rs`(추가)

**Interfaces:**
```rust
// card
pub fn card_data_with(a: &Activity, trim_m: f64) -> CardData   // trim_m > 0이면 privacy와 같은 기준으로 앞·뒤를 잘라낸 경로
pub fn card_data(a: &Activity) -> CardData                      // = card_data_with(a, 0.0)
// service
pub fn set_card_route_trim(&mut self, on: bool) -> Result<()>  // settings "card_route_trim" ("500" | 없음)
// ClaudeStatusDto와 별개로 SettingsDto { card_route_trim: bool } 를 claude_status 옆에 추가: pub fn settings(&self) -> Result<SettingsDto>
```
기본값은 **꺼짐(지금처럼 전체 경로 표시)** — 2026-11-14 사용자 결정. 설정에서 켤 수 있다. 켜면 새로 만드는 카드부터 적용되고, 게시 준비 화면에 "카드 다시 만들기" 버튼(`regenerate_cards(id)`)으로 기존 카드도 다시 그린다.

- [ ] **Step 1: 실패하는 테스트**
```rust
#[test] fn trimmed_card_route_hides_ends() {
    let a = parse_fit(&read(FIT_1025)).unwrap();
    let full = card_data(&a); let cut = card_data_with(&a, 500.0);
    assert!(cut.route.len() < full.route.len() && cut.route.len() > 100);
    assert_ne!(cut.route[0], full.route[0]); }
#[test] fn regenerate_cards_respects_setting() {   // service
    let (_t, mut lib, _h) = temp_library_with_mock(); let id = import_1025(&mut lib);
    lib.set_card_route_trim(true).unwrap(); let n = lib.regenerate_cards(&id).unwrap();
    assert_eq!(n, 2); assert!(lib.settings().unwrap().card_route_trim); }
```
- [ ] **Step 2:** FAIL 확인. **Step 3:** 구현. **Step 4:** PASS. **Step 5:** 커밋 `feat(card): optional route trimming for privacy`.

---

### Task 6: Tauri 명령과 화면

**Files:** Modify `src-tauri/src/main.rs`, `src-tauri/tauri.conf.json`(CSP 변경 없음 — 외부 호출은 Rust에서만), `ui/src/lib/api.ts`, `ui/src/lib/mock.ts`, `ui/src/components/PostPanel.svelte`, `ui/src/App.svelte`; Create `ui/src/lib/enrich.ts`, `ui/src/components/{RefinePreview,Settings}.svelte`; Test `ui/src/lib/enrich.test.ts`, `ui/src/lib/api.test.ts`(추가)

**Interfaces:**
- 새 명령: `enrich_activity(id)`, `set_place(activityId, name)`, `set_route_points(activityId, points)`, `set_weather(activityId, weather)`, `refine_post(activityId, includePhotos)`, `set_claude_key(key)`, `claude_status()`, `set_claude_model(model)`, `set_card_route_trim(on)`, `settings()`, `regenerate_cards(activityId)` — 모두 2단계와 같은 `run(spawn_blocking)` 패턴.
- `enrich.ts`:
```ts
export function parseRoutePoints(input: string): string[]   // "반포 → 잠실, 뚝섬" → ["반포","잠실","뚝섬"] (→ , 공백 구분, 빈값·중복 제거)
export function routeText(points: string[]): string         // ["반포","잠실"] → "반포 → 잠실"
export function weatherText(w: WeatherDto | null): string   // {맑음, 17.4} → "맑음 17도", null → ""
export function parseWeather(input: string): WeatherDto | null   // "맑음 17도" / "맑음, 17" → {맑음, 17}, 숫자 없으면 null
```
- 화면:
  - `PostPanel`: 본문 위에 장소(추정이면 "추정" 배지, 직접 고치면 배지 사라짐), 경로(`반포 → 잠실` 입력), 날씨(`맑음 17도` 입력) 세 칸. 칸을 고치고 벗어나면 저장(`set_*`). 칸 옆 작은 "자동으로" 링크로 직접 고침 해제. "장소·날씨 다시 가져오기" 버튼(진행 중 표시, `enrichError` 있으면 그 문구). "Claude로 다듬기" 버튼 + "사진도 보내기" 체크(기본 꺼짐, 켜면 "사진이 Anthropic 서버로 전송돼요" 안내).
  - `RefinePreview`: 현재 글과 다듬은 글을 나란히, [적용](→ `savePost`) [취소]. 오류(키 없음·거절·잘림)는 패널 안 문구.
  - `Settings`(대시보드 상단 "설정"): Claude API 키 입력(저장 후 칸 비움, "저장됨" 표시만), 키 지우기, 모델 입력(기본 `claude-opus-5-5`), 카드 경로선 출발·도착 가리기 토글 + 설명.
  - 가져오기 후: 새로 추가된 운동마다 `enrich_activity`를 순서대로 호출(백그라운드, 상단에 "장소·날씨 채우는 중 n/N").
- `mockApi`에 새 메서드(가짜 장소 "가짜공원", 경로 ["가짜동"], 날씨 맑음 17).

- [ ] **Step 0:** `frontend-design` 스킬 불러오기 (2단계 시각 언어 유지).
- [ ] **Step 1: 실패하는 테스트**
```ts
it('parseRoutePoints', () => { expect(parseRoutePoints('반포 → 잠실, 뚝섬  잠실')).toEqual(['반포','잠실','뚝섬']); expect(parseRoutePoints(' ')).toEqual([]) })
it('routeText', () => { expect(routeText(['반포','잠실'])).toBe('반포 → 잠실'); expect(routeText([])).toBe('') })
it('weather text roundtrip', () => { expect(weatherText({ description: '맑음', tempC: 17.4 })).toBe('맑음 17도');
  expect(parseWeather('맑음 17도')).toEqual({ description: '맑음', tempC: 17 }); expect(parseWeather('맑음, -3')).toEqual({ description: '맑음', tempC: -3 }); expect(parseWeather('맑음')).toBeNull() })
it('mock enrich fills place', async () => { expect((await mockApi.enrichActivity('m3')).place?.name).toBe('가짜공원') })
```
- [ ] **Step 2:** FAIL. **Step 3:** 구현. **Step 4:** vitest·svelte-check·`npm run build`·`cargo build -p workout-desktop` 통과.
- [ ] **Step 5: 브라우저 확인(mock):** 게시 준비 화면에서 장소 고치기 → 배지 사라짐 → 다시 가져오기 후에도 유지, Claude 다듬기 미리보기 → 적용 시 본문 교체.
- [ ] **Step 6:** 커밋 `feat(ui): place/weather fields, Claude refine preview, settings`.

---

### Task 7: 실제 서비스로 확인 (사용자 확인 후)

- [ ] **Step 1:** 사용자에게 확인: "20261025 라이딩으로 Nominatim(중간 지점 최대 5개)·Open-Meteo(반올림 1개 좌표) 실제 조회를 해도 될지".
- [ ] **Step 2:** 승인되면 `cargo run -p workout-service --example enrich -- "<앱 라이브러리>" <운동 id>` (Task 4에서 `examples/enrich.rs` 추가) → 장소·경로·날씨 출력. 결과 장소명이 그럴듯한지 사용자에게 보여 주고 `area_name` 규칙 조정이 필요한지 묻는다.
- [ ] **Step 3:** Claude 호출 확인: 사용자가 앱 설정에서 직접 API 키를 넣고 "Claude로 다듬기"를 눌러 보도록 안내(키는 Claude가 다루지 않음). 결과가 초안 A 규칙(반말체, 숫자 유지, 출발·도착 이름 없음)을 지키는지 함께 본다.
- [ ] **Step 4:** 발견한 규칙 수정은 각각 실패 테스트 → 수정 → 커밋.

---

## 완료 기준

- `cargo test --workspace`, `npx vitest run`, clippy, fmt, svelte-check, `npm run build`, `cargo build -p workout-desktop` 통과.
- 테스트는 모두 오프라인(고정 JSON, MockHttp, MemoryStore). 실제 외부 호출은 Task 7에서 사용자 확인 후에만.
- 외부로 나가는 요청 본문·URL 어디에도 시작·끝 500m 안 좌표, 사진 GPS, API 키(Anthropic 헤더 제외)가 없다는 테스트가 있다.
