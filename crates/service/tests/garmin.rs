mod common;
use std::path::PathBuf;
use std::sync::Arc;

use workout_core::chrono::{DateTime, NaiveDate, Utc};
use workout_core::model::{ActivityHeader, Sport, Summary};
use workout_enrich::HttpResponse;
use workout_enrich::mock::MockHttp;
use workout_service::garmin::GARMIN_LAST_CHECK_KEY;
use workout_service::secrets::{GARMIN_ACCESS, GARMIN_KEY, MemoryStore, SecretStore};
use workout_service::{Library, SportKind};
use workout_store::ActivityRow;

fn open_mock(http: Arc<dyn workout_enrich::Http>) -> (tempfile::TempDir, Library) {
    let t = tempfile::tempdir().unwrap();
    let secrets = Arc::new(MemoryStore::default());
    secrets.set(GARMIN_KEY, Some("test-personal-key")).unwrap();
    secrets.set(GARMIN_ACCESS, Some("tok|9999999999")).unwrap();
    let lib = Library::open_with(&t.path().join("lib"), http, secrets).unwrap();
    (t, lib)
}

#[test]
fn already_imported_marks_matching_store_activity() {
    let activities_json = serde_json::json!([
        {
            "activityId": 101,
            "activityName": "이미 있는 러닝",
            "startTimeGMT": "2026-11-15 00:00:00",
            "startTimeLocal": "2026-11-15 09:00:00",
            "activityType": { "typeKey": "running" },
            "distance": 5000.0,
            "duration": 1500.0
        },
        {
            "activityId": 102,
            "activityName": "새 라이딩",
            "startTimeGMT": "2026-11-14 01:00:00",
            "startTimeLocal": "2026-11-14 10:00:00",
            "activityType": { "typeKey": "cycling" },
            "distance": 30000.0,
            "duration": 3600.0
        },
        {
            "activityId": 103,
            "activityName": "새 러닝",
            "startTimeGMT": "2026-11-13 00:00:00",
            "startTimeLocal": "2026-11-13 09:00:00",
            "activityType": { "typeKey": "running" },
            "distance": 8000.0,
            "duration": 2400.0
        }
    ]);
    let (_t, lib) = open_mock(Arc::new(MockHttp::new(vec![HttpResponse {
        status: 200,
        body: serde_json::to_vec(&activities_json).unwrap(),
    }])));

    let start_utc = DateTime::parse_from_rfc3339("2026-11-15T00:00:30Z")
        .unwrap()
        .with_timezone(&Utc);
    lib.store_for_tests()
        .insert_activity(&ActivityRow {
            header: ActivityHeader {
                id: "existing-1".into(),
                sport: Sport::Running,
                start: start_utc,
                utc_offset_s: 9 * 3600,
                summary: Summary {
                    distance_m: 5000.0,
                    moving_s: 1500.0,
                    elapsed_s: 1500.0,
                    ascent_m: Some(10),
                    avg_hr: None,
                    max_hr: None,
                    avg_speed_mps: 3.3,
                    avg_cadence: None,
                },
            },
            fit_path: "files/existing-1.fit".into(),
            route: vec![],
        })
        .unwrap();

    let from = NaiveDate::from_ymd_opt(2026, 11, 10).unwrap();
    let to = NaiveDate::from_ymd_opt(2026, 11, 15).unwrap();
    let list = lib.garmin_activities(from, to).unwrap();
    assert_eq!(list.len(), 3);
    assert!(list[0].already_imported);
    assert_eq!(list[0].sport, SportKind::Running);
    assert!(!list[1].already_imported);
    assert_eq!(list[1].sport, SportKind::Cycling);
    assert!(!list[2].already_imported);
}

#[test]
fn fetch_garmin_fits_caches_and_continues_on_error() {
    let (_t, lib) = open_mock(Arc::new(MockHttp::new(vec![
        HttpResponse {
            status: 200,
            body: b"FIT_A".to_vec(),
        },
        HttpResponse {
            status: 500,
            body: b"server error".to_vec(),
        },
        HttpResponse {
            status: 200,
            body: b"FIT_C".to_vec(),
        },
    ])));

    let first = lib
        .fetch_garmin_fits(vec!["a".into(), "b".into(), "c".into()])
        .unwrap();
    assert_eq!(first.paths.len(), 2);
    assert_eq!(first.failed.len(), 1);
    assert_eq!(first.failed[0].file, "b");

    // "a" is already in garmin-inbox, so no HTTP response is consumed!
    let cached = lib.fetch_garmin_fits(vec!["a".into()]).unwrap();
    assert_eq!(cached.paths.len(), 1);
    assert!(cached.failed.is_empty());
}

#[test]
fn garmin_new_since_last_check_updates_only_on_success() {
    let empty = b"[]".to_vec();
    let (_t, mut lib) = open_mock(Arc::new(MockHttp::new(vec![HttpResponse {
        status: 200,
        body: empty,
    }])));

    assert!(
        lib.store_for_tests()
            .setting(GARMIN_LAST_CHECK_KEY)
            .unwrap()
            .is_none()
    );
    let r1 = lib.garmin_new_since_last_check().unwrap();
    assert!(r1.paths.is_empty());
    let first_ts = lib
        .store_for_tests()
        .setting(GARMIN_LAST_CHECK_KEY)
        .unwrap()
        .expect("garmin_last_check should be set");

    // Network failure case: MockHttp::failing()
    let t2 = tempfile::tempdir().unwrap();
    let secrets2 = Arc::new(MemoryStore::default());
    secrets2.set(GARMIN_KEY, Some("k")).unwrap();
    secrets2.set(GARMIN_ACCESS, Some("tok|9999999999")).unwrap();
    let mut lib2 = Library::open_with(
        &t2.path().join("lib"),
        Arc::new(MockHttp::failing()),
        secrets2,
    )
    .unwrap();
    lib2.store_for_tests()
        .set_setting(GARMIN_LAST_CHECK_KEY, Some(&first_ts))
        .unwrap();
    assert!(lib2.garmin_new_since_last_check().is_err());
    let after_err_ts = lib2
        .store_for_tests()
        .setting(GARMIN_LAST_CHECK_KEY)
        .unwrap()
        .unwrap();
    assert_eq!(first_ts, after_err_ts);
}

#[test]
fn fetched_fit_plans_and_commit_cleans_only_created_inbox_file() {
    workout_testkit::require_samples!();
    let fit_bytes =
        std::fs::read(common::sample_path("20261025/24842957203_ACTIVITY.fit")).unwrap();
    let (_t, mut lib) = open_mock(Arc::new(MockHttp::new(vec![
        HttpResponse {
            status: 200,
            body: fit_bytes.clone(),
        },
        HttpResponse {
            status: 200,
            body: fit_bytes,
        },
    ])));

    let fetched = lib
        .fetch_garmin_fits(vec!["act1".into(), "act2".into()])
        .unwrap();
    assert_eq!(fetched.paths.len(), 2);
    let p1 = PathBuf::from(&fetched.paths[0]);
    let p2 = PathBuf::from(&fetched.paths[1]);

    let plan = lib.plan_upload(std::slice::from_ref(&p1), &[]).unwrap();
    assert_eq!(plan.new_activities.len(), 1);

    let res = lib
        .commit_upload(&[p1.clone(), p2.clone()], std::slice::from_ref(&p2))
        .unwrap();
    assert_eq!(res.created.len(), 1);
    assert!(!p1.exists(), "created inbox FIT should be removed");
    assert!(p2.exists(), "excluded inbox FIT should remain");
}
