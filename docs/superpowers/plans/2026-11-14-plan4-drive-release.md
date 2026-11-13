# 4단계: 구글 로그인·드라이브 백업·3개 OS 빌드 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 구글 계정으로 로그인하면 라이브러리(파일 + DB 스냅샷)가 구글 드라이브에 자동으로 백업되고, 새 PC의 빈 라이브러리로 복원할 수 있다. mac·Windows·Linux 설치 파일을 CI에서 만든다.

**Architecture:** 새 crate `workout-gdrive`가 OAuth(설치형 앱 흐름: 시스템 브라우저 + `127.0.0.1` 임시 포트 + PKCE)와 Drive v3 호출을 맡고, `DriveApi` 트레이트 뒤에 HTTP 구현과 테스트용 메모리 구현(`FakeDrive`)을 둔다. `workout-service`는 백업을 3단계(`prepare_backup` 잠금 → `BackupJob::run` 잠금 없음 → `apply_backup` 잠금)로 나눠 3단계의 장소 조회처럼 화면이 멈추지 않게 한다. Tauri는 10분마다 "백업이 필요한지"를 확인하는 백그라운드 작업과 로그인·백업·복원 명령을 둔다. HTTP 계층(`workout-enrich::Http`)은 일반 요청(`send`)으로 넓혀 OAuth·Drive도 같이 쓴다.

**Tech Stack:** 기존 `reqwest`(blocking, rustls)·`keyring`·`rusqlite`에 `sha2`, `rand`(PKCE·state), `url`(쿼리 파싱) 추가. GitHub Actions + `tauri-apps/tauri-action`(서명 없음).

**Spec:** `docs/superpowers/specs/2026-11-13-workout-threads-app-design.md` §6.8, §8-1 (로드맵: `docs/superpowers/plans/2026-11-13-roadmap.md`)

## Global Constraints

- 1–3단계 Global Constraints 그대로 유효.
- OAuth: 사용자가 만든 Google Cloud "데스크톱 앱" OAuth 클라이언트의 **client ID·client secret**을 앱 설정에 넣는다(설계 §8-1: 데스크톱 앱의 secret은 비밀로 취급되지 않음, 보호는 PKCE S256 + state). 권한 범위는 **`https://www.googleapis.com/auth/drive.file` 하나만**.
- refresh token·access token은 **OS 키 보관함에만** 저장(DB·로그·화면 금지). client ID·secret은 settings 테이블.
- 로그인 비밀번호·2단계 인증은 사용자가 브라우저에서 직접 한다. 앱(그리고 Claude)은 비밀번호를 다루지 않는다.
- 드라이브 구조: `내 드라이브/WorkoutApp/` 아래 `files/YYYYMMDD/…`(로컬과 같은 상대 경로) + `library.db`(스냅샷). 앱이 만든 파일만 다룬다(drive.file).
- 백업은 **로컬 → 드라이브 단방향**. 로컬 파일은 절대 지우지 않는다. 드라이브의 파일도 지우지 않는다(덮어쓰기만).
- 복원은 **로컬 라이브러리가 비어 있을 때만**(운동 0개, 사진 0개) 허용. 아니면 거절하고 안내만.
- 백업 시점: (1) 앱 시작 시, (2) 앱이 켜져 있는 동안 10분마다 "바뀐 것이 있으면", (3) 마지막 백업 후 24시간이 지났으면, (4) "지금 백업" 버튼. 실패하면 다음 확인 때 다시 시도하고, 화면에 마지막 성공 시각·마지막 오류를 보여 준다.
- DB 스냅샷은 `VACUUM INTO '<임시 파일>'`로 만든 일관된 사본을 올린다(열려 있는 DB 파일을 직접 올리지 않음).
- CI 저장소에는 사용자 실제 FIT·사진을 **절대 넣지 않는다**(GPS로 집 위치 노출). 실제 샘플이 필요한 테스트는 샘플 폴더가 없으면 건너뛴다.
- 네트워크 호출은 모두 `Http` 트레이트를 통한다. 테스트는 오프라인(MockHttp, FakeDrive).
- 실제 구글 로그인·드라이브 업로드·GitHub 저장소 생성/푸시는 **Task 8에서 사용자 확인 후**에만.

