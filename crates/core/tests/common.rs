#![allow(dead_code)]
use chrono::{DateTime, Duration, Utc};
use std::path::PathBuf;
use workout_core::fit::RawSession;
use workout_core::model::*;

pub fn sample_path(rel: &str) -> PathBuf {
    workout_testkit::sample(rel)
}
pub fn fit_20261011() -> PathBuf {
    sample_path("20261011/24680396566_ACTIVITY.fit")
}
pub fn fit_20261025() -> PathBuf {
    sample_path("20261025/24842957203_ACTIVITY.fit")
}
pub fn read(p: PathBuf) -> Vec<u8> {
    std::fs::read(p).unwrap()
}
pub fn utc(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
}

pub fn raw_session(sport: u8, start: DateTime<Utc>, distance_m: f64, timer_s: f64) -> RawSession {
    RawSession {
        sport,
        start,
        distance_m,
        timer_s,
        elapsed_s: timer_s,
        avg_hr: None,
        max_hr: None,
        avg_speed_mps: None,
        ascent_m: None,
        avg_cadence: None,
    }
}

/// 1초 간격, 초당 3m, GPS 없음.
pub fn samples_without_pos(start: DateTime<Utc>, n: usize) -> Vec<Sample> {
    (0..n)
        .map(|i| Sample {
            time: start + Duration::seconds(i as i64),
            pos: None,
            distance_m: Some(i as f64 * 3.0),
            heart_rate: None,
            speed_mps: Some(3.0),
            altitude_m: None,
            cadence: None,
        })
        .collect()
}

/// GPS 없는 합성 러닝 (초당 3m).
pub fn synthetic_run_on(start: &str, secs: usize) -> Activity {
    let st = utc(start);
    workout_core::fit::activity_from_records(
        raw_session(1, st, secs as f64 * 3.0, secs as f64),
        samples_without_pos(st, secs),
        Some(1),
        Some(9 * 3600),
    )
}

pub fn header(sport: Sport, start: &str, distance_m: f64, moving_s: f64) -> ActivityHeader {
    ActivityHeader {
        id: format!("t-{start}"),
        sport,
        start: utc(start),
        utc_offset_s: 9 * 3600,
        summary: Summary {
            distance_m,
            moving_s,
            elapsed_s: moving_s,
            avg_hr: None,
            max_hr: None,
            avg_speed_mps: distance_m / moving_s,
            ascent_m: None,
            avg_cadence: None,
        },
    }
}
