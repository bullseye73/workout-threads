//! 구글 로그인(OAuth, 설치형 앱 흐름)과 구글 드라이브 백업 저장소.
//! 권한은 drive.file 하나 (앱이 만든 파일만 볼 수 있음).
pub mod drive;
pub mod oauth;

use workout_enrich::EnrichError;

#[derive(thiserror::Error, Debug)]
pub enum DriveError {
    #[error(transparent)]
    Http(#[from] EnrichError),
    #[error("구글 서버 오류({0}): {1}")]
    Status(u16, String),
    #[error("응답을 해석하지 못했어요: {0}")]
    Parse(String),
    #[error("로그인이 취소됐어요")]
    Cancelled,
    #[error("로그인 확인값이 맞지 않아요. 다시 시도해 주세요")]
    StateMismatch,
    #[error("로그인 시간이 지났어요. 다시 시도해 주세요")]
    Timeout,
    #[error("구글에 다시 로그인해 주세요")]
    NeedLogin,
    #[error("라이브러리가 비어 있을 때만 복원할 수 있어요")]
    NotEmpty,
    #[error(
        "드라이브에 다른 PC(라이브러리)의 백업이 있어요. 이 PC를 비우고 복원하거나, 다른 구글 계정·클라이언트를 쓰세요"
    )]
    OtherBackup,
    #[error("드라이브에서 찾을 수 없어요: {0}")]
    NotFound(String),
}

/// 액세스 토큰 (만료 unix초)과 리프레시 토큰.
#[derive(Clone, Debug, PartialEq)]
pub struct Tokens {
    pub access: String,
    pub expires_at: i64,
    pub refresh: Option<String>,
}
