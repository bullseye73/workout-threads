# 6단계: 작업대 화면 개편 + 테마(시스템/밝게/어둡게) 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 대시보드·상세·게시 세 화면을 "한 달 사진 띠 + 운동 목록 + 상세(기록/게시 준비 탭)" 한 화면으로 바꾸고, 시스템·밝게·어둡게 테마를 지원한다.

**Architecture:** 백엔드는 `MonthView.days[].items[]`에 목록·사진 띠에 필요한 값(제목, 시간, 속도/페이스, 대표 카드)만 더한다. 새 Tauri 명령은 없다. UI는 Tailwind CSS v4 + shadcn-svelte(bits-ui) 컴포넌트를 `src/lib/components/ui`에 두고, 테마는 mode-watcher가 `<html class="dark">`를 붙이는 방식으로 CSS 변수 토큰을 바꾼다. 화면 계산(띠, 주별 묶음, 목표 진행, 사진 핀, 구간 막대)은 순수 함수로 빼서 vitest로 검증한다.

**Tech Stack:** Rust(workout-service, ts-rs), Svelte 5, Vite 8, Tailwind CSS v4(`@tailwindcss/vite`), shadcn-svelte(bits-ui), mode-watcher, Chart.js 4, Leaflet 1.9, vitest.

**Spec:** `docs/design/redesign-proposal-v2.html` (시안, 승인됨 2026-11-14). 시안과 다른 결정:
- 게시 탭의 "카드 빼기·넣기"는 **범위 밖**. 게시는 사용자가 폴더에서 직접 올리므로 저장할 대상이 없다. 미리보기는 카드 전부를 보여 준다.
- 시안의 5km 구간표 → 기존 데이터 그대로 **1km 구간**에 속도 막대만 더한다.
- 시안의 SVG 지도 그림 → 실제 Leaflet 지도. 어두운 테마는 같은 OSM 타일에 CSS 필터(새 외부 서비스 없음).
- 다듬기 버튼 문구는 현재 앱 그대로 `{ai.label}로 다듬기`(5단계 결과).

## Global Constraints
- 1–5단계 제약 유지. 좌표·키를 화면 문구·로그에 넣지 않는다. 게시는 사용자가 직접 한다.
- 글꼴은 Pretendard 하나(`crates/card/assets/fonts`), 숫자는 `tabular-nums`.
- 색은 `ui/src/app.css`의 토큰만 쓴다. 컴포넌트(`ui/src/components/**`, `ui/src/lib/components/**`)에 `#rrggbb`·`white`·`black` 직접 쓰기 금지(카드 사진 위 흰 글자는 토큰 `--on-photo`).
- 토큰 값(밝게 / 어둡게):
  `--background #eceef0 / #0e1115`, `--card #ffffff / #161a1f`(시안의 `--pane`), `--muted #f3f5f6 / #1d2228`, `--sunken #e2e6e9 / #0a0c0f`, `--foreground #161e28 / #e6e9ed`, `--muted-foreground #5d6977 / #97a1ad`, `--faint #98a2ad / #5b6570`, `--border #dce0e4 / #262c33`, `--selected #e8effe / #1c2840`, `--ride #2f6feb / #6b98ff`, `--run #e2661a / #ff9150`, `--done #148a5c / #3cc48e`, `--wait #b8740f / #e8b04b`, `--hr #d93f4c / #ff6b77`, `--alt #6d7a88 / #8b97a4`, `--destructive #b42318 / #ff7b6b`, `--on-photo #ffffff`.
- 게시 상태 표시: `none` 점선 원 "글 없음", `ready` 호박색(`--wait`) "게시 대기", `published` 초록(`--done`) "게시함". 종목 색(파랑·주황)과 섞지 않는다.
- 테마 선택은 세 가지 "시스템 / 밝게 / 어둡게", 기본 시스템, mode-watcher 기본 저장(localStorage). 앱 시작 시 번쩍임 없음.
- 레이아웃: 860px 이상은 목록 330px + 상세. 그보다 좁으면 위아래로 쌓고, 가로 스크롤은 사진 띠 안에서만.
- `prefers-reduced-motion` 존중, 키보드 포커스 보임, 최소 너비 320px.
- 브라우저 모의 모드(`mockApi`)에서도 모든 화면이 뜬다.
- 커밋은 noreply 작성자(로컬 설정됨), 브랜치 `public-main`.

