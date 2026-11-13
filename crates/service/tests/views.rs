mod common;
use common::*;
use workout_service::*;

#[test]
fn month_view_october() {
    workout_testkit::require_samples!();
    let (_t, mut lib) = temp_library();
    lib.import_paths(&[sample_path("20261011"), sample_path("20261025")])
        .unwrap();
    let v = lib.month_view(2026, 10).unwrap();
    assert_eq!(v.cycling.count, 2);
    assert!((v.total_km - 107.54).abs() < 0.05);
    assert!(v.cycling.avg_speed_kmh.unwrap() > 23.0);
    assert!(v.running.avg_pace.is_none());
    assert_eq!(v.days.len(), 2);
    assert_eq!(v.days[0].date, "2026-10-11");
    assert_eq!(v.days[0].items[0].sport, SportKind::Cycling);
    assert_eq!(v.days[0].items[0].post_status, PostStatus::Ready);
    assert_eq!(v.weekly.len(), 5);
}

#[test]
fn empty_month_is_clean() {
    let (_t, lib) = temp_library();
    let v = lib.month_view(2026, 12).unwrap();
    assert_eq!(v.total_km, 0.0);
    assert!(v.goal_km.is_none());
    assert!(v.avg_hr.is_none());
    assert!(v.running.avg_pace.is_none() && v.cycling.avg_speed_kmh.is_none());
    assert!(v.days.is_empty());
    assert!(!serde_json::to_string(&v).unwrap().contains("NaN"));
}

#[test]
fn goal_is_reported() {
    let (_t, mut lib) = temp_library();
    lib.set_goal_km(Some(300.0)).unwrap();
    assert_eq!(lib.month_view(2026, 10).unwrap().goal_km, Some(300.0));
    lib.set_goal_km(None).unwrap();
    assert_eq!(lib.month_view(2026, 10).unwrap().goal_km, None);
}

#[test]
fn activity_detail_ride() {
    workout_testkit::require_samples!();
    let (_t, mut lib) = temp_library();
    let id = lib
        .import_paths(&[sample_path("20261025")])
        .unwrap()
        .added_activities[0]
        .clone();
    let d = lib.activity_detail(&id).unwrap();
    assert_eq!(d.title, "일요일 오후 라이딩");
    assert_eq!(d.date, "2026.10.25");
    assert_eq!(d.metrics.moving, "1:58:22");
    assert!(d.metrics.pace.is_none() && d.metrics.speed_kmh.is_some());
    assert!(d.route.len() <= 1500 && d.start.is_some() && d.end.is_some());
    assert!((d.route[0][0] - 37.0).abs() < 2.0, "route is [lat, lon]");
    assert_eq!(d.series.distance_km.len(), 1000);
    assert_eq!(d.series.hr.len(), 1000);
    assert_eq!(d.series.lat_lon.len(), 1000);
    assert_eq!(d.splits.len(), 49);
    assert_eq!(d.photos.len(), 2);
}

#[test]
fn unknown_activity_is_not_found() {
    let (_t, lib) = temp_library();
    assert!(matches!(
        lib.activity_detail("nope"),
        Err(ServiceError::NotFound(_))
    ));
}
