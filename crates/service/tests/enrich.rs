mod common;
use common::*;
use workout_core::model::*;
use workout_core::route::haversine_m;
use workout_service::*;

fn queried_coords(http: &MockHttp) -> Vec<LatLon> {
    http.calls()
        .iter()
        .filter(|u| u.contains("nominatim"))
        .map(|u| {
            let get = |k: &str| {
                u.split(&format!("{k}="))
                    .nth(1)
                    .unwrap()
                    .split('&')
                    .next()
                    .unwrap()
                    .parse::<f64>()
                    .unwrap()
            };
            LatLon {
                lat: get("lat"),
                lon: get("lon"),
            }
        })
        .collect()
}

#[test]
fn enrich_fills_place_route_weather() {
    workout_testkit::require_samples!();
    let (_t, mut lib, http) = temp_library_with_mock();
    let id = import_1025(&mut lib);
    let p = lib.enrich_activity(&id).unwrap();
    let place = p.place.clone().unwrap();
    assert_eq!(place.name, "반포한강공원");
    assert!(place.estimated);
    assert_eq!(p.route_points, vec!["반포", "잠실"]);
    assert_eq!(p.weather.as_ref().unwrap().description, "흐림");
    assert!(p.body.contains("반포 → 잠실 찍고 복귀"), "{}", p.body);
    assert!(p.body.contains("날씨 흐림, 17도"));
    assert!(p.enrich_error.is_none());
    assert!(http.count("nominatim") <= 5);
    assert!(http.log.lock().unwrap().iter().all(|c| {
        c.headers
            .iter()
            .any(|(k, v)| k == "User-Agent" && v.starts_with("workout-app"))
    }));
}

#[test]
fn lookups_never_include_ends() {
    workout_testkit::require_samples!();
    let (_t, mut lib, http) = temp_library_with_mock();
    let id = import_1025(&mut lib);
    lib.enrich_activity(&id).unwrap();
    let route = lib.activity_detail(&id).unwrap().route;
    let s = LatLon {
        lat: route[0][0],
        lon: route[0][1],
    };
    let e = LatLon {
        lat: route[route.len() - 1][0],
        lon: route[route.len() - 1][1],
    };
    let pts = queried_coords(&http);
    assert!(!pts.is_empty());
    for p in pts {
        assert!(haversine_m(p, s) > 450.0 && haversine_m(p, e) > 450.0);
    }
}

#[test]
fn second_enrich_uses_cache() {
    workout_testkit::require_samples!();
    let (_t, mut lib, http) = temp_library_with_mock();
    let id = import_1025(&mut lib);
    lib.enrich_activity(&id).unwrap();
    let n = http.count("nominatim");
    lib.enrich_activity(&id).unwrap();
    assert_eq!(http.count("nominatim"), n);
}

#[test]
fn user_place_survives_reenrich() {
    workout_testkit::require_samples!();
    let (_t, mut lib, _h) = temp_library_with_mock();
    let id = import_1025(&mut lib);
    lib.set_place(&id, Some("뚝섬".into())).unwrap();
    let p = lib.enrich_activity(&id).unwrap();
    let place = p.place.clone().unwrap();
    assert_eq!(place.name, "뚝섬");
    assert!(!place.estimated);
    assert!(p.place_user_set);
    assert_eq!(p.route_points, vec!["반포", "잠실"]);
    // 되돌리면 다시 자동 값
    let back = lib.set_place(&id, None).unwrap();
    assert!(!back.place_user_set);
}

#[test]
fn manual_route_and_weather_are_kept() {
    workout_testkit::require_samples!();
    let (_t, mut lib, _h) = temp_library_with_mock();
    let id = import_1025(&mut lib);
    lib.set_route_points(&id, vec!["여의도".into()]).unwrap();
    lib.set_weather(
        &id,
        Some(WeatherDto {
            description: "맑음".into(),
            temp_c: 20.0,
        }),
    )
    .unwrap();
    let p = lib.enrich_activity(&id).unwrap();
    assert_eq!(p.route_points, vec!["여의도"]);
    assert_eq!(p.weather.unwrap().description, "맑음");
    assert!(p.route_user_set && p.weather_user_set);
}