## Review Focus
1. 운동이 없는 달 → 띠는 빈 칸 30개, 목록은 "이번 달 기록이 없어요. 가져오기로 FIT 파일과 사진을 넣거나 창에 끌어다 놓으세요.", 상세는 "왼쪽에서 운동을 고르세요." → Task 3 (`pickSelection` 빈 목록 → `null`).
2. 달을 넘기면 이전 선택이 새 달에 없음 → 새 달의 가장 최근 운동을 고르고, 없으면 `null`. → Task 3.
3. 같은 날 운동이 둘 이상 → 띠 칸은 첫 운동의 카드 + 오른쪽 아래 "2" 배지, 칸을 누르면 그날 첫 운동. → Task 3 (`ribbonDays`).
4. 사진·GPS 없는 운동 → 목록 썸네일 자리 "사진 없음", 사진 핀 0개, 지도 자리는 기존 "GPS 기록이 없어요" 문구. `kmMark`가 없는 사진은 핀에서 빼고, 거리보다 큰 값은 끝(100%)에 붙인다. → Task 4 (`photoPins`).
5. 게시 탭에 저장 안 한 수정이 있는데 다른 운동이나 "기록" 탭을 누름 → "저장하지 않은 수정 내용이 있어요. 나갈까요?" 확인(AlertDialog). → Task 5.
6. 테마를 바꿀 때 그래프·지도가 열려 있음 → 그래프 색이 새 토큰으로 다시 그려진다. → Task 4.

---

### Task 1: 목록용 값 더하기 (`workout-core`, `workout-card`, `workout-service`)

**Files:**
- Modify: `crates/core/src/format.rs` (제목 함수), `crates/card/src/data.rs:44-49` (새 함수 사용), `crates/service/src/dto.rs:100-105`, `crates/service/src/views.rs:59-78`
- Modify: `ui/src/lib/mock.ts` (새 필드 채우기), 생성물 `ui/src/lib/bindings/DayItemDto.ts`
- Test: `crates/core` 기존 format 테스트 파일, `crates/service/tests/views.rs`

**Interfaces:**
- Produces:
```rust
// crates/core/src/format.rs — 카드 제목과 목록 제목이 같은 문구를 쓰도록 한 곳에서 만든다.
pub fn activity_title(local: NaiveDateTime, sport: Sport) -> String   // "일요일 오전 라이딩"

// crates/service/src/dto.rs
pub struct DayItemDto {
    pub activity_id: String, pub sport: SportKind, pub km: f64, pub post_status: PostStatus,
    pub title: String,               // activity_title 결과
    pub start_time: String,          // 현지 "07:12"
    pub moving_s: f64,
    pub speed_kmh: Option<f64>,      // 라이딩만
    pub pace: Option<String>,        // 라이딩 아닌 종목, 기존 pace_per_km 형식
    pub cover_card: Option<String>,  // 첫 카드 절대 경로 (photos_for 순서), 없으면 None
    pub card_count: u32,
}
```
- TS(ts-rs 생성): `DayItemDto`에 `title, startTime, movingS, speedKmh, pace, coverCard, cardCount`.

- [ ] **Step 1: 실패하는 테스트**
  - core: `activity_title(2026-11-08 07:12, Cycling) == "일요일 오전 라이딩"`, `(…18:30, Running) == "일요일 저녁 러닝"`(시간대 문구는 기존 `time_of_day_ko` 기준).
  - service `views.rs`: `month_view_items_carry_list_fields` — 테스트 라이브러리에 라이딩(카드 2장)·러닝(사진 없음)을 넣고 `month_view`의 각 item에 대해 `title`, `start_time == "07:12"`, 라이딩 `speed_kmh.is_some() && pace.is_none()`, 러닝 반대, 라이딩 `card_count == 2 && cover_card`가 절대 경로이고 첫 카드, 러닝 `card_count == 0 && cover_card.is_none()`.
