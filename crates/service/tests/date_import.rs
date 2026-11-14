mod common;
use common::*;
use workout_core::chrono::NaiveDate;
use workout_service::*;

const P_0723: &str = "20261025/0723ACAA-C18C-412E-BDF8-0CB9253CB50F_1_102_a.jpeg";
const FIT_1025: &str = "20261025/24842957203_ACTIVITY.fit";

fn day(m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, m, d).unwrap()
}
fn reasons(r: &ImportReport) -> Vec<(String, String)> {
    r.skipped
        .iter()
        .map(|i| (i.file.clone(), i.reason.clone()))
        .collect()
}

#[test]
fn same_day_files_are_imported() {
    workout_testkit::require_samples!();
    let (_t, mut lib) = temp_library();
    let r = lib
        .import_for_date(day(10, 25), &[sample_path("20261025")])
        .unwrap();
    assert_eq!(r.added_activities.len(), 1);
    assert_eq!(r.cards_created, 2);
    assert!(r.skipped.is_empty());
}

#[test]
fn fit_from_other_day_is_skipped() {
    workout_testkit::require_samples!();
    let (_t, mut lib) = temp_library();
    let r = lib
        .import_for_date(day(10, 25), &[sample_path("20261011")])
        .unwrap();
    assert!(r.added_activities.is_empty());
    let rs = reasons(&r);
    assert!(rs.contains(&(
        "24680396566_ACTIVITY.fit".into(),
        "10월 11일 기록이에요".into()
    )));
    assert_eq!(
        rs.iter()
            .filter(|(_, why)| why == "10월 11일에 찍었어요")
            .count(),
        4
    );
    assert_eq!(rs.len(), 5);
}

#[test]
fn photo_without_date_is_skipped() {
    workout_testkit::require_samples!();
    let (t, mut lib) = temp_library();
    let plain = t.path().join("in/plain.jpg");
    std::fs::create_dir_all(plain.parent().unwrap()).unwrap();
    std::fs::write(&plain, b"\xFF\xD8\xFF\xD9").unwrap();
    let r = lib
        .import_for_date(day(10, 25), &[sample_path(FIT_1025), plain])
        .unwrap();
    assert_eq!(r.added_activities.len(), 1);
    assert_eq!(
        reasons(&r),
        vec![("plain.jpg".into(), "촬영 날짜가 없어요".into())]
    );
}

#[test]
fn same_day_photo_outside_ride_joins_the_only_ride() {
    workout_testkit::require_samples!();
    let (t, mut lib) = temp_library();
    let id = lib
        .import_for_date(day(10, 25), &[sample_path(FIT_1025)])
        .unwrap()
        .added_activities[0]
        .clone();
    let late = t.path().join("in/late.jpeg");
    real_jpeg_with_local_time(P_0723, &late, "2026:10:25 20:00:00");
    let r = lib.import_for_date(day(10, 25), &[late]).unwrap();
    assert!(r.skipped.is_empty());
    assert!(r.unmatched_photos.is_empty(), "{:?}", r.unmatched_photos);
    assert_eq!(r.cards_created, 1);
    assert_eq!(lib.post_view(&id).unwrap().card_paths.len(), 1);
}

#[test]
fn same_day_photo_without_ride_stays_unmatched() {
    workout_testkit::require_samples!();
    let (t, mut lib) = temp_library();
    let late = t.path().join("in/late.jpeg");
    real_jpeg_with_local_time(P_0723, &late, "2026:10:25 20:00:00");
    let r = lib.import_for_date(day(10, 25), &[late]).unwrap();
    assert!(r.skipped.is_empty());
    assert_eq!(r.unmatched_photos.len(), 1);
}

#[test]
fn already_imported_photo_counts_as_duplicate() {
    workout_testkit::require_samples!();
    let (_t, mut lib) = temp_library();
    lib.import_for_date(day(10, 25), &[sample_path("20261025")])
        .unwrap();
    let r = lib
        .import_for_date(day(10, 25), &[sample_path("20261025")])
        .unwrap();
    assert_eq!(r.duplicate_files.len(), 3);
    assert!(r.skipped.is_empty());
}
