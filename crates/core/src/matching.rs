//! 사진 촬영 시각 ↔ 운동 매칭.
use chrono::{DateTime, Duration, Utc};
use std::path::{Path, PathBuf};

use crate::photo::read_photo_meta;

use crate::model::Activity;

/// 운동 시작 전·종료 후 이만큼까지는 그 운동의 사진으로 본다.
pub const MATCH_WINDOW_MIN: i64 = 10;

#[derive(Clone, Debug, PartialEq)]
pub struct PhotoMatch {
    pub activity_id: String,
    /// 촬영 지점 (km, 소수 1자리).
    pub km_mark: f64,
}

pub fn match_photo(activities: &[Activity], taken_at: DateTime<Utc>) -> Option<PhotoMatch> {
    // 운동 중에 찍힌 사진을 우선하고, 없으면 ±10분 창 안에서 가장 가까운 운동.
    let w = Duration::minutes(MATCH_WINDOW_MIN);
    let gap = |a: &Activity| {
        if taken_at < a.header.start {
            a.header.start - taken_at
        } else if taken_at > a.header.end() {
            taken_at - a.header.end()
        } else {
            Duration::zero()
        }
    };
    let a = activities
        .iter()
        .filter(|a| gap(a) <= w)
        .min_by_key(|a| gap(a))?;
    let meters = if taken_at <= a.header.start {
        0.0
    } else if taken_at >= a.header.end() {
        a.samples
            .iter()
            .rev()
            .find_map(|s| s.distance_m)
            .unwrap_or(a.header.summary.distance_m)
    } else {
        match a.samples.binary_search_by_key(&taken_at, |s| s.time) {
            Ok(i) => a.samples[i].distance_m.unwrap_or(0.0),
            Err(i) => {
                let prev = if i > 0 {
                    (0..i)
                        .rev()
                        .find_map(|idx| a.samples[idx].distance_m.map(|d| (a.samples[idx].time, d)))
                } else {
                    None
                };
                let next = (i..a.samples.len())
                    .find_map(|idx| a.samples[idx].distance_m.map(|d| (a.samples[idx].time, d)));
                match (prev, next) {
                    (Some((t1, d1)), Some((t2, d2))) => {
                        if (taken_at - t1).abs() <= (t2 - taken_at).abs() {
                            d1
                        } else {
                            d2
                        }
                    }
                    (Some((_, d1)), None) => d1,
                    (None, Some((_, d2))) => d2,
                    (None, None) => 0.0,
                }
            }
        }
    };
    Some(PhotoMatch {
        activity_id: a.header.id.clone(),
        km_mark: (meters / 100.0).round() / 10.0,
    })
}

/// 사진 한 장의 매칭 결과.
#[derive(Clone, Debug)]
pub struct PhotoAssignment {
    pub photo: PathBuf,
    pub taken_at: Option<DateTime<Utc>>,
    pub matched: Option<PhotoMatch>,
}

/// 사진들의 메타를 읽어 운동에 매칭한다. EXIF에 오프셋이 없으면
/// 운동이 있을 때 첫 운동의 UTC 오프셋, 없으면 0을 쓴다.
pub fn assign_photos(activities: &[Activity], photos: &[PathBuf]) -> Vec<PhotoAssignment> {
    let fallback = activities
        .first()
        .map(|a| a.header.utc_offset_s)
        .unwrap_or(0);
    photos
        .iter()
        .map(|p| assign_photo_with_fallback(activities, p, fallback))
        .collect()
}

/// 사진 한 장을 매칭한다. EXIF에 오프셋이 없으면 `fallback`(UTC 오프셋 초)을 쓴다.
pub fn assign_photo_with_fallback(
    activities: &[Activity],
    photo: &Path,
    fallback: i32,
) -> PhotoAssignment {
    let taken_at = read_photo_meta(photo, fallback)
        .ok()
        .and_then(|m| m.taken_at);
    PhotoAssignment {
        photo: photo.to_path_buf(),
        taken_at,
        matched: taken_at.and_then(|t| match_photo(activities, t)),
    }
}
