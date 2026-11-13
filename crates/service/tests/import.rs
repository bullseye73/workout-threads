mod common;
use common::*;

#[test]
fn imports_folder() {
    workout_testkit::require_samples!();
    let (_t, mut lib) = temp_library();
    let r = lib.import_paths(&[sample_path("20261025")]).unwrap();
    assert_eq!(r.added_activities.len(), 1);
    assert_eq!(r.cards_created, 2);
    assert!(r.failed.is_empty(), "{:?}", r.failed);
    assert!(r.unmatched_photos.is_empty());
    let day = lib.root().join("files/20261025");
    assert!(day.join("24842957203_ACTIVITY.fit").exists());
    assert!(
        day.join("78D7CE5C-F354-4728-8E97-063487771792_1_105_c_card.jpg")
            .exists()
    );
}

#[test]
fn second_import_is_duplicate() {
    workout_testkit::require_samples!();
    let (_t, mut lib) = temp_library();
    lib.import_paths(&[sample_path("20261025")]).unwrap();
    let r = lib.import_paths(&[sample_path("20261025")]).unwrap();
    assert!(r.added_activities.is_empty());
    assert_eq!(r.cards_created, 0);
    assert_eq!(r.duplicate_files.len(), 3);
}

#[test]
fn photos_later_match_existing_activity() {
    workout_testkit::require_samples!();
    let (_t, mut lib) = temp_library();
    lib.import_paths(&[sample_path("20261025/24842957203_ACTIVITY.fit")])
        .unwrap();
    let r = lib
        .import_paths(&[sample_path(
            "20261025/0723ACAA-C18C-412E-BDF8-0CB9253CB50F_1_102_a.jpeg",
        )])
        .unwrap();
    assert_eq!(r.cards_created, 1);
    assert!(r.unmatched_photos.is_empty());
}

#[test]
fn unmatched_photo_is_kept_for_manual_assignment() {
    workout_testkit::require_samples!();
    let (_t, mut lib) = temp_library();
    let r = lib
        .import_paths(&[
            sample_path("20261025/24842957203_ACTIVITY.fit"),
            sample_path("20261011/31CCE40E-34DE-405F-ACE0-A1B5DEFC07D4_4_5005_c.jpeg"),
        ])
        .unwrap();
    assert_eq!(r.unmatched_photos.len(), 1);
    assert!(std::path::Path::new(&r.unmatched_photos[0].path).exists());
}

#[test]
fn broken_fit_reported() {
    let (t, mut lib) = temp_library();
    let bad = t.path().join("bad.fit");
    std::fs::write(&bad, b"garbage").unwrap();
    let r = lib.import_paths(&[bad]).unwrap();
    assert_eq!(r.failed.len(), 1);
    assert!(r.added_activities.is_empty());
}

#[test]
fn source_folder_is_untouched() {
    workout_testkit::require_samples!();
    let before: Vec<_> = std::fs::read_dir(sample_path("20261025"))
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    let (_t, mut lib) = temp_library();
    lib.import_paths(&[sample_path("20261025")]).unwrap();
    let after: Vec<_> = std::fs::read_dir(sample_path("20261025"))
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(before.len(), after.len());
}
