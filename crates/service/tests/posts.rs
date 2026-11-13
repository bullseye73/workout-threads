mod common;
use common::*;
use workout_service::*;

#[test]
fn post_view_has_cards_and_template() {
    workout_testkit::require_samples!();
    let (_t, mut lib) = temp_library();
    let id = lib
        .import_paths(&[sample_path("20261025")])
        .unwrap()
        .added_activities[0]
        .clone();
    let p = lib.post_view(&id).unwrap();
    assert!(p.body.starts_with("일요일 오후 48km 🚲"));
    assert_eq!(p.card_paths.len(), 2);
    assert!(p.folder.ends_with("20261025"));
    assert_eq!(p.status, PostStatus::Ready);
    assert!(!p.edited);
}

#[test]
fn edited_post_survives_reopen() {
    workout_testkit::require_samples!();
    let (_t, mut lib) = temp_library();
    let id = lib
        .import_paths(&[sample_path("20261025")])
        .unwrap()
        .added_activities[0]
        .clone();
    lib.save_post(&id, "직접 쓴 글", &["#라이딩".into()])
        .unwrap();
    lib.import_paths(&[sample_path("20261025")]).unwrap();
    let p = lib.post_view(&id).unwrap();
    assert_eq!(p.body, "직접 쓴 글");
    assert!(p.edited);
    assert_eq!(p.full_text, "직접 쓴 글\n\n#라이딩");
    let r = lib.regenerate_post(&id).unwrap();
    assert!(r.body.starts_with("일요일 오후 48km 🚲"));
    assert!(!r.edited);
}

#[test]
fn mark_published_exports_and_records() {
    workout_testkit::require_samples!();
    let (_t, mut lib) = temp_library();
    let id = lib
        .import_paths(&[sample_path("20261025")])
        .unwrap()
        .added_activities[0]
        .clone();
    let p = lib.mark_published(&id).unwrap();
    assert_eq!(p.status, PostStatus::Published);
    assert!(std::path::Path::new(&p.folder).join("post.txt").exists());
    assert_eq!(
        lib.month_view(2026, 10).unwrap().days[0].items[0].post_status,
        PostStatus::Published
    );
}

#[test]
fn assign_unmatched_photo_creates_card() {
    workout_testkit::require_samples!();
    let (_t, mut lib) = temp_library();
    let r = lib
        .import_paths(&[
            sample_path("20261025/24842957203_ACTIVITY.fit"),
            sample_path("20261011/31CCE40E-34DE-405F-ACE0-A1B5DEFC07D4_4_5005_c.jpeg"),
        ])
        .unwrap();
    let id = &r.added_activities[0];
    assert_eq!(lib.post_view(id).unwrap().status, PostStatus::None);
    let ph = lib.assign_photo(r.unmatched_photos[0].id, id).unwrap();
    assert!(std::path::Path::new(ph.card_path.as_ref().unwrap()).exists());
    assert!(ph.path.contains("20261025"));
    let p = lib.post_view(id).unwrap();
    assert_eq!(p.card_paths.len(), 1);
    assert_eq!(p.status, PostStatus::Ready);
}
