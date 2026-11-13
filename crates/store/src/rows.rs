use workout_core::chrono::{DateTime, Utc};
use workout_core::model::{ActivityHeader, LatLon};
use workout_post::{PlaceText, PublishMethod, WeatherText};

/// 운동 한 개. 경로는 화면용으로 줄인 좌표(≤1500), 파일 경로는 라이브러리 기준 상대 경로.
#[derive(Clone, Debug)]
pub struct ActivityRow {
    pub header: ActivityHeader,
    pub fit_path: String,
    pub route: Vec<LatLon>,
}

/// 사진 한 장. `id`는 insert 때 무시된다.
#[derive(Clone, Debug, PartialEq)]
pub struct PhotoRow {
    pub id: i64,
    pub activity_id: Option<String>,
    pub path: String,
    pub taken_at: Option<DateTime<Utc>>,
    pub km_mark: Option<f64>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub card_path: Option<String>,
    /// 파일 내용 해시 (같은 사진 중복 판단).
    pub content_hash: Option<String>,
}

/// 운동별 게시 초안.
#[derive(Clone, Debug, PartialEq)]
pub struct PostRow {
    pub activity_id: String,
    pub body: String,
    pub hashtags: Vec<String>,
    pub place: Option<PlaceText>,
    pub route_points: Vec<String>,
    pub weather: Option<WeatherText>,
    /// 사용자가 직접 고쳤는지. true면 자동으로 다시 쓰지 않는다.
    pub edited: bool,
    pub published_at: Option<DateTime<Utc>>,
    /// 사용자가 직접 정한 값이면 자동 조회가 덮어쓰지 않는다.
    pub place_user_set: bool,
    pub route_user_set: bool,
    pub weather_user_set: bool,
    /// 마지막 자동 조회 값 (장소는 항상 추정).
    pub auto_place: Option<String>,
    pub auto_route_points: Vec<String>,
    pub auto_weather: Option<WeatherText>,
}

#[derive(Clone, Debug)]
pub struct PublishRow {
    pub activity_id: String,
    pub method: PublishMethod,
    pub at: DateTime<Utc>,
    pub external_id: Option<String>,
}
