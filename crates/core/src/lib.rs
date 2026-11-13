//! 운동 기록 앱의 도메인 코어: 모델, FIT 해석, 사진 메타데이터, 매칭, 통계, 공용 포맷.
//! UI·네트워크 의존이 없어 CLI와 데스크톱 앱이 함께 쓴다.

pub mod fit;
pub mod format;
pub mod matching;
pub mod model;
pub mod photo;
pub mod privacy;
pub mod route;
pub mod splits;
pub mod stats;

pub use chrono;
pub use model::*;

/// 코어 전반의 오류.
#[derive(thiserror::Error, Debug)]
pub enum CoreError {
    #[error("FIT 해석 실패: {0}")]
    Fit(String),
    #[error("사진 읽기 실패: {0}")]
    Photo(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
