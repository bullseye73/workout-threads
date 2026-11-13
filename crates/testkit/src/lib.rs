//! 테스트 도우미: 실제 운동 샘플(사용자의 FIT·사진) 위치.
//! 샘플에는 GPS(집 위치)가 있어 저장소·CI에 넣지 않는다. 없으면 그 테스트를 건너뛴다.
use std::path::PathBuf;

/// `WORKOUT_SAMPLES` 환경변수, 없으면 저장소 바깥의 sns_create 폴더.
pub fn samples_root() -> PathBuf {
    match std::env::var_os("WORKOUT_SAMPLES") {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.."),
    }
}

pub fn sample(rel: &str) -> PathBuf {
    samples_root().join(rel)
}

pub fn has_samples() -> bool {
    let r = samples_root();
    r.join("20261011").is_dir() && r.join("20261025").is_dir()
}

/// 실제 샘플이 없으면(예: CI) 이 테스트를 건너뛴다.
#[macro_export]
macro_rules! require_samples {
    () => {
        if !$crate::has_samples() {
            eprintln!("실제 샘플 없음: 건너뜀");
            return;
        }
    };
}
