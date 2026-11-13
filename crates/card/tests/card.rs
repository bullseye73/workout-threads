use std::path::PathBuf;
use workout_card::*;
use workout_core::fit::parse_fit;

fn sample(rel: &str) -> PathBuf {
    workout_testkit::sample(rel)
}
fn data(rel: &str) -> CardData {
    card_data(&parse_fit(&std::fs::read(sample(rel)).unwrap()).unwrap())
}
const FIT_1011: &str = "20261011/24680396566_ACTIVITY.fit";
const FIT_1025: &str = "20261025/24842957203_ACTIVITY.fit";

#[test]
fn card_data_matches_python() {
    workout_testkit::require_samples!();
    let d = data(FIT_1011);
    assert_eq!(d.date, "2026.10.11");
    assert_eq!(d.title, "일요일 오전 라이딩");
    assert_eq!(d.distance, "59.38");
    assert_eq!(d.sub, "2:30:52  ·  23.6 km/h  ·  147 bpm");
    assert!(d.route.len() > 1000 && d.route.len() <= 1500);
    let d2 = data(FIT_1025);
    assert_eq!(d2.title, "일요일 오후 라이딩");
    assert_eq!(d2.sub, "1:58:22  ·  24.4 km/h  ·  146 bpm");
}

#[test]
fn renders_small_photo_upscaled_without_exif() {
    workout_testkit::require_samples!();
    let jpg = render_card(
        &sample("20261011/31CCE40E-34DE-405F-ACE0-A1B5DEFC07D4_4_5005_c.jpeg"),
        &data(FIT_1011),
    )
    .unwrap();
    let img = image::load_from_memory(&jpg).unwrap();
    assert_eq!(img.width().max(img.height()), 2160);
    assert!(img.height() > img.width());
    assert!(
        exif::Reader::new()
            .read_from_container(&mut std::io::Cursor::new(&jpg))
            .is_err()
    );
}

#[test]
fn renders_without_route() {
    workout_testkit::require_samples!();
    let mut d = data(FIT_1025);
    d.route.clear();
    assert!(
        render_card(
            &sample("20261025/78D7CE5C-F354-4728-8E97-063487771792_1_105_c.jpeg"),
            &d
        )
        .is_ok()
    );
}

#[test]
fn unreadable_photo_is_error() {
    workout_testkit::require_samples!();
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("x.jpg");
    std::fs::write(&p, b"nope").unwrap();
    assert!(render_card(&p, &data(FIT_1025)).is_err());
}

#[test]
fn render_to_file_writes_jpeg() {
    workout_testkit::require_samples!();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("o_card.jpg");
    render_card_to_file(
        &sample("20261025/78D7CE5C-F354-4728-8E97-063487771792_1_105_c.jpeg"),
        &data(FIT_1025),
        &out,
    )
    .unwrap();
    let img = image::open(&out).unwrap(); // 1182×664 가로 사진 → 긴 변 2160
    assert_eq!((img.width(), img.height()), (2160, 1213));
}

/// 사람이 눈으로 비교할 미리보기: cargo test -p workout-card -- --ignored
#[test]
#[ignore]
fn write_previews() {
    workout_testkit::require_samples!();
    let out = PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/card-preview"
    ));
    std::fs::create_dir_all(&out).unwrap();
    for p in [
        "0723ACAA-C18C-412E-BDF8-0CB9253CB50F_1_102_a",
        "78D7CE5C-F354-4728-8E97-063487771792_1_105_c",
    ] {
        render_card_to_file(
            &sample(&format!("20261025/{p}.jpeg")),
            &data(FIT_1025),
            &out.join(format!("{p}_card.jpg")),
        )
        .unwrap();
    }
}

#[test]
fn trimmed_card_route_hides_ends() {
    workout_testkit::require_samples!();
    let a = parse_fit(&std::fs::read(sample(FIT_1025)).unwrap()).unwrap();
    let full = card_data(&a);
    let cut = card_data_with(&a, 500.0);
    assert!(cut.route.len() < full.route.len() && cut.route.len() > 100);
    assert_ne!(cut.route[0], full.route[0]);
    assert_eq!(card_data_with(&a, 0.0), full);
}
