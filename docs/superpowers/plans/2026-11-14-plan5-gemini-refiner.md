# 5단계: 글 다듬기 AI 선택 (Gemini / Claude) 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 게시 글 다듬기에 Gemini 키를 쓸 수 있게 하고, 설정에서 Gemini / Claude 중 하나를 고르게 한다(기본 Gemini).

**설계(승인됨, 2026-11-14):** `workout-enrich`에 `Refiner` 트레이트(공통 틀)를 두고 Claude·Gemini 구현이 시스템 지시문·사실 정보(좌표 없음)·해시태그 정리를 공유한다. 설정 `refine_provider`(기본 `gemini`), 키는 키 보관함 `gemini_api_key` / `anthropic_api_key`, 모델은 `gemini_model`(기본 `gemini-3.8-flash`) / `claude_model`. 화면은 "글 다듬기 AI" 선택 + 고른 AI의 키·모델, 게시 준비 버튼 "AI로 다듬기", 사진 전송 안내에 업체 이름. 비교 기능은 범위 밖.
접근 검토: (a) 선택형(채택) — 기존 Claude 코드·테스트 유지, 다른 AI 확장 쉬움 / (b) Gemini로 교체 — 단순하지만 기존 코드 삭제. 사용자가 (a)를 골랐다.

**Gemini 요청 형식 (공식 문서 2026-11-14 확인):** `POST https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent`, 헤더 `x-goog-api-key`. 본문 `{systemInstruction:{parts:[{text}]}, contents:[{role:"user", parts:[{inlineData:{mimeType:"image/jpeg", data:b64}}…, {text}]}], generationConfig:{responseMimeType:"application/json", responseJsonSchema:{…}}}`. 응답 `candidates[0].content.parts[*].text`, `candidates[0].finishReason`(`STOP`/`MAX_TOKENS`/`SAFETY`/`RECITATION`/`PROHIBITED_CONTENT`…), `promptFeedback.blockReason`. 오류 `{error:{code,message,status}}`.

## Global Constraints
- 1–4단계 제약 유지. 좌표·사진 GPS·키를 요청 본문/URL/로그/화면에 넣지 않는다 (Gemini 키는 URL 쿼리 `?key=`가 아니라 헤더로).
- 결과는 미리보기만, 저장하지 않는다. 테스트는 오프라인(MockHttp).
- 이 저장소의 커밋은 noreply 작성자(로컬 설정됨), 브랜치 `public-main` → GitHub `main`.

## Review Focus
1. Gemini가 JSON을 코드 펜스(```json … ```)로 감싸 주는 경우 → 벗겨서 읽는다. → Task 1.
2. `candidates`가 비고 `promptFeedback.blockReason`만 있는 차단 응답 → "거절" 오류. → Task 1.
3. 공급자를 바꿨는데 그 공급자 키가 없음 → 요청 없이 "설정에서 Gemini API 키를 넣어 주세요". → Task 2.
4. 기존 사용자의 Claude 키·모델 설정 → 그대로 유지되고, 공급자를 Claude로 고르면 이전처럼 동작. → Task 2.
5. 키가 URL에 들어가지 않음(로그·오류 문구 노출 방지). → Task 1.

