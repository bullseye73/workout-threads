use workout_core::format::{activity_title, hms, pace_per_km, speed_kmh};
use workout_core::model::{Activity, Sport};
use workout_core::route::simplify_route;

/// 화면용 경로 최대 점 수.
pub const ROUTE_MAX_POINTS: usize = 1500;

/// 카드에 쓰는 글자와 경로. `route`는 [x = 경도·cos(시작 위도), y = 위도].
#[derive(Clone, Debug, PartialEq)]
pub struct CardData {
    pub date: String,
    pub title: String,
    pub distance: String,
    pub sub: String,
    pub route: Vec<[f64; 2]>,
}

/// 전체 경로를 그리는 카드 데이터.
pub fn card_data(a: &Activity) -> CardData {
    card_data_with(a, 0.0)
}

/// trim_m > 0이면 출발·도착 부근을 뺀 경로를 그린다.
pub fn card_data_with(a: &Activity, trim_m: f64) -> CardData {
    let h = &a.header;
    let local = h.start_local();
    let sm = &h.summary;
    let mut parts = vec![hms(sm.moving_s)];
    if h.sport == Sport::Cycling {
        parts.push(format!("{:.1} km/h", speed_kmh(sm.moving_s, sm.distance_m)));
    } else if sm.distance_m > 0.0 {
        parts.push(format!("{}/km", pace_per_km(sm.moving_s, sm.distance_m)));
    }
    if let Some(hr) = sm.avg_hr {
        parts.push(format!("{hr} bpm"));
    }
    let mut pts = simplify_route(&a.samples, ROUTE_MAX_POINTS);
    if trim_m > 0.0 {
        pts = workout_core::privacy::trim_route(&pts, trim_m);
    }
    let cos0 = pts.first().map(|p| p.lat.to_radians().cos()).unwrap_or(1.0);
    CardData {
        date: local.format("%Y.%m.%d").to_string(),
        title: activity_title(local, h.sport),
        distance: format!("{:.2}", sm.distance_m / 1000.0),
        sub: parts.join("  ·  "),
        route: pts.iter().map(|p| [p.lon * cos0, p.lat]).collect(),
    }
}