- [ ] **Step 2:** `cargo test -p workout-core -p workout-service` → 새 테스트 FAIL(없는 함수·필드).
- [ ] **Step 3: 구현.** `activity_title` 추가 후 `card_data_with`의 `format!`을 이 함수로 바꾼다. `month_view`에서 item마다 `self.store.photos_for(id)`로 카드 경로를 모으고(`post_view`와 같은 `filter_map` → 작은 비공개 함수 `card_paths(&self, id) -> Result<Vec<String>>`로 빼서 `post_view`도 그것을 쓰게), 속도/페이스는 `activity_detail`이 쓰는 `speed_kmh`/`pace_per_km`를 재사용.
- [ ] **Step 4:** `cargo test --workspace` PASS, `ui/src/lib/bindings/DayItemDto.ts`에 새 필드 생성 확인. `mock.ts`의 `mockMonth` items에 새 필드를 채우고(제목은 기존 `mockDetail`의 제목 규칙, `coverCard`는 null) `npm --prefix ui test` · `npm --prefix ui run check` PASS.
- [ ] **Step 5: 커밋** `feat(service): list fields on month view items`

### Task 2: Tailwind + shadcn-svelte + 테마 토대 (`ui`)

**Files:**
- Modify: `ui/package.json`, `ui/vite.config.ts`, `ui/tsconfig.app.json`, `ui/src/app.css`, `ui/src/App.svelte`(ModeWatcher 마운트), `ui/src/main.ts` 필요 시
- Create: `ui/components.json`, `ui/src/lib/utils.ts`(shadcn `cn`), `ui/src/lib/components/ui/*`(아래 목록), `ui/src/lib/components/ThemeToggle.svelte`, `ui/src/lib/components/StatusBadge.svelte`, `ui/src/lib/theme.ts`
- Test: `ui/src/lib/theme.test.ts`, `ui/scripts/check-colors.sh`

**Interfaces:**
- Produces:
```ts
// src/lib/theme.ts
export type ThemePref = 'system' | 'light' | 'dark'
export const THEME_OPTIONS: { value: ThemePref; label: string }[]   // 시스템 / 밝게 / 어둡게 (이 순서)
export function cssVar(name: string, el?: Element): string           // getComputedStyle 값, 그래프·지도가 씀
export const STATUS_LABEL: Record<PostStatus, string>                // none "글 없음", ready "게시 대기", published "게시함"
```
  - `ThemeToggle.svelte`: props 없음. shadcn ToggleGroup(single), mode-watcher `userPrefersMode`/`setMode`/`resetMode`. 좁은 화면(<640px)에서는 아이콘만, `aria-label`은 항상 문구.
  - `StatusBadge.svelte`: `{ status: PostStatus; size?: 'sm' | 'md' }`.
  - shadcn 컴포넌트: `button, toggle-group, tabs, dialog, alert-dialog, dropdown-menu, scroll-area, tooltip, input, textarea, label, checkbox`.

- [ ] **Step 1: 실패하는 테스트** `theme.test.ts`: `THEME_OPTIONS.map(o => o.value)` 가 `['system','light','dark']`, 라벨 `['시스템','밝게','어둡게']`; `STATUS_LABEL` 세 값 정확히 일치.
- [ ] **Step 2:** `npm --prefix ui test` → FAIL(모듈 없음).
- [ ] **Step 3: 설치·설정.** `tailwindcss @tailwindcss/vite mode-watcher` 설치, vite 플러그인 추가, `$lib → src/lib` 별칭을 `vite.config.ts`(`resolve.alias`)와 `tsconfig.app.json`(`paths`)에 추가(SvelteKit이 아니므로 직접). `npx shadcn-svelte@latest init`(base color 무시하고 우리 토큰으로 덮음) 후 위 컴포넌트 `add`. `app.css`: `@import "tailwindcss"`, `@custom-variant dark (&:is(.dark *))`, Pretendard `@font-face` 유지, `:root`/`.dark`에 Global Constraints 토큰, `@theme inline`으로 토큰을 Tailwind 색 이름(`bg-card`, `text-muted-foreground`, `bg-ride` …)에 연결, 기존 `.btn` 클래스 삭제는 Task 6에서.
- [ ] **Step 4: 색 검사 스크립트** `ui/scripts/check-colors.sh`: 검사 대상 폴더 목록(`src/lib/components src/components/workbench src/components/detail`)에서 `grep -rnE '#[0-9a-fA-F]{3,8}\b|\bwhite\b|\bblack\b' --include=*.svelte` 결과가 있으면 exit 1. `package.json`에 `"check:colors"`로 등록. 옛 컴포넌트(`src/components/*.svelte`)는 Task 6에서 대상에 넣는다. 지금 PASS 확인.
- [ ] **Step 5: 구현** `theme.ts`, `ThemeToggle`, `StatusBadge`; `App.svelte` 맨 위에 `<ModeWatcher />`와 임시로 `ThemeToggle` 배치.
- [ ] **Step 6:** `npm --prefix ui test`, `npm --prefix ui run check` PASS. 브라우저(`workout-ui`)에서 세 테마 전환, 새로고침 후 유지, 시스템 선택 시 OS 설정 따름 확인.
- [ ] **Step 7: 커밋** `feat(ui): tailwind, shadcn-svelte and theme tokens`