### Task 1: 공통 틀 + Gemini (`workout-enrich`)
**Files:** Create `crates/enrich/src/{refine.rs,gemini.rs}`, fixtures `gemini_ok.json, gemini_fenced.json, gemini_blocked.json, gemini_safety.json, gemini_max_tokens.json`; Modify `claude.rs`, `lib.rs`; Test `crates/enrich/tests/gemini.rs`, 기존 `claude.rs` 테스트 유지.
```rust
// refine.rs
pub struct RefineInput { pub ctx: PostContext, pub draft: PostText, pub photos_jpeg: Vec<Vec<u8>> }   // claude.rs에서 이동, claude::RefineInput 재노출
pub fn system_prompt() -> &'static str          // claude.rs에서 이동
pub fn facts(ctx: &PostContext) -> Value          // 이동 (좌표 없음)
pub fn user_text(input: &RefineInput) -> String   // 사실 JSON + 초안 + 해시태그
pub fn output_schema() -> Value                   // {body, hashtags}
pub fn parse_output_json(text: &str) -> Result<PostText, EnrichError>   // 코드 펜스 제거, 해시태그 정리
#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum Provider { Gemini, Claude }
impl Provider { pub fn key_name(self) -> &'static str; pub fn default_model(self) -> &'static str; pub fn label(self) -> &'static str /* "Gemini" | "Claude" */; pub fn parse(s: &str) -> Option<Self> }
pub fn refine_with(provider: Provider, http: &dyn Http, key: &str, model: &str, input: &RefineInput) -> Result<PostText, EnrichError>
// gemini.rs
pub const DEFAULT_MODEL: &str = "gemini-3.8-flash";
pub fn endpoint(model: &str) -> String
pub fn build_request(input: &RefineInput) -> Value
pub fn parse_response(json: &[u8]) -> Result<PostText, EnrichError>
pub fn refine(http: &dyn Http, key: &str, model: &str, input: &RefineInput) -> Result<PostText, EnrichError>   // 401/403 → Status(code,"API 키를 확인해 주세요")
```
Tests: 요청(헤더 `x-goog-api-key`, URL에 키 없음, `responseMimeType`, `responseJsonSchema`, `systemInstruction`, 좌표 없음, 사진 켰을 때만 `inlineData`), 응답(정상·코드 펜스·`blockReason`·`SAFETY`·`MAX_TOKENS`·JSON 아님), 403 → 키 확인, `refine_with`가 공급자별 URL로 보냄. 커밋 `feat(enrich): Gemini refiner and shared refine core`.

### Task 2: 서비스 (`workout-service`)
**Files:** Modify `crates/service/src/{enrich.rs,dto.rs,secrets.rs}`; Test `crates/service/tests/refine_provider.rs`, 기존 enrich 테스트 갱신.
```rust
pub const GEMINI_KEY: &str = "gemini_api_key";   // secrets.rs
pub struct RefineStatusDto { pub provider: String /* "gemini"|"claude" */, pub label: String, pub has_key: bool, pub model: String,
                             pub providers: Vec<ProviderDto> }   // ProviderDto { id, label, has_key, model }
impl Library {
    pub fn set_refine_provider(&mut self, provider: &str) -> Result<()>;       // 알 수 없는 값 → Failed
    pub fn set_refine_key(&mut self, provider: &str, key: Option<String>) -> Result<()>;
    pub fn set_refine_model(&mut self, provider: &str, model: Option<String>) -> Result<()>;
    pub fn refine_status(&self) -> Result<RefineStatusDto>;
}
```
기존 `set_claude_key`, `set_claude_model`, `claude_status`는 새 함수로 옮기고 제거(호출부 Tauri·UI도 함께). `prepare_refine`는 선택된 공급자·키·모델로 `RefineJob`을 만든다. 키 없음 오류 문구에 공급자 이름. Tests: 기본 Gemini, Gemini 키로 Gemini URL 호출, Claude 선택 시 Anthropic URL, 공급자별 키 분리, 키 없으면 요청 0건 + 문구에 "Gemini", 상태 JSON에 키 값 없음, 기존 Claude 키 이름 유지. 커밋 `feat(service): choose refine provider`.

### Task 3: Tauri·화면
명령 `refine_status`, `set_refine_provider(provider)`, `set_refine_key(provider, key)`, `set_refine_model(provider, model)` (기존 claude 명령 제거). `api.ts`·`mock.ts` 갱신, Settings "글 다듬기 AI"(라디오 Gemini/Claude, 고른 AI의 키·모델), PostPanel 버튼 "AI로 다듬기"(툴팁에 공급자), 사진 안내 "{Google|Anthropic} 서버", RefinePreview 제목 "{label}가 다듬은 글". vitest: mock 공급자 전환·키 상태. 브라우저(mock) 확인. 커밋 `feat(ui): choose Gemini or Claude for refining`.

### 완료
cargo test(샘플 있음·없음), vitest, clippy, fmt, svelte-check, build, desktop build, 최종 리뷰, 푸시 후 CI 녹색.