## Review Focus

1. **로그인 도중 창을 닫거나 브라우저에서 거부** — 임시 포트가 5분 안에 닫히고, 오류 문구("로그인이 취소됐어요")로 끝나며 앱이 멈추지 않는다. state가 다르면 거절. → Task 3 테스트.
2. **access token 만료(401) 중간 발생** — refresh token으로 한 번 갱신 후 재시도, refresh도 실패(`invalid_grant`, 7일 만료 등)하면 "다시 로그인해 주세요"로 표시하고 토큰을 지운다. → Task 4·5 테스트.
3. **백업 도중 네트워크 끊김** — 이미 올라간 파일은 기록되고, 나머지는 다음 확인 때 이어서 올린다. 같은 파일이 드라이브에 두 번 생기지 않는다(드라이브 id 기록 후 PATCH). → Task 5 테스트.
4. **복원 대상 라이브러리에 데이터가 있음** — 거절, 로컬은 손대지 않음. 복원 중 실패하면 반쯤 내려받은 파일을 지우고 빈 라이브러리로 돌려 둔다. → Task 5 테스트.
5. **샘플 폴더 없는 환경(CI)에서 테스트** — 샘플이 필요한 테스트는 건너뛰고 나머지는 모두 통과. → Task 7.

---

## 파일 구조

```
crates/enrich/src/lib.rs            (수정) HttpRequest, Http::send + 기존 get/post_json 기본 구현, post_form
crates/enrich/src/mock.rs           (수정) send 기록, 메서드·본문 확인 도우미
crates/gdrive/                      (신규 crate workout-gdrive)
  src/lib.rs                        DriveError, Tokens
  src/oauth.rs                      Pkce, auth_url, parse_callback, LoopbackServer, exchange_code, refresh
  src/drive.rs                      DriveApi 트레이트, HttpDrive, FakeDrive(feature "fake")
  tests/fixtures/*.json             토큰·files.list 응답 예시
crates/service/src/secrets.rs       (수정) SecretStore를 이름 있는 키로: get(name)/set(name, v)
crates/service/src/backup.rs        (신규) 로그인·백업·복원·상태
crates/store/src/schema.rs          (수정) v5: sync_files, sync_folders, sync_state
src-tauri/src/main.rs               (수정) 로그인·백업·복원 명령, 10분 백그라운드 확인
src-tauri/capabilities/default.json (수정) opener:allow-open-url (accounts.google.com)
ui/src/components/Settings.svelte   (수정) "구글 드라이브 백업" 구역
ui/src/components/Dashboard.svelte  (수정) 마지막 백업 표시
ui/src/lib/backup.ts                (신규) backupStatusText
.github/workflows/build.yml         (신규) 3 OS 테스트·빌드
docs/setup-google-oauth.md          (신규) 클라이언트 만드는 방법(한국어)
```

---

### Task 1: HTTP 계층 넓히기 (`workout-enrich`)

**Files:** Modify `crates/enrich/src/{lib.rs,mock.rs}`; Test `crates/enrich/tests/http.rs`

**Interfaces:**
```rust
pub enum Method { Get, Post, Patch }
pub enum Body { Empty, Json(serde_json::Value), Form(Vec<(String, String)>), Bytes { content_type: String, data: Vec<u8> } }
pub enum TimeoutClass { Lookup /* 15s */, Long /* 120s: Claude, 업로드·다운로드 */ }
pub struct HttpRequest { pub method: Method, pub url: String, pub headers: Vec<(String, String)>, pub body: Body, pub timeout: TimeoutClass }
pub trait Http: Send + Sync {
    fn send(&self, req: HttpRequest) -> Result<HttpResponse, EnrichError>;
    fn get(&self, url: &str, headers: &[(&str, &str)]) -> Result<HttpResponse, EnrichError> { /* 기본 구현: send(Get, Lookup) */ }
    fn post_json(&self, url: &str, headers: &[(&str, &str)], body: &Value) -> Result<HttpResponse, EnrichError> { /* send(Post, Json, Long) */ }
}
// MockHttp: send를 기록 (Call에 method 추가), last_form(needle) -> Vec<(String,String)>, last_bytes(needle) -> Vec<u8>
```
`ReqwestHttp::send`: Form은 `application/x-www-form-urlencoded`, Bytes는 주어진 content-type. `Authorization` 헤더도 `x-api-key`처럼 sensitive 처리.