### Task 3: 작업대 틀 — 상단 줄, 사진 띠, 운동 목록 (`ui`)

**Files:**
- Create: `ui/src/lib/workbench.ts`, `ui/src/components/workbench/TopBar.svelte`, `MonthRibbon.svelte`, `ActivityList.svelte`, `Workbench.svelte`
- Modify: `ui/src/App.svelte` (화면 상태 → 선택 상태)
- Test: `ui/src/lib/workbench.test.ts`

**Interfaces:**
- Consumes: Task 1 `DayItemDto` 새 필드, Task 2 `StatusBadge`, `ThemeToggle`, shadcn `dropdown-menu`, `toggle-group`, `scroll-area`.
- Produces:
```ts
// src/lib/workbench.ts
export type Item = DayItemDto & { date: string }                        // date "YYYY-MM-DD"
export type RibbonDay = { day: number; date: string; weekend: boolean; today: boolean; items: Item[] }
export function ribbonDays(year: number, month: number, days: DayDto[], todayIso: string): RibbonDay[]   // 그 달 날짜 수만큼
export type WeekGroup = { weekStart: string; label: string; km: number; items: Item[] }               // label "11/9–11/15"
export function groupByWeek(days: DayDto[]): WeekGroup[]               // 월요일 시작, 최근 주 먼저, 주 안에서도 최근 먼저
export type ListFilter = 'all' | 'ready' | 'published'
export function filterGroups(groups: WeekGroup[], f: ListFilter): WeekGroup[]   // 빈 주는 뺀다, km는 남은 항목 합
export function pickSelection(groups: WeekGroup[], current: string | null): string | null
export type GoalProgress = { rideRatio: number; runRatio: number; paceRatio: number | null; perDayKm: number | null }
export function goalProgress(v: MonthView, today: Date): GoalProgress
```
  - `goalProgress`: 비율은 `goalKm` 대비, 합이 1을 넘으면 비례 축소. `paceRatio` = 어제까지 지난 날수(`today.getDate() − 1`)/그 달 날수(지난 달은 1, 다음 달은 0), 목표 없으면 null. `perDayKm` = (목표−합계)/남은 날수(오늘 포함), 이미 넘었거나 지난 달이면 null.
  - `Workbench.svelte` props: `{ year, month, reloadKey, onMonth, onImport, onUnmatched, onSettings }`(기존 Dashboard와 같은 이름). 선택 id는 Workbench 안 `$state`.
  - 문구: 필터 "전체 / 게시 대기 N / 게시함", 가져오기 메뉴 "파일 가져오기 / 폴더 가져오기", 목표 입력은 TopBar 목표 막대를 누르면 열리는 작은 Dialog "월 목표 (km)"(비우면 목표 없음), 목표 옆 문구 "하루 {perDayKm}km면 목표"(`perDayKm` null이면 숨김), 백업 상태는 기존 `backupStatusText`.

