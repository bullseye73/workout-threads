//! BullsNote 볼트에 월별 운동 목표와 달성 현황 노트를 기록한다.
use std::path::{Path, PathBuf};

use serde::Deserialize;
use workout_core::chrono::{DateTime, Datelike, Local, NaiveDate, Utc};

use crate::{BullsNoteStatusDto, Library, MonthView, Result, ServiceError, SportKind};

pub const BULLSNOTE_ENABLED_KEY: &str = "bullsnote_enabled";
pub const BULLSNOTE_VAULT_KEY: &str = "bullsnote_vault_path";
pub const BULLSNOTE_FOLDER_KEY: &str = "bullsnote_folder";
const BULLSNOTE_LAST_OK_KEY: &str = "bullsnote_last_ok";
const BULLSNOTE_LAST_ERR_KEY: &str = "bullsnote_last_err";

const DEFAULT_FOLDER: &str = "02_Areas";
const FALLBACK_FOLDER: &str = "AI 노트";
const FALLBACK_PROJECT_ID: &str = "proj-ai-notes";

pub const START_MARKER: &str = "<!-- workout-app:start -->";
pub const END_MARKER: &str = "<!-- workout-app:end -->";

pub fn default_vault_path() -> PathBuf {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join("Documents").join("BullsNote")
}

#[derive(Deserialize)]
struct Sidecar {
    #[serde(default)]
    projects: Vec<SidecarProject>,
}

#[derive(Deserialize)]
struct SidecarProject {
    id: String,
    name: String,
}

/// `.bullsnote.json`에서 폴더 이름으로 projectId를 찾고, 없으면 대체 폴더("AI 노트") -> "proj-ai-notes"로 간다.
pub fn resolve_project_id(vault: &Path, folder_name: &str) -> String {
    let sidecar_path = vault.join(".bullsnote.json");
    let Ok(raw) = std::fs::read_to_string(sidecar_path) else {
        return FALLBACK_PROJECT_ID.to_string();
    };
    let Ok(sidecar) = serde_json::from_str::<Sidecar>(&raw) else {
        return FALLBACK_PROJECT_ID.to_string();
    };
    if let Some(p) = sidecar
        .projects
        .iter()
        .find(|p| p.name == folder_name || p.id == folder_name)
    {
        return p.id.clone();
    }
    if let Some(p) = sidecar
        .projects
        .iter()
        .find(|p| p.name == FALLBACK_FOLDER || p.id == FALLBACK_PROJECT_ID)
    {
        return p.id.clone();
    }
    FALLBACK_PROJECT_ID.to_string()
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let (ny, nm) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    NaiveDate::from_ymd_opt(ny, nm, 1)
        .and_then(|d| d.pred_opt())
        .map(|d| d.day())
        .unwrap_or(30)
}

fn remaining_days(year: i32, month: u32, today: NaiveDate) -> u32 {
    let total = days_in_month(year, month);
    if today.year() == year && today.month() == month {
        total.saturating_sub(today.day()) + 1
    } else if (today.year(), today.month()) < (year, month) {
        total
    } else {
        0
    }
}

fn sport_label(s: SportKind) -> &'static str {
    match s {
        SportKind::Running => "러닝",
        SportKind::Cycling => "라이딩",
        SportKind::Walking => "걷기",
        SportKind::Hiking => "등산",
        SportKind::Other => "기타",
    }
}

fn format_goal_row(name: &str, goal: Option<f64>, done_km: f64, rem_days: u32) -> String {
    match goal {
        Some(g) if g > 0.0 => {
            let pct = ((done_km / g) * 100.0).round() as i64;
            let rem_km = (g - done_km).max(0.0);
            let daily = if rem_days > 0 && rem_km > 0.0 {
                format!("{:.1}", rem_km / rem_days as f64)
            } else if rem_km <= 0.0 {
                "0.0".to_string()
            } else {
                "-".to_string()
            };
            let rem_days_str = if rem_days > 0 {
                format!("{rem_days}일")
            } else {
                "0일".to_string()
            };
            format!(
                "| {name} | {g:.1} | {done_km:.1} | {pct}% | {rem_km:.1} | {rem_days_str} | {daily} |"
            )
        }
        _ => format!("| {name} | 목표 없음 | {done_km:.1} | - | - | - | - |"),
    }
}

