mod common;
use common::*;
use std::sync::Arc;
use workout_core::chrono::{Duration, TimeZone, Utc};

fn now() -> workout_core::chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 14, 12, 0, 0).unwrap()
}

#[test]
fn first_backup_uploads_everything_in_tree() {
    workout_testkit::require_samples!();
    let t = tempfile::tempdir().unwrap();
    let drive = Arc::new(FakeDrive::default());
    let mut lib = library_with_drive(&t.path().join("lib"), drive.clone());
    lib.import_paths(&[sample_path("20261025")]).unwrap();
    let s = lib.backup_now(now()).unwrap();
    assert!(s.last_error.is_none(), "{:?}", s.last_error);
    assert!(s.last_success.is_some());
    assert_eq!(s.pending, 0);
    let paths = drive.paths();
    assert!(paths.contains(&"WorkoutApp/library.db".to_string()));
    assert!(paths.contains(&"WorkoutApp/files/20261025/24842957203_ACTIVITY.fit".to_string()));
    assert!(
        paths.contains(
            &"WorkoutApp/files/20261025/78D7CE5C-F354-4728-8E97-063487771792_1_105_c_card.jpg"
                .to_string()
        )
    );
    assert_eq!(
        drive.file_count(),
        count_files(&lib.root().join("files")) + 2
    );
}

#[test]
fn second_backup_uploads_only_changes() {
    workout_testkit::require_samples!();
    let t = tempfile::tempdir().unwrap();
    let drive = Arc::new(FakeDrive::default());
    let mut lib = library_with_drive(&t.path().join("lib"), drive.clone());
    let id = lib
        .import_paths(&[sample_path("20261025")])
        .unwrap()
        .added_activities[0]
        .clone();
    lib.backup_now(now()).unwrap();
    let n = drive.file_count();
    assert!(!lib.needs_backup(now()).unwrap());
    lib.save_post(&id, "고친 글", &[]).unwrap();
    assert!(lib.needs_backup(now()).unwrap());
    lib.backup_now(now()).unwrap();
    assert_eq!(drive.file_count(), n);
    assert!(!lib.needs_backup(now()).unwrap());
}

#[test]
fn needs_backup_rules() {
    workout_testkit::require_samples!();
    let t = tempfile::tempdir().unwrap();
    let drive = Arc::new(FakeDrive::default());
    let mut lib = library_with_drive(&t.path().join("lib"), drive);
    assert!(!lib.needs_backup(now()).unwrap()); // 빈 라이브러리는 백업하지 않음
    lib.import_paths(&[sample_path(
        "20261025/0723ACAA-C18C-412E-BDF8-0CB9253CB50F_1_102_a.jpeg",
    )])
    .unwrap();
    assert!(lib.needs_backup(now()).unwrap());
    lib.backup_now(now()).unwrap();
    assert!(!lib.needs_backup(now()).unwrap());
    assert!(lib.needs_backup(now() + Duration::hours(25)).unwrap());
    lib.import_paths(&[sample_path("20261025/24842957203_ACTIVITY.fit")])
        .unwrap();
    assert!(lib.needs_backup(now()).unwrap());
    assert!(lib.backup_status().unwrap().pending >= 1);
}

#[test]
fn not_signed_in_means_no_backup() {
    let (_t, lib, _h) = temp_library_with_mock();
    assert!(!lib.needs_backup(now()).unwrap());
    let s = lib.backup_status().unwrap();
    assert!(!s.configured && !s.signed_in);
}

#[test]
fn interrupted_backup_resumes_without_duplicates() {
    workout_testkit::require_samples!();
    let t = tempfile::tempdir().unwrap();
    let drive = Arc::new(FakeDrive::default());
    let mut lib = library_with_drive(&t.path().join("lib"), drive.clone());
    lib.import_paths(&[sample_path("20261025")]).unwrap();
    drive.fail_after(4);
    let s = lib.backup_now(now()).unwrap();
    assert!(s.last_error.is_some());
    assert!(s.pending > 0);
    drive.heal();
    let s = lib.backup_now(now()).unwrap();
    assert!(s.last_error.is_none());
    assert_eq!(s.pending, 0);
    let paths = drive.paths();
    let mut dedup = paths.clone();
    dedup.dedup();
    assert_eq!(paths.len(), dedup.len(), "중복 파일 {paths:?}");
    assert_eq!(
        drive.file_count(),
        count_files(&lib.root().join("files")) + 2
    );
}