- [ ] **Step 1: 실패하는 테스트** `workbench.test.ts`:
  - `ribbonDays(2026, 11, …)` 길이 30, `[0].weekend`(11/1 일요일) true, `today`는 `todayIso`와 같은 칸만, 같은 날 두 운동 → 그 칸 `items.length == 2`.
  - `groupByWeek`: 11/2·11/3·11/8·11/13 → 두 그룹, 첫 그룹 `label "11/9–11/15"`, `items[0].date "2026-11-13"`, km 합 일치.
  - `filterGroups(…, 'ready')`가 published만 있는 주를 뺀다.
  - `pickSelection`: current가 있으면 유지, 없으면 첫 그룹 첫 항목, 빈 배열 → null.
  - `goalProgress`: 목표 300, 라이딩 102·러닝 8.1, today 2026-11-14 → `rideRatio≈0.34`, `paceRatio≈13/30`, `perDayKm≈(300-110.1)/17`; 목표 null → `paceRatio,perDayKm` null; 지난 달 → `paceRatio 1, perDayKm null`.
- [ ] **Step 2:** `npm --prefix ui test` → FAIL.
- [ ] **Step 3: 구현** `workbench.ts`.
- [ ] **Step 4:** 테스트 PASS.
- [ ] **Step 5: 컴포넌트.** TopBar(달 이동·목표 막대·종목별 합계·ThemeToggle·설정·가져오기 메뉴), MonthRibbon(칸 = button, 대표 카드 `api.fileUrl(coverCard)` 배경, 아래 3px 종목 띠, 오른쪽 위 상태 점, 두 개 이상이면 배지, 운동 없는 칸은 `tabindex=-1`, `aria-label "11월 8일 반포… 42.6km 게시 대기"`), ActivityList(주 머리줄 + 행: 40×54 썸네일 또는 "사진 없음", 날짜·제목·StatusBadge, 오른쪽 거리와 `시간 속도/페이스`, 선택 행 `aria-current`), Workbench(그리드 레이아웃, Review Focus 1·2 문구). 상세 자리는 Task 4 전까지 기존 `ActivityDetail`을 그대로 넣는다.
- [ ] **Step 6:** `npm --prefix ui run check` PASS, `npm --prefix ui run check:colors`(새 파일만) PASS. 브라우저에서 띠·목록 선택 연동, 달 이동 시 선택 바뀜, 375px에서 가로 넘침 없음(`scrollWidth <= innerWidth`).
- [ ] **Step 7: 커밋** `feat(ui): workbench with month ribbon and activity list`

### Task 4: 상세 패널 — 기록 탭 (`ui`)

**Files:**
- Create: `ui/src/lib/profile.ts`, `ui/src/components/detail/ActivityPane.svelte`, `MetricStrip.svelte`, `PhotoRail.svelte`, `DistanceProfile.svelte`
- Modify: `ui/src/components/RouteMap.svelte`(토큰 색, 사진 강조, 어두운 타일), `ui/src/components/SplitsTable.svelte`(속도 막대), `ui/src/components/MetricCharts.svelte` → `DistanceProfile`로 대체 후 삭제
- Test: `ui/src/lib/profile.test.ts`

**Interfaces:**
- Consumes: `ActivityDetail`, `PhotoDto`, `SplitDto`, Task 2 `cssVar`, `tabs`, Task 3 `Workbench`(선택 id 전달).
- Produces:
```ts
// src/lib/profile.ts
export type Pin = { id: number; km: number; leftPct: number; src: string }      // src = cardPath ?? path
export function photoPins(photos: PhotoDto[], totalKm: number): Pin[]         // kmMark null·경로 없음 제외, 0–100 고정, km 순
export function durationSec(s: string): number                                // "m:ss" | "h:mm:ss"
export type SplitBar = SplitDto & { sec: number; widthPct: number; best: boolean }
export function splitBars(splits: SplitDto[]): SplitBar[]                      // 구간 길이 = km − 앞 구간 km(첫 구간은 km). sec는 km당 시간으로 환산. widthPct = 가장 짧은 sec / sec × 100, 가장 빠른 것 best
```
  - `ActivityPane` props `{ id: string; onDirtyChange?: (dirty: boolean) => void }`. 머리: 종목 색 막대 + 제목, 날짜, StatusBadge, 오른쪽 "글 복사"(게시 준비가 있으면 바로 복사), 탭 "기록 / 게시 준비".
  - 강조 상태 하나(`highlight: number` 시계열 index, `activePhoto: number | null`)를 ActivityPane이 갖고 RouteMap·PhotoRail·DistanceProfile에 내려준다. 사진에 마우스·포커스 → 그 km의 index로 강조.
  - `DistanceProfile`: Chart.js 세 줄(고도 `--alt`, 심박 `--hr`, 속도/페이스 `--ride`/`--run`, 페이스는 y 반전 유지) + 위쪽 사진 핀 줄 + 읽기 줄 "30.2km 고도 63m 심박 134bpm 속도 19km/h". mode-watcher `mode.current`가 바뀌면 차트를 지우고 `cssVar`로 다시 만든다.
  - RouteMap: 경로선 `--ride`, 출발 `--done`, 도착 `--hr`, 사진 점 `--run`, 강조 사진 점 크게. `.dark .leaflet-tile { filter: invert(1) hue-rotate(180deg) brightness(.85) contrast(.9) }`.

