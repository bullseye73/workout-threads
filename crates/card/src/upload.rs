//! 외부(Claude)로 보낼 사진: EXIF(GPS 포함) 없이 다시 인코딩한 축소본.
use std::path::Path;

use crate::CardError;
use crate::render::open_oriented;

const UPLOAD_QUALITY: u8 = 85;

/// EXIF 회전 반영, 긴 변이 max_side보다 크면 줄인 JPEG. 메타데이터는 쓰지 않는다.
pub fn photo_for_upload(path: &Path, max_side: u32) -> Result<Vec<u8>, CardError> {
    let img = open_oriented(path)?;
    let img = if img.width().max(img.height()) > max_side {
        img.resize(max_side, max_side, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, UPLOAD_QUALITY)
        .encode_image(&img.to_rgb8())?;
    Ok(out)
}

/// EXIF 회전 반영, 긴 변이 max_side보다 크면 줄인 JPEG를 파일로 저장한다.
pub fn photo_for_upload_to_file(path: &Path, max_side: u32, out: &Path) -> Result<(), CardError> {
    std::fs::write(out, photo_for_upload(path, max_side)?)?;
    Ok(())
}