- [ ] **Step 1: 실패하는 테스트**: MockHttp로 `send(Post, Form)` 기록이 남고 `last_form`이 값을 돌려줌; `get`·`post_json` 기본 구현이 각각 `Get+Lookup`, `Post+Json+Long`으로 기록됨; 기존 enrich·service 테스트 전부 통과(호환).
- [ ] **Step 2:** FAIL 확인. **Step 3:** 구현. **Step 4:** `cargo test --workspace` PASS. **Step 5:** 커밋 `refactor(enrich): general HTTP requests`.

---

### Task 2: 이름 있는 비밀 저장 (`workout-service`)

**Files:** Modify `crates/service/src/{secrets.rs,enrich.rs}`, `crates/service/tests/common.rs`; Test `crates/service/tests/secrets.rs`

**Interfaces:**
```rust
pub const CLAUDE_KEY: &str = "anthropic_api_key";
pub const GOOGLE_REFRESH: &str = "google_refresh_token";
pub const GOOGLE_ACCESS: &str = "google_access_token";   // "토큰|만료unix" 형식
pub trait SecretStore: Send + Sync { fn get(&self, name: &str) -> Option<String>; fn set(&self, name: &str, value: Option<&str>) -> Result<()>; }
```
`KeyringStore`는 계정 이름 = `name`. 기존 Claude 키 저장 위치(계정 `anthropic_api_key`)와 같으므로 사용자 키는 그대로 유지된다.

- [ ] **Step 1: 실패하는 테스트**: `MemoryStore`에 두 이름을 따로 저장·삭제; `set_claude_key` 후 구글 토큰 이름에는 값이 없음; 기존 `key_status_never_reveals_key` 통과.
- [ ] **Step 2–4:** FAIL → 구현 → PASS. **Step 5:** 커밋 `refactor(service): named secrets`.

---

### Task 3: 구글 OAuth (`workout-gdrive::oauth`)

**Files:** Create `crates/gdrive/{Cargo.toml,src/lib.rs,src/oauth.rs}`, `tests/fixtures/{token_ok.json,token_invalid_grant.json}`; Test `crates/gdrive/tests/oauth.rs`

**Interfaces:**
```rust
pub const SCOPE: &str = "https://www.googleapis.com/auth/drive.file";
pub const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
pub struct ClientConfig { pub client_id: String, pub client_secret: String }
pub struct Pkce { pub verifier: String /* 64자 URL-safe */, pub challenge: String /* S256 base64url, 패딩 없음 */ }
impl Pkce { pub fn new() -> Self; pub fn from_verifier(v: &str) -> Self }
pub fn new_state() -> String                      // 32자 무작위
pub fn auth_url(cfg: &ClientConfig, redirect: &str, pkce: &Pkce, state: &str) -> String
    // response_type=code, scope=SCOPE, code_challenge(_method=S256), state, access_type=offline, prompt=consent
pub enum Callback { Code(String), Denied(String) }
pub fn parse_callback(request_line: &str, expected_state: &str) -> Result<Callback, DriveError>   // "GET /?code=..&state=.. HTTP/1.1"
pub struct LoopbackServer { /* TcpListener 127.0.0.1:0 */ }
impl LoopbackServer { pub fn bind() -> Result<Self, DriveError>; pub fn redirect_uri(&self) -> String; pub fn wait(&self, state: &str, timeout: Duration) -> Result<String /* code */, DriveError> }
    // 응답 본문: 한국어 HTML "로그인 완료. 앱으로 돌아가세요." / 거부 시 "로그인이 취소됐어요."
pub struct Tokens { pub access: String, pub expires_at: i64, pub refresh: Option<String> }
pub fn exchange_code(http: &dyn Http, cfg: &ClientConfig, code: &str, redirect: &str, pkce: &Pkce, now: i64) -> Result<Tokens, DriveError>
pub fn refresh(http: &dyn Http, cfg: &ClientConfig, refresh_token: &str, now: i64) -> Result<Tokens, DriveError>
#[derive(thiserror::Error)] pub enum DriveError { Http(EnrichError), Status(u16, String), Parse(String), Cancelled, StateMismatch, Timeout, NeedLogin, NotEmpty, NotFound(String) }
```
`invalid_grant` 응답 → `NeedLogin`.