#[test]
fn expired_login_asks_to_sign_in_again() {
    workout_testkit::require_samples!();
    let t = tempfile::tempdir().unwrap();
    let drive = Arc::new(FakeDrive::default());
    let mut lib = library_with_drive(&t.path().join("lib"), drive.clone());
    lib.import_paths(&[sample_path("20261025/24842957203_ACTIVITY.fit")])
        .unwrap();
    drive.need_login(true);
    let s = lib.backup_now(now()).unwrap();
    assert!(!s.signed_in);
    assert!(s.last_error.unwrap().contains("다시 로그인"));
    assert!(
        lib.secrets_for_tests()
            .get(workout_service::secrets::GOOGLE_REFRESH)
            .is_none()
    );
}

#[test]
fn restore_into_empty_library() {
    workout_testkit::require_samples!();
    let t = tempfile::tempdir().unwrap();
    let drive = Arc::new(FakeDrive::default());
    let mut a = library_with_drive(&t.path().join("a"), drive.clone());
    a.import_paths(&[sample_path("20261025")]).unwrap();
    a.backup_now(now()).unwrap();
    let n = drive.file_count();

    let mut b = library_with_drive(&t.path().join("b"), drive.clone());
    assert!(b.backup_status().unwrap().can_restore);
    b.restore_from_drive().unwrap();
    let v = b.month_view(2026, 10).unwrap();
    assert_eq!(v.cycling.count, 1);
    assert_eq!(
        count_files(&b.root().join("files")),
        count_files(&a.root().join("files"))
    );
    let id = &v.days[0].items[0].activity_id;
    assert_eq!(b.post_view(id).unwrap().card_paths.len(), 2);
    // 복원한 쪽에서 백업해도 드라이브에 중복이 생기지 않는다.
    b.backup_now(now()).unwrap();
    assert_eq!(drive.file_count(), n);
}

#[test]
fn restore_refused_when_not_empty() {
    workout_testkit::require_samples!();
    let t = tempfile::tempdir().unwrap();
    let drive = Arc::new(FakeDrive::default());
    let mut lib = library_with_drive(&t.path().join("lib"), drive);
    lib.import_paths(&[sample_path("20261025")]).unwrap();
    let before = count_files(&lib.root().join("files"));
    assert!(!lib.backup_status().unwrap().can_restore);
    assert!(
        lib.restore_from_drive()
            .unwrap_err()
            .to_string()
            .contains("비어 있을 때만")
    );
    assert_eq!(count_files(&lib.root().join("files")), before);
}

#[test]
fn failed_restore_leaves_empty_library() {
    workout_testkit::require_samples!();
    let t = tempfile::tempdir().unwrap();
    let drive = Arc::new(FakeDrive::default());
    let mut a = library_with_drive(&t.path().join("a"), drive.clone());
    a.import_paths(&[sample_path("20261025")]).unwrap();
    a.backup_now(now()).unwrap();
    let mut b = library_with_drive(&t.path().join("b"), drive.clone());
    drive.fail_after(2);
    assert!(b.restore_from_drive().is_err());
    assert_eq!(count_files(&b.root().join("files")), 0);
    assert!(b.backup_status().unwrap().can_restore);
}

#[test]
fn tokens_never_in_status_or_db() {
    let t = tempfile::tempdir().unwrap();
    let drive = Arc::new(FakeDrive::default());
    let mut lib = library_with_drive(&t.path().join("lib"), drive);
    lib.backup_now(now()).unwrap();
    let s = serde_json::to_string(&lib.backup_status().unwrap()).unwrap();
    assert!(!s.contains("ACCESS-SECRET") && !s.contains("REFRESH-SECRET"));
    let db = std::fs::read(lib.root().join("library.db")).unwrap();
    let text = String::from_utf8_lossy(&db);
    assert!(!text.contains("ACCESS-SECRET") && !text.contains("REFRESH-SECRET"));
}

#[test]
fn login_needs_client_config() {
    let (_t, lib, _h) = temp_library_with_mock();
    assert!(lib.begin_login().is_err());
}

