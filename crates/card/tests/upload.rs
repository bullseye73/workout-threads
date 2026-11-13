use workout_card::photo_for_upload;

#[test]
fn upload_photo_is_small_and_has_no_exif() {
    workout_testkit::require_samples!();
    let p = workout_testkit::sample("20261025/0723ACAA-C18C-412E-BDF8-0CB9253CB50F_1_102_a.jpeg");
    let b = photo_for_upload(&p, 1024).unwrap();
    let img = image::load_from_memory(&b).unwrap();
    assert_eq!(img.width().max(img.height()), 1024);
    assert!(img.height() > img.width());
    assert!(
        exif::Reader::new()
            .read_from_container(&mut std::io::Cursor::new(&b))
            .is_err()
    );
}
