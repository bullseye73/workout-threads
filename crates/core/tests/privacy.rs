mod common;
use common::*;
use workout_core::fit::parse_fit;
use workout_core::model::LatLon;
use workout_core::privacy::*;
use workout_core::route::{haversine_m, simplify_route};

#[test]
fn points_avoid_both_ends() {
    workout_testkit::require_samples!();
    let a = parse_fit(&read(fit_20261025())).unwrap();
    let route = simplify_route(&a.samples, 1500);
    let pts = privacy_points(&route, TRIM_M, MAX_PLACE_POINTS);
    assert_eq!(pts.len(), 5);
    let (s, e) = (route[0], *route.last().unwrap());
    assert!(
        pts.iter()
            .all(|p| haversine_m(*p, s) > 500.0 && haversine_m(*p, e) > 500.0)
    );
}

#[test]
fn short_route_gives_nothing() {
    let route = vec![
        LatLon {
            lat: 37.5,
            lon: 127.0,
        },
        LatLon {
            lat: 37.505,
            lon: 127.0,
        },
    ];
    assert!(privacy_points(&route, TRIM_M, 5).is_empty());
    assert!(weather_point(&route, TRIM_M).is_none());
    assert!(privacy_points(&[], TRIM_M, 5).is_empty());
}

#[test]
fn weather_point_is_rounded() {
    workout_testkit::require_samples!();
    let a = parse_fit(&read(fit_20261025())).unwrap();
    let p = weather_point(&simplify_route(&a.samples, 1500), TRIM_M).unwrap();
    assert_eq!((p.lat * 100.0).round() / 100.0, p.lat);
    assert_eq!((p.lon * 100.0).round() / 100.0, p.lon);
}