pub fn render_managed_block(view: &MonthView, now: DateTime<Local>) -> String {
    let rem = remaining_days(view.year, view.month, now.date_naive());
    let mut lines = vec![
        START_MARKER.to_string(),
        format!("## {}년 {}월 운동 목표", view.year, view.month),
        String::new(),
        "| 종목 | 목표 km | 달성 km | 달성률 | 남은 km | 남은 날 | 하루 필요 km |".to_string(),
        "|---|---:|---:|---:|---:|---:|---:|".to_string(),
        format_goal_row("라이딩", view.goal_cycling_km, view.cycling.km, rem),
        format_goal_row("러닝", view.goal_running_km, view.running.km, rem),
        String::new(),
        "### 이번 달 운동 목록".to_string(),
        String::new(),
    ];

    let mut has_any = false;
    for day in &view.days {
        for item in &day.items {
            has_any = true;
            lines.push(format!(
                "- {} · {} · {} · {:.1} km",
                day.date,
                sport_label(item.sport),
                item.title,
                item.km
            ));
        }
    }
    if !has_any {
        lines.push("- 아직 기록된 운동이 없어요.".to_string());
    }

    lines.push(String::new());
    lines.push(format!("마지막 갱신: {}", now.format("%Y-%m-%d %H:%M")));
    lines.push(END_MARKER.to_string());
    lines.join("\n")
}

