mod common;
use std::time::Duration;

use workout_core::chrono::{Local, TimeZone};
use workout_service::bullsnote::{END_MARKER, START_MARKER, resolve_project_id, write_month_note};
use workout_service::{DayDto, DayItemDto, MonthView, PostStatus, SportKind, SportSummaryDto};

fn sample_view(cycling_goal: Option<f64>, cycling_km: f64, with_items: bool) -> MonthView {
    let sport = |count, km| SportSummaryDto {
        count,
        km,
        moving_s: 3600.0,
        avg_pace: None,
        avg_speed_kmh: None,
    };
    let days = if with_items {
        vec![DayDto {
            date: "2026-11-10".into(),
            items: vec![DayItemDto {
                activity_id: "a1".into(),
                sport: SportKind::Cycling,
                km: cycling_km,
                post_status: PostStatus::Ready,
                title: "화요일 저녁 라이딩".into(),
                start_time: "18:30".into(),
                moving_s: 3600.0,
                speed_kmh: Some(28.0),
                pace: None,
                cover_card: None,
                card_count: 1,
            }],
        }]
    } else {
        vec![]
    };
    MonthView {
        year: 2026,
        month: 11,
        goal_cycling_km: cycling_goal,
        goal_running_km: None,
        total_km: cycling_km,
        total_moving_s: 3600.0,
        running: sport(0, 0.0),
        cycling: sport(if with_items { 1 } else { 0 }, cycling_km),
        other_count: 0,
        avg_hr: None,
        weekly: vec![],
        days,
    }
}

#[test]
fn creates_new_note_with_frontmatter_and_table_values() {
    let t = tempfile::tempdir().unwrap();
    let vault = t.path().join("vault");
    std::fs::create_dir_all(&vault).unwrap();
    std::fs::write(
        vault.join(".bullsnote.json"),
        r#"{"projects":[{"id":"proj-areas","name":"02_Areas"}]}"#,
    )
    .unwrap();

    let now = Local.with_ymd_and_hms(2026, 11, 15, 10, 0, 0).unwrap();
    let view = sample_view(Some(300.0), 120.0, true);
    let wrote = write_month_note(&vault, "02_Areas", &view, now).unwrap();
    assert!(wrote);

    let note = vault.join("workout-goal-2026-11.md");
    assert!(note.exists());
    let text = std::fs::read_to_string(&note).unwrap();
    assert!(text.contains("id: \"workout-goal-2026-11\""));
    assert!(text.contains("title: \"2026년 11월 운동 목표\""));
    assert!(text.contains("projectId: \"proj-areas\""));
    assert!(text.contains("tags: [\"운동\",\"목표\"]"));
    assert!(text.contains("| 라이딩 | 300.0 | 120.0 | 40% | 180.0 |"));
    assert!(text.contains("| 러닝 | 목표 없음 | 0.0 | - | - | - | - |"));
    assert!(text.contains("화요일 저녁 라이딩"));
}

#[test]
fn updates_existing_note_preserving_user_fields_and_memo() {
    let t = tempfile::tempdir().unwrap();
    let vault = t.path().join("vault");
    std::fs::create_dir_all(&vault).unwrap();
    let note = vault.join("workout-goal-2026-11.md");
    std::fs::write(
        &note,
        format!(
            "---\nid: \"workout-goal-2026-11\"\ntitle: \"old\"\nprojectId: \"custom-proj\"\ncolor: \"green\"\npinned: true\ncreatedAt: 1700000000000\nupdatedAt: 1700000000000\n---\n\n내 메모 상단\n\n{START_MARKER}\nold block\n{END_MARKER}\n\n내 메모 하단\n"
        ),
    )
    .unwrap();

    let now = Local.with_ymd_and_hms(2026, 11, 15, 11, 0, 0).unwrap();
    let view = sample_view(Some(300.0), 150.0, true);
    let wrote = write_month_note(&vault, "02_Areas", &view, now).unwrap();
    assert!(wrote);

    let text = std::fs::read_to_string(&note).unwrap();
    assert!(text.contains("createdAt: 1700000000000"));
    assert!(text.contains("pinned: true"));
    assert!(text.contains("color: \"green\""));
    assert!(text.contains("projectId: \"custom-proj\""));
    assert!(!text.contains("updatedAt: 1700000000000"));
    assert!(text.contains("내 메모 상단"));
    assert!(text.contains("내 메모 하단"));
    assert!(text.contains("50%"));
}

