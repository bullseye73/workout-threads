//! 경로 좌표 계산.
use crate::model::{LatLon, Sample};

const EARTH_R_M: f64 = 6_371_000.0;

/// 두 지점 사이 거리 (m).
pub fn haversine_m(a: LatLon, b: LatLon) -> f64 {
    let (p1, p2) = (a.lat.to_radians(), b.lat.to_radians());
    let dp = p2 - p1;
    let dl = (b.lon - a.lon).to_radians();
    let h = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    2.0 * EARTH_R_M * h.sqrt().asin()
}

/// GPS가 있는 샘플만 골라 최대 `max_points`개로 균등하게 줄인다. 첫·끝점은 보존.
pub fn simplify_route(samples: &[Sample], max_points: usize) -> Vec<LatLon> {
    let pts: Vec<LatLon> = samples.iter().filter_map(|s| s.pos).collect();
    if pts.len() <= max_points || max_points < 2 {
        return pts;
    }
    let step = (pts.len() - 1) as f64 / (max_points - 1) as f64;
    (0..max_points)
        .map(|i| pts[((i as f64 * step).round() as usize).min(pts.len() - 1)])
        .collect()
}
