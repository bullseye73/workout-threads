// 사진만 올릴 때도, EXIF에 시간대가 없으면 짝이 될 운동의 시간대로 읽어야 한다 (이 PC 시간대가 달라도).
// TZ를 바꾸므로 프로세스를 나눈다.
mod common;
use common::*;

const P_0723: &str = "20261025/0723ACAA-C18C-412E-BDF8-0CB9253CB50F_1_102_a.jpeg";
const FIT_1025: &str = "20261025/24842957203_ACTIVITY.fit";

#[test]
fn photo_only_upload_uses_the_workouts_time_zone() {
    workout_testkit::require_samples!();
    // SAFETY: 이 파일의 유일한 테스트라 다른 스레드가 환경을 읽지 않는다. 시간 계산 전에 바꾼다.
    unsafe { std::env::set_var("TZ", "America/Los_Angeles") };
    let (t, mut lib) = temp_library();
    lib.import_paths(&[sample_path(FIT_1025)]).unwrap();
    let p = t.path().join("in/during.jpeg");
    real_jpeg_with_local_time(P_0723, &p, "2026:10:25 15:30:00"); // 라이딩 중(현지 +09:00)
    let plan = lib.plan_upload(&[p], &[]).unwrap();
    let photos = &plan.existing[0].photos;
    assert_eq!(photos.len(), 1);
    assert!(
        photos[0].km_mark.is_some(),
        "운동 시간으로 맞아 km 지점이 있어야 한다"
    );
}