#[test]
fn prepends_managed_block_when_markers_missing() {
    let t = tempfile::tempdir().unwrap();
    let vault = t.path().join("vault");
    std::fs::create_dir_all(&vault).unwrap();
    let note = vault.join("workout-goal-2026-11.md");
    std::fs::write(
        &note,
        "---\nid: \"workout-goal-2026-11\"\ncreatedAt: 123\n---\n\n직접 쓴 글\n",
    )
    .unwrap();

    let now = Local.with_ymd_and_hms(2026, 11, 15, 12, 0, 0).unwrap();
    let view = sample_view(None, 0.0, false);
    write_month_note(&vault, "02_Areas", &view, now).unwrap();

    let text = std::fs::read_to_string(&note).unwrap();
    assert!(text.contains(START_MARKER));
    assert!(text.contains("아직 기록된 운동이 없어요."));
    assert!(text.contains("직접 쓴 글"));
    assert!(text.find(END_MARKER).unwrap() < text.find("직접 쓴 글").unwrap());
}

#[test]
fn skips_write_when_content_unchanged() {
    let t = tempfile::tempdir().unwrap();
    let vault = t.path().join("vault");
    std::fs::create_dir_all(&vault).unwrap();
    let view = sample_view(Some(300.0), 120.0, true);
    let t1 = Local.with_ymd_and_hms(2026, 11, 15, 10, 0, 0).unwrap();
    assert!(write_month_note(&vault, "02_Areas", &view, t1).unwrap());

    let note = vault.join("workout-goal-2026-11.md");
    let mtime1 = std::fs::metadata(&note).unwrap().modified().unwrap();
    std::thread::sleep(Duration::from_millis(20));

    let t2 = Local.with_ymd_and_hms(2026, 11, 15, 10, 5, 0).unwrap();
    assert!(!write_month_note(&vault, "02_Areas", &view, t2).unwrap());
    let mtime2 = std::fs::metadata(&note).unwrap().modified().unwrap();
    assert_eq!(mtime1, mtime2);
}

#[test]
fn resolves_folder_and_falls_back() {
    let t = tempfile::tempdir().unwrap();
    let vault = t.path().join("vault");
    std::fs::create_dir_all(&vault).unwrap();
    std::fs::write(
        vault.join(".bullsnote.json"),
        r#"{"projects":[{"id":"proj-1","name":"운동"},{"id":"proj-ai-custom","name":"AI 노트"}]}"#,
    )
    .unwrap();
    assert_eq!(resolve_project_id(&vault, "운동"), "proj-1");
    assert_eq!(resolve_project_id(&vault, "없는폴더"), "proj-ai-custom");
}

#[test]
fn missing_vault_errors_sync_but_set_goals_succeeds() {
    let (t, mut lib) = common::temp_library();
    let missing = t.path().join("does-not-exist");
    lib.set_bullsnote_settings(
        true,
        Some(missing.to_string_lossy().into_owned()),
        Some("02_Areas".into()),
    )
    .unwrap();

    // set_goals must succeed even though BullsNote vault doesn't exist
    lib.set_goals(Some(300.0), Some(50.0)).unwrap();
    let st = lib.bullsnote_status().unwrap();
    assert!(st.last_error.is_some());

    // Direct sync returns Err
    assert!(lib.sync_bullsnote_month(2026, 11).is_err());
}
