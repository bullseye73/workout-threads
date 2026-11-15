# Gemini 작업 요청 프롬프트 (2026-11-15 검토)

세 기능을 검토한 결과와, Gemini(Gemini CLI 등 저장소에서 코드를 고칠 수 있는 환경)에게 그대로 붙여 넣을 프롬프트입니다.
순서는 **1 → 2 → 3**을 권장합니다. 2는 1에서 만든 Garmin 연결 코드를 그대로 씁니다. 3은 다른 둘과 독립이라 언제 해도 됩니다.

## 검토 요약

| 기능 | 가능 여부 | 근거 / 주의 |
|---|---|---|
| 1. 개인 API 키로 Garmin 데이터 활용 | **가능** | 2026년 10월 Garmin Connect Developer Program이 다시 열리면서 **Personal 티어**가 생겼습니다. Garmin Connect 계정 설정에서 개인 API 키를 받고 액세스 토큰을 만듭니다. 신청이나 승인은 없습니다. 내 데이터만 받을 수 있고, 받을 수 있는 것은 Activity(JSON, .FIT), Health(JSON), Women's Health(JSON)입니다. **Pull 방식만 됩니다.** 인증 방식, 엔드포인트, 호출 제한은 공식 Swagger 문서에서 직접 확인해야 합니다. |
| 2. Garmin에서 FIT를 받아 게시물 작성 | **가능 (수동, 반자동)** | Personal 티어의 Activity API가 .FIT를 줍니다. Push(웹훅)가 없어서 "운동 끝나면 바로" 받는 진짜 자동은 안 됩니다. 앱을 열 때나 게시물 작성 창을 열 때 새 활동을 가져오는 반자동이 현실적입니다. API로 받은 FIT에서는 VO2max, 회복 시간 같은 비공개 필드가 빠진다는 보고가 있지만, 이 앱이 쓰는 GPS, 심박, 속도, 랩에는 영향이 없습니다. |
| 3. BullsNote에 목표와 달성 현황 기록 | **가능** | BullsNote 데스크톱 앱은 볼트 폴더(기본 `~/Documents/BullsNote`)에 노트 한 개를 frontmatter가 붙은 `.md` 파일 한 개로 저장하고, 밖에서 고친 파일도 다시 읽습니다. 그래서 운동 앱이 **월별 노트 파일 하나를 고정 id로 덮어쓰는 방식**이 가장 단순합니다. BullsNote MCP에는 노트 수정 도구가 없어서 이 용도에는 맞지 않습니다. |

