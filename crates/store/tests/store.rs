mod common;
use common::*;
use workout_core::model::Sport;
use workout_post::{PlaceText, PublishMethod, WeatherText};
use workout_store::*;

#[test]
fn activity_roundtrip() {
    let s = Store::open_in_memory().unwrap();
    let row = activity_row("a1", Sport::Running, "2026-11-01T22:00:00Z", 5000.0);
    s.insert_activity(&row).unwrap();
    assert!(s.has_activity("a1").unwrap());
    assert!(!s.has_activity("zz").unwrap());
    let back = s.activity("a1").unwrap().unwrap();
    assert_eq!(back.header.sport, Sport::Running);
    assert_eq!(back.header.start, row.header.start);
    assert_eq!(back.header.summary.avg_hr, Some(140));
    assert_eq!(back.route, row.route);
    assert_eq!(back.fit_path, row.fit_path);
}

#[test]
fn month_query_uses_local_date() {
    let s = Store::open_in_memory().unwrap();
    s.insert_activity(&activity_row(
        "nov",
        Sport::Running,
        "2026-11-01T22:00:00Z",
        5000.0,
    ))
    .unwrap();
    s.insert_activity(&activity_row(
        "oct",
        Sport::Cycling,
        "2026-10-31T14:30:00Z",
        9000.0,
    ))
    .unwrap();
    assert_eq!(ids(s.activities_in_month(2026, 11).unwrap()), vec!["nov"]);
    assert_eq!(ids(s.activities_in_month(2026, 10).unwrap()), vec!["oct"]);
}

#[test]
fn activities_near_window() {
    let s = Store::open_in_memory().unwrap();
    s.insert_activity(&activity_row(
        "a",
        Sport::Cycling,
        "2026-10-25T05:41:29Z",
        48000.0,
    ))
    .unwrap();
    assert_eq!(
        ids(s.activities_near(utc("2026-10-25T06:30:00Z"), 36).unwrap()),
        vec!["a"]
    );
    assert!(
        s.activities_near(utc("2026-10-28T06:30:00Z"), 36)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn photos_and_unmatched() {
    let s = Store::open_in_memory().unwrap();
    s.insert_activity(&activity_row(
        "a1",
        Sport::Cycling,
        "2026-10-25T05:41:29Z",
        48000.0,
    ))
    .unwrap();
    let id = s
        .insert_photo(&photo_row(None, "files/20261025/p.jpeg"))
        .unwrap();
    assert_eq!(s.unmatched_photos().unwrap().len(), 1);
    let mut p = s.photo(id).unwrap().unwrap();
    p.activity_id = Some("a1".into());
    p.km_mark = Some(12.3);
    p.card_path = Some("files/20261025/p_card.jpg".into());
    s.update_photo(&p).unwrap();
    assert!(s.unmatched_photos().unwrap().is_empty());
    let got = &s.photos_for("a1").unwrap()[0];
    assert_eq!(got.km_mark, Some(12.3));
    assert_eq!(got.card_path.as_deref(), Some("files/20261025/p_card.jpg"));
    assert_eq!(
        s.photo_by_path("files/20261025/p.jpeg")
            .unwrap()
            .unwrap()
            .id,
        id
    );
    assert!(
        s.insert_photo(&photo_row(None, "files/20261025/p.jpeg"))
            .is_err()
    );
}

#[test]
fn post_upsert_keeps_latest() {
    let s = Store::open_in_memory().unwrap();
    s.insert_activity(&activity_row(
        "a1",
        Sport::Cycling,
        "2026-10-25T05:41:29Z",
        48000.0,
    ))
    .unwrap();
    s.upsert_post(&post_row("a1", "첫 글", false)).unwrap();
    let mut p = post_row("a1", "고친 글", true);
    p.place = Some(PlaceText {
        name: "한강".into(),
        estimated: true,
    });
    p.weather = Some(WeatherText {
        description: "맑음".into(),
        temp_c: 17.4,
    });
    p.route_points = vec!["반포".into()];
    p.published_at = Some(utc("2026-10-25T10:00:00Z"));
    s.upsert_post(&p).unwrap();
    let back = s.post("a1").unwrap().unwrap();
    assert_eq!(back, p);
    assert!(s.post("zz").unwrap().is_none());
}

#[test]
fn publish_record_insert() {
    let s = Store::open_in_memory().unwrap();
    s.insert_activity(&activity_row(
        "a1",
        Sport::Cycling,
        "2026-10-25T05:41:29Z",
        48000.0,
    ))
    .unwrap();
    s.insert_publish(&PublishRow {
        activity_id: "a1".into(),
        method: PublishMethod::ManualExport,
        at: utc("2026-10-25T10:00:00Z"),
        external_id: None,
    })
    .unwrap();
}

#[test]
fn settings_set_and_clear() {
    let s = Store::open_in_memory().unwrap();
    s.set_setting("monthly_goal_km", Some("300")).unwrap();
    assert_eq!(
        s.setting("monthly_goal_km").unwrap().as_deref(),
        Some("300")
    );
    s.set_setting("monthly_goal_km", None).unwrap();
    assert!(s.setting("monthly_goal_km").unwrap().is_none());
}

#[test]
fn reopen_file_db_keeps_data() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("library.db");
    Store::open(&p)
        .unwrap()
        .insert_activity(&activity_row(
            "a1",
            Sport::Running,
            "2026-11-01T22:00:00Z",
            5000.0,
        ))
        .unwrap();
    assert!(Store::open(&p).unwrap().has_activity("a1").unwrap());
}

#[test]
fn v3_flags_and_geocode_cache() {
    let s = Store::open_in_memory().unwrap();
    s.insert_activity(&activity_row(
        "a1",
        Sport::Cycling,
        "2026-10-25T05:41:29Z",
        48000.0,
    ))
    .unwrap();
    let mut p = post_row("a1", "글", false);
    p.place_user_set = true;
    p.weather_user_set = true;
    s.upsert_post(&p).unwrap();
    let back = s.post("a1").unwrap().unwrap();
    assert!(back.place_user_set && !back.route_user_set && back.weather_user_set);
    assert!(s.geocode_cached("37.5000,127.0000").unwrap().is_none());
    s.put_geocode("37.5000,127.0000", "{}").unwrap();
    assert_eq!(
        s.geocode_cached("37.5000,127.0000").unwrap().as_deref(),
        Some("{}")
    );
}