- [ ] **Step 1: 실패하는 테스트** `profile.test.ts`:
  - `photoPins`: kmMark `[6.2, null, 50]`, totalKm 42.6 → 2개, `leftPct` `≈14.55`, `100`; cardPath 없으면 path 사용; 둘 다 빈 문자열이면 제외.
  - `durationSec("4:05") == 245`, `durationSec("1:02:03") == 3723`.
  - `splitBars`: `[{km:1,"2:30"},{km:2,"2:00"},{km:2.4,"0:50"}]` → 두 번째 `best` & `widthPct 100`, 첫 번째 `80`, 마지막은 0.4km라 km당 125초 → `widthPct 96`.
- [ ] **Step 2:** FAIL 확인.
- [ ] **Step 3: 구현** `profile.ts` → PASS.
- [ ] **Step 4: 컴포넌트** 위 Interfaces대로. MetricStrip은 기존 `ActivityDetail.svelte`의 `metrics` 목록 규칙을 그대로 옮기고 거리 칸을 크게. 지도+사진 줄은 `minmax(0,1fr) 210px`, 860px 미만에서 사진은 4열 아래로. 기존 `ActivityDetail.svelte`, `MetricCharts.svelte` 삭제, Workbench가 `ActivityPane`을 쓰게.
- [ ] **Step 5:** `npm --prefix ui test`, `check`, `check:colors`(새·수정 파일) PASS. 브라우저: 그래프 hover → 지도 점 이동·읽기 줄 갱신, 사진 hover → 핀·지도 점 강조, 테마 전환 시 그래프 색 바뀜·지도 타일 어두워짐, GPS 없는 모의 운동(m2)에서 "GPS 기록이 없어요".
- [ ] **Step 6: 커밋** `feat(ui): activity pane with distance profile and photo pins`

### Task 5: 상세 패널 — 게시 준비 탭 (`ui`)

**Files:**
- Create: `ui/src/components/detail/PostTab.svelte`, `ui/src/components/detail/FeedPreview.svelte`
- Modify: `ui/src/components/PostPanel.svelte` → 로직을 `PostTab`으로 옮기고 삭제, `ui/src/components/RefinePreview.svelte`(shadcn Dialog), `ActivityPane.svelte`(탭 연결, 저장 안 한 수정 확인)
- Test: `ui/src/lib/enrich.test.ts` 기존 유지(로직 변경 없음), 수동 브라우저 확인

