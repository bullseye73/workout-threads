# Garmin Connect Personal API 조사 및 활용 제안

> 작성일: 2026-11-15  
> 목적: `workout-app`에서 공식 Garmin Connect Personal API(개인 API 키)를 연동하기 위한 기술 조사 및 활용 우선순위 정리

---

## 1. API 조사 결과

### 1.1 개요 및 제약
- **티어 성격**: 2026년 10월 Garmin Connect Developer Program에 추가된 **Personal 티어**. 별도 사업자 심사나 승인 절차 없이 개인이 자신의 Garmin Connect 데이터만 조회할 수 있다.
  - 근거: [Garmin Developer Program Overview](https://developer.garmin.com/gc-developer-program/overview)
  - 근거: [the5krunner - Garmin personal API (2026-10-22)](https://the5krunner.com/2026/10/22/garmin-personal-api/)
- **전송 방식**: **Pull 방식 전용**(읽기 전용 `GET`). 비즈니스 티어와 달리 Webhook/Push/Ping 알림은 제공되지 않는다.
- **사용 제약**: 본인 계정의 데이터 조회만 허용되며, 다수 사용자를 대상으로 하는 상업용 서비스나 멀티유저 앱에는 사용할 수 없다.
- **리전별 Base URL**:
  - 글로벌: `https://personal.connectapi.garmin.com`
  - 중국 리전: `https://personal.connectapi.garmin.cn`

---

### 1.2 키 발급, 토큰 생성, 만료 및 갱신

1. **개인 API 키(Personal API Key) 발급**:
   - Garmin Connect Web 로그인 → 계정 설정(Account Settings) → **Personal API Key** 페이지에서 만료 기간(30일, 90일, 180일, 1년 등)을 선택해 생성한다.
   - 생성 직후 한 번만 표시되므로 안전하게 보관해야 한다(이 앱에서는 OS 키체인 `SecretStore`에 저장).
2. **액세스 토큰 발급(교환)**:
   - API 키를 데이터 엔드포인트에 직접 전달할 수 없으며, 토큰 엔드포인트로 Bearer 액세스 토큰을 발급받아야 한다.
   - **요청**:
     ```http
     POST https://personal.connectapi.garmin.com/personalapitoken-service/personal-api/personal-api-key/token
     Authorization: ApiKey <personal-api-key>
     ```
   - **응답**: 순수 텍스트(`plain text`)로 **JWT 액세스 토큰 문자열**을 직접 반환한다(JSON 객체가 아님).
   - **데이터 요청 헤더**: `Authorization: Bearer <access_token>`
3. **만료 및 갱신**:
   - 액세스 토큰 유효 기간은 **24시간**(또는 개인 API 키 만료 시각 중 더 이른 시점)이다.
   - **Refresh Token은 제공되지 않는다.** 토큰이 만료(또는 `401 Unauthorized` 응답)되면 저장된 개인 API 키로 위 토큰 엔드포인트에 다시 `POST` 요청해 새 액세스 토큰을 발급받는다.
   - 근거: [Garmin Developer Portal / Personal API 인증](https://developer.garmin.com/gc-developer-program/overview), [Gadgets & Wearables 보도 (2026-10-21)](https://gadgetsandwearables.com/2026/10/21/garmin-connect-personal-api-health-data-access/)
4. **미확인 사항**:
   - 토큰 응답 JSON의 기타 부가 필드(`token_type`, `scope` 등) 정확한 스키마: **미확인** (개발자 포털 로그인 후 Swagger 문서에서 확인 필요).

---

### 1.3 활동(Activity) 및 FIT 다운로드 엔드포인트

| 용도 | 엔드포인트 | 주요 파라미터 / 응답 |
|---|---|---|
| **활동 목록 조회** | `GET /activitylist-service/activities/search/activities` | 쿼리: `start`(오프셋, 0부터), `limit`(페이지 크기), `startDate`(`YYYY-MM-DD`), `endDate`(`YYYY-MM-DD`). JSON 배열 반환 (`activityId`, `activityName`, `startTimeGMT`, `startTimeLocal`, `activityType.typeKey`, `distance`(m), `duration`(초) 등). |
| **활동 상세 JSON** | `GET /activity-service/activity/{activityId}` | 경로: `activityId`. 활동 요약/상세 JSON 반환. |
| **원본 FIT 다운로드** | `GET /download-service/files/activity/{activityId}` | 경로: `activityId`. **ZIP 바이트**로 응답하며 내부에 원본 `.fit`(또는 `.tcx`/`.gpx`) 파일이 들어 있다. (단일 `.fit` 직접 응답 여부도 대비해 ZIP 매직 바이트 `PK` 확인 후 압축 해제 처리 권장) |

- 근거: [Garmin Activity API](https://developer.garmin.com/gc-developer-program/activity-api/), [the5krunner (2026-10-22)](https://the5krunner.com/2026/10/22/garmin-personal-api/)
- **참고(FIT 필드 차이)**: Personal API로 내려받은 `.fit` 파일은 일부 비공개/고유 지표(예: 일부 VO2max, 회복 시간 필드)가 빠질 수 있다는 보고가 있으나, 본 앱이 사용하는 GPS 경로, 심박, 속도/페이스, 랩(km 구간)에는 영향이 없다.
- **미확인 사항**:
  - `activitylist-service/activities/search/activities`의 최대 `limit` 상한(20 또는 100 등)과 모든 응답 필드 타입: **미확인** (Swagger 확인 필요).
  - 중국 리전 외 다중 시간대 활동의 `startTimeLocal` 오프셋 표기 형식(ISO 문자열인지 로컬 무오프셋 문자열인지): **미확인** (실계정 응답 확인 필요).

---

### 1.4 Health / Wellness 엔드포인트 (이 앱 활용 후보)

| 항목 | 엔드포인트 | 조회 단위 | 주요 데이터 |
|---|---|---|---|
| **일일 종합 요약** | `GET /usersummary-service/v2/dailysummaries` | **기간 단위** (`startDate`, `endDate`, `YYYY-MM-DD`) | 걸음 수(`totalSteps`), 안정시 심박(`restingHeartRate`), 스트레스 평균(`averageStressLevel`), 칼로리, Body Battery 요약 등 |
| **수면 요약 (단일/기간)** | `GET /sleep-service/sleep/dailySleep` / `GET /sleep-service/sleep/dailySleepSummariesByDate` | **날짜 단위** (`date`) / **기간 단위** (`startDate`, `endDate`) | 총 수면 시간(`sleepTimeSeconds`), 수면 점수, 깊은/얕은/REM 수면 |
| **Body Battery** | `GET /wellness-service/wellness/bodyBattery/reports/daily` | **기간/날짜 단위** | 충전/소모, 최저/최고 Body Battery |
| **HRV** | `GET /hrv-service/hrv/{calendarDate}` / `daily/{fromDate}/{toDate}` | **날짜 / 기간 단위** | 야간 평균 HRV, 기준선 상태 |
| **일일 심박** | `GET /wellness-service/wellness/dailyHeartRate` | **날짜 단위** | 안정시 심박, 일일 심박 추이 |

- **미확인 사항**:
  - 각 Health 엔드포인트의 정확한 JSON 필드명 대소문자/중첩 구조(`dailySleepDTO` 등) 및 최대 조회 기간(일부 엔드포인트 28~32일 제한 보고): **미확인** (Swagger 확인 필요).

---

### 1.5 호출 제한 및 과거 데이터 조회 범위

- **호출 제한 (`429 Too Many Requests`)**:
  - 과도한 반복 폴링, 단시간 대량 FIT 다운로드, 토큰 반복 재발급 시 `429` 응답 또는 일시적 차단(1시간~48시간)이 발생할 수 있다.
  - 따라서 발급받은 액세스 토큰(24시간 유효)을 캐시해 재사용하고, 활동/FIT 다운로드 시 불필요한 재요청을 피해야 한다.
  - **정확한 수치(분당/일당 허용 호출 수)**: **미확인** (비즈니스 Starter 티어 문서에는 분당/일당 쿼터 언급이 있으나 Personal API의 정확한 숫자는 공개 문서에서 확인 불가, Swagger 확인 필요).
- **과거 데이터 조회 범위**:
  - 개인 계정의 과거 활동 및 일일 요약 조회가 가능하나, 단건 요청당 권장 기간 제한(예: 일일 요약 30~60일 단위 분할 권장) 외에 **최대 과거 몇 년까지 조회 가능한지**: **미확인**.

---

## 2. 이 앱에서의 활용 제안

기존 `workout-app`의 흐름(`crates/enrich`, `crates/service`, `crates/core`, `ui`)과 연결해 사용자 가치와 API 호출 비용을 기준으로 우선순위를 정리한다.

| 우선순위 | 활용 영역 | 설명 | 필요한 엔드포인트 | 바꿀 파일 | 사용자 가치 | 비용 (호출 수) |
|---|---|---|---|---|---|---|
| **1순위** | **게시물 글 보강 (`enrich`)** | 운동 당일의 수면 시간, 안정시 심박, Body Battery(또는 걸음 수) 등 컨디션 한 줄을 AI 글 다듬기(`RefineInput`) 입력에 추가해 더 맥락 있는 스레드 글을 생성한다. | `GET /usersummary-service/v2/dailysummaries?startDate=YYYY-MM-DD&endDate=YYYY-MM-DD` (필요 시 `dailySleep` 1회 추가) | `crates/garmin`, `crates/enrich/src/refine.rs`, `crates/service/src/lib.rs` (또는 `posts.rs`) | **높음** — 사용자가 별도 입력 없이 "수면 7시간·안정시 심박 52" 같은 컨디션 맥락이 자연스럽게 글에 녹아든다. | **매우 낮음** — 게시물 다듬기 시 날짜당 1회 호출(결과 캐시 가능). |
| **2순위** | **운동 상세 화면 메타데이터** | `.fit` 파싱으로 얻기 어렵거나 누락될 수 있는 Garmin 고유 요약(Training Effect, 총 칼로리, 활동명 등)을 기록 탭에 보조 표시한다. | `GET /activity-service/activity/{activityId}` | `crates/garmin`, `crates/service/src/dto.rs`, `crates/service/src/views.rs`, `ui/src/components/detail/` | **중간** — 기록 탭 정보가 풍부해지지만 기존 FIT 통계만으로도 핵심 기능은 동작한다. | **낮음** — 활동당 1회 조회 후 SQLite 캐시. |
| **3순위** | **한 달 요약 (걸음 수·수면 추세)** | 작업대 상단 월 요약에 라이딩·러닝 거리 외에 월간 평균 수면·일일 걸음 수 추세를 함께 표시한다. | `GET /usersummary-service/v2/dailysummaries`, `GET /sleep-service/sleep/dailySleepSummariesByDate` | `crates/garmin`, `crates/service/src/views.rs`, `ui/src/components/workbench/` | **낮음~중간** — 본 앱의 핵심 목적(운동 게시물 작성)과는 부차적이며 Garmin Connect 앱과 기능이 겹친다. | **중간** — 달을 바꿀 때마다 기간 조회 1~2회 필요. |

> **3단계 최소 구현 범위**: 위 1순위를 기준으로 `crates/garmin`에 `list_activities`, `download_fit`, `daily_wellness`(일일 종합 요약 기반: 걸음 수·안정시 심박·평균 스트레스·Body Battery 요약)를 구현하고, `Settings.svelte`에 Garmin 개인 API 키 입력 및 연결 확인 UI를 연결한다.
