# Workout Threads

Garmin FIT 운동 기록과 사진을 넣으면 한 달 운동을 한눈에 보여 주고, 날짜마다 스레드(Threads)에 올릴 **글·해시태그·카드 이미지**를 준비해 주는 데스크톱 앱입니다. macOS, Windows, Linux에서 돌아갑니다.

게시는 앱이 하지 않습니다. 앱은 글을 복사하고 카드를 만들어 두고, 사용자가 스레드에 직접 올린 뒤 "게시 완료"로 표시합니다.

## 무엇을 해 주나요

- **게시물 작성:** 어디에 있는 FIT 파일과 사진이든 올리면, 무엇이 만들어질지 먼저 보여 주고 앱 보관함에 저장해 **운동마다 게시물을 하나씩** 만듭니다. 사진은 운동 시간(또는 그날 하나뿐인 운동)에 맞춰 붙고 카드 이미지가 만들어집니다. 원본 파일은 그대로 두고, 같은 파일은 다시 넣지 않습니다.
- **작업대 화면:** 위쪽에 이번 달 목표 막대와 종목별 합계, 그 아래 한 달 사진 띠(운동한 날은 카드 사진), 왼쪽에 주별 운동 목록, 오른쪽에 선택한 운동의 상세가 한 화면에 나옵니다.
- **운동 상세(기록 탭):** 핵심 수치, 경로 지도(출발·도착·사진 위치), 거리 축 고도·심박·속도(러닝은 페이스) 그래프, km 구간표. 사진은 찍힌 km 지점에 꽂혀 있고, 그래프나 사진에 마우스를 올리면 지도에 그 위치가 표시됩니다.
- **게시 준비 탭:** 장소·지나간 곳·날씨 자동 입력(직접 고칠 수 있음), 템플릿 글과 해시태그 편집, AI(Gemini 또는 Claude)로 다듬기, 피드에 올라갈 모습 미리보기, 글 복사, 게시 완료 표시.
- **위치 보호:** 집 근처일 수 있는 출발·도착 500m 안의 지점은 장소 조회와 글에 쓰지 않고, 설정에서 카드 경로선의 출발·도착 부분도 가릴 수 있습니다.
- **목표:** 라이딩·러닝 월 목표를 따로 정하고, 바꾸기 전까지 다음 달에도 그대로 적용됩니다.
- **테마:** 설정에서 시스템 / 밝게 / 어둡게.
- **백업:** 구글 드라이브에 10분마다 자동 백업, 빈 라이브러리에서 복원.

## 사용 방법

### 1. 게시물 작성 (FIT·사진 올리기)

1. 오른쪽 위 **게시물 작성**을 누르거나, FIT 파일·사진·폴더를 앱 창에 끌어다 놓습니다. 폴더는 하위 폴더 3단계까지 찾습니다.
2. 창에 미리보기가 나옵니다: **만들 게시물**(FIT마다 하나), **이미 있는 운동**(다시 올린 FIT, 사진을 보탤 운동), **제외**(촬영 날짜가 없거나 붙일 운동이 없는 사진, 이미 있는 사진).
3. 빼고 싶은 운동은 **빼기**, 사진은 이름을 눌러 뺍니다.
4. **게시물 N개 만들기**를 누르면 파일이 앱 보관함에 복사되고 게시물이 만들어집니다. 하나만 만들었으면 바로 그 게시 준비 탭이 열립니다.
5. 새 운동은 인터넷으로 장소·날씨를 하나씩 채웁니다(아래쪽 알림에 진행 상황이 보입니다).
6. 예전에 들어와 운동에 붙지 않은 사진이 있으면 위쪽 **안 붙은 사진 N장** 버튼으로 직접 붙일 수 있습니다.

사진이 붙는 순서: 이번에 올린 운동과 시간이 맞으면 그 운동 → 보관함의 운동과 시간이 맞으면 그 운동 → 촬영한 날 운동이 하나뿐이면 그 운동 → 어디에도 안 맞으면 제외(보관하지 않음).

### 2. 한 달 보기

- **‹ ›** 로 달을 바꿉니다.
- 목표 막대를 누르면 라이딩·러닝 월 목표(km)를 따로 정할 수 있습니다. 막대의 세로선은 오늘까지 채워야 할 거리이고, 옆에 "하루 N km면 목표"가 나옵니다. 목표는 바꾸기 전까지 다음 달에도 그대로입니다.
- 사진 띠의 칸이나 목록의 줄을 누르면 그 운동이 오른쪽에 열립니다.
- 목록 위 **전체 / 게시 대기 / 게시함** 으로 거를 수 있습니다.

