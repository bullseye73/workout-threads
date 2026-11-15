//! Garmin Connect Personal API HTTP 클라이언트.
//! 앱 로직은 넣지 않고 인증 토큰 교환, 활동 목록 조회, FIT 다운로드, 일일 웰니스 조회만 담당한다.
use std::io::Read;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use flate2::read::DeflateDecoder;
use serde::Deserialize;
use workout_core::chrono::NaiveDate;
use workout_enrich::{Body, EnrichError, Http, HttpRequest, HttpResponse, Method, TimeoutClass};

pub const DEFAULT_BASE_URL: &str = "https://personal.connectapi.garmin.com";
const TOKEN_PATH: &str = "/personalapitoken-service/personal-api/personal-api-key/token";
const PAGE_LIMIT: usize = 100;
const EXPIRY_SKEW_SECS: u64 = 60;

#[derive(thiserror::Error, Debug)]
pub enum GarminError {
    #[error("Garmin API 키가 설정되지 않았어요")]
    NoKey,
    #[error("Garmin 토큰이 만료되었거나 키가 올바르지 않아요")]
    Unauthorized,
    #[error("Garmin API 호출 제한에 걸렸어요. 잠시 후 다시 시도해 주세요")]
    RateLimited,
    #[error("네트워크 오류: {0}")]
    Network(String),
    #[error("Garmin 서버 오류({0}): {1}")]
    Status(u16, String),
    #[error("응답을 해석하지 못했어요: {0}")]
    Parse(String),
}

pub type Result<T> = std::result::Result<T, GarminError>;

