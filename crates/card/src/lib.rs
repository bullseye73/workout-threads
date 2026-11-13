//! 사진 위에 운동 기록을 얹은 SNS 카드 이미지 (순수 Rust, 3 OS 공통).
mod data;
mod render;
mod text;
mod upload;

pub use data::{CardData, card_data, card_data_with};
pub use render::{render_card, render_card_to_file};
pub use upload::photo_for_upload;

#[derive(thiserror::Error, Debug)]
pub enum CardError {
    #[error("이미지 처리 실패: {0}")]
    Image(#[from] image::ImageError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("그리기 실패: {0}")]
    Draw(String),
}