게시 상태 표시:

| 표시 | 뜻 |
|---|---|
| 점선 원 · 글 없음 | 아직 게시 준비가 안 됨 |
| 호박색 · 게시 대기 | 글과 카드가 준비됨 |
| 초록 · 게시함 | 게시 완료로 표시함 |

### 3. 게시 준비하고 올리기

1. 운동을 고르고 **게시 준비** 탭으로 갑니다.
2. 장소·지나간 곳·날씨를 확인합니다. 칸을 고치면 저장되고, **자동으로**를 누르면 조회한 값으로 돌아갑니다.
3. 본문과 해시태그를 고칩니다. **Gemini로 다듬기**(설정에서 고른 AI)를 누르면 다듬은 글을 나란히 보여 주고, 적용할지 고를 수 있습니다. **사진도 보내기**를 켜면 위치 정보를 지운 작은 사본이 AI 서버로 갑니다.
4. 오른쪽 **올라갈 모습**을 보고 **글 복사**를 누른 뒤 스레드에 붙여 넣습니다. 카드 이미지는 **카드 폴더 열기**에서 꺼내 올립니다.
5. 다 올렸으면 **게시 완료로 표시**를 누릅니다. 목록과 사진 띠에 초록 표시가 붙습니다.

저장하지 않은 수정이 있는 채로 탭·운동·달을 바꾸면 나갈지 먼저 묻습니다.

### 4. 설정

오른쪽 위 **설정**에서:

- **화면 테마:** 시스템 / 밝게 / 어둡게.
- **글 다듬기 AI:** Gemini(기본) 또는 Claude를 고르고 API 키와 모델을 넣습니다. 키는 이 컴퓨터의 키 보관함(macOS 키체인 등)에만 저장되고 다시 보여 주지 않습니다.
- **카드 경로선:** 출발·도착 500m를 카드에서 가리기. 이미 만든 카드는 게시 준비 탭의 **카드 다시 만들기**로 바꿉니다.
- **구글 드라이브 백업:** 구글 Cloud의 "데스크톱 앱" OAuth 클라이언트 ID와 secret을 넣고 로그인합니다. 만드는 방법은 [docs/setup-google-oauth.md](docs/setup-google-oauth.md)에 있습니다. 앱이 만든 파일만 드라이브의 `WorkoutApp` 폴더에 올리고, 어느 쪽 파일도 지우지 않습니다.

### 데이터는 어디에 있나요

앱 데이터 폴더 안의 라이브러리(`library.db` + `files/`)에 저장됩니다.

