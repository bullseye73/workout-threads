//! 카드와 글이 함께 쓰는 한국어 표시 형식.
use chrono::{Datelike, NaiveDate, NaiveDateTime, Timelike, Weekday};

use crate::model::Sport;

/// 초 → "H:MM:SS" (1시간 미만은 "M:SS").
pub fn hms(seconds: f64) -> String {
    let s = seconds.round() as u64;
    let (h, m, s) = (s / 3600, s % 3600 / 60, s % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

/// km당 페이스 "M'SS\"". 거리가 0이면 빈 문자열.
pub fn pace_per_km(moving_s: f64, distance_m: f64) -> String {
    if distance_m <= 0.0 {
        return String::new();
    }
    let pace = (moving_s / (distance_m / 1000.0)) as u64;
    format!("{}'{:02}\"", pace / 60, pace % 60)
}

pub fn speed_kmh(moving_s: f64, distance_m: f64) -> f64 {
    if moving_s <= 0.0 {
        0.0
    } else {
        distance_m / 1000.0 / moving_s * 3600.0
    }
}

pub fn weekday_ko(d: NaiveDate) -> &'static str {
    match d.weekday() {
        Weekday::Mon => "월요일",
        Weekday::Tue => "화요일",
        Weekday::Wed => "수요일",
        Weekday::Thu => "목요일",
        Weekday::Fri => "금요일",
        Weekday::Sat => "토요일",
        Weekday::Sun => "일요일",
    }
}

pub fn time_of_day_ko(hour: u32) -> &'static str {
    match hour {
        0..=4 => "새벽",
        5..=7 => "이른아침",
        8..=11 => "오전",
        12..=16 => "오후",
        17..=19 => "저녁",
        _ => "밤",
    }
}

pub fn sport_ko(s: Sport) -> &'static str {
    match s {
        Sport::Running => "러닝",
        Sport::Cycling => "라이딩",
        Sport::Walking => "걷기",
        Sport::Hiking => "하이킹",
        Sport::Other(_) => "운동",
    }
}

/// 운동 제목 "일요일 오전 라이딩" (카드와 목록이 같이 쓴다).
pub fn activity_title(local: NaiveDateTime, sport: Sport) -> String {
    format!(
        "{} {} {}",
        weekday_ko(local.date()),
        time_of_day_ko(local.hour()),
        sport_ko(sport)
    )
}
