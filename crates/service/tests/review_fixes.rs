mod common;
use common::*;

const FIT_1025: &str = "20261025/24842957203_ACTIVITY.fit";
const P_0723: &str = "20261025/0723ACAA-C18C-412E-BDF8-0CB9253CB50F_1_102_a.jpeg";
const P_78D7: &str = "20261025/78D7CE5C-F354-4728-8E97-063487771792_1_105_c.jpeg";
const P_31CC: &str = "20261011/31CCE40E-34DE-405F-ACE0-A1B5DEFC07D4_4_5005_c.jpeg";

/// I1: 같은 날 같은 이름의 다른 사진 두 장은 둘 다 들어가야 한다.
#[test]
fn same_name_different_photos_both_kept() {
    workout_testkit::require_samples!();
    let (t, mut lib) = temp_library();
    lib.import_paths(&[sample_path(FIT_1025)]).unwrap();
    let a = copy_as(P_0723, &t.path().join("phone"), "IMG_0001.jpeg");
    let b = copy_as(P_78D7, &t.path().join("camera"), "IMG_0001.jpeg");
    let r1 = lib.import_paths(&[a]).unwrap();
    let r2 = lib.import_paths(&[b]).unwrap();
    assert_eq!(r1.cards_created + r2.cards_created, 2, "{r2:?}");
    assert!(r2.duplicate_files.is_empty());
    let id = &lib.month_view(2026, 10).unwrap().days[0].items[0]
        .activity_id
        .clone();
    let cards = lib.post_view(id).unwrap().card_paths;
    assert_eq!(cards.len(), 2);
    assert_ne!(cards[0], cards[1]);
}

/// I1: 이름만 다른 같은 사진은 중복으로 본다 (내용 기준).
#[test]
fn same_photo_different_name_is_duplicate() {
    workout_testkit::require_samples!();
    let (t, mut lib) = temp_library();
    lib.import_paths(&[sample_path(FIT_1025), sample_path(P_0723)])
        .unwrap();
    let again = copy_as(P_0723, &t.path().join("x"), "renamed.jpeg");
    let r = lib.import_paths(&[again]).unwrap();
    assert_eq!(r.duplicate_files, vec!["renamed.jpeg"]);
    assert_eq!(r.cards_created, 0);
}

/// I1: 대소문자·확장자만 다른 이름도 서로 덮어쓰지 않는다.
#[test]
fn case_and_extension_variants_do_not_overwrite() {
    workout_testkit::require_samples!();
    let (t, mut lib) = temp_library();
    lib.import_paths(&[sample_path(FIT_1025)]).unwrap();
    let a = copy_as(P_0723, &t.path().join("a"), "IMG_1.JPG");
    let b = copy_as(P_78D7, &t.path().join("b"), "img_1.jpeg");
    lib.import_paths(&[a, b]).unwrap();
    let id = lib.month_view(2026, 10).unwrap().days[0].items[0]
        .activity_id
        .clone();
    let mut cards = lib.post_view(&id).unwrap().card_paths;
    cards.sort();
    cards.dedup_by(|x, y| x.to_lowercase() == y.to_lowercase());
    assert_eq!(cards.len(), 2);
}

/// C1: 운동 지정 시 같은 이름의 기존 사진을 덮어쓰지 않는다.
#[test]
fn assign_does_not_overwrite_existing_photo() {
    workout_testkit::require_samples!();
    let (t, mut lib) = temp_library();
    let ride = copy_as(P_78D7, &t.path().join("in"), "IMG_0001.jpeg");
    let stray = copy_as(P_31CC, &t.path().join("other"), "IMG_0001.jpeg");
    let r = lib
        .import_paths(&[sample_path(FIT_1025), ride.clone()])
        .unwrap();
    let id = r.added_activities[0].clone();
    let r2 = lib.import_paths(&[stray]).unwrap();
    let ph = lib.assign_photo(r2.unmatched_photos[0].id, &id).unwrap();
    let detail = lib.activity_detail(&id).unwrap();
    assert_eq!(detail.photos.len(), 2);
    let original = detail.photos.iter().find(|p| p.id != ph.id).unwrap();
    assert_eq!(
        std::fs::metadata(&original.path).unwrap().len(),
        std::fs::metadata(&ride).unwrap().len()
    );
    assert_ne!(original.path, ph.path);
    assert!(std::path::Path::new(ph.card_path.as_ref().unwrap()).exists());
}