참고:
- [Garmin 개발자 프로그램 개요](https://developer.garmin.com/gc-developer-program/overview)
- [Activity API 기능](https://developer.garmin.com/gc-developer-program/activity-api/)
- [the5krunner: Garmin personal API (2026-10-22)](https://the5krunner.com/2026/10/22/garmin-personal-api/)
- [Gadgets & Wearables 보도](https://gadgetsandwearables.com/2026/10/21/garmin-connect-personal-api-health-data-access/)

비공식 라이브러리(garth, python-garminconnect처럼 아이디와 비밀번호로 로그인하는 방식)는 약관 위반 소지가 있고, MFA나 차단으로 자주 깨집니다. 공식 Personal 티어가 생겼으니 쓰지 않습니다.

---

## 공통 맥락 (각 프롬프트 앞에 함께 붙여 넣기)

```text
너는 workout-app 저장소(/Users/chang-hunjeong/workspace/sns_create/workout-app)에서 작업한다.

[프로젝트]
- Garmin FIT와 사진으로 Threads 게시물(글 + 카드 이미지)을 만드는 개인용 데스크톱 앱. README.md를 먼저 읽어라.
- 백엔드: Rust workspace. crates/core(FIT 파싱·도메인), card, post, store(SQLite, settings 키-값),
  service(앱 로직, DTO, 비밀 값), enrich(Gemini/Claude 호출), gdrive, cli, testkit. 데스크톱 셸은 src-tauri(Tauri 2).
- DTO는 ts-rs로 ui/src/lib/bindings에 TypeScript로 내보낸다(`cargo test -p workout-service` 실행 시 생성).
- 프론트: ui/ (Svelte 5 runes, Tailwind v4, shadcn-svelte/bits-ui). 색은 ui/src/app.css 토큰만 쓴다(`npm run check:colors`가 하드코딩 색을 막는다).
- 비밀 값은 crates/service/src/secrets.rs의 SecretStore(OS 키체인, 서비스명 dev.sns-create.workout)에 저장한다. 키 이름 상수를 추가해서 쓴다.
- 일반 설정은 store의 set_setting/get_setting(키-값)을 쓴다.
- 업로드 흐름: Service::plan_upload(paths, exclude) -> UploadPlan, commit_upload(paths, exclude) -> UploadResult
  (crates/service/src/upload.rs). 운동마다 게시물 하나. 같은 FIT 해시는 이미 중복 처리된다.
- 목표: settings 키 goal_cycling_km, goal_running_km. Service::set_goals, month_view(year, month)의 MonthView에 목표와 월 거리가 있다
  (crates/service/src/views.rs).

[작업 규칙]
- 코드 재사용성을 먼저 생각해서 설계한다. 이미 있는 함수와 흐름(plan_upload/commit_upload, SecretStore, settings)을 다시 만들지 말고 연결한다.
- TDD: 실패하는 테스트를 먼저 쓰고, 실패를 확인한 다음 구현한다. 네트워크는 테스트에서 가짜 HTTP 응답(fixture JSON/파일)으로 대체한다.
  기존 crates/enrich/tests/gemini.rs의 fixture 방식을 따른다.
- 끝나기 전에 모두 통과시킨다: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`,
  ui에서 `npm test`, `npm run check`, `npm run check:colors`.
- 화면 문구는 짧고 쉬운 한국어 존댓말("~해요")로 쓴다.
- 커밋과 푸시는 하지 마라. 끝나면 바꾼 파일, 테스트 결과, 직접 확인하지 못한 것을 보고한다.
- 확인되지 않은 API 사실을 추측으로 코드에 박지 마라. 모르면 문서를 확인하고, 확인할 수 없으면 보고서에 "미확인"으로 적는다.
```

---

## 프롬프트 1: Garmin 개인 API 연결과 데이터 활용

```text
[목표]
Garmin Connect Personal API(개인 API 키)로 내 Garmin 데이터를 받아 오는 기반을 만들고, 이 앱에서 쓸 만한 데이터를 정리해 제안한다.

[배경]
- 2026년 10월부터 Garmin Connect Developer Program에 Personal 티어가 있다. Garmin Connect 계정 설정의 개인 API 토큰 페이지에서
  키를 받고 액세스 토큰을 만든다. 승인 절차는 없다. 내 데이터만 받을 수 있다.
  Activity(JSON, .FIT), Health(JSON), Women's Health(JSON)를 받을 수 있고, Pull 방식만 된다.
- 개요: https://developer.garmin.com/gc-developer-program/overview
- 정확한 인증 방식(키 -> 액세스 토큰 발급, 만료와 갱신), 엔드포인트, 요청과 응답 스키마, 호출 제한은
  developer.garmin.com의 Personal API Swagger 문서에서 확인해야 한다. 이것부터 하라.

[1단계: 조사 (코드보다 먼저)]
docs/garmin-personal-api.md에 아래를 정리한다. 각 항목에 근거 URL을 붙인다.
1. 키 발급과 토큰 생성 절차, 토큰 형식(Bearer 등), 만료와 갱신 방법.
2. 활동 목록 조회(기간 필터, 페이지), 활동 상세 JSON, 활동 FIT 파일 다운로드 엔드포인트.
3. Health 엔드포인트 중 이 앱에 쓸 만한 것: 수면, 안정시 심박, HRV, 스트레스, Body Battery, 걸음 수 등. 날짜 단위인지 기간 단위인지.
4. 호출 제한, 과거 데이터를 얼마나 거슬러 받을 수 있는지, 약관상 개인 사용 제약.
5. 확인하지 못한 항목은 "미확인"으로 적는다.

[2단계: 활용 제안]
같은 문서에 "이 앱에서의 활용" 절을 쓴다. 기존 기능과 연결해 우선순위를 매긴다. 후보:
- 게시물 글 보강: 그날 수면, 안정시 심박, Body Battery 같은 컨디션 한 줄을 enrich(글 다듬기) 입력에 추가.
- 운동 상세 화면: 활동 JSON의 Training Effect, 칼로리처럼 FIT 파싱으로 얻기 어려운 값.
- 월 요약: 걸음 수와 수면 추세.
각 후보마다 필요한 엔드포인트, 바꿀 파일, 사용자 가치, 비용(호출 수)을 적는다. 구현은 2단계에서 하지 않는다.

[3단계: 최소 구현 (Garmin 연결 기반)]
- 새 crate crates/garmin을 만든다. 책임은 "Garmin Personal API HTTP 클라이언트"만이다(앱 로직을 넣지 않는다).
  - 다른 crate처럼 동기 HTTP 클라이언트(기존 enrich/gdrive가 쓰는 라이브러리)를 쓴다.
  - 공개 API(이름은 조사 결과에 맞춰 조정 가능):
    - GarminClient::new(token_provider) // 토큰 공급자를 주입받아 테스트에서 가짜를 넣는다
    - list_activities(from: NaiveDate, to: NaiveDate) -> Result<Vec<GarminActivity>>  // id, 시작 시각(UTC와 오프셋), 종목, 거리, 시간
    - download_fit(activity_id) -> Result<Vec<u8>>
    - daily_wellness(date) -> Result<Wellness>  // 2단계 제안에서 1순위로 고른 Health 항목만
  - 401은 "토큰 만료/잘못됨", 429는 "호출 제한" 같은 구분된 에러 타입으로 돌려준다.
- service에 연결한다:
  - secrets.rs에 GARMIN_KEY(개인 API 키)와 필요하면 액세스 토큰 캐시 키를 추가한다.
    토큰 갱신이 필요하면 GOOGLE_ACCESS처럼 "토큰|만료unix초" 형식을 재사용한다.
  - 설정 화면(ui/src/components/Settings.svelte)에 "Garmin 연결" 칸: 키 입력, 저장, "연결 확인"(최근 7일 활동 수 표시), 연결 끊기.
    기존 Gemini/Claude 키 입력 UI와 같은 컴포넌트와 패턴을 재사용한다.
  - Tauri 명령: set_garmin_key(key: Option<String>), test_garmin() -> 최근 활동 수.
- 테스트: fixture JSON으로 list_activities 파싱, download_fit 바이트, 401과 429 에러 매핑, 토큰 만료 시 갱신.

[완료 조건]
- docs/garmin-personal-api.md에 조사와 활용 제안이 근거 URL과 함께 있다.
- 설정에서 키를 넣고 "연결 확인"을 누르면 최근 활동 수가 보인다(실제 키 확인은 사용자가 한다. 키 값을 로그나 파일에 남기지 마라).
- 공통 검사 명령이 모두 통과한다.
```

---

## 프롬프트 2: Garmin에서 FIT를 받아 게시물 작성 (수동, 반자동)

```text
[전제]
프롬프트 1의 crates/garmin(GarminClient: list_activities, download_fit)과 Garmin 키 설정이 이미 있다. 없으면 먼저 그것부터 하라고 보고하고 멈춰라.

[목표]
게시물 작성 창(ui/src/components/Composer.svelte)에서 FIT 파일을 직접 고르지 않아도 Garmin에서 활동을 받아 게시물을 만들게 한다.
- 수동: 작성 창의 "Garmin에서 가져오기" 버튼 -> 기간(기본 최근 7일)의 활동 목록 -> 고른 활동의 FIT가 업로드 목록에 들어간다.
- 반자동: 설정 "Garmin 새 활동 자동 확인"을 켜면, 작성 창을 열 때(그리고 앱 시작 시 한 번) 마지막 확인 이후의 새 활동 FIT를 받아
  작성 창 목록에 미리 넣어 둔다. 실제로 게시물을 만드는 것은 사용자가 "게시물 N개 만들기"를 눌렀을 때뿐이다.
- Personal API는 Pull만 되므로 웹훅이나 백그라운드 상주 동기화는 만들지 않는다.

[설계 (재사용 우선)]
- 새 업로드 경로를 만들지 마라. 받은 FIT를 앱 데이터의 임시 받기 폴더($APPDATA/garmin-inbox/<activityId>.fit)에 저장하고,
  그 경로를 기존 plan_upload/commit_upload의 paths에 넣는다. 미리보기, 빼기, 사진 붙이기, 중복 처리는 기존 흐름이 그대로 한다.
- service에 추가한다:
  - garmin_activities(from, to) -> Vec<GarminActivityDto>. 각 항목에 already_imported: bool을 넣는다.
    FIT를 받기 전에 판단해야 하므로 시작 시각과 종목이 같은 운동이 store에 있는지로 판단한다.
  - fetch_garmin_fits(ids: Vec<String>) -> Vec<PathBuf>. 이미 받은 파일은 다시 받지 않는다. 실패한 id는 이유와 함께 따로 돌려준다.
  - garmin_new_since_last_check() -> Vec<PathBuf>. settings의 garmin_last_check(UTC ISO)를 쓴다.
    처음이면 최근 3일. 성공했을 때만 값을 갱신한다.
  - commit_upload이 끝나면 게시물로 만든 garmin-inbox 파일은 지운다(보관함에 복사되었으므로). 빼거나 남은 파일은 둔다.
- DTO는 dto.rs에 ts-rs로 추가하고, Tauri 명령과 ui/src/lib/api.ts(모의 구현 포함)를 함께 추가한다.
- UI:
  - Composer의 업로드 영역에 "Garmin에서 가져오기" 버튼. Garmin 키가 없으면 버튼 대신 "설정에서 Garmin을 연결하세요" 링크.
  - 활동 고르기 목록: 날짜, 종목 아이콘, 제목, 거리. 이미 가져온 활동은 회색에 "이미 있음"으로 표시하고 기본 선택 해제.
  - 받는 중 진행 표시, 실패 항목 목록. 기존 Composer의 문구 톤과 컴포넌트(Dialog, Button)를 재사용한다.
  - 반자동으로 미리 넣은 항목에는 "Garmin" 표시를 붙여 사용자가 직접 올린 파일과 구분한다.
  - 설정 화면에 "Garmin 새 활동 자동 확인" 켜기/끄기(기본 끔).

[테스트 (실패 먼저)]
- fixture 활동 3개 중 1개가 이미 store에 있을 때 already_imported가 정확하다.
- fetch_garmin_fits가 같은 id를 두 번 받지 않는다. 하나가 실패해도 나머지 경로는 돌려준다.
- garmin_new_since_last_check: 처음 실행, 이후 실행, 네트워크 실패 시 garmin_last_check를 갱신하지 않는 경우.
- 받은 FIT 경로로 plan_upload를 부르면 newActivities에 나온다(기존 testkit FIT fixture 사용).
- commit 후 만든 운동의 inbox 파일만 지워진다.
- UI: api 모의 구현으로 Composer에서 Garmin 목록 -> 선택 -> 미리보기 흐름을 컴포넌트 테스트(happy-dom, 기존 테스트 방식)로 확인한다.

[완료 조건]
- 파일을 고르지 않고 Garmin에서 받은 활동만으로 미리보기와 게시물 만들기가 된다(모의 데이터 기준. 실제 계정 확인은 사용자 몫이라고 보고).
- 자동 확인을 꺼 두면 네트워크 호출이 전혀 없다.
- 공통 검사 명령이 모두 통과한다.
```

---

## 프롬프트 3: BullsNote에 라이딩, 러닝 목표와 달성 현황 기록

```text
[목표]
workout-app의 월 목표(라이딩, 러닝)와 달성 현황을 BullsNote 노트로 자동 기록한다. 월마다 노트 하나를 계속 갱신한다.

[BullsNote 사실 (확인됨)]
- BullsNote 데스크톱 앱(/Users/chang-hunjeong/workspace/bullsNote, Electron)은 볼트 폴더(기본 ~/Documents/BullsNote)에
  노트 한 개를 `.md` 파일 한 개로 저장한다. 밖에서 고친 파일도 앱이 감지해 다시 읽는다.
- 파일은 YAML frontmatter와 Markdown 본문으로 되어 있다. 실제 노트의 frontmatter 필드:
  id, title, projectId, color, reminder, parentId, pinned, favorite, tags, attachments, todoMeta, systemGroupType, createdAt, updatedAt(ms 단위 unix 시각).
  파일 이름은 `<id>.md`.
- 폴더(projects)는 볼트의 .bullsnote.json에 있다. 예: 02_Areas -> "proj-muw79y3g-oaubj", AI 노트 -> "proj-ai-notes". id는 사용자마다 다르니 이름으로 찾아라.
- 저장 형식의 기준은 bullsNote 저장소의 저장, 파싱 코드다. 필수 필드와 기본값을 그 코드에서 확인하고 그대로 맞춰라.
  참고 구현: bullsNote/.githooks/save_to_bullsnote.py(임시 파일에 쓴 뒤 os.replace로 원자적 저장).
- BullsNote MCP(save_note, add_todo 등)에는 노트 수정 기능이 없어서 이 용도에는 쓰지 않는다.

[설계]
- workout-app 쪽에 새 모듈 crates/service/src/bullsnote.rs(또는 독립 crate crates/bullsnote)를 만든다.
  책임은 "MonthView를 받아 BullsNote 노트 파일을 만들거나 갱신"이다. 계산은 새로 하지 말고 Service::month_view(year, month)의 결과를 쓴다.
  - 노트 id는 고정: `workout-goal-YYYY-MM`. 제목: "YYYY년 M월 운동 목표".
  - 폴더: 설정값(기본 "02_Areas"를 이름으로 찾고, 없으면 "AI 노트"). 태그: ["운동", "목표"].
  - 파일이 이미 있으면 createdAt과 사용자가 바꾼 frontmatter 필드(pinned, color, projectId 등)는 유지하고 updatedAt만 바꾼다.
  - 본문에서 앱이 관리하는 부분은 `<!-- workout-app:start -->`와 `<!-- workout-app:end -->` 사이만 바꾼다. 그 밖에 사용자가 쓴 메모는 그대로 둔다.
    표식이 없는 기존 파일이면 맨 위에 관리 영역을 추가한다.
  - 원자적 저장(같은 폴더 임시 파일에 쓰고 rename). 내용이 같으면 파일을 쓰지 않는다(BullsNote가 불필요하게 다시 읽지 않게).
- 관리 영역 내용(Markdown, BullsNote 에디터가 읽을 수 있는 기본 문법만):
  - 종목별 표: 목표 km | 달성 km | 달성률 | 남은 km | 남은 날 | 하루 필요 km. 목표가 없는 종목은 "목표 없음"으로 표시하고 달성 km만.
  - 이번 달 운동 목록: 날짜, 종목, 제목, 거리.
  - "마지막 갱신: YYYY-MM-DD HH:MM".
- 언제 갱신하나:
  - 설정 "BullsNote에 목표 기록"이 켜져 있을 때(기본 끔)만.
  - set_goals 성공 후, commit_upload 성공 후(바뀐 운동의 달), 앱 시작 시 이번 달 한 번.
  - 설정 화면에 "지금 반영" 버튼과 마지막 결과(성공 시각이나 오류 문구).
  - 갱신 실패는 본래 작업(목표 저장, 게시물 만들기)을 실패시키지 않는다. 경고만 남긴다.
- 설정: 볼트 경로(기본 ~/Documents/BullsNote, 폴더 고르기 버튼), 폴더 이름, 켜기/끄기. 일반 settings 키로 저장한다.
  기존 Settings.svelte의 섹션 패턴을 재사용한다.

[테스트 (실패 먼저, 임시 폴더 사용)]
- 새 파일: frontmatter 필드와 파일 이름, 관리 영역 표 값(목표 300, 달성 120 -> 40%, 남은 180).
- 기존 파일 갱신: createdAt, pinned, 사용자 메모(관리 영역 밖)가 보존되고 updatedAt과 표만 바뀐다.
- 표식 없는 기존 파일 처리.
- 내용이 같으면 파일 수정 시각이 바뀌지 않는다.
- 폴더 이름을 .bullsnote.json에서 찾고, 없을 때 대체 폴더로 간다.
- 볼트 경로가 없거나 쓰기 실패 시 에러를 돌려주되, set_goals 자체는 성공한다.
- 목표가 없는 종목, 운동이 없는 달.

[완료 조건]
- 설정에서 켜고 목표를 저장하면 볼트에 workout-goal-YYYY-MM.md가 생기고, BullsNote 앱에서 그 노트가 보인다
  (실제 앱 확인은 사용자가 한다. bullsNote 저장소 코드는 읽기만 하고 고치지 마라).
- 공통 검사 명령이 모두 통과한다.
```
