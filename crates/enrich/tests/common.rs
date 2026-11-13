#![allow(dead_code, unused_imports)]
pub use workout_enrich::mock::MockHttp;

pub fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/").to_string() + name)
        .unwrap()
}

use workout_core::chrono::{DateTime, Utc};
use workout_core::model::*;
use workout_enrich::refine::RefineInput;
use workout_post::{PlaceText, PostContext, PostText, WeatherText};

pub fn input_with_route(photos: Vec<Vec<u8>>) -> RefineInput {
    let header = ActivityHeader {
        id: "a".into(),
        sport: Sport::Cycling,
        start: "2026-10-25T05:41:29Z".parse::<DateTime<Utc>>().unwrap(),
        utc_offset_s: 9 * 3600,
        summary: Summary {
            distance_m: 48159.0,
            moving_s: 7101.6,
            elapsed_s: 8957.6,
            avg_hr: Some(146),
            max_hr: Some(169),
            avg_speed_mps: 6.78,
            ascent_m: Some(139),
            avg_cadence: None,
        },
    };
    RefineInput {
        ctx: PostContext {
            activity: header,
            place: Some(PlaceText {
                name: "한강".into(),
                estimated: true,
            }),
            route_points: vec!["반포".into(), "잠실".into()],
            weather: Some(WeatherText {
                description: "맑음".into(),
                temp_c: 17.4,
            }),
            photo_km_marks: vec![16.0, 19.9],
        },
        draft: PostText {
            body: "일요일 오후 한강 48km 🚲".into(),
            hashtags: vec!["#라이딩".into()],
        },
        photos_jpeg: photos,
    }
}
