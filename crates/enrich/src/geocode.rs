//! Nominatim 역지오코딩: 좌표 → 장소명·동네 이름.
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use workout_core::model::LatLon;

use crate::EnrichError;

pub fn reverse_url(p: LatLon) -> String {
    format!(
        "https://nominatim.openstreetmap.org/reverse?format=jsonv2&lat={:.5}&lon={:.5}&zoom=16&accept-language=ko",
        p.lat, p.lon
    )
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReverseResult {
    /// 공원·강 같은 이름 있는 장소.
    pub poi: Option<String>,
    /// 짧게 줄인 동네 이름 (반포, 잠실).
    pub area: Option<String>,
    /// 구 단위 (서초구).
    pub district: Option<String>,
}

const POI_CATEGORIES: [&str; 5] = ["leisure", "natural", "waterway", "tourism", "place"];

pub fn parse_reverse(json: &[u8]) -> Result<ReverseResult, EnrichError> {
    let v: Value = serde_json::from_slice(json).map_err(|e| EnrichError::Parse(e.to_string()))?;
    let empty = ReverseResult {
        poi: None,
        area: None,
        district: None,
    };
    if v.get("error").is_some() {
        return Ok(empty);
    }
    let s = |x: &Value| {
        x.as_str()
            .filter(|t| !t.trim().is_empty())
            .map(|t| t.trim().to_string())
    };
    let addr = &v["address"];
    let first = |keys: &[&str]| keys.iter().find_map(|k| s(&addr[*k]));
    let category = v["category"].as_str().unwrap_or("");
    Ok(ReverseResult {
        poi: POI_CATEGORIES
            .contains(&category)
            .then(|| s(&v["name"]))
            .flatten(),
        area: first(&["quarter", "suburb", "neighbourhood"]).map(|a| area_name(&a)),
        district: first(&["city_district", "borough", "county"]),
    })
}

/// 행정동 이름을 짧게: 반포2동 → 반포, 잠실본동 → 잠실, 성수1가1동 → 성수.
pub fn area_name(raw: &str) -> String {
    let Some(mut s) = raw.strip_suffix('동').map(str::to_string) else {
        return raw.to_string();
    };
    s = s.trim_end_matches(|c: char| c.is_ascii_digit()).to_string();
    if let Some(t) = s.strip_suffix('본') {
        s = t.to_string();
    }
    if let Some(t) = s.strip_suffix('가') {
        let trimmed = t.trim_end_matches(|c: char| c.is_ascii_digit());
        if trimmed.len() < t.len() {
            s = trimmed.to_string();
        }
    }
    if s.is_empty() { raw.to_string() } else { s }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlaceSummary {
    pub place: Option<String>,
    pub route_points: Vec<String>,
}

fn most_common(items: impl Iterator<Item = String>) -> Option<String> {
    let mut order: Vec<String> = Vec::new();
    let mut count: HashMap<String, usize> = HashMap::new();
    for it in items {
        if !count.contains_key(&it) {
            order.push(it.clone());
        }
        *count.entry(it).or_default() += 1;
    }
    let max = count.values().copied().max()?;
    order.into_iter().find(|o| count[o] == max)
}

/// 대표 장소(공원 등 최빈값, 없으면 구)와 지나간 동네(최대 3개, 장소와 같은 이름 제외).
pub fn summarize_places(results: &[ReverseResult]) -> PlaceSummary {
    let place = most_common(results.iter().filter_map(|r| r.poi.clone()))
        .or_else(|| most_common(results.iter().filter_map(|r| r.district.clone())));
    let mut route_points: Vec<String> = Vec::new();
    for a in results.iter().filter_map(|r| r.area.clone()) {
        if Some(&a) != place.as_ref() && !route_points.contains(&a) {
            route_points.push(a);
        }
    }
    route_points.truncate(3);
    PlaceSummary {
        place,
        route_points,
    }
}

/// 호출 사이 최소 간격을 지킨다 (Nominatim: 초당 1건).
pub struct RateLimiter {
    min: Duration,
    last: Mutex<Option<Instant>>,
}

impl RateLimiter {
    pub fn new(min_interval: Duration) -> Self {
        RateLimiter {
            min: min_interval,
            last: Mutex::new(None),
        }
    }

    pub fn wait(&self) {
        let mut last = self.last.lock().unwrap();
        if let Some(t) = *last {
            let elapsed = t.elapsed();
            if elapsed < self.min {
                std::thread::sleep(self.min - elapsed);
            }
        }
        *last = Some(Instant::now());
    }
}
