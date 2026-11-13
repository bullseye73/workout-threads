//! 게시 방식. 지금은 수동 내보내기, 나중에 Threads API를 같은 트레이트로 붙인다.
use std::path::PathBuf;
use workout_core::chrono::{DateTime, Utc};

use crate::PostError;
use crate::context::PostText;

#[derive(Clone, Debug)]
pub struct PostDraft {
    pub activity_id: String,
    pub text: PostText,
    pub card_paths: Vec<PathBuf>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublishMethod {
    ManualExport,
    ThreadsApi,
}

#[derive(Clone, Debug)]
pub struct PublishReceipt {
    pub method: PublishMethod,
    pub at: DateTime<Utc>,
    /// Threads 게시물 ID (자동 게시 때만).
    pub external_id: Option<String>,
}

pub trait Publisher {
    fn publish(&self, draft: &PostDraft) -> Result<PublishReceipt, PostError>;
}

/// `out_dir/post.txt`에 글을 쓰고 카드 파일을 복사한다. 클립보드 복사는 화면 쪽 몫.
pub struct ManualExport {
    pub out_dir: PathBuf,
}

impl Publisher for ManualExport {
    fn publish(&self, draft: &PostDraft) -> Result<PublishReceipt, PostError> {
        std::fs::create_dir_all(&self.out_dir)?;
        std::fs::write(self.out_dir.join("post.txt"), draft.text.full_text())?;
        for card in &draft.card_paths {
            if let Some(name) = card.file_name() {
                std::fs::copy(card, self.out_dir.join(name))?;
            }
        }
        Ok(PublishReceipt {
            method: PublishMethod::ManualExport,
            at: Utc::now(),
            external_id: None,
        })
    }
}
