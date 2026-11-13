mod common;
use chrono::NaiveDateTime;
use common::*;
use std::path::Path;
use workout_core::fit::parse_fit;
use workout_core::matching::*;
use workout_core::photo::*;

fn naive(s: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").unwrap()
}

#[test]
fn offset_tag_wins() {
    let t = resolve_taken_at(naive("2026-10-25 15:39:56"), Some("+09:00"), 0);
    assert_eq!(t.to_rfc3339(), "2026-10-25T06:39:56+00:00");
}
#[test]
fn fallback_offset_when_no_tag() {
    let t = resolve_taken_at(naive("2026-10-25 15:39:56"), None, 9 * 3600);
    assert_eq!(t.to_rfc3339(), "2026-10-25T06:39:56+00:00");
}
#[test]
fn bad_offset_tag_uses_fallback() {
    let t = resolve_taken_at(naive("2026-10-25 15:39:56"), Some("garbage"), 9 * 3600);
    assert_eq!(t.to_rfc3339(), "2026-10-25T06:39:56+00:00");
}
#[test]
fn reads_real_photo() {
    workout_testkit::require_samples!();
    let m = read_photo_meta(
        &sample_path("20261025/0723ACAA-C18C-412E-BDF8-0CB9253CB50F_1_102_a.jpeg"),
        9 * 3600,
    )
    .unwrap();
    assert_eq!(
        m.taken_at.unwrap().to_rfc3339(),
        "2026-10-25T06:39:56+00:00"
    );
    // 좌표 값은 공개 저장소에 적지 않는다: 한반도 위도 범위인지만 본다.
    assert!((33.0..39.0).contains(&m.pos.unwrap().lat));
}
#[test]
fn photo_without_exif_has_no_time() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("plain.jpg");
    std::fs::write(&p, b"\xFF\xD8\xFF\xD9").unwrap();
    let m = read_photo_meta(&p, 0).unwrap();
    assert!(m.taken_at.is_none());
    assert_eq!(m.orientation, 1);
}
#[test]
fn filters_inputs() {
    assert!(is_photo_input(Path::new("a.JPEG")));
    assert!(is_photo_input(Path::new("b.png")));
    assert!(!is_photo_input(Path::new("a_card.jpg")));
    assert!(!is_photo_input(Path::new("x.fit")));
}
#[test]
fn matches_photo_to_right_activity() {
    workout_testkit::require_samples!();
    let ride = parse_fit(&read(fit_20261025())).unwrap();
    let run = synthetic_run_on("2026-10-25T00:00:00Z", 1800);
    let m = match_photo(&[run, ride.clone()], utc("2026-10-25T06:39:56Z")).unwrap();
    assert_eq!(m.activity_id, ride.header.id);
    assert!(m.km_mark > 0.0 && m.km_mark < 48.2);
}
#[test]
fn matches_within_ten_minutes_before_start() {
    workout_testkit::require_samples!();
    let ride = parse_fit(&read(fit_20261025())).unwrap();
    let before = ride.header.start - chrono::Duration::minutes(9);
    assert_eq!(match_photo(&[ride], before).unwrap().km_mark, 0.0);
}
#[test]
fn unmatched_photo_is_none() {
    workout_testkit::require_samples!();
    let ride = parse_fit(&read(fit_20261025())).unwrap();
    assert!(match_photo(&[ride], utc("2026-10-25T12:00:00Z")).is_none());
}

#[test]
fn photo_inside_activity_beats_neighbor_window() {
    workout_testkit::require_samples!();
    let ride = parse_fit(&read(fit_20261025())).unwrap();
    // 러닝이 라이딩 시작 1분 29초 전에 끝남 → 라이딩 중 사진이 러닝 창(+10분)에도 걸림
    let run = synthetic_run_on("2026-10-25T05:00:00Z", 2400);
    let photo = ride.header.start + chrono::Duration::minutes(3);
    let m = match_photo(&[run, ride.clone()], photo).unwrap();
    assert_eq!(m.activity_id, ride.header.id);
}

#[test]
fn assigns_batch() {
    workout_testkit::require_samples!();
    let ride = parse_fit(&read(fit_20261025())).unwrap();
    let photos = vec![
        sample_path("20261025/78D7CE5C-F354-4728-8E97-063487771792_1_105_c.jpeg"),
        sample_path("20261011/31CCE40E-34DE-405F-ACE0-A1B5DEFC07D4_4_5005_c.jpeg"),
    ];
    let r = assign_photos(std::slice::from_ref(&ride), &photos);
    assert_eq!(r[0].matched.as_ref().unwrap().activity_id, ride.header.id);
    assert!(r[1].taken_at.is_some() && r[1].matched.is_none());
}
