//! 외부 서비스로 보낼 좌표 고르기. 집 근처일 수 있는 출발·도착 부근은 보내지 않는다.
use crate::model::LatLon;
use crate::route::haversine_m;

/// 출발·도착에서 이만큼(m) 안쪽은 보내지 않는다.
pub const TRIM_M: f64 = 500.0;
/// 장소·경로 조회에 쓰는 최대 점 수.
pub const MAX_PLACE_POINTS: usize = 5;

/// 보내도 되는 점들: 경로를 따라 앞·뒤 trim_m 밖이고, 출발·도착점과 직선거리로도 trim_m 밖인 점.
/// (순환 코스가 집 근처를 다시 지나가는 경우까지 막는다.)
fn safe_points(route: &[LatLon], trim_m: f64) -> Vec<(f64, LatLon)> {
    let (Some(&start), Some(&end)) = (route.first(), route.last()) else {
        return vec![];
    };
    let mut cum = vec![0.0];
    for w in route.windows(2) {
        cum.push(cum.last().unwrap() + haversine_m(w[0], w[1]));
    }
    let total = *cum.last().unwrap();
    route
        .iter()
        .zip(&cum)
        .filter(|(p, c)| {
            **c > trim_m
                && total - **c > trim_m
                && haversine_m(**p, start) > trim_m
                && haversine_m(**p, end) > trim_m
        })
        .map(|(p, c)| (*c, *p))
        .collect()
}

/// 출발·도착 부근(trim_m)을 뺀 경로 (카드 경로선 가리기용). 순서는 유지한다.
pub fn trim_route(route: &[LatLon], trim_m: f64) -> Vec<LatLon> {
    safe_points(route, trim_m)
        .into_iter()
        .map(|(_, p)| p)
        .collect()
}

/// 보호 구간을 거리 기준으로 max개로 균등 분할한 점. 남는 구간이 없으면 빈 Vec.
pub fn privacy_points(route: &[LatLon], trim_m: f64, max: usize) -> Vec<LatLon> {
    let safe = safe_points(route, trim_m);
    if safe.is_empty() || max == 0 {
        return vec![];
    }
    let (lo, hi) = (safe[0].0, safe[safe.len() - 1].0);
    let mut out: Vec<LatLon> = Vec::new();
    for i in 0..max {
        let target = lo + (hi - lo) * (i as f64 + 0.5) / max as f64;
        let p = safe
            .iter()
            .min_by(|a, b| (a.0 - target).abs().total_cmp(&(b.0 - target).abs()))
            .unwrap()
            .1;
        if out.last() != Some(&p) {
            out.push(p);
        }
    }
    out
}

/// 보호 구간 가운데 지점을 소수 2자리(약 1km)로 반올림. 남는 구간이 없으면 None.
pub fn weather_point(route: &[LatLon], trim_m: f64) -> Option<LatLon> {
    let p = *privacy_points(route, trim_m, 1).first()?;
    let r = |v: f64| (v * 100.0).round() / 100.0;
    Some(LatLon {
        lat: r(p.lat),
        lon: r(p.lon),
    })
}
