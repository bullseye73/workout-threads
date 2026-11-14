// 이 PC의 시간대가 운동한 곳과 달라도(여행·서머타임) 날짜 지정 등록은 같은 기준으로 사진을 판단·저장해야 한다.
// TZ 환경변수를 바꾸므로 다른 테스트와 프로세스를 나눈다.
mod common;
use common::*;
use workout_core::chrono::NaiveDate;

const P_0723: &str = "20261025/0723ACAA-C18C-412E-BDF8-0CB9253CB50F_1_102_a.jpeg";
const FIT_1025: &str = "20261025/24842957203_ACTIVITY.fit";

#[test]
fn late_photo_joins_ride_even_when_this_pc_is_in_another_zone() {
    workout_testkit::require_samples!();
    let date = NaiveDate::from_ymd_opt(2026, 10, 25).unwrap();
    let (t, mut lib) = temp_library();
    let id = lib
        .import_for_date(date, &[sample_path(FIT_1025)])
        .unwrap()
        .added_activities[0]
        .clone();
    // SAFETY: 이 테스트 파일에는 테스트가 하나뿐이라 다른 스레드가 환경을 읽지 않는다.
    unsafe { std::env::set_var("TZ", "America/Los_Angeles") };
    let late = t.path().join("in/late.jpeg");
    real_jpeg_with_local_time(P_0723, &late, "2026:10:25 23:30:00");
    // FIT는 이미 있어 이번 묶음의 새 운동이 아니다 (중복).
    let r = lib
        .import_for_date(date, &[sample_path(FIT_1025), late])
        .unwrap();
    assert!(r.skipped.is_empty(), "{:?}", r.skipped);
    assert!(
        r.unmatched_photos.is_empty(),
        "사진이 그날 유일한 운동에 붙어야 한다"
    );
    assert_eq!(lib.post_view(&id).unwrap().card_paths.len(), 1);
}