**Interfaces:**
- Consumes: 기존 `api.postView/savePost/regeneratePost/markPublished/setPlace/setRoutePoints/setWeather/enrichActivity/refinePost/refineStatus/regenerateCards/copyText/openFolder`, `mergeEditor` 등 `lib/enrich.ts`, Task 4 `ActivityPane`의 `onDirtyChange`.
- Produces: `PostTab` props `{ id: string; onDirtyChange: (dirty: boolean) => void; onStatusChange: (s: PostStatus) => void }`. `FeedPreview` props `{ text: string; hashtags: string[]; cards: string[] }`.
  - 배치: 왼쪽 = 장소·지나간 곳·날씨 칸(한 줄 칩), 본문(글자 수 "96자"), 해시태그, `{ai.label}로 다듬기`·"템플릿으로 다시 쓰기"·"저장", 사진 보내기 체크 + 기존 안내 문구, 카드 줄(작은 썸네일 + "카드 다시 만들기"·"카드 폴더 열기"). 오른쪽 300px = "올라갈 모습" 미리보기(본문·해시태그·카드 가로 줄) + "글 복사"(primary) + "게시 완료로 표시"(초록 테두리) + "스레드에 직접 올린 뒤 완료로 표시하세요.".
  - 기존 `confirm()` 두 곳(다시 쓰기, 나가기)을 AlertDialog로. 나가기 확인은 ActivityPane이 탭 전환·다른 운동 선택 때 `dirty`면 띄운다(Workbench 선택 변경도 이 확인을 거침 → `ActivityPane`이 `requestLeave(): Promise<boolean>`를 `bind:this`로 노출).
  - 게시 완료로 표시하면 `onStatusChange('published')` → Workbench가 `reloadKey++`로 띠·목록 갱신.

- [ ] **Step 1:** `PostPanel.svelte`의 상태·`act` 흐름을 그대로 `PostTab`으로 옮기고 화면만 위 배치로. 기존 동작(저장, 복사 메시지, 다시 쓰기, 다듬기 미리보기·적용, 장소·경로·날씨 저장과 "자동으로", 카드 다시 만들기 cacheBust) 유지.
- [ ] **Step 2:** `npm --prefix ui test`, `check`, `check:colors` PASS.
- [ ] **Step 3: 브라우저 확인** (모의 모드): 본문 고치고 "기록" 탭 → 확인창, 취소하면 그대로, 나가면 탭 이동. 다른 운동 선택도 같은 확인. "게시 완료로 표시" → 머리·목록·띠 상태가 "게시함"으로. 밝게/어둡게 둘 다 미리보기 대비 확인.
- [ ] **Step 4: 커밋** `feat(ui): post tab with feed preview inside activity pane`

### Task 6: 남은 창 옮기기, 옛 화면 삭제, 최종 확인 (`ui`)

**Files:**
- Modify: `ui/src/components/Settings.svelte`, `ui/src/components/ImportPanel.svelte`(shadcn Dialog, 토큰 색), `ui/src/App.svelte`(알림 줄을 토큰 색으로, 화면 상태 제거), `ui/src/app.css`(옛 `.btn`·`--paper`·`--ink`·`--track` 등 제거)
- Delete: `ui/src/components/Dashboard.svelte`, `Calendar.svelte`, `Gauge.svelte`, `WeekBars.svelte`, `ui/src/lib/calendar.ts`·`calendar.test.ts`(쓰는 곳이 없어지면)
- Test: `ui/scripts/check-colors.sh`를 `src/components`까지 넓힘

- [ ] **Step 1:** `check-colors.sh` 대상을 `src/components` 전체와 `src/lib/components`로 바꿈 → 실행해 FAIL 목록 확인.
- [ ] **Step 2:** Settings·ImportPanel을 Dialog로, 고정 색을 토큰으로 바꾸고 옛 컴포넌트·옛 토큰 삭제. `grep -rn "Dashboard\|Calendar\|Gauge\|WeekBars\|buildMonthGrid" ui/src` 결과 없음.
- [ ] **Step 3:** `npm --prefix ui test`, `npm --prefix ui run check`, `npm --prefix ui run check:colors`, `npm --prefix ui run build`, `cargo test --workspace` 모두 PASS.
- [ ] **Step 4: 브라우저 최종 확인** — 1360×900과 375×812, 밝게·어둡게·시스템 각각: 띠/목록/기록 탭/게시 탭/설정/가져오기 결과 창. Review Focus 1–6을 하나씩 재현해 확인하고 스크린샷을 남긴다. Tauri 창에서 파일 끌어다 놓기 가져오기가 그대로 되는지 `cargo tauri dev`로 한 번 확인.
- [ ] **Step 5: 커밋** `refactor(ui): move dialogs to shadcn, drop old dashboard screens`