fn strip_timestamp_line(block: &str) -> String {
    block
        .lines()
        .filter(|l| !l.starts_with("마지막 갱신:"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_frontmatter(raw: &str) -> (Vec<(String, String)>, String) {
    let normalized = raw.replace("\r\n", "\n");
    if let Some(rest) = normalized.strip_prefix("---\n")
        && let Some(end_idx) = rest.find("\n---")
    {
        let fm_block = &rest[..end_idx];
        let after = &rest[end_idx + 4..];
        let body = after.strip_prefix('\n').unwrap_or(after).to_string();
        let mut pairs = Vec::new();
        for line in fm_block.lines() {
            if let Some((k, v)) = line.split_once(':') {
                pairs.push((k.trim().to_string(), v.trim().to_string()));
            }
        }
        return (pairs, body);
    }
    (Vec::new(), normalized)
}

fn upsert_fm(pairs: &mut Vec<(String, String)>, key: &str, val: String, only_if_missing: bool) {
    if let Some((_, existing)) = pairs.iter_mut().find(|(k, _)| k == key) {
        if !only_if_missing {
            *existing = val;
        }
    } else {
        pairs.push((key.to_string(), val));
    }
}

fn atomic_write(path: &Path, content: &str) -> Result<()> {
    let pid = std::process::id();
    let nanos = Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("note.md");
    let tmp = path.with_file_name(format!("{file_name}.{pid}.{nanos}.tmp"));
    std::fs::write(&tmp, content.as_bytes())?;
    if let Err(e) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(ServiceError::Io(e));
    }
    Ok(())
}

/// `MonthView`를 받아 볼트에 `workout-goal-YYYY-MM.md`를 만들거나 갱신한다.
/// 실제 파일을 썼으면 `true`, 내용이 같아 건너뛰었으면 `false`를 돌려준다.
pub fn write_month_note(
    vault: &Path,
    folder_name: &str,
    view: &MonthView,
    now: DateTime<Local>,
) -> Result<bool> {
    if !vault.exists() {
        return Err(ServiceError::Failed(format!(
            "BullsNote 볼트 폴더가 없어요: {}",
            vault.display()
        )));
    }
    let note_id = format!("workout-goal-{:04}-{:02}", view.year, view.month);
    let title = format!("{}년 {}월 운동 목표", view.year, view.month);
    let project_id = resolve_project_id(vault, folder_name);
    let note_path = vault.join(format!("{note_id}.md"));
    let now_ms = now.timestamp_millis();

    let new_block = render_managed_block(view, now);
    let (mut fm, body) = if note_path.exists() {
        let existing = std::fs::read_to_string(&note_path)?;
        parse_frontmatter(&existing)
    } else {
        (Vec::new(), String::new())
    };

    let new_body = if let (Some(s_idx), Some(e_idx)) =
        (body.find(START_MARKER), body.find(END_MARKER))
        && s_idx < e_idx
    {
        let end_full = e_idx + END_MARKER.len();
        let old_block = &body[s_idx..end_full];
        if strip_timestamp_line(old_block) == strip_timestamp_line(&new_block) {
            return Ok(false);
        }
        format!("{}{}{}", &body[..s_idx], new_block, &body[end_full..])
    } else if body.trim().is_empty() {
        new_block
    } else {
        format!("{new_block}\n\n{}", body.trim_start())
    };

    upsert_fm(&mut fm, "id", format!("\"{note_id}\""), false);
    upsert_fm(
        &mut fm,
        "title",
        serde_json::to_string(&title).unwrap_or_else(|_| format!("\"{title}\"")),
        false,
    );
    upsert_fm(&mut fm, "projectId", format!("\"{project_id}\""), true);
    upsert_fm(&mut fm, "color", "\"blue\"".to_string(), true);
    upsert_fm(
        &mut fm,
        "tags",
        serde_json::to_string(&["운동", "목표"]).unwrap(),
        true,
    );
    upsert_fm(&mut fm, "createdAt", now_ms.to_string(), true);
    upsert_fm(&mut fm, "updatedAt", now_ms.to_string(), false);

    let mut out = String::from("---\n");
    for (k, v) in fm {
        out.push_str(&format!("{k}: {v}\n"));
    }
    out.push_str("---\n\n");
    out.push_str(new_body.trim_end());
    out.push('\n');

    atomic_write(&note_path, &out)?;
    Ok(true)
}

impl Library {
    pub fn bullsnote_status(&self) -> Result<BullsNoteStatusDto> {
        let enabled = self
            .store
            .setting(BULLSNOTE_ENABLED_KEY)?
            .is_some_and(|v| v == "1");
        let vault_path = self
            .store
            .setting(BULLSNOTE_VAULT_KEY)?
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| default_vault_path().to_string_lossy().into_owned());
        let folder_name = self
            .store
            .setting(BULLSNOTE_FOLDER_KEY)?
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_FOLDER.to_string());
        Ok(BullsNoteStatusDto {
            enabled,
            vault_path,
            folder_name,
            last_success: self.store.setting(BULLSNOTE_LAST_OK_KEY)?,
            last_error: self.store.setting(BULLSNOTE_LAST_ERR_KEY)?,
        })
    }

    pub fn set_bullsnote_settings(
        &mut self,
        enabled: bool,
        vault_path: Option<String>,
        folder_name: Option<String>,
    ) -> Result<BullsNoteStatusDto> {
        self.store
            .set_setting(BULLSNOTE_ENABLED_KEY, enabled.then_some("1"))?;
        let vp = vault_path
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        self.store.set_setting(BULLSNOTE_VAULT_KEY, vp.as_deref())?;
        let fn_ = folder_name
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        self.store
            .set_setting(BULLSNOTE_FOLDER_KEY, fn_.as_deref())?;
        self.bullsnote_status()
    }

    /// BullsNote에 해당 연·월 목표/현황 노트를 반영한다.
    pub fn sync_bullsnote_month(&mut self, year: i32, month: u32) -> Result<BullsNoteStatusDto> {
        let status = self.bullsnote_status()?;
        let view = self.month_view(year, month)?;
        let now = Local::now();
        match write_month_note(
            Path::new(&status.vault_path),
            &status.folder_name,
            &view,
            now,
        ) {
            Ok(_) => {
                self.store
                    .set_setting(BULLSNOTE_LAST_OK_KEY, Some(&Utc::now().to_rfc3339()))?;
                self.store.set_setting(BULLSNOTE_LAST_ERR_KEY, None)?;
            }
            Err(e) => {
                let msg = e.to_string();
                let _ = self.store.set_setting(BULLSNOTE_LAST_ERR_KEY, Some(&msg));
                return Err(e);
            }
        }
        self.bullsnote_status()
    }

    /// 설정이 켜져 있을 때만 조용히 반영한다(실패해도 본래 작업을 실패시키지 않음).
    pub(crate) fn maybe_sync_bullsnote_month(&mut self, year: i32, month: u32) {
        if self
            .store
            .setting(BULLSNOTE_ENABLED_KEY)
            .ok()
            .flatten()
            .is_some_and(|v| v == "1")
        {
            let _ = self.sync_bullsnote_month(year, month);
        }
    }
}