/// I3: 매칭 안 된 사진은 나중에도 목록으로 볼 수 있다.
#[test]
fn unmatched_photos_listed_later() {
    workout_testkit::require_samples!();
    let (_t, mut lib) = temp_library();
    lib.import_paths(&[sample_path(FIT_1025), sample_path(P_31CC)])
        .unwrap();
    assert_eq!(lib.unmatched_photos().unwrap().len(), 1);
}

/// I3: 사진을 먼저, FIT를 나중에 가져와도 사진이 붙는다.
#[test]
fn photos_first_then_fit_get_matched() {
    workout_testkit::require_samples!();
    let (_t, mut lib) = temp_library();
    let r1 = lib.import_paths(&[sample_path(P_0723)]).unwrap();
    assert_eq!(r1.unmatched_photos.len(), 1);
    let r2 = lib.import_paths(&[sample_path(FIT_1025)]).unwrap();
    assert_eq!(r2.cards_created, 1);
    assert!(lib.unmatched_photos().unwrap().is_empty());
    let id = &r2.added_activities[0];
    assert_eq!(lib.post_view(id).unwrap().card_paths.len(), 1);
}

/// I4: 시간대 태그 없고 근처 운동도 없는 사진은 이 PC 시간대로 해석한다.
#[test]
fn tagless_unmatched_photo_uses_local_offset() {
    use workout_core::chrono::{Local, NaiveDateTime, TimeZone, Utc};
    let (t, mut lib) = temp_library();
    let p = t.path().join("cam.jpg");
    jpeg_with_local_time(&p, "2026:10:20 23:00:00");
    let r = lib.import_paths(&[p]).unwrap();
    let naive = NaiveDateTime::parse_from_str("2026-10-20 23:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
    let expected = Local
        .from_local_datetime(&naive)
        .single()
        .unwrap()
        .with_timezone(&Utc)
        .to_rfc3339();
    assert_eq!(
        r.unmatched_photos[0].taken_at.as_deref(),
        Some(expected.as_str())
    );
    assert!(r.unmatched_photos[0].path.contains("20261020"));
}

/// M6(중요로 상향): 게시한 글은 나중에 사진이 붙어도 바뀌지 않는다.
#[test]
fn published_post_is_not_rewritten() {
    workout_testkit::require_samples!();
    let (_t, mut lib) = temp_library();
    let id = lib
        .import_paths(&[sample_path(FIT_1025)])
        .unwrap()
        .added_activities[0]
        .clone();
    let before = lib.mark_published(&id).unwrap().body;
    lib.import_paths(&[sample_path(P_0723)]).unwrap();
    assert_eq!(lib.post_view(&id).unwrap().body, before);
}

/// 해시 열이 생기기 전에 가져온 사진도 다시 가져오면 중복으로 본다.
#[test]
fn old_photos_get_hash_backfilled() {
    workout_testkit::require_samples!();
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("lib");
    {
        let mut lib = workout_service::Library::open(&root).unwrap();
        lib.import_paths(&[sample_path(FIT_1025), sample_path(P_0723)])
            .unwrap();
    }
    // 예전 버전처럼 해시를 지운다.
    let conn = rusqlite::Connection::open(root.join("library.db")).unwrap();
    conn.execute("UPDATE photos SET content_hash = NULL", [])
        .unwrap();
    drop(conn);
    let mut lib = workout_service::Library::open(&root).unwrap();
    let again = copy_as(P_0723, &t.path().join("x"), "copy.jpeg");
    assert_eq!(
        lib.import_paths(&[again]).unwrap().duplicate_files,
        vec!["copy.jpeg"]
    );
}
