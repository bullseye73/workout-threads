# Workout Threads 아키텍처 구성도

이 문서는 `workout-app`의 현재 구현을 기준으로 전체 시스템 계층, Rust 워크스페이스 크레이트 간 의존 관계, 그리고 주요 데이터 흐름을 정리합니다.

## 1. 전체 시스템 구성도

```mermaid
flowchart TB
    subgraph UI["Frontend (ui/ — Svelte 5 + Tailwind v4 + shadcn-svelte)"]
        Workbench["Workbench\n(TopBar · MonthRibbon · ActivityList)"]
        Detail["Detail Pane\n(ActivityPane · PostTab · FeedPreview)"]
        Composer["Composer Modal\n(FIT/사진 드롭 · Garmin Picker)"]
        Settings["Settings Modal\n(테마 · AI 키 · Garmin · BullsNote · Drive)"]
    end

    subgraph Tauri["Desktop Shell (src-tauri/)"]
        TauriCmd["Tauri Commands & App Lifecycle\n(10분 주기 자동 백업 워커)"]
    end

    subgraph Service["Application Service Layer (crates/service)"]
        Lib["Library Facade\n(import · enrich · upload · backup · garmin · bullsnote)"]
        DTO["DTO & ts-rs TypeScript Bindings"]
    end

    subgraph Domain["Domain & Infrastructure Crates"]
        Core["crates/core\n(FIT 파싱 · 통계 · 구간 · 사진 매칭 · 프라이버시 500m)"]
        Card["crates/card\n(대표 사진 카드 렌더링 · 나머지 사진 2160px 최적화)"]
        Post["crates/post\n(템플릿 글 생성 · 게시 상태)"]
        Enrich["crates/enrich\n(Nominatim · Open-Meteo · Gemini/Claude)"]
        Garmin["crates/garmin\n(Personal API JWT · 활동/FIT 다운로드)"]
        GDrive["crates/gdrive\n(OAuth2 · Drive 증분 업로드/복원)"]
        Store["crates/store\n(SQLite WAL · 마이그레이션)"]
        CLI["crates/cli\n(workout-cli prepare)"]
    end

    subgraph Local["로컬 저장소 및 연동"]
        DB[("SQLite (library.db)\n+ files/ · garmin-inbox/")]
        Keyring[("OS Keyring\n(AI · Garmin · OAuth Secret)")]
        BullsNoteVault[("BullsNote Vault\n(workout-goal-YYYY-MM.md)")]
    end

    subgraph External["외부 서비스"]
        GarminAPI["Garmin Connect Personal API"]
        GeoWeather["OpenStreetMap Nominatim\n& Open-Meteo"]
        AI["Google Gemini / Anthropic Claude"]
        DriveAPI["Google Drive API"]
    end

    UI -->|invoke IPC| TauriCmd
    TauriCmd --> Lib
    CLI --> Core
    CLI --> Card
    CLI --> Post

    Lib --> Core
    Lib --> Card
    Lib --> Post
    Lib --> Enrich
    Lib --> Garmin
    Lib --> GDrive
    Lib --> Store

    Store --> DB
    Lib --> Keyring
    Lib --> BullsNoteVault

    Garmin --> GarminAPI
    Enrich --> GeoWeather
    Enrich --> AI
    GDrive --> DriveAPI
```

## 2. 크레이트별 책임

| 크레이트 | 주요 역할 |
|---|---|
| `crates/core` | Garmin `.fit` 파일 디코딩, 거리/고도/심박/페이스 통계, km 스플릿, EXIF 시각 기반 사진-운동 이진 탐색(`binary_search_by_key`) 매칭, 출발·도착 500m 프라이버시 마스킹 |
| `crates/card` | 대표 사진 1장에 운동 요약·경로선 카드 오버레이 렌더링(`Pretendard` 내장), 나머지 첨부 사진 EXIF 제거 및 최대 변 2160px 이하 리사이즈 최적화 |
| `crates/post` | 활동/날씨/장소 컨텍스트 기반 스레드 본문·해시태그 템플릿 렌더링 및 게시 상태 관리 |
| `crates/enrich` | Nominatim 역지오코딩(프라이버시 반경 밖 좌표), Open-Meteo 기상 조회, Gemini/Claude 글 다듬기(선택 시 위치 제거된 저해상도 사진 첨부) |
| `crates/garmin` | Garmin Personal API 키 → JWT Bearer 토큰 교환(24시간 캐시), 활동 목록 조회, `.fit` ZIP 다운로드 및 압축 해제 |
| `crates/gdrive` | Google Desktop OAuth 및 `WorkoutApp` 폴더 증분 백업/복원 |
| `crates/store` | SQLite(`journal_mode = WAL`, `synchronous = NORMAL`) 기반 활동·사진·게시물·월간 목표 영속화 |
| `crates/service` | 위 크레이트를 조합하는 애플리케이션 서비스 파사드, `ts-rs` DTO 바인딩 생성, `bullsnote.rs`를 통한 월별 목표 노트 원자적 갱신 |
| `crates/cli` | 데스크톱 UI 없이 단일 날짜 폴더를 처리하는 CLI (`workout-cli prepare`) |

## 3. 핵심 데이터 흐름

### 3-1. 게시물 작성 및 사진 처리 파이프라인
```mermaid
sequenceDiagram
    actor User
    participant UI as Composer (Svelte)
    participant Svc as service::import / garmin
    participant Core as core::matching
    participant Card as card::render / upload
    participant Store as store (SQLite WAL)

    alt Garmin Personal API 가져오기
        User->>UI: Garmin 활동 선택
        UI->>Svc: fetch_garmin_fits()
        Svc-->>UI: garmin-inbox/*.fit 경로 반환
    end

    User->>UI: 파일/사진 드롭 & "게시물 만들기"
    UI->>Svc: commit_upload(plan)
    Svc->>Core: FIT 파싱 & 사진 시간 매칭 (이진 탐색)
    loop 운동별 첨부 사진
        alt 대표 사진 (첫 번째 사진)
            Svc->>Card: 오버레이 카드 이미지 생성 (*_card.jpg)
        else 추가 사진
            Svc->>Card: EXIF 제거 + 2160px 이하 크기 최적화
        end
    end
    Svc->>Store: 활동·사진·초안 게시물 저장
    Svc->>Svc: BullsNote 연동 활성 시 월별 목표 노트 갱신
    Svc-->>UI: UploadResult 반환 및 백그라운드 장소/날씨 보강 시작
```