#[test]
fn edited_post_body_is_not_rewritten() {
    workout_testkit::require_samples!();
    let (_t, mut lib, _h) = temp_library_with_mock();
    let id = import_1025(&mut lib);
    lib.save_post(&id, "직접 쓴 글", &[]).unwrap();
    assert_eq!(lib.enrich_activity(&id).unwrap().body, "직접 쓴 글");
}

#[test]
fn no_gps_activity_skips_network() {
    let (_t, mut lib, http) = temp_library_with_mock();
    let id = "no-gps".to_string();
    lib.store_for_tests()
        .insert_activity(&workout_store::ActivityRow {
            header: ActivityHeader {
                id: id.clone(),
                sport: Sport::Running,
                start: "2026-11-01T22:00:00Z".parse().unwrap(),
                utc_offset_s: 9 * 3600,
                summary: Summary {
                    distance_m: 5000.0,
                    moving_s: 1650.0,
                    elapsed_s: 1700.0,
                    avg_hr: None,
                    max_hr: None,
                    avg_speed_mps: 3.0,
                    ascent_m: None,
                    avg_cadence: None,
                },
            },
            fit_path: "files/20261102/x.fit".into(),
            route: vec![],
        })
        .unwrap();
    let p = lib.enrich_activity(&id).unwrap();
    assert!(p.place.is_none() && p.weather.is_none() && p.enrich_error.is_none());
    assert_eq!(http.calls().len(), 0);
}

#[test]
fn network_failure_is_reported_not_fatal() {
    workout_testkit::require_samples!();
    let (_t, mut lib, _h) = temp_library_with_failing_http();
    let id = import_1025(&mut lib);
    let p = lib.enrich_activity(&id).unwrap();
    assert!(p.enrich_error.is_some());
    assert!(p.place.is_none());
}

#[test]
fn refine_without_key_sends_nothing() {
    workout_testkit::require_samples!();
    let (_t, mut lib, http) = temp_library_with_mock();
    let id = import_1025(&mut lib);
    let err = lib.refine_post(&id, false).unwrap_err().to_string();
    assert!(err.contains("API 키"));
    assert_eq!(http.count("anthropic"), 0);
}

#[test]
fn refine_returns_preview_without_saving() {
    workout_testkit::require_samples!();
    let (_t, mut lib, http) = temp_library_with_mock();
    let id = import_1025(&mut lib);
    lib.set_refine_provider("claude").unwrap();
    lib.set_refine_key("claude", Some("test-key".into()))
        .unwrap();
    let before = lib.post_view(&id).unwrap().body;
    let r = lib.refine_post(&id, true).unwrap();
    assert!(r.body.starts_with("일요일 오후"));
    assert_eq!(r.hashtags, vec!["#라이딩", "#한강"]);
    assert_eq!(lib.post_view(&id).unwrap().body, before);
    let body = http.last_body("anthropic");
    assert_eq!(body["messages"][0]["content"][0]["type"], "image");
    assert_eq!(body["model"], "claude-opus-5-5");
    assert!(!body.to_string().contains("\"lat"));
}

#[test]
fn model_setting_is_used() {
    workout_testkit::require_samples!();
    let (_t, mut lib, http) = temp_library_with_mock();
    let id = import_1025(&mut lib);
    lib.set_refine_provider("claude").unwrap();
    lib.set_refine_key("claude", Some("k".into())).unwrap();
    lib.set_refine_model("claude", Some("claude-sonnet-5-5".into()))
        .unwrap();
    lib.refine_post(&id, false).unwrap();
    assert_eq!(http.last_body("anthropic")["model"], "claude-sonnet-5-5");
    assert_eq!(lib.refine_status().unwrap().model, "claude-sonnet-5-5");
}

