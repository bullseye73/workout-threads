//! 사진 EXIF 메타데이터 (촬영 시각, GPS, 회전).
use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone, Utc};
use exif::{In, Reader, Tag, Value};
use std::path::{Path, PathBuf};

use crate::CoreError;
use crate::model::LatLon;

#[derive(Clone, Debug)]
pub struct PhotoMeta {
    pub path: PathBuf,
    pub taken_at: Option<DateTime<Utc>>,
    pub pos: Option<LatLon>,
    /// EXIF Orientation 1..8 (없으면 1).
    pub orientation: u16,
}

/// 사진의 메타데이터를 읽는다. EXIF가 없거나 깨졌으면 오류가 아니라 빈 값.
/// `fallback_offset_s`: EXIF에 오프셋 태그가 없을 때 쓸 UTC 오프셋.
pub fn read_photo_meta(path: &Path, fallback_offset_s: i32) -> Result<PhotoMeta, CoreError> {
    let file = std::fs::File::open(path)?;
    let mut meta = PhotoMeta {
        path: path.to_path_buf(),
        taken_at: None,
        pos: None,
        orientation: 1,
    };
    let Ok(exif) = Reader::new().read_from_container(&mut std::io::BufReader::new(file)) else {
        return Ok(meta);
    };
    let ascii = |tag| match exif.get_field(tag, In::PRIMARY).map(|f| &f.value) {
        Some(Value::Ascii(v)) => v
            .first()
            .map(|b| String::from_utf8_lossy(b).trim().to_string()),
        _ => None,
    };
    if let Some(local) = ascii(Tag::DateTimeOriginal)
        .or_else(|| ascii(Tag::DateTime))
        .and_then(|s| NaiveDateTime::parse_from_str(&s, "%Y:%m:%d %H:%M:%S").ok())
    {
        let offset = ascii(Tag::OffsetTimeOriginal).or_else(|| ascii(Tag::OffsetTime));
        meta.taken_at = Some(resolve_taken_at(
            local,
            offset.as_deref(),
            fallback_offset_s,
        ));
    }
    meta.orientation = exif
        .get_field(Tag::Orientation, In::PRIMARY)
        .and_then(|f| f.value.get_uint(0))
        .map(|v| v as u16)
        .filter(|v| (1..=8).contains(v))
        .unwrap_or(1);
    let coord = |tag, ref_tag, neg: &str| {
        let Some(Value::Rational(r)) = exif.get_field(tag, In::PRIMARY).map(|f| &f.value) else {
            return None;
        };
        if r.len() < 3 {
            return None;
        }
        let deg = r[0].to_f64() + r[1].to_f64() / 60.0 + r[2].to_f64() / 3600.0;
        let sign = if ascii(ref_tag).as_deref() == Some(neg) {
            -1.0
        } else {
            1.0
        };
        Some(deg * sign)
    };
    if let (Some(lat), Some(lon)) = (
        coord(Tag::GPSLatitude, Tag::GPSLatitudeRef, "S"),
        coord(Tag::GPSLongitude, Tag::GPSLongitudeRef, "W"),
    ) {
        meta.pos = Some(LatLon { lat, lon });
    }
    Ok(meta)
}

/// 현지 시각 + 오프셋 태그("+09:00") → UTC. 태그가 없거나 깨졌으면 fallback 오프셋.
pub fn resolve_taken_at(
    local: NaiveDateTime,
    offset_tag: Option<&str>,
    fallback_offset_s: i32,
) -> DateTime<Utc> {
    let offset = offset_tag
        .and_then(parse_offset)
        .or_else(|| FixedOffset::east_opt(fallback_offset_s))
        .unwrap_or(FixedOffset::east_opt(0).unwrap());
    offset
        .from_local_datetime(&local)
        .single()
        .map(|t| t.with_timezone(&Utc))
        .unwrap_or_else(|| Utc.from_utc_datetime(&local))
}

fn parse_offset(s: &str) -> Option<FixedOffset> {
    let (sign, rest) = match s.as_bytes().first()? {
        b'+' => (1, &s[1..]),
        b'-' => (-1, &s[1..]),
        _ => return None,
    };
    let (h, m) = rest.split_once(':')?;
    FixedOffset::east_opt(sign * (h.parse::<i32>().ok()? * 3600 + m.parse::<i32>().ok()? * 60))
}

/// 입력으로 쓸 사진인지: jpg/jpeg/png, 그리고 이미 만든 `_card` 결과물이 아님.
pub fn is_photo_input(path: &Path) -> bool {
    let ext_ok = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| matches!(e.to_ascii_lowercase().as_str(), "jpg" | "jpeg" | "png"))
        .unwrap_or(false);
    let not_card = path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| !s.ends_with("_card"))
        .unwrap_or(false);
    ext_ok && not_card
}