- [ ] **Step 1: 실패하는 테스트**
```rust
#[test] fn pkce_matches_rfc7636_example() {   // RFC 7636 부록 B
    let p = Pkce::from_verifier("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk");
    assert_eq!(p.challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"); }
#[test] fn auth_url_has_required_params() { /* scope 인코딩, code_challenge_method=S256, access_type=offline, redirect_uri=127.0.0.1 */ }
#[test] fn callback_parsing() {
    assert!(matches!(parse_callback("GET /?code=abc&state=s1 HTTP/1.1", "s1"), Ok(Callback::Code(c)) if c == "abc"));
    assert!(matches!(parse_callback("GET /?error=access_denied&state=s1 HTTP/1.1", "s1"), Ok(Callback::Denied(_))));
    assert!(matches!(parse_callback("GET /?code=abc&state=other HTTP/1.1", "s1"), Err(DriveError::StateMismatch))); }
#[test] fn loopback_receives_code() {   // 실제 127.0.0.1 소켓, 외부 네트워크 없음
    let srv = LoopbackServer::bind().unwrap(); let uri = srv.redirect_uri();
    std::thread::spawn(move || { let mut s = TcpStream::connect(uri.trim_start_matches("http://")).unwrap();
        s.write_all(b"GET /?code=xyz&state=st HTTP/1.1\r\nHost: x\r\n\r\n").unwrap(); });
    assert_eq!(srv.wait("st", Duration::from_secs(5)).unwrap(), "xyz"); }
#[test] fn loopback_times_out() { let srv = LoopbackServer::bind().unwrap();
    assert!(matches!(srv.wait("st", Duration::from_millis(200)), Err(DriveError::Timeout))); }
#[test] fn exchange_sends_pkce_and_parses_tokens() { /* MockHttp: last_form에 code_verifier·grant_type=authorization_code; Tokens.refresh = Some */ }
#[test] fn refresh_invalid_grant_needs_login() { /* token_invalid_grant.json (400) → DriveError::NeedLogin */ }
```
- [ ] **Step 2–4:** FAIL → 구현 → PASS. **Step 5:** 커밋 `feat(gdrive): installed-app OAuth with PKCE`.

---

### Task 4: 드라이브 API (`workout-gdrive::drive`)

**Files:** Create `crates/gdrive/src/drive.rs`, `tests/fixtures/{files_list_folder.json,files_list_empty.json,file_created.json}`; Test `crates/gdrive/tests/drive.rs`

