use workout_core::chrono::{DateTime, Utc};
use workout_core::fit::parse_fit;
use workout_core::model::*;
use workout_post::*;

fn header_20261025() -> ActivityHeader {
    let p = workout_testkit::sample("20261025/24842957203_ACTIVITY.fit");
    parse_fit(&std::fs::read(p).unwrap()).unwrap().header
}
fn run_header(distance_m: f64, moving_s: f64, hr: Option<u8>) -> ActivityHeader {
    ActivityHeader {
        id: "run".into(),
        sport: Sport::Running,
        start: "2026-11-01T22:00:00Z".parse::<DateTime<Utc>>().unwrap(),
        utc_offset_s: 9 * 3600,
        summary: Summary {
            distance_m,
            moving_s,
            elapsed_s: moving_s,
            avg_hr: hr,
            max_hr: None,
            avg_speed_mps: distance_m / moving_s,
            ascent_m: None,
            avg_cadence: None,
        },
    }
}
fn ctx_for(h: ActivityHeader) -> PostContext {
    PostContext {
        activity: h,
        place: None,
        route_points: vec![],
        weather: None,
        photo_km_marks: vec![],
    }
}

#[test]
fn full_template() {
    workout_testkit::require_samples!();
    let ctx = PostContext {
        activity: header_20261025(),
        place: Some(PlaceText {
            name: "한강".into(),
            estimated: true,
        }),
        route_points: vec!["반포".into(), "잠실".into()],
        weather: Some(WeatherText {
            description: "맑음".into(),
            temp_c: 17.4,
        }),
        photo_km_marks: vec![12.34, 25.0],
    };
    let t = TemplateWriter.write(&ctx).unwrap();
    let lines: Vec<&str> = t.body.lines().collect();
    assert_eq!(lines[0], "일요일 오후 한강 48km 🚲");
    assert_eq!(lines[1], "반포 → 잠실 찍고 복귀");
    assert_eq!(lines[2], "평속 24.4km/h / 이동 1:58:22 / 평균 심박 146");
    assert_eq!(lines[3], "사진은 12.3km, 25.0km 지점에서 찍었음");
    assert_eq!(lines[4], "날씨 맑음, 17도");
    assert_eq!(lines.len(), 6);
    assert_eq!(
        t.hashtags,
        vec!["#라이딩", "#자전거", "#한강", "#반포", "#잠실"]
    );
    assert!(!t.full_text().contains("추정"));
}

#[test]
fn minimal_template_drops_empty_lines() {
    workout_testkit::require_samples!();
    let t = TemplateWriter.write(&ctx_for(header_20261025())).unwrap();
    assert_eq!(t.body.lines().next().unwrap(), "일요일 오후 48km 🚲");
    assert_eq!(t.body.lines().count(), 3);
    assert_eq!(t.hashtags, vec!["#라이딩", "#자전거"]);
}

#[test]
fn running_uses_pace() {
    let t = TemplateWriter
        .write(&ctx_for(run_header(5000.0, 1650.0, None)))
        .unwrap();
    assert!(t.body.lines().next().unwrap().ends_with("5.0km 🏃"));
    assert_eq!(
        t.body.lines().nth(1).unwrap(),
        "페이스 5'30\"/km / 이동 27:30"
    );
}

#[test]
fn place_with_spaces_becomes_one_hashtag_and_dedupes() {
    workout_testkit::require_samples!();
    let mut c = ctx_for(header_20261025());
    c.place = Some(PlaceText {
        name: "한강 공원".into(),
        estimated: false,
    });
    c.route_points = vec!["한강 공원".into(), "잠실".into()];
    assert_eq!(
        TemplateWriter.write(&c).unwrap().hashtags,
        vec!["#라이딩", "#자전거", "#한강공원", "#잠실"]
    );
}

#[test]
fn question_is_deterministic() {
    workout_testkit::require_samples!();
    let a = TemplateWriter.write(&ctx_for(header_20261025())).unwrap();
    let b = TemplateWriter.write(&ctx_for(header_20261025())).unwrap();
    assert_eq!(a.body, b.body);
    assert!(a.body.lines().last().unwrap().ends_with('?'));
}

#[test]
fn manual_export_writes_files() {
    let dir = tempfile::tempdir().unwrap();
    let card = dir.path().join("c_card.jpg");
    std::fs::write(&card, b"x").unwrap();
    let out = dir.path().join("out");
    let r = ManualExport {
        out_dir: out.clone(),
    }
    .publish(&PostDraft {
        activity_id: "a".into(),
        text: PostText {
            body: "본문".into(),
            hashtags: vec!["#러닝".into()],
        },
        card_paths: vec![card],
    })
    .unwrap();
    assert!(matches!(r.method, PublishMethod::ManualExport));
    assert!(r.external_id.is_none());
    assert_eq!(
        std::fs::read_to_string(out.join("post.txt")).unwrap(),
        "본문\n\n#러닝"
    );
    assert!(out.join("c_card.jpg").exists());
}
