//! Garmin Connect Personal API 연동: 키 관리, 활동 목록, FIT 받기, 새 활동 자동 확인.
use std::path::PathBuf;
use std::sync::Arc;

use workout_core::chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, Utc};
use workout_garmin::{GarminActivity, GarminClient, GarminError, KeyTokenProvider};

use crate::{
    GarminActivityDto, GarminFetchResult, ImportIssue, Library, Result, ServiceError, SportKind,
    secrets,
};

pub const GARMIN_AUTO_CHECK_KEY: &str = "garmin_auto_check";
pub const GARMIN_LAST_CHECK_KEY: &str = "garmin_last_check";

impl From<GarminError> for ServiceError {
    fn from(e: GarminError) -> Self {
        ServiceError::Failed(e.to_string())
    }
}

pub(crate) fn sport_from_garmin(key: &str) -> SportKind {
    let k = key.to_lowercase();
    if k.contains("run") {
        SportKind::Running
    } else if k.contains("cycl") || k.contains("bik") || k.contains("ride") {
        SportKind::Cycling
    } else if k.contains("walk") {
        SportKind::Walking
    } else if k.contains("hik") {
        SportKind::Hiking
    } else {
        SportKind::Other
    }
}

fn parse_garmin_utc(s: &str) -> Option<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }
    if let Ok(ndt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Some(ndt.and_utc());
    }
    None
}

fn sanitize_id(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn local_date_and_time(a: &GarminActivity) -> (String, String) {
    let src = if a.start_time_local.is_empty() {
        &a.start_time_gmt
    } else {
        &a.start_time_local
    };
    let date = src.get(0..10).unwrap_or("").to_string();
    let time = src.get(11..16).unwrap_or("").to_string();
    (date, time)
}

impl Library {
    pub fn garmin_inbox_dir(&self) -> PathBuf {
        self.root.join("garmin-inbox")
    }

    fn garmin_client(&self) -> Result<GarminClient> {
        if !self.has_garmin_key() {
            return Err(ServiceError::Failed(
                "설정에서 Garmin API 키를 먼저 넣어 주세요".into(),
            ));
        }
        let s_key = self.secrets.clone();
        let s_read = self.secrets.clone();
        let s_write = self.secrets.clone();
        let tokens = Arc::new(KeyTokenProvider::new(
            Arc::new(move || s_key.get(secrets::GARMIN_KEY)),
            Arc::new(move || s_read.get(secrets::GARMIN_ACCESS)),
            Arc::new(move |v| {
                let _ = s_write.set(secrets::GARMIN_ACCESS, v.as_deref());
            }),
        ));
        Ok(GarminClient::new(self.http.clone(), tokens))
    }

    pub fn has_garmin_key(&self) -> bool {
        self.secrets.get(secrets::GARMIN_KEY).is_some()
    }

    pub fn set_garmin_key(&mut self, key: Option<String>) -> Result<()> {
        let cleaned = key.map(|k| k.trim().to_string()).filter(|k| !k.is_empty());
        self.secrets.set(secrets::GARMIN_KEY, cleaned.as_deref())?;
        self.secrets.set(secrets::GARMIN_ACCESS, None)?;
        Ok(())
    }

    pub fn garmin_auto_check(&self) -> bool {
        self.store
            .setting(GARMIN_AUTO_CHECK_KEY)
            .ok()
            .flatten()
            .is_some_and(|v| v == "1")
    }

    pub fn set_garmin_auto_check(&mut self, on: bool) -> Result<()> {
        self.store
            .set_setting(GARMIN_AUTO_CHECK_KEY, on.then_some("1"))?;
        Ok(())
    }

    /// 최근 7일 Garmin 활동 수를 확인한다(연결 테스트용).
    pub fn test_garmin(&self) -> Result<u32> {
        let today = Utc::now().date_naive();
        let from = today - Duration::days(6);
        let client = self.garmin_client()?;
        let list = client.list_activities(from, today)?;
        Ok(list.len() as u32)
    }

    fn is_already_imported(&self, a: &GarminActivity) -> Result<bool> {
        let Some(utc) = parse_garmin_utc(&a.start_time_gmt) else {
            return Ok(false);
        };
        let sport = sport_from_garmin(&a.sport_type_key);
        let near = self.store.activities_near(utc, 1)?;
        Ok(near.into_iter().any(|row| {
            let row_sport: SportKind = row.header.sport.into();
            let diff = (row.header.start - utc).num_seconds().abs();
            row_sport == sport && diff <= 120
        }))
    }

    pub fn garmin_activities(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<GarminActivityDto>> {
        let client = self.garmin_client()?;
        let raw = client.list_activities(from, to)?;
        let mut out = Vec::with_capacity(raw.len());
        for a in raw {
            let (date, start_time) = local_date_and_time(&a);
            let already_imported = self.is_already_imported(&a)?;
            out.push(GarminActivityDto {
                id: a.id,
                name: a.name,
                sport: sport_from_garmin(&a.sport_type_key),
                date,
                start_time,
                km: a.distance_m / 1000.0,
                duration_s: a.duration_s,
                already_imported,
            });
        }
        Ok(out)
    }

    pub fn fetch_garmin_fits(&self, ids: Vec<String>) -> Result<GarminFetchResult> {
        let inbox = self.garmin_inbox_dir();
        std::fs::create_dir_all(&inbox)?;
        let mut res = GarminFetchResult::default();
        let mut client: Option<GarminClient> = None;

        for id in ids {
            let safe = sanitize_id(&id);
            let target = inbox.join(format!("{safe}.fit"));
            if target.exists() && target.metadata().map(|m| m.len() > 0).unwrap_or(false) {
                res.paths.push(target.to_string_lossy().into_owned());
                continue;
            }
            if client.is_none() {
                client = Some(self.garmin_client()?);
            }
            match client
                .as_ref()
                .expect("client initialized")
                .download_fit(&id)
            {
                Ok(bytes) => {
                    if let Err(e) = std::fs::write(&target, bytes) {
                        res.failed.push(ImportIssue {
                            file: id,
                            reason: e.to_string(),
                        });
                    } else {
                        res.paths.push(target.to_string_lossy().into_owned());
                    }
                }
                Err(e) => {
                    res.failed.push(ImportIssue {
                        file: id,
                        reason: e.to_string(),
                    });
                }
            }
        }
        Ok(res)
    }

    pub fn garmin_new_since_last_check(&mut self) -> Result<GarminFetchResult> {
        let now = Utc::now();
        let since = self
            .store
            .setting(GARMIN_LAST_CHECK_KEY)?
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(|| now - Duration::days(3));

        let client = self.garmin_client()?;
        let list = client.list_activities(since.date_naive(), now.date_naive())?;
        let mut ids = Vec::new();
        for a in list {
            let after_since = parse_garmin_utc(&a.start_time_gmt)
                .map(|dt| dt >= since)
                .unwrap_or(true);
            if after_since && !self.is_already_imported(&a)? {
                ids.push(a.id);
            }
        }
        let fetched = self.fetch_garmin_fits(ids)?;
        if fetched.failed.is_empty() {
            self.store
                .set_setting(GARMIN_LAST_CHECK_KEY, Some(&now.to_rfc3339()))?;
        }
        Ok(fetched)
    }
}