**Interfaces:**
```rust
pub struct RemoteFile { pub id: String, pub name: String, pub is_folder: bool }
pub trait DriveApi: Send + Sync {
    fn find_folder(&self, name: &str, parent: Option<&str>) -> Result<Option<String>, DriveError>;
    fn create_folder(&self, name: &str, parent: Option<&str>) -> Result<String, DriveError>;
    fn upload_new(&self, parent: &str, name: &str, content_type: &str, data: &[u8]) -> Result<String, DriveError>;  // multipart
    fn update(&self, id: &str, content_type: &str, data: &[u8]) -> Result<(), DriveError>;                          // PATCH uploadType=media
    fn list(&self, parent: &str) -> Result<Vec<RemoteFile>, DriveError>;                                               // 페이지 모두
    fn download(&self, id: &str) -> Result<Vec<u8>, DriveError>;                                                       // alt=media
}
/// 토큰을 들고 Drive v3를 부르는 구현. 401이면 refresh 콜백으로 한 번 갱신 후 재시도.
pub struct HttpDrive { /* http, access token(Mutex), refresh: Box<dyn Fn() -> Result<Tokens, DriveError> + Send + Sync> */ }
impl HttpDrive { pub fn new(http: Arc<dyn Http>, tokens: Tokens, refresh: Box<dyn Fn() -> Result<Tokens, DriveError> + Send + Sync>) -> Self; pub fn current_tokens(&self) -> Tokens }
#[cfg(feature = "fake")] pub struct FakeDrive { /* 메모리 파일 트리, 실패 주입: fail_after(n) */ }
```
쿼리: `q = "name='{name}' and mimeType='application/vnd.google-apps.folder' and '{parent|root}' in parents and trashed=false"`, `fields=files(id,name,mimeType),nextPageToken`. 이름의 `'`·`\`는 이스케이프.

- [ ] **Step 1: 실패하는 테스트** (MockHttp)
```rust
#[test] fn find_folder_builds_query() { /* URL에 q·fields·root, 결과 id */ }
#[test] fn upload_new_is_multipart() { /* uploadType=multipart, 본문에 메타데이터 JSON(name, parents)과 바이트 */ }
#[test] fn update_uses_patch_media() { /* Method::Patch, uploadType=media */ }
#[test] fn list_follows_next_page_token() { /* 두 페이지 합침 */ }
#[test] fn retries_once_after_401_with_refreshed_token() {   // Review Focus 2
    /* 첫 응답 401 → refresh 콜백 1회 → 새 토큰의 Authorization으로 재시도 성공, current_tokens 갱신 */ }
#[test] fn second_401_needs_login() { /* refresh가 NeedLogin → 그대로 NeedLogin */ }
#[test] fn name_with_quote_is_escaped() { /* "a'b" → a\'b */ }
```
- [ ] **Step 2–4:** FAIL → 구현 → PASS (FakeDrive도 같은 트레이트 테스트 1개). **Step 5:** 커밋 `feat(gdrive): Drive v3 client`.

---

### Task 5: 백업·복원 엔진 (`workout-service`)

**Files:** Modify `crates/store/src/{schema.rs,lib.rs}`, `crates/service/src/{lib.rs,dto.rs}`; Create `crates/service/src/backup.rs`; Test `crates/store/tests/store.rs`(추가), `crates/service/tests/backup.rs`

**Interfaces:**
- Store v5: `sync_files(path TEXT PK, drive_id TEXT NOT NULL, size INT NOT NULL, mtime INT NOT NULL)`, `sync_folders(path TEXT PK, drive_id TEXT NOT NULL)`, `sync_state(key TEXT PK, value TEXT)` (키: `last_success`, `last_error`, `db_hash`). Store 메서드: `synced_file(path)`, `put_synced_file(path, id, size, mtime)`, `synced_folder(path)`, `put_synced_folder(path, id)`, `sync_value(key)`, `set_sync_value(key, Option<&str>)`, `snapshot_to(path)`(`VACUUM INTO`), `is_empty()`(운동·사진 0).
```rust
// dto.rs
pub struct BackupStatusDto { pub configured: bool /* client id 있음 */, pub signed_in: bool, pub last_success: Option<String> /* RFC3339 */,
                             pub last_error: Option<String>, pub pending: u32 /* 아직 안 올린 파일 수 */, pub can_restore: bool /* 로컬 비어 있음 */ }
