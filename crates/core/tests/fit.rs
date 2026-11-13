mod common;
use common::*;
use workout_core::CoreError;
use workout_core::fit::*;
use workout_core::format::hms;
use workout_core::model::*;

#[test]
fn parses_ride_20261011() {
    workout_testkit::require_samples!();
    let a = parse_fit(&read(fit_20261011())).unwrap();
    assert_eq!(a.header.sport, Sport::Cycling);
    assert_eq!(a.header.utc_offset_s, 9 * 3600);
    assert_eq!(a.header.start_local().to_string(), "2026-10-11 09:25:57");
    assert!((a.header.summary.distance_m - 59381.06).abs() < 1.0);
    assert_eq!(hms(a.header.summary.moving_s), "2:30:52");
    assert_eq!(a.header.summary.avg_hr, Some(147));
    assert_eq!(a.samples.len(), 9071);
    assert!(a.samples.iter().all(|s| s.pos.is_some()));
    assert!(
        a.header
            .id
            .ends_with(&format!("-{}", a.header.start.timestamp()))
    );
    assert!(!a.header.id.starts_with("0-"));
}

#[test]
fn parses_ride_20261025() {
    workout_testkit::require_samples!();
    let a = parse_fit(&read(fit_20261025())).unwrap();
    assert!((a.header.summary.distance_m / 1000.0 - 48.16).abs() < 0.005);
    assert_eq!(hms(a.header.summary.moving_s), "1:58:22");
    assert_eq!(a.header.summary.avg_hr, Some(146));
    assert_eq!(a.samples.len(), 7135);
    assert_eq!(a.header.start_local().to_string(), "2026-10-25 14:41:29");
}

#[test]
fn truncated_fit_is_error_not_panic() {
    workout_testkit::require_samples!();
    let b = read(fit_20261011());
    assert!(matches!(
        parse_fit(&b[..b.len() / 3]),
        Err(CoreError::Fit(_))
    ));
    assert!(matches!(parse_fit(b"not a fit"), Err(CoreError::Fit(_))));
}

#[test]
fn no_gps_activity_has_empty_positions() {
    let st = utc("2026-11-01T22:00:00Z");
    let a = activity_from_records(
        raw_session(1, st, 1800.0, 600.0),
        samples_without_pos(st, 600),
        None,
        Some(32400),
    );
    assert_eq!(a.header.sport, Sport::Running);
    assert!(a.samples.iter().all(|s| s.pos.is_none()));
    assert_eq!(a.header.id, format!("0-{}", a.header.start.timestamp()));
    assert!((a.header.summary.avg_speed_mps - 3.0).abs() < 1e-9);
}

#[test]
fn fit_cut_at_message_boundary_is_error() {
    workout_testkit::require_samples!();
    let b = read(fit_20261011());
    assert!(matches!(parse_fit(&b[..43456]), Err(CoreError::Fit(_))));
}
