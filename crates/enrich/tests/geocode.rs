mod common;
use common::*;
use std::time::{Duration, Instant};
use workout_core::chrono::{NaiveDate, NaiveDateTime};
use workout_core::model::LatLon;
use workout_enrich::geocode::*;
use workout_enrich::weather::*;

#[test]
fn area_names() {
    for (a, b) in [
        ("반포2동", "반포"),
        ("잠실본동", "잠실"),
        ("여의도동", "여의도"),
        ("성수1가1동", "성수"),
        ("망원1동", "망원"),
        ("서교동", "서교"),
        ("Yeouido", "Yeouido"),
    ] {
        assert_eq!(area_name(a), b, "{a}");
    }
}

#[test]
fn parses_park() {
    let r = parse_reverse(&fixture("nominatim_park.json")).unwrap();
    assert_eq!(r.poi.as_deref(), Some("반포한강공원"));
    assert_eq!(r.area.as_deref(), Some("반포"));
    assert_eq!(r.district.as_deref(), Some("서초구"));
}

#[test]
fn parses_error_as_empty() {
    let r = parse_reverse(&fixture("nominatim_sea.json")).unwrap();
    assert!(r.poi.is_none() && r.area.is_none() && r.district.is_none());
    assert!(parse_reverse(b"not json").is_err());
}

#[test]
fn summarizes() {
    let park = parse_reverse(&fixture("nominatim_park.json")).unwrap();
    let dong = parse_reverse(&fixture("nominatim_dong.json")).unwrap();
    let empty = ReverseResult {
        poi: None,
        area: None,
        district: None,
    };
    let s = summarize_places(&[park.clone(), dong.clone(), park.clone(), empty]);
    assert_eq!(s.place.as_deref(), Some("반포한강공원"));
    assert_eq!(s.route_points, vec!["반포", "잠실"]);
    assert!(summarize_places(&[]).place.is_none());
    let only_dong = summarize_places(&[dong]);
    assert_eq!(only_dong.place.as_deref(), Some("송파구"));
}

#[test]
fn url_has_rounded_coords_and_korean() {
    let u = reverse_url(LatLon {
        lat: 37.123456,
        lon: 127.654321,
    });
    assert!(u.starts_with("https://nominatim.openstreetmap.org/reverse?"));
    assert!(u.contains("lat=37.12346") && u.contains("lon=127.65432"));
    assert!(
        u.contains("accept-language=ko") && u.contains("zoom=16") && u.contains("format=jsonv2")
    );
}

#[test]
fn weather_endpoint_by_age() {
    let d = |s: &str| NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap();
    let p = LatLon {
        lat: 37.52,
        lon: 126.93,
    };
    let recent = weather_url(p, d("2026-10-25"), d("2026-11-14"));
    assert!(recent.starts_with("https://api.open-meteo.com/v1/forecast"));
    assert!(
        recent.contains("latitude=37.52")
            && recent.contains("start_date=2026-10-25")
            && recent.contains("timezone=auto")
    );
    assert!(
        weather_url(p, d("2026-01-05"), d("2026-11-14"))
            .starts_with("https://archive-api.open-meteo.com/v1/archive")
    );
}

#[test]
fn parses_weather_hour() {
    let t = NaiveDateTime::parse_from_str("2026-10-25 15:20:00", "%Y-%m-%d %H:%M:%S").unwrap();
    assert_eq!(
        parse_weather(&fixture("openmeteo_hourly.json"), t).unwrap(),
        ("흐림".to_string(), 16.9)
    );
    let missing =
        NaiveDateTime::parse_from_str("2026-10-26 09:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
    assert!(parse_weather(&fixture("openmeteo_hourly.json"), missing).is_err());
}

#[test]
fn wmo_codes() {
    assert_eq!(wmo_ko(0), "맑음");
    assert_eq!(wmo_ko(2), "구름 조금");
    assert_eq!(wmo_ko(63), "비");
    assert_eq!(wmo_ko(95), "뇌우");
    assert_eq!(wmo_ko(200), "날씨 정보 없음");
}

#[test]
fn rate_limiter_spaces_calls() {
    let r = RateLimiter::new(Duration::from_millis(200));
    let t0 = Instant::now();
    r.wait();
    r.wait();
    r.wait();
    assert!(t0.elapsed() >= Duration::from_millis(400));
}
