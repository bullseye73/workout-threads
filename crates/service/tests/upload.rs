mod common;
use common::*;
use std::path::PathBuf;

const P_0723: &str = "20261025/0723ACAA-C18C-412E-BDF8-0CB9253CB50F_1_102_a.jpeg";
const FIT_1025: &str = "20261025/24842957203_ACTIVITY.fit";

fn late_photo(dir: &std::path::Path, name: &str, local: &str) -> PathBuf {
    let p = dir.join(name);
    real_jpeg_with_local_time(P_0723, &p, local);
    p
}

#[test]
fn one_fit_with_photos_plans_one_post() {
    workout_testkit::require_samples!();
    let (_t, lib) = temp_library();
    let plan = lib.plan_upload(&[sample_path("20261025")]).unwrap();
    assert_eq!(plan.new_activities.len(), 1);
    let a = &plan.new_activities[0];
    assert_eq!(a.photos.len(), 2);
    assert_eq!(a.date, "2026-10-25");
    assert_eq!(a.title, "일요일 오후 라이딩");
    assert!(
        lib.month_view(2026, 10).unwrap().days.is_empty(),
        "미리보기는 저장하지 않는다"
    );
}

#[test]
fn two_fits_plan_two_posts() {
    workout_testkit::require_samples!();
    let (_t, lib) = temp_library();
    let plan = lib
        .plan_upload(&[sample_path("20261025"), sample_path("20261011")])
        .unwrap();
    let dates: Vec<_> = plan
        .new_activities
        .iter()
        .map(|a| a.date.as_str())
        .collect();
    assert_eq!(dates, vec!["2026-10-11", "2026-10-25"]);
}

#[test]
fn nested_folders_are_searched() {
    workout_testkit::require_samples!();
    let (t, lib) = temp_library();
    let deep = t.path().join("root/a/b");
    copy_as(FIT_1025, &deep, "ride.fit");
    copy_as(P_0723, &deep, "p1.jpeg");
    copy_as(
        "20261025/78D7CE5C-F354-4728-8E97-063487771792_1_105_c.jpeg",
        &deep,
        "p2.jpeg",
    );
    let plan = lib.plan_upload(&[t.path().join("root")]).unwrap();
    assert_eq!(plan.new_activities.len(), 1);
    assert_eq!(plan.new_activities[0].photos.len(), 2);
}

#[test]
fn same_fit_twice_makes_one_activity() {
    workout_testkit::require_samples!();
    let (t, lib) = temp_library();
    let a = copy_as(FIT_1025, &t.path().join("x"), "a.fit");
    let b = copy_as(FIT_1025, &t.path().join("y"), "b.fit");
    let plan = lib.plan_upload(&[a, b]).unwrap();
    assert_eq!(plan.new_activities.len(), 1);
}

#[test]
fn already_imported_fit_is_existing() {
    workout_testkit::require_samples!();
    let (_t, mut lib) = temp_library();
    lib.import_paths(&[sample_path("20261025")]).unwrap();
    let plan = lib.plan_upload(&[sample_path("20261025")]).unwrap();
    assert!(plan.new_activities.is_empty());
    assert!(plan.existing[0].uploaded_fit);
    assert_eq!(plan.duplicates.len(), 2);
}

#[test]
fn photo_without_date_is_excluded() {
    workout_testkit::require_samples!();
    let (t, lib) = temp_library();
    let plain = t.path().join("in/plain.jpg");
    std::fs::create_dir_all(plain.parent().unwrap()).unwrap();
    std::fs::write(&plain, b"\xFF\xD8\xFF\xD9").unwrap();
    let plan = lib.plan_upload(&[sample_path(FIT_1025), plain]).unwrap();
    let ex: Vec<_> = plan
        .excluded
        .iter()
        .map(|i| (i.file.clone(), i.reason.clone()))
        .collect();
    assert_eq!(
        ex,
        vec![("plain.jpg".to_string(), "촬영 날짜가 없어요".to_string())]
    );
}

#[test]
fn photo_on_a_day_without_workout_is_excluded() {
    workout_testkit::require_samples!();
    let (t, lib) = temp_library();
    let p = late_photo(&t.path().join("in"), "lonely.jpeg", "2026:10:20 12:00:00");
    let plan = lib.plan_upload(&[p]).unwrap();
    assert_eq!(plan.excluded.len(), 1);
    assert_eq!(plan.excluded[0].reason, "붙일 운동이 없어 제외");
}

#[test]
fn late_photo_joins_the_only_workout_that_day() {
    workout_testkit::require_samples!();
    let (t, lib) = temp_library();
    let p = late_photo(&t.path().join("in"), "late.jpeg", "2026:10:25 20:00:00");
    let plan = lib.plan_upload(&[sample_path(FIT_1025), p]).unwrap();
    let photos = &plan.new_activities[0].photos;
    assert_eq!(photos.len(), 1);
    assert_eq!(photos[0].km_mark, None);
    assert!(plan.excluded.is_empty());
}

#[test]
fn photo_for_stored_workout_goes_to_existing() {
    workout_testkit::require_samples!();
    let (t, mut lib) = temp_library();
    lib.import_paths(&[sample_path(FIT_1025)]).unwrap();
    let p = late_photo(&t.path().join("in"), "late.jpeg", "2026:10:25 20:00:00");
    let plan = lib.plan_upload(&[p]).unwrap();
    assert!(plan.new_activities.is_empty());
    assert_eq!(plan.existing.len(), 1);
    assert!(!plan.existing[0].uploaded_fit);
    assert_eq!(plan.existing[0].photos.len(), 1);
}

#[test]
fn library_unmatched_photo_counts_for_new_workout() {
    workout_testkit::require_samples!();
    let (t, mut lib) = temp_library();
    let p = late_photo(&t.path().join("in"), "late.jpeg", "2026:10:25 20:00:00");
    let r = lib.import_paths(&[p]).unwrap();
    assert_eq!(r.unmatched_photos.len(), 1);
    let plan = lib.plan_upload(&[sample_path(FIT_1025)]).unwrap();
    assert_eq!(plan.new_activities[0].library_photos, 1);
}

#[test]
fn locked_post_is_flagged() {
    workout_testkit::require_samples!();
    let (t, mut lib) = temp_library();
    let id = lib
        .import_paths(&[sample_path(FIT_1025)])
        .unwrap()
        .added_activities[0]
        .clone();
    lib.save_post(&id, "직접 쓴 글", &[]).unwrap();
    let p = late_photo(&t.path().join("in"), "late.jpeg", "2026:10:25 20:00:00");
    let plan = lib.plan_upload(&[p]).unwrap();
    assert!(plan.existing[0].post_locked);
}
