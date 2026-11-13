use chrono::NaiveDate;
use workout_core::format::*;
use workout_core::model::Sport;

#[test]
fn hms_formats() {
    assert_eq!(hms(9052.377), "2:30:52");
    assert_eq!(hms(593.0), "9:53");
}
#[test]
fn pace_formats() {
    assert_eq!(pace_per_km(1800.0, 6000.0), "5'00\"");
}
#[test]
fn speed() {
    assert!((speed_kmh(9052.377, 59381.06) - 23.6).abs() < 0.05);
}
#[test]
fn time_of_day_bounds() {
    assert_eq!(time_of_day_ko(4), "새벽");
    assert_eq!(time_of_day_ko(9), "오전");
    assert_eq!(time_of_day_ko(14), "오후");
    assert_eq!(time_of_day_ko(20), "밤");
}
#[test]
fn weekday() {
    assert_eq!(
        weekday_ko(NaiveDate::from_ymd_opt(2026, 10, 11).unwrap()),
        "일요일"
    );
}
#[test]
fn sport_codes() {
    assert_eq!(Sport::from_fit(1), Sport::Running);
    assert_eq!(Sport::from_fit(2), Sport::Cycling);
    assert_eq!(Sport::from_fit(5), Sport::Other(5));
    assert_eq!(sport_ko(Sport::Cycling), "라이딩");
}

#[test]
fn sport_roundtrip() {
    for c in [1u8, 2, 11, 17, 5] {
        assert_eq!(Sport::from_fit(c).to_fit(), c);
    }
}