pub struct GoogleClientDto { pub client_id: String, pub has_secret: bool }
// backup.rs
impl Library {
    pub fn set_google_client(&mut self, client_id: Option<String>, client_secret: Option<String>) -> Result<()>;
    pub fn google_client(&self) -> Result<Option<GoogleClientDto>>;
    pub fn begin_login(&self) -> Result<LoginJob>;            // Pkce·state·LoopbackServer·auth_url 준비 (잠금 짧게)
    pub fn finish_login(&mut self, tokens: Tokens) -> Result<BackupStatusDto>;   // 토큰을 키 보관함에
    pub fn logout(&mut self) -> Result<()>;
    pub fn backup_status(&self) -> Result<BackupStatusDto>;
    pub fn needs_backup(&self, now: DateTime<Utc>) -> Result<bool>;   // 바뀐 파일·DB 해시 변화·24시간 경과
    pub fn prepare_backup(&self, now: DateTime<Utc>) -> Result<BackupJob>;   // 스냅샷 생성, 올릴 목록, 알려진 drive id
    pub fn apply_backup(&mut self, done: BackupDone) -> Result<BackupStatusDto>;  // 올린 것 기록, 상태 갱신, 새 토큰 저장
    pub fn backup_now(&mut self, now: DateTime<Utc>) -> Result<BackupStatusDto>;  // prepare→run→apply
    pub fn restore_from_drive(root: &Path, drive: &dyn DriveApi, ...) -> Result<Library>;   // 빈 라이브러리일 때만, 실패 시 정리
}
pub struct LoginJob { pub auth_url: String, /* server, pkce, state, cfg, http */ }
impl LoginJob { pub fn run(self, timeout: Duration) -> Result<Tokens> }      // 브라우저 열기는 호출하는 쪽(Tauri)
pub struct BackupJob { /* drive: Arc<dyn DriveApi>, items: Vec<(rel, abs, size, mtime, known_id)>, snapshot: PathBuf, folder ids */ }
impl BackupJob { pub fn run(self) -> BackupDone }   // 실패해도 그때까지 올린 것을 담아 돌려줌
```
`Library`는 `drive_factory: Arc<dyn Fn(Tokens) -> Arc<dyn DriveApi>>`를 가진다(실제: HttpDrive, 테스트: FakeDrive). 대상 파일: `files/` 아래 전부(사진·카드·FIT·post.txt) 중 `sync_files`에 없거나 size·mtime이 바뀐 것 + `library.db` 스냅샷(해시가 바뀌었을 때).

- [ ] **Step 1: 실패하는 테스트** (FakeDrive, MemoryStore, 가져오기는 실제 샘플)
```rust
#[test] fn first_backup_uploads_everything_in_tree() { /* import 20261025 → backup_now → FakeDrive에 WorkoutApp/files/20261025/{fit, 사진2, 카드2} + library.db; status.pending == 0, last_success Some */ }
#[test] fn second_backup_uploads_only_changes() { /* 아무것도 안 바꾸면 업로드 0, 글 저장 후엔 library.db만 update */ }
#[test] fn needs_backup_rules() { /* 직후 false, 파일 추가 후 true, 24시간 뒤 true */ }
#[test] fn interrupted_backup_resumes_without_duplicates() {   // Review Focus 3
    /* FakeDrive.fail_after(2) → 일부만 기록, last_error Some → 다시 backup_now → 드라이브 파일 수 == 로컬 파일 수 (중복 없음) */ }
#[test] fn expired_login_asks_to_sign_in_again() {   // Review Focus 2
    /* drive가 NeedLogin → status.signed_in false, last_error "다시 로그인해 주세요", 키 보관함 토큰 삭제 */ }
#[test] fn restore_into_empty_library() { /* 백업된 FakeDrive → 새 폴더에 restore → month_view(2026,10) 라이딩 1회, 카드 파일 존재 */ }
#[test] fn restore_refused_when_not_empty() {   // Review Focus 4
    /* 데이터 있는 라이브러리 → DriveError::NotEmpty 기반 ServiceError, 로컬 파일 수 그대로 */ }