| OS | 위치 |
|---|---|
| macOS | `~/Library/Application Support/dev.sns-create.workout/` |
| Windows | `%APPDATA%\dev.sns-create.workout\` |
| Linux | `~/.local/share/dev.sns-create.workout/` |

### 쓰는 외부 서비스

| 용도 | 서비스 | 보내는 것 |
|---|---|---|
| 장소 이름 | OpenStreetMap Nominatim | 출발·도착 500m 밖 지점의 좌표 |
| 날씨 | Open-Meteo | 운동 시각과 대략적인 위치 |
| 지도 타일 | OpenStreetMap | 보고 있는 지도 영역 |
| 글 다듬기(누를 때만) | Google Gemini 또는 Anthropic Claude | 글, 사실 정보(좌표 없음), 고르면 위치를 지운 사진 |
| 백업(설정했을 때만) | Google Drive | 라이브러리 파일 |

## 빌드와 실행

### 준비물

- [Rust](https://rustup.rs/) 1.85 이상 (crate들이 edition 2024를 씁니다)
- Node.js 24 (CI와 같은 버전)
- Tauri 2가 요구하는 OS별 도구 — [Tauri 사전 준비 안내](https://v2.tauri.app/start/prerequisites/)
  - macOS: Xcode Command Line Tools (`xcode-select --install`)
  - Windows: Microsoft C++ Build Tools, WebView2(Windows 10/11에는 기본 설치)
  - Linux(Ubuntu 22.04 기준):

```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev libdbus-1-dev pkg-config build-essential file
```

### 처음 한 번

저장소 루트(이 README가 있는 폴더)에서 화면 의존성을 설치합니다.

```bash
npm ci --prefix ui
```

커밋 전에 `cargo fmt`·`cargo clippy` 검사를 자동으로 돌리려면 저장소의 훅을 켭니다.

```bash
git config core.hooksPath .githooks
```

### 데스크톱 앱 개발 모드

화면 개발 서버(5173 포트)를 띄우고 앱 창을 엽니다. 화면 코드를 고치면 바로 반영됩니다. 저장소 루트에서 실행합니다.

```bash
npm --prefix ui run app:dev
```

`app:dev`는 `ui/node_modules`의 Tauri CLI를 `src-tauri/` 폴더에서 실행합니다. 다른 폴더에서 `npx --prefix ../ui tauri dev`를 직접 실행하면 `../ui`가 엉뚱한 곳을 가리켜 `failed to run 'cargo metadata'` 오류가 납니다. 직접 실행하려면 반드시 `src-tauri/` 안에서 하세요.

환경 점검(Rust·Node·Tauri 버전, 프로젝트 인식)은 다음 명령으로 합니다.

```bash
npm --prefix ui run app:info
```

### 브라우저에서 화면만 보기

Rust 없이 가짜 데이터(`ui/src/lib/mock.ts`)로 화면을 띄웁니다. 화면 작업이나 디자인 확인에 씁니다. 파일 끌어다 놓기, 키 보관함, 드라이브 백업은 동작하지 않습니다.

```bash
npm --prefix ui run dev
```

브라우저에서 http://localhost:5173 을 엽니다.

### 설치 파일 만들기

```bash
npm --prefix ui run app:build
```

결과물은 `target/release/bundle/` 아래(macOS `dmg/`, Windows `msi/`·`nsis/`, Linux `deb/`·`appimage/`)에 생깁니다. 서명은 하지 않으므로 처음 실행할 때 OS의 확인 창이 뜰 수 있습니다.

GitHub에 `main`으로 푸시하면 [.github/workflows/build.yml](.github/workflows/build.yml)이 세 OS에서 테스트하고 설치 파일을 만들어 Actions 결과물로 올립니다.

### 명령줄 도구 (선택)

앱 없이 날짜 폴더 하나에서 카드와 글만 만들 때 씁니다. 폴더의 `.fit`과 사진으로 `<사진>_card.jpg`와 `post_HHMM.txt`를 만듭니다.

```bash
cargo run -p workout-cli -- prepare ../20261025
```

### 테스트와 검사

```bash
cargo test --workspace
```

```bash
npm --prefix ui test
```

```bash
npm --prefix ui run check
```

```bash
npm --prefix ui run check:colors
```

- 실제 운동 샘플(GPS 포함)이 필요한 Rust 테스트는 `WORKOUT_SAMPLES` 환경변수의 폴더(없으면 저장소 바깥 상위 폴더 `sns_create/`)에서 샘플을 찾고, 없으면 건너뜁니다.
- `check:colors`는 컴포넌트에 색을 직접 쓰지 않았는지 검사합니다. 색은 `ui/src/app.css`의 토큰만 씁니다.
- Rust 타입을 바꾸면 `cargo test` 실행 때 `ui/src/lib/bindings/`의 TypeScript 타입이 다시 생성됩니다(ts-rs). 생성물도 커밋합니다.

## 프로젝트 구조

```text
workout-app/
├─ crates/
│  ├─ core      FIT 읽기, 통계, km 구간, 사진 매칭, 위치 보호, 표시 형식
│  ├─ card      카드 이미지 렌더러 (Pretendard 글꼴 포함)
│  ├─ post      템플릿 글과 게시 기록
│  ├─ enrich    장소(Nominatim)·날씨(Open-Meteo), Gemini·Claude 글 다듬기
│  ├─ gdrive    구글 OAuth와 드라이브 업로드
│  ├─ store     SQLite 저장소
│  ├─ service   앱 기능을 모은 라이브러리 (화면이 쓰는 DTO 포함)
│  ├─ cli       workout-cli (prepare 명령)
│  └─ testkit   테스트용 샘플 경로 도우미
├─ src-tauri/   데스크톱 앱 (Tauri 명령, 백업 반복 작업)
├─ ui/          화면 (Svelte 5, Tailwind CSS v4, shadcn-svelte)
│  └─ src/
│     ├─ components/workbench  상단 줄, 사진 띠, 운동 목록
│     ├─ components/detail     상세 패널(기록·게시 준비 탭)
│     ├─ lib/                  API, 계산 함수(+테스트), 테마, 생성된 타입
│     └─ lib/components/ui     shadcn-svelte 컴포넌트 (CLI로 추가)
└─ docs/        설계, 구현 계획, 디자인 시안, 구글 OAuth 설정
```

설계와 단계별 구현 계획은 [docs/superpowers/specs](docs/superpowers/specs)와 [docs/superpowers/plans](docs/superpowers/plans)에 있습니다.