#[test]
fn key_status_never_reveals_key() {
    let (_t, mut lib, _h) = temp_library_with_mock();
    assert!(!lib.refine_status().unwrap().has_key);
    lib.set_refine_provider("claude").unwrap();
    lib.set_refine_key("claude", Some("sk-secret".into()))
        .unwrap();
    let s = serde_json::to_string(&lib.refine_status().unwrap()).unwrap();
    assert!(s.contains("\"hasKey\":true") && !s.contains("sk-secret"));
    lib.set_refine_provider("claude").unwrap();
    lib.set_refine_key("claude", None).unwrap();
    assert!(!lib.refine_status().unwrap().has_key);
}

#[test]
fn regenerate_cards_respects_setting() {
    workout_testkit::require_samples!();
    let (_t, mut lib, _h) = temp_library_with_mock();
    let id = import_1025(&mut lib);
    assert!(!lib.settings().unwrap().card_route_trim);
    lib.set_card_route_trim(true).unwrap();
    assert_eq!(lib.regenerate_cards(&id).unwrap(), 2);
    assert!(lib.settings().unwrap().card_route_trim);
}

/// 리뷰 I1: 인터넷 조회는 라이브러리를 잡지 않은 채 할 수 있어야 한다.
#[test]
fn enrich_job_runs_without_library() {
    workout_testkit::require_samples!();
    let (_t, mut lib, _h) = temp_library_with_mock();
    let id = import_1025(&mut lib);
    let job = lib.prepare_enrich(&id).unwrap();
    assert_eq!(lib.month_view(2026, 10).unwrap().days.len(), 1); // 조회 전에도 다른 명령 가능
    let fetched = job.run();
    let p = lib.apply_enrich(fetched).unwrap();
    assert_eq!(p.place.unwrap().name, "반포한강공원");
    assert_eq!(p.route_points, vec!["반포", "잠실"]);
}

/// 리뷰 I1: 조회하는 사이 사용자가 장소를 고쳤다면 조회 결과가 덮어쓰지 않는다.
#[test]
fn enrich_apply_rechecks_user_edits() {
    workout_testkit::require_samples!();
    let (_t, mut lib, _h) = temp_library_with_mock();
    let id = import_1025(&mut lib);
    let job = lib.prepare_enrich(&id).unwrap();
    lib.set_place(&id, Some("뚝섬".into())).unwrap();
    let p = lib.apply_enrich(job.run()).unwrap();
    assert_eq!(p.place.unwrap().name, "뚝섬");
}

/// 리뷰 I2: 다듬기는 화면에 있는(저장 안 된) 글을 보낸다.
#[test]
fn refine_uses_given_text() {
    workout_testkit::require_samples!();
    let (_t, mut lib, http) = temp_library_with_mock();
    let id = import_1025(&mut lib);
    lib.set_refine_provider("claude").unwrap();
    lib.set_refine_key("claude", Some("k".into())).unwrap();
    let job = lib
        .prepare_refine(&id, "내가 방금 고친 글", &["#직접".into()], false)
        .unwrap();
    job.run().unwrap();
    let body = http.last_body("anthropic").to_string();
    assert!(body.contains("내가 방금 고친 글") && body.contains("#직접"));
}

/// 리뷰 I4: 직접 정한 값을 지우면 마지막 자동 값으로 돌아간다.
#[test]
fn clearing_user_values_restores_auto_values() {
    workout_testkit::require_samples!();
    let (_t, mut lib, _h) = temp_library_with_mock();
    let id = import_1025(&mut lib);
    lib.enrich_activity(&id).unwrap();
    lib.set_place(&id, Some("뚝섬".into())).unwrap();
    lib.set_route_points(&id, vec!["여의도".into()]).unwrap();
    lib.set_weather(
        &id,
        Some(WeatherDto {
            description: "맑음".into(),
            temp_c: 20.0,
        }),
    )
    .unwrap();
    let p = lib.set_place(&id, None).unwrap();
    let place = p.place.unwrap();
    assert_eq!(place.name, "반포한강공원");
    assert!(place.estimated);
    assert_eq!(
        lib.set_route_points(&id, vec![]).unwrap().route_points,
        vec!["반포", "잠실"]
    );
    assert_eq!(
        lib.set_weather(&id, None)
            .unwrap()
            .weather
            .unwrap()
            .description,
        "흐림"
    );
}