#[test]
fn begin_login_builds_auth_url() {
    let (_t, mut lib, _h) = temp_library_with_mock();
    lib.set_google_client(Some("cid.apps".into()), Some("sec".into()))
        .unwrap();
    let job = lib.begin_login().unwrap();
    assert!(job.auth_url.contains("client_id=cid.apps") && job.auth_url.contains("127.0.0.1"));
    let c = lib.google_client().unwrap().unwrap();
    assert_eq!(c.client_id, "cid.apps");
    assert!(c.has_secret);
}

#[test]
fn saving_client_without_secret_keeps_old_secret() {
    let (_t, mut lib, _h) = temp_library_with_mock();
    lib.set_google_client(Some("cid".into()), Some("sec".into()))
        .unwrap();
    lib.set_google_client(Some("cid2".into()), None).unwrap();
    let c = lib.google_client().unwrap().unwrap();
    assert_eq!(c.client_id, "cid2");
    assert!(c.has_secret);
    lib.set_google_client(None, None).unwrap();
    assert!(lib.google_client().unwrap().is_none());
    assert!(!lib.backup_status().unwrap().configured);
}

// ---- 리뷰 수정 ----

/// C1: 새 PC(빈 라이브러리)는 복원 전에 빈 DB를 올리지 않고, 복원은 진짜 백업을 고른다.
#[test]
fn empty_library_never_uploads_and_restore_gets_real_backup() {
    workout_testkit::require_samples!();
    let t = tempfile::tempdir().unwrap();
    let drive = Arc::new(FakeDrive::default());
    let mut a = library_with_drive(&t.path().join("a"), drive.clone());
    a.import_paths(&[sample_path("20261025")]).unwrap();
    a.backup_now(now()).unwrap();
    let n = drive.file_count();
    let mut b = library_with_drive(&t.path().join("b"), drive.clone());
    assert!(!b.needs_backup(now()).unwrap());
    b.backup_now(now()).unwrap();
    assert_eq!(drive.file_count(), n);
    b.restore_from_drive().unwrap();
    assert_eq!(b.month_view(2026, 10).unwrap().cycling.count, 1);
}

/// C1: 드라이브에 다른 PC의 백업이 있으면 덮어쓰거나 DB를 하나 더 만들지 않고 멈춘다.
#[test]
fn backup_refuses_when_drive_has_another_backup() {
    workout_testkit::require_samples!();
    let t = tempfile::tempdir().unwrap();
    let drive = Arc::new(FakeDrive::default());
    let mut a = library_with_drive(&t.path().join("a"), drive.clone());
    a.import_paths(&[sample_path("20261025")]).unwrap();
    a.backup_now(now()).unwrap();
    let n = drive.file_count();
    let mut c = library_with_drive(&t.path().join("c"), drive.clone());
    c.import_paths(&[sample_path("20261011")]).unwrap();
    let s = c.backup_now(now()).unwrap();
    assert!(s.last_error.unwrap().contains("다른 PC"));
    assert_eq!(drive.file_count(), n);
}

/// C1: 드라이브에 library.db가 여러 개면 가장 최근 것으로 복원한다.
#[test]
fn restore_picks_newest_db() {
    workout_testkit::require_samples!();
    let t = tempfile::tempdir().unwrap();
    let drive = Arc::new(FakeDrive::default());
    let mut a = library_with_drive(&t.path().join("a"), drive.clone());
    a.import_paths(&[sample_path("20261025")]).unwrap();
    a.backup_now(now()).unwrap();
    drive.add_root_file_before("library.db", b"stale old db"); // 더 오래된 사본
    let mut b = library_with_drive(&t.path().join("b"), drive.clone());
    b.restore_from_drive().unwrap();
    assert_eq!(b.month_view(2026, 10).unwrap().cycling.count, 1);
}

