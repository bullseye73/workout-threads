//! 화면으로 보내는 데이터. ts-rs가 ui/src/lib/bindings 에 TypeScript 타입을 만든다.
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ImportIssue {
    pub file: String,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PhotoDto {
    #[ts(type = "number")]
    pub id: i64,
    pub file_name: String,
    /// 절대 경로.
    pub path: String,
    pub card_path: Option<String>,
    /// RFC3339.
    pub taken_at: Option<String>,
    pub km_mark: Option<f64>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ImportReport {
    pub added_activities: Vec<String>,
    pub duplicate_files: Vec<String>,
    pub failed: Vec<ImportIssue>,
    pub cards_created: u32,
    pub unmatched_photos: Vec<PhotoDto>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum SportKind {
    Running,
    Cycling,
    Walking,
    Hiking,
    Other,
}

impl From<workout_core::model::Sport> for SportKind {
    fn from(s: workout_core::model::Sport) -> Self {
        use workout_core::model::Sport::*;
        match s {
            Running => SportKind::Running,
            Cycling => SportKind::Cycling,
            Walking => SportKind::Walking,
            Hiking => SportKind::Hiking,
            Other(_) => SportKind::Other,
        }
    }
}

/// 게시 상태: 준비 안 됨 / 준비됨(글 + 카드 1장 이상) / 게시함.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum PostStatus {
    None,
    Ready,
    Published,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SportSummaryDto {
    pub count: u32,
    pub km: f64,
    pub moving_s: f64,
    /// 러닝만, "5'30\"".
    pub avg_pace: Option<String>,
    /// 라이딩만.
    pub avg_speed_kmh: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WeekDto {
    pub week_start: String,
    pub running_km: f64,
    pub cycling_km: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DayItemDto {
    pub activity_id: String,
    pub sport: SportKind,
    pub km: f64,
    pub post_status: PostStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DayDto {
    /// YYYY-MM-DD.
    pub date: String,
    pub items: Vec<DayItemDto>,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MonthView {
    pub year: i32,
    pub month: u32,
    pub goal_km: Option<f64>,
    pub total_km: f64,
    pub total_moving_s: f64,
    pub running: SportSummaryDto,
    pub cycling: SportSummaryDto,
    pub other_count: u32,
    pub avg_hr: Option<f64>,
    pub weekly: Vec<WeekDto>,
    /// 운동이 있는 날만, 날짜순.
    pub days: Vec<DayDto>,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MetricsDto {
    pub distance_km: f64,
    pub moving: String,
    pub elapsed: String,
    pub pace: Option<String>,
    pub speed_kmh: Option<f64>,
    pub avg_hr: Option<u8>,
    pub max_hr: Option<u8>,
    pub ascent_m: Option<u16>,
    pub avg_cadence: Option<u8>,
}

/// 그래프용 시계열 (최대 1000점, 모든 배열 같은 길이).
#[derive(Clone, Debug, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SeriesDto {
    pub distance_km: Vec<f64>,
    pub hr: Vec<Option<u8>>,
    pub speed_kmh: Vec<Option<f64>>,
    pub altitude_m: Vec<Option<f64>>,
    /// [lat, lon].
    pub lat_lon: Vec<Option<[f64; 2]>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SplitDto {
    pub km: u32,
    pub duration: String,
    pub avg_hr: Option<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ActivityDetail {
    pub id: String,
    pub sport: SportKind,
    pub title: String,
    /// YYYY.MM.DD.
    pub date: String,
    pub metrics: MetricsDto,
    /// [lat, lon].
    pub route: Vec<[f64; 2]>,
    pub start: Option<[f64; 2]>,
    pub end: Option<[f64; 2]>,
    pub photos: Vec<PhotoDto>,
    pub series: SeriesDto,
    pub splits: Vec<SplitDto>,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlaceDto {
    pub name: String,
    pub estimated: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PostView {
    pub activity_id: String,
    pub body: String,
    pub hashtags: Vec<String>,
    pub full_text: String,
    pub place: Option<PlaceDto>,
    /// 절대 경로.
    pub card_paths: Vec<String>,
    /// 카드가 있는 운동 날짜 폴더 (절대 경로).
    pub folder: String,
    pub status: PostStatus,
    pub edited: bool,
    /// 지나간 동네 (출발·도착 제외).
    pub route_points: Vec<String>,
    pub weather: Option<WeatherDto>,
    pub place_user_set: bool,
    pub route_user_set: bool,
    pub weather_user_set: bool,
    /// 장소·날씨 조회 중 생긴 문제 (조회 직후에만 채워짐).
    pub enrich_error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WeatherDto {
    pub description: String,
    pub temp_c: f64,
}

/// Claude로 다듬은 글 (저장 전 미리보기).
#[derive(Clone, Debug, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RefineDto {
    pub body: String,
    pub hashtags: Vec<String>,
    pub full_text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProviderDto {
    /// "gemini" | "claude"
    pub id: String,
    pub label: String,
    /// 사진을 받는 회사 (Google | Anthropic).
    pub company: String,
    pub has_key: bool,
    pub model: String,
}

/// 글 다듬기 AI 상태: 고른 AI와 전체 목록.
#[derive(Clone, Debug, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RefineStatusDto {
    pub provider: String,
    pub label: String,
    pub company: String,
    pub has_key: bool,
    pub model: String,
    pub providers: Vec<ProviderDto>,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SettingsDto {
    /// 카드 경로선에서 출발·도착 500m를 가린다.
    pub card_route_trim: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BackupStatusDto {
    /// 구글 클라이언트 ID·secret이 설정됨.
    pub configured: bool,
    pub signed_in: bool,
    /// RFC3339.
    pub last_success: Option<String>,
    pub last_error: Option<String>,
    /// 아직 올리지 않은 파일 수.
    pub pending: u32,
    /// 로컬이 비어 있어 복원할 수 있음.
    pub can_restore: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GoogleClientDto {
    pub client_id: String,
    pub has_secret: bool,
}
