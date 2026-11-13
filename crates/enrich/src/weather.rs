//! Open-Meteo 과거 날씨: 운동한 시각의 기온과 날씨.
use serde_json::Value;
use workout_core::chrono::{NaiveDate, NaiveDateTime, Timelike};
use workout_core::model::LatLon;

use crate::EnrichError;

/// 이 기간 안이면 forecast API(최근 과거 포함), 넘으면 archive API.
const FORECAST_DAYS: i64 = 60;

pub fn weather_url(p: LatLon, local_date: NaiveDate, today: NaiveDate) -> String {
    let base = if (today - local_date).num_days() <= FORECAST_DAYS {
        "https://api.open-meteo.com/v1/forecast"
    } else {
        "https://archive-api.open-meteo.com/v1/archive"
    };
    format!(
        "{base}?latitude={:.2}&longitude={:.2}&hourly=temperature_2m,weather_code&start_date={local_date}&end_date={local_date}&timezone=auto",
        p.lat, p.lon
    )
}

/// `local_hour`와 같은 시(時)의 (날씨 설명, 기온).
pub fn parse_weather(json: &[u8], local_hour: NaiveDateTime) -> Result<(String, f64), EnrichError> {
    let v: Value = serde_json::from_slice(json).map_err(|e| EnrichError::Parse(e.to_string()))?;
    let key = local_hour
        .with_minute(0)
        .and_then(|t| t.with_second(0))
        .unwrap_or(local_hour)
        .format("%Y-%m-%dT%H:%M")
        .to_string();
    let times = v["hourly"]["time"]
        .as_array()
        .ok_or_else(|| EnrichError::Parse("시간 목록 없음".into()))?;
    let i = times
        .iter()
        .position(|t| t.as_str() == Some(&key))
        .ok_or_else(|| EnrichError::Parse(format!("{key} 날씨 없음")))?;
    let temp = v["hourly"]["temperature_2m"][i]
        .as_f64()
        .ok_or_else(|| EnrichError::Parse("기온 없음".into()))?;
    let code = v["hourly"]["weather_code"][i]
        .as_u64()
        .ok_or_else(|| EnrichError::Parse("날씨 코드 없음".into()))?;
    Ok((wmo_ko(code.min(255) as u8).to_string(), temp))
}

/// WMO 날씨 코드 → 한국어.
pub fn wmo_ko(code: u8) -> &'static str {
    match code {
        0 => "맑음",
        1 | 2 => "구름 조금",
        3 => "흐림",
        45 | 48 => "안개",
        51..=57 => "이슬비",
        61..=67 => "비",
        71..=77 => "눈",
        80..=82 => "소나기",
        85 | 86 => "소낙눈",
        95..=99 => "뇌우",
        _ => "날씨 정보 없음",
    }
}
