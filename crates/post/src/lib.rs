//! 스레드 게시물 준비: 초안 A 템플릿 글과 게시 방식(지금은 수동 내보내기).
mod context;
mod publish;
mod template;

pub use context::{PlaceText, PostContext, PostText, WeatherText};
pub use publish::{ManualExport, PostDraft, PublishMethod, PublishReceipt, Publisher};
pub use template::{DraftWriter, TemplateWriter};

#[derive(thiserror::Error, Debug)]
pub enum PostError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("글 작성 실패: {0}")]
    Writer(String),
}