#[test] fn failed_restore_leaves_empty_library() { /* 다운로드 도중 실패 → 내려받은 파일 삭제, is_empty */ }
#[test] fn tokens_never_in_status_or_db() { /* status JSON·DB 덤프에 토큰 문자열 없음 */ }
```
- [ ] **Step 2–4:** FAIL → 구현 → PASS. 기존 v4 DB가 v5로 오르는지 실제 앱 라이브러리로 `user_version` 확인.
- [ ] **Step 5:** 커밋 `feat(service): Google Drive backup and restore`.

---

### Task 6: Tauri 명령과 화면

**Files:** Modify `src-tauri/src/main.rs`, `src-tauri/capabilities/default.json`, `ui/src/lib/{api.ts,mock.ts}`, `ui/src/components/{Settings.svelte,Dashboard.svelte}`, `ui/src/App.svelte`; Create `ui/src/lib/backup.ts`; Test `ui/src/lib/backup.test.ts`

**Interfaces:**
- 명령: `set_google_client(clientId, clientSecret)`, `google_client()`, `google_login()`(잠금으로 `begin_login` → `opener::open_url(auth_url)` → 잠금 없이 `LoginJob::run(5분)` → 잠금으로 `finish_login`), `google_logout()`, `backup_status()`, `backup_now()`(잠금 없이 run), `restore_from_drive()`(빈 라이브러리일 때만, 끝나면 `Mutex` 안의 `Library` 교체).
- 백그라운드: `setup`에서 `tauri::async_runtime::spawn` 루프 — 시작 직후 1회, 이후 10분마다 `needs_backup` → 참이면 백업(잠금 없이 업로드). 결과는 이벤트 `backup-status`로 화면에 알림.
- capability: `{ "identifier": "opener:allow-open-url", "allow": [{ "url": "https://accounts.google.com/*" }] }`.
- `backup.ts`:
```ts
export function backupStatusText(s: BackupStatusDto, now: Date): string
// 미설정 → "드라이브 백업이 꺼져 있어요" / 로그인 전 → "구글 로그인이 필요해요" / 오류 → "백업 실패: …" / 성공 → "마지막 백업 3분 전" · "어제 21:40"
```
- 화면: Settings에 "구글 드라이브 백업" 구역(클라이언트 ID·secret 입력 + 만드는 법 링크 `docs/setup-google-oauth.md` 요약, [구글로 로그인]/[로그아웃], 상태, [지금 백업], 빈 라이브러리일 때만 [드라이브에서 복원] + 확인 대화). Dashboard 상단 오른쪽에 작은 상태 문구(`backupStatusText`), 오류면 빨간 점.

- [ ] **Step 0:** frontend-design 스킬 지침 유지(기존 시각 언어).
- [ ] **Step 1: 실패하는 테스트**
```ts
it('not configured', () => expect(backupStatusText({ configured: false, signedIn: false, lastSuccess: null, lastError: null, pending: 0, canRestore: false }, now)).toBe('드라이브 백업이 꺼져 있어요'))
it('needs login', ...)  it('error first', ...)  it('minutes ago', () => ... '마지막 백업 3분 전')  it('yesterday', () => ... '어제 21:40')
```
- [ ] **Step 2–4:** FAIL → 구현 → vitest·svelte-check·build·`cargo build -p workout-desktop` 통과.
- [ ] **Step 5: 브라우저 확인(mock):** 설정 → 클라이언트 입력 → 로그인(가짜) → 지금 백업 → 대시보드 상태 문구 갱신.
- [ ] **Step 6:** 커밋 `feat(ui): Google Drive backup settings and status`.

---

### Task 7: 3개 OS CI와 설정 안내

**Files:** Create `.github/workflows/build.yml`, `docs/setup-google-oauth.md`; Modify 실제 샘플을 쓰는 테스트 도우미(`crates/*/tests/common.rs` 등 — 샘플이 없으면 건너뛰는 `samples_or_skip!()`)

**Interfaces:**
```rust
// crates/core/tests/common.rs 등 각 crate 테스트 도우미
/// 실제 샘플 폴더가 없으면(예: CI) 이 테스트를 건너뛴다.
#[macro_export] macro_rules! samples_or_skip { () => { if !std::path::Path::new(SAMPLES).join("20261025").exists() { eprintln!("샘플 없음: 건너뜀"); return; } }; }
```
워크플로: `on: push, pull_request, workflow_dispatch`; matrix `macos-latest`, `windows-latest`, `ubuntu-22.04`; Ubuntu는 `libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libdbus-1-dev pkg-config` 설치; Node 24, Rust stable; `npm ci --prefix ui`; `cargo test --workspace`; `npm --prefix ui test`; `tauri-apps/tauri-action@v0`로 번들(서명 없음) → artifact 업로드(`.dmg`, `.msi`, `.AppImage`, `.deb`).
`docs/setup-google-oauth.md`(한국어): Google Cloud 프로젝트 만들기 → Drive API 사용 설정 → OAuth 동의 화면(외부, 앱 이름, 본인 이메일을 테스트 사용자로) → 사용자 인증 정보 → OAuth 클라이언트 ID → 유형 "데스크톱 앱" → ID·secret을 앱 설정에 붙여넣기. **주의:** 동의 화면이 "테스트" 상태면 refresh token이 7일 뒤 만료되어 다시 로그인해야 한다. `drive.file`은 검증이 필요 없는 범위로 알려져 있으므로 "프로덕션으로 게시"하면 만료가 없어진다 — 실행 시점에 Google 문서로 이 내용을 확인하고 적는다.

- [ ] **Step 1:** 샘플이 필요한 모든 테스트 맨 위에 `samples_or_skip!();` 추가. 사용자 폴더는 건드리지 않고 **환경변수 `WORKOUT_SAMPLES`로 샘플 경로를 바꿔** `WORKOUT_SAMPLES=/nonexistent cargo test --workspace`가 전부 통과(건너뜀 포함)하는지 확인. `SAMPLES` 상수는 `option_env!` 대신 실행 시 `std::env::var("WORKOUT_SAMPLES")`가 있으면 그 값.
- [ ] **Step 2:** 워크플로·문서 작성. `actionlint`가 있으면 검사(없으면 YAML 문법만 `python3 -c "import yaml…"`로).
- [ ] **Step 3:** 커밋 `ci: build and test on macOS, Windows, Linux`.

---

### Task 8: 실제 확인 (사용자 확인 후)

- [ ] **Step 1:** 사용자에게 `docs/setup-google-oauth.md` 순서로 OAuth 클라이언트를 만들어 앱 설정에 넣어 달라고 안내.
- [ ] **Step 2:** 사용자가 앱에서 [구글로 로그인] → 브라우저에서 직접 로그인·허용. [지금 백업] 후 드라이브 `WorkoutApp` 폴더에 파일이 생겼는지 사용자가 확인(원하면 연결된 Google Drive 도구로 `WorkoutApp` 검색해 목록만 보여 줌 — 내용은 열지 않음).
- [ ] **Step 3:** CI: 사용자에게 **비공개** GitHub 저장소를 만들어 푸시할지 확인. 승인 시 `gh repo create <이름> --private --source . --push`(이름은 사용자에게 받음) → Actions 결과 확인 → 3 OS 산출물 링크 전달. 실패하면 로그를 보고 고친다.
- [ ] **Step 4:** 발견한 문제는 각각 실패 테스트 → 수정 → 커밋.

---

## 완료 기준

- `cargo test --workspace`, `WORKOUT_SAMPLES=/nonexistent cargo test --workspace`, vitest, clippy, fmt, svelte-check, `npm run build`, `cargo build -p workout-desktop` 통과.
- 토큰·secret이 DB·로그·화면 응답에 없다는 테스트가 있다.
- 백업은 로컬·드라이브 어느 쪽도 지우지 않는다.
- (Task 8 승인 시) 실제 드라이브 백업 1회 성공, CI 3 OS 녹색.