/// I1: 내려받은 DB가 깨져 있으면 원래(빈) 라이브러리가 그대로 쓸 수 있게 남는다.
#[test]
fn corrupt_backup_db_leaves_library_usable() {
    workout_testkit::require_samples!();
    let t = tempfile::tempdir().unwrap();
    let drive = Arc::new(FakeDrive::default());
    let mut a = library_with_drive(&t.path().join("a"), drive.clone());
    a.import_paths(&[sample_path("20261025")]).unwrap();
    a.backup_now(now()).unwrap();
    drive.overwrite_named("library.db", b"garbage, not sqlite");
    let mut b = library_with_drive(&t.path().join("b"), drive.clone());
    assert!(b.restore_from_drive().is_err());
    assert_eq!(count_files(&b.root().join("files")), 0);
    assert!(b.backup_status().unwrap().can_restore);
    b.set_goals(Some(100.0), None).unwrap();
    drop(b);
    let reopened = library_with_drive(&t.path().join("b"), drive);
    assert_eq!(
        reopened.month_view(2026, 10).unwrap().goal_cycling_km,
        Some(100.0)
    ); // 디스크 DB에 저장됐음
}

/// I2: 업로드 뒤 기록 전에 앱이 꺼져도 다음 백업에서 중복이 생기지 않는다.
#[test]
fn crash_before_apply_does_not_duplicate() {
    workout_testkit::require_samples!();
    let t = tempfile::tempdir().unwrap();
    let drive = Arc::new(FakeDrive::default());
    let mut lib = library_with_drive(&t.path().join("lib"), drive.clone());
    lib.import_paths(&[sample_path("20261025")]).unwrap();
    let _lost = lib.prepare_backup().unwrap().run(); // 기록하지 않고 버림
    lib.backup_now(now()).unwrap();
    let paths = drive.paths();
    let mut d = paths.clone();
    d.dedup();
    assert_eq!(paths.len(), d.len(), "{paths:?}");
}

/// I3: 백업이 진행 중이면 두 번째 백업은 시작하지 않는다.
#[test]
fn second_backup_job_is_refused_while_running() {
    let t = tempfile::tempdir().unwrap();
    let drive = Arc::new(FakeDrive::default());
    let mut lib = library_with_drive(&t.path().join("lib"), drive);
    lib.set_goals(Some(1.0), None).unwrap(); // 빈 라이브러리가 아니게: 설정만으로는 비어 있음 → 파일 하나 추가
    std::fs::create_dir_all(lib.root().join("files/x")).unwrap();
    std::fs::write(lib.root().join("files/x/a.txt"), b"a").unwrap();
    lib.store_for_tests()
        .insert_photo(&workout_store::PhotoRow {
            id: 0,
            activity_id: None,
            path: "files/x/a.txt".into(),
            taken_at: None,
            km_mark: None,
            lat: None,
            lon: None,
            card_path: None,
            content_hash: None,
        })
        .unwrap();
    let first = lib.prepare_backup().unwrap();
    assert!(lib.prepare_backup().is_err());
    drop(first);
    assert!(lib.prepare_backup().is_ok());
}

/// I4: 백업 도중 로그아웃하면 백업이 끝나도 로그아웃 상태로 남는다.
#[test]
fn logout_during_backup_stays_logged_out() {
    workout_testkit::require_samples!();
    let t = tempfile::tempdir().unwrap();
    let drive = Arc::new(FakeDrive::default());
    let mut lib = library_with_drive(&t.path().join("lib"), drive);
    lib.import_paths(&[sample_path("20261025")]).unwrap();
    let job = lib.prepare_backup().unwrap();
    lib.logout().unwrap();
    let done = job.run();
    let s = lib.apply_backup(done, now()).unwrap();
    assert!(!s.signed_in);
}

/// I5: 드라이브에서 지운 파일은 다시 올리고, 로그아웃하면 백업 기록을 지운다.
#[test]
fn deleted_remote_file_is_reuploaded_and_logout_clears_sync() {
    workout_testkit::require_samples!();
    let t = tempfile::tempdir().unwrap();
    let drive = Arc::new(FakeDrive::default());
    let mut lib = library_with_drive(&t.path().join("lib"), drive.clone());
    let id = lib
        .import_paths(&[sample_path("20261025")])
        .unwrap()
        .added_activities[0]
        .clone();
    lib.backup_now(now()).unwrap();
    drive.delete_named("library.db");
    lib.save_post(&id, "바뀐 글", &[]).unwrap();
    let s = lib.backup_now(now()).unwrap();
    assert!(s.last_error.is_none(), "{:?}", s.last_error);
    assert!(drive.paths().contains(&"WorkoutApp/library.db".to_string()));
    lib.logout().unwrap();
    assert!(lib.backup_status().unwrap().pending > 0);
}
