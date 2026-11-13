#![allow(dead_code)]
use workout_core::chrono::{DateTime, Utc};
use workout_core::model::*;
use workout_store::*;

pub fn utc(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
}

pub fn activity_row(id: &str, sport: Sport, start: &str, distance_m: f64) -> ActivityRow {
    ActivityRow {
        header: ActivityHeader {
            id: id.into(),
            sport,
            start: utc(start),
            utc_offset_s: 9 * 3600,
            summary: Summary {
                distance_m,
                moving_s: 1800.0,
                elapsed_s: 1900.0,
                avg_hr: Some(140),
                max_hr: Some(170),
                avg_speed_mps: distance_m / 1800.0,
                ascent_m: Some(12),
                avg_cadence: None,
            },
        },
        fit_path: format!("files/x/{id}.fit"),
        route: vec![
            LatLon {
                lat: 37.5,
                lon: 126.9,
            },
            LatLon {
                lat: 37.51,
                lon: 126.91,
            },
        ],
    }
}

pub fn photo_row(activity_id: Option<&str>, path: &str) -> PhotoRow {
    PhotoRow {
        id: 0,
        activity_id: activity_id.map(Into::into),
        path: path.into(),
        taken_at: Some(utc("2026-10-25T06:00:00Z")),
        km_mark: None,
        lat: None,
        lon: None,
        card_path: None,
        content_hash: None,
    }
}

pub fn post_row(activity_id: &str, body: &str, edited: bool) -> PostRow {
    PostRow {
        activity_id: activity_id.into(),
        body: body.into(),
        hashtags: vec!["#라이딩".into()],
        place: None,
        route_points: vec![],
        weather: None,
        edited,
        published_at: None,
        place_user_set: false,
        route_user_set: false,
        weather_user_set: false,
        auto_place: None,
        auto_route_points: vec![],
        auto_weather: None,
    }
}

pub fn ids(rows: Vec<ActivityRow>) -> Vec<String> {
    rows.into_iter().map(|r| r.header.id).collect()
}
