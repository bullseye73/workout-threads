mod common;
use chrono::NaiveDate;
use common::*;
use workout_core::fit::parse_fit;
use workout_core::model::Sport;
use workout_core::route::*;
use workout_core::splits::km_splits;
use workout_core::stats::month_stats;

#[test]
fn haversine_one_degree_lat() {
    use workout_core::model::LatLon;
    let d = haversine_m(
        LatLon {
            lat: 37.0,
            lon: 127.0,
        },
        LatLon {
            lat: 38.0,
            lon: 127.0,
        },
    );
    assert!((d - 111_195.0).abs() < 200.0);
}
#[test]
fn simplify_caps_points_and_keeps_ends() {
    workout_testkit::require_samples!();
    let a = parse_fit(&read(fit_20261011())).unwrap();
    let r = simplify_route(&a.samples, 1500);
    assert!(r.len() <= 1500 && r.len() > 1000, "len {}", r.len());
    assert_eq!(r[0], a.samples[0].pos.unwrap());
    assert_eq!(*r.last().unwrap(), a.samples.last().unwrap().pos.unwrap());
}
#[test]
fn simplify_without_gps_is_empty() {
    assert!(
        simplify_route(&samples_without_pos(utc("2026-01-01T00:00:00Z"), 100), 1500).is_empty()
    );
}
#[test]
fn splits_count() {
    workout_testkit::require_samples!();
    let a = parse_fit(&read(fit_20261025())).unwrap();
    let s = km_splits(&a.samples);
    assert_eq!(s.len(), 49);
    assert_eq!(s[0].km, 1);
    assert!(s[0].duration_s > 60.0 && s[0].duration_s < 600.0);
}
#[test]
fn month_stats_sums() {
    workout_testkit::require_samples!();
    let h1 = parse_fit(&read(fit_20261011())).unwrap().header;
    let h2 = parse_fit(&read(fit_20261025())).unwrap().header;
    let run_nov = header(Sport::Running, "2026-11-01T22:00:00Z", 5000.0, 1800.0);
    let s = month_stats(&[h1, h2, run_nov.clone()], 2026, 10);
    assert_eq!(s.cycling.count, 2);
    assert_eq!(s.running.count, 0);
    assert!((s.total_distance_m - 107_542.0).abs() < 20.0);
    assert_eq!(s.days.len(), 2);
    assert!(
        s.days
            .contains_key(&NaiveDate::from_ymd_opt(2026, 10, 25).unwrap())
    );
    assert!((s.avg_hr.unwrap() - 146.5).abs() < 1.0);
    assert_eq!(s.weekly.iter().filter(|w| w.cycling_m > 0.0).count(), 2);
    let nov = month_stats(&[run_nov], 2026, 11);
    assert_eq!(nov.running.count, 1);
    assert!(
        nov.days
            .contains_key(&NaiveDate::from_ymd_opt(2026, 11, 2).unwrap())
    );
    assert!(nov.avg_hr.is_none());
}
#[test]
fn weekly_buckets_start_on_monday_and_cover_month() {
    let s = month_stats(&[], 2026, 11);
    assert_eq!(
        s.weekly.first().unwrap().week_start,
        NaiveDate::from_ymd_opt(2026, 10, 26).unwrap()
    );
    assert_eq!(
        s.weekly.last().unwrap().week_start,
        NaiveDate::from_ymd_opt(2026, 11, 30).unwrap()
    );
    assert_eq!(s.weekly.len(), 6);
}