#[derive(Clone, Debug, PartialEq)]
pub struct GarminActivity {
    pub id: String,
    pub name: String,
    pub sport_type_key: String,
    pub start_time_gmt: String,
    pub start_time_local: String,
    pub distance_m: f64,
    pub duration_s: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DailyWellness {
    pub date: NaiveDate,
    pub total_steps: Option<u32>,
    pub resting_hr: Option<u32>,
    pub avg_stress: Option<i32>,
    pub body_battery_high: Option<i32>,
    pub body_battery_low: Option<i32>,
}

pub trait TokenProvider: Send + Sync {
    fn access_token(&self, http: &dyn Http) -> Result<String>;
    fn refresh_token(&self, http: &dyn Http) -> Result<String>;
}

type ReadOptFn = Arc<dyn Fn() -> Option<String> + Send + Sync>;
type WriteOptFn = Arc<dyn Fn(Option<String>) + Send + Sync>;

/// 개인 API 키로 24시간 Bearer 액세스 토큰을 발급·캐시한다.
/// 캐시 형식은 `"토큰|만료unix초"`이다.
pub struct KeyTokenProvider {
    read_key: ReadOptFn,
    read_cache: ReadOptFn,
    write_cache: WriteOptFn,
    base_url: String,
}

impl KeyTokenProvider {
    pub fn new(read_key: ReadOptFn, read_cache: ReadOptFn, write_cache: WriteOptFn) -> Self {
        Self {
            read_key,
            read_cache,
            write_cache,
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    fn exchange(&self, http: &dyn Http) -> Result<String> {
        let key = (self.read_key)().ok_or(GarminError::NoKey)?;
        let req = HttpRequest {
            method: Method::Post,
            url: format!("{}{TOKEN_PATH}", self.base_url),
            headers: vec![("Authorization".into(), format!("ApiKey {key}"))],
            body: Body::Empty,
            timeout: TimeoutClass::Lookup,
        };
        let resp = send_checked(http, req)?;
        let parsed: TokenResponse =
            serde_json::from_slice(&resp.body).map_err(|e| GarminError::Parse(e.to_string()))?;
        let ttl = parsed.expires_in.unwrap_or(86_400);
        let exp = Self::now_secs().saturating_add(ttl);
        (self.write_cache)(Some(format!("{}|{exp}", parsed.access_token)));
        Ok(parsed.access_token)
    }
}

impl TokenProvider for KeyTokenProvider {
    fn access_token(&self, http: &dyn Http) -> Result<String> {
        if let Some(cached) = (self.read_cache)()
            && let Some((tok, exp_str)) = cached.split_once('|')
            && let Ok(exp) = exp_str.parse::<u64>()
            && exp > Self::now_secs().saturating_add(EXPIRY_SKEW_SECS)
        {
            return Ok(tok.to_string());
        }
        self.exchange(http)
    }

    fn refresh_token(&self, http: &dyn Http) -> Result<String> {
        (self.write_cache)(None);
        self.exchange(http)
    }
}

pub struct GarminClient {
    http: Arc<dyn Http>,
    tokens: Arc<dyn TokenProvider>,
    base_url: String,
}

impl GarminClient {
    pub fn new(http: Arc<dyn Http>, tokens: Arc<dyn TokenProvider>) -> Self {
        Self {
            http,
            tokens,
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    fn get_authed(&self, path_and_query: &str, timeout: TimeoutClass) -> Result<HttpResponse> {
        let mut tok = self.tokens.access_token(self.http.as_ref())?;
        for attempt in 0..2 {
            let req = HttpRequest {
                method: Method::Get,
                url: format!("{}{path_and_query}", self.base_url),
                headers: vec![("Authorization".into(), format!("Bearer {tok}"))],
                body: Body::Empty,
                timeout,
            };
            match send_checked(self.http.as_ref(), req) {
                Ok(resp) => return Ok(resp),
                Err(GarminError::Unauthorized) if attempt == 0 => {
                    tok = self.tokens.refresh_token(self.http.as_ref())?;
                }
                Err(e) => return Err(e),
            }
        }
        Err(GarminError::Unauthorized)
    }

    pub fn list_activities(&self, from: NaiveDate, to: NaiveDate) -> Result<Vec<GarminActivity>> {
        let mut all = Vec::new();
        let mut start = 0usize;
        loop {
            let q = format!(
                "/activitylist-service/activities/search/activities?start={start}&limit={PAGE_LIMIT}&startDate={}&endDate={}",
                from.format("%Y-%m-%d"),
                to.format("%Y-%m-%d")
            );
            let resp = self.get_authed(&q, TimeoutClass::Lookup)?;
            let page: Vec<RawActivity> = serde_json::from_slice(&resp.body)
                .map_err(|e| GarminError::Parse(e.to_string()))?;
            let count = page.len();
            all.extend(page.into_iter().map(GarminActivity::from));
            if count < PAGE_LIMIT {
                break;
            }
            start += count;
        }
        Ok(all)
    }

    pub fn download_fit(&self, activity_id: &str) -> Result<Vec<u8>> {
        let path = format!("/download-service/files/activity/{activity_id}");
        let resp = self.get_authed(&path, TimeoutClass::Long)?;
        if resp.body.starts_with(b"PK\x03\x04") {
            extract_fit_from_zip(&resp.body)
        } else {
            Ok(resp.body)
        }
    }

    pub fn daily_wellness(&self, date: NaiveDate) -> Result<DailyWellness> {
        let d = date.format("%Y-%m-%d");
        let path = format!("/usersummary-service/v2/dailysummaries?startDate={d}&endDate={d}");
        let resp = self.get_authed(&path, TimeoutClass::Lookup)?;
        let rows: Vec<RawDailySummary> =
            serde_json::from_slice(&resp.body).map_err(|e| GarminError::Parse(e.to_string()))?;
        Ok(match rows.into_iter().next() {
            Some(r) => DailyWellness {
                date,
                total_steps: r.total_steps,
                resting_hr: r.resting_heart_rate,
                avg_stress: r.average_stress_level,
                body_battery_high: r.body_battery_highest_value,
                body_battery_low: r.body_battery_lowest_value,
            },
            None => DailyWellness {
                date,
                total_steps: None,
                resting_hr: None,
                avg_stress: None,
                body_battery_high: None,
                body_battery_low: None,
            },
        })
    }
}

fn send_checked(http: &dyn Http, req: HttpRequest) -> Result<HttpResponse> {
    let resp = http.send(req).map_err(|e| match e {
        EnrichError::Network(m) => GarminError::Network(m),
        other => GarminError::Network(other.to_string()),
    })?;
    match resp.status {
        200..=299 => Ok(resp),
        401 => Err(GarminError::Unauthorized),
        429 => Err(GarminError::RateLimited),
        s => {
            let msg = String::from_utf8_lossy(&resp.body)
                .chars()
                .take(200)
                .collect();
            Err(GarminError::Status(s, msg))
        }
    }
}

fn extract_fit_from_zip(zip: &[u8]) -> Result<Vec<u8>> {
    let mut pos = 0usize;
    let mut first_file: Option<Vec<u8>> = None;
    while pos + 30 <= zip.len() && &zip[pos..pos + 4] == b"PK\x03\x04" {
        let method = u16::from_le_bytes([zip[pos + 8], zip[pos + 9]]);
        let comp_size =
            u32::from_le_bytes([zip[pos + 18], zip[pos + 19], zip[pos + 20], zip[pos + 21]])
                as usize;
        let name_len = u16::from_le_bytes([zip[pos + 26], zip[pos + 27]]) as usize;
        let extra_len = u16::from_le_bytes([zip[pos + 28], zip[pos + 29]]) as usize;
        let name_start = pos + 30;
        let data_start = name_start + name_len + extra_len;
        if data_start > zip.len() {
            break;
        }
        let name = String::from_utf8_lossy(&zip[name_start..name_start + name_len]).to_lowercase();
        let data_end = if comp_size > 0 && data_start + comp_size <= zip.len() {
            data_start + comp_size
        } else {
            zip.len()
        };
        if !name.ends_with('/') {
            let bytes = match method {
                0 => zip[data_start..data_end].to_vec(),
                8 => {
                    let mut dec = DeflateDecoder::new(&zip[data_start..data_end]);
                    let mut out = Vec::new();
                    dec.read_to_end(&mut out)
                        .map_err(|e| GarminError::Parse(format!("ZIP 압축 해제 실패: {e}")))?;
                    out
                }
                m => {
                    return Err(GarminError::Parse(format!(
                        "지원하지 않는 ZIP 압축 방식: {m}"
                    )));
                }
            };
            if name.ends_with(".fit") {
                return Ok(bytes);
            }
            if first_file.is_none() {
                first_file = Some(bytes);
            }
        }
        if comp_size == 0 {
            break;
        }
        pos = data_end;
    }
    first_file.ok_or_else(|| GarminError::Parse("ZIP 안에 FIT 파일이 없어요".into()))
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawActivity {
    activity_id: serde_json::Value,
    #[serde(default)]
    activity_name: Option<String>,
    #[serde(rename = "startTimeGMT", default)]
    start_time_gmt: Option<String>,
    #[serde(default)]
    start_time_local: Option<String>,
    #[serde(default)]
    activity_type: Option<RawActivityType>,
    #[serde(default)]
    distance: Option<f64>,
    #[serde(default)]
    duration: Option<f64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawActivityType {
    #[serde(default)]
    type_key: Option<String>,
}

impl From<RawActivity> for GarminActivity {
    fn from(r: RawActivity) -> Self {
        let id = match &r.activity_id {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        Self {
            id,
            name: r.activity_name.unwrap_or_default(),
            sport_type_key: r
                .activity_type
                .and_then(|t| t.type_key)
                .unwrap_or_else(|| "other".into()),
            start_time_gmt: r.start_time_gmt.unwrap_or_default(),
            start_time_local: r.start_time_local.unwrap_or_default(),
            distance_m: r.distance.unwrap_or(0.0),
            duration_s: r.duration.unwrap_or(0.0),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawDailySummary {
    total_steps: Option<u32>,
    resting_heart_rate: Option<u32>,
    average_stress_level: Option<i32>,
    body_battery_highest_value: Option<i32>,
    body_battery_lowest_value: Option<i32>,
}
