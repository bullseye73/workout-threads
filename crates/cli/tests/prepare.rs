use workout_cli::prepare::prepare;

fn copy_to_temp(files: &[&str]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = workout_testkit::samples_root();
    for f in files {
        let src = root.join(f);
        std::fs::copy(&src, dir.path().join(src.file_name().unwrap())).unwrap();
    }
    dir
}
const FIT_1011: &str = "20261011/24680396566_ACTIVITY.fit";
const FIT_1025: &str = "20261025/24842957203_ACTIVITY.fit";
const P_0723: &str = "20261025/0723ACAA-C18C-412E-BDF8-0CB9253CB50F_1_102_a.jpeg";
const P_78D7: &str = "20261025/78D7CE5C-F354-4728-8E97-063487771792_1_105_c.jpeg";

#[test]
fn prepares_20261025() {
    workout_testkit::require_samples!();
    let dir = copy_to_temp(&[FIT_1025, P_0723, P_78D7]);
    let r = prepare(dir.path()).unwrap();
    assert_eq!(r.activities, 1);
    assert_eq!(r.cards.len(), 2);
    assert!(
        dir.path()
            .join("0723ACAA-C18C-412E-BDF8-0CB9253CB50F_1_102_a_card.jpg")
            .exists()
    );
    let post = std::fs::read_to_string(dir.path().join("post_1441.txt")).unwrap();
    assert!(post.starts_with("일요일 오후 48km 🚲"));
    assert!(post.contains("km 지점에서 찍었음"));
}

#[test]
fn bad_fit_and_unmatched_photo_do_not_stop_run() {
    workout_testkit::require_samples!();
    let dir = copy_to_temp(&[
        FIT_1025,
        "20261011/31CCE40E-34DE-405F-ACE0-A1B5DEFC07D4_4_5005_c.jpeg",
    ]);
    std::fs::write(dir.path().join("broken.fit"), b"garbage").unwrap();
    let r = prepare(dir.path()).unwrap();
    assert_eq!(r.skipped_fit.len(), 1);
    assert_eq!(r.unmatched_photos.len(), 1);
    assert!(r.cards.is_empty());
    assert_eq!(r.posts.len(), 1);
}

#[test]
fn two_activities_get_their_own_posts() {
    workout_testkit::require_samples!();
    let dir = copy_to_temp(&[FIT_1011, FIT_1025, P_78D7]);
    let r = prepare(dir.path()).unwrap();
    assert_eq!(r.posts.len(), 2);
    assert_eq!(r.cards.len(), 1);
    let p1011 = std::fs::read_to_string(dir.path().join("post_0925.txt")).unwrap();
    assert!(!p1011.contains("지점에서 찍었음"));
}

#[test]
fn ignores_existing_cards() {
    workout_testkit::require_samples!();
    let dir = copy_to_temp(&[FIT_1025]);
    std::fs::copy(
        workout_testkit::samples_root()
            .join("20261025/78D7CE5C-F354-4728-8E97-063487771792_1_105_c_card.jpg"),
        dir.path()
            .join("78D7CE5C-F354-4728-8E97-063487771792_1_105_c_card.jpg"),
    )
    .unwrap();
    assert!(prepare(dir.path()).unwrap().cards.is_empty());
}

#[test]
fn folder_without_fit_is_error() {
    let dir = tempfile::tempdir().unwrap();
    assert!(prepare(dir.path()).is_err());
}

#[test]
fn duplicate_fit_is_counted_once() {
    workout_testkit::require_samples!();
    let dir = copy_to_temp(&[FIT_1025, P_78D7]);
    std::fs::copy(
        dir.path().join("24842957203_ACTIVITY.fit"),
        dir.path().join("24842957203_ACTIVITY (1).fit"),
    )
    .unwrap();
    let r = prepare(dir.path()).unwrap();
    assert_eq!(r.activities, 1);
    assert_eq!(r.posts.len(), 1);
    assert_eq!(r.duplicate_fit.len(), 1);
    assert!(
        std::fs::read_to_string(dir.path().join("post_1441.txt"))
            .unwrap()
            .contains("지점에서 찍었음")
    );
}

#[test]
fn failed_card_does_not_stop_run() {
    workout_testkit::require_samples!();
    let dir = copy_to_temp(&[FIT_1025, P_0723, P_78D7]);
    // 출력 경로를 폴더로 막아 이 사진의 카드 저장을 실패시킨다
    std::fs::create_dir(
        dir.path()
            .join("78D7CE5C-F354-4728-8E97-063487771792_1_105_c_card.jpg"),
    )
    .unwrap();
    let r = prepare(dir.path()).unwrap();
    assert_eq!(r.failed_cards.len(), 1);
    assert_eq!(r.cards.len(), 1);
    assert_eq!(r.posts.len(), 1);
}
