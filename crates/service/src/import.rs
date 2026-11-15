//! FIT·사진 가져오기. 원본은 라이브러리로 복사하고 원래 폴더에는 쓰지 않는다.
use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use workout_card::{card_data_with, render_card_to_file};
use workout_core::chrono::{DateTime, Local, NaiveDate, Offset, TimeZone, Utc};
use workout_core::fit::parse_fit;
use workout_core::matching::{assign_photo_with_fallback, match_photo};
use workout_core::model::{Activity, ActivityHeader};
use workout_core::photo::{is_photo_input, read_photo_meta};
use workout_core::route::simplify_route;
use workout_post::{DraftWriter, PostContext, TemplateWriter};
use workout_store::{ActivityRow, PhotoRow, PostRow};

use crate::files::{content_hash, copy_new, join_rel, unique_rel};
use crate::{ImportIssue, ImportReport, Library, Result, ServiceError};

/// 경로 화면용 최대 점 수 (설계 §6.7).
const ROUTE_MAX_POINTS: usize = 1500;
/// 사진만 따로 가져올 때 기존 운동을 찾는 범위.
pub(crate) const NEAR_HOURS: i64 = 36;

pub(crate) fn file_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn has_ext(p: &Path, ext: &str) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(ext))
}

/// 폴더 안을 하위 3단계까지 찾는다 (이름이 `.`으로 시작하는 폴더는 건너뜀).
const MAX_DEPTH: usize = 3;

fn walk(dir: &Path, depth: usize, out: &mut Vec<std::path::PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
    entries.sort();
    for p in entries {
        if p.is_file() {
            out.push(p);
        } else if p.is_dir() && depth < MAX_DEPTH {
            let hidden = p
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with('.'));
            if !hidden {
                walk(&p, depth + 1, out);
            }
        }
    }
}

/// 폴더는 그 안 파일로 펼치고, FIT과 사진만 남긴다. `recursive`면 하위 폴더도(게시물 작성), 아니면 그 폴더만(예전 가져오기).
pub(crate) fn expand(
    paths: &[std::path::PathBuf],
    recursive: bool,
) -> (Vec<std::path::PathBuf>, Vec<std::path::PathBuf>) {
    let mut files = Vec::new();
    for p in paths {
        if p.is_dir() {
            walk(p, if recursive { 0 } else { MAX_DEPTH }, &mut files);
        } else {
            files.push(p.clone());
        }
    }
    let fits = files
        .iter()
        .filter(|p| has_ext(p, "fit"))
        .cloned()
        .collect();
    let photos = files
        .iter()
        .filter(|p| is_photo_input(p))
        .cloned()
        .collect();
    (fits, photos)
}

/// "10월 11일" (사유 문구용).
fn md(d: NaiveDate) -> String {
    use workout_core::chrono::Datelike;
    format!("{}월 {}일", d.month(), d.day())
}

pub(crate) fn day_dir(d: NaiveDate) -> String {
    format!("files/{}", d.format("%Y%m%d"))
}

pub(crate) fn local_offset_at(t: DateTime<Utc>) -> i32 {
    Local
        .offset_from_utc_datetime(&t.naive_utc())
        .fix()
        .local_minus_utc()
}

pub(crate) fn local_date(t: DateTime<Utc>, offset_s: i32) -> NaiveDate {
    (t + workout_core::chrono::Duration::seconds(offset_s as i64)).date_naive()
}

/// 한 번의 가져오기 동안 다시 읽은 운동을 기억한다.
type Cache = HashMap<String, Activity>;

impl Library {
    pub fn import_paths(&mut self, paths: &[std::path::PathBuf]) -> Result<ImportReport> {
        let (fits, photos) = expand(paths, false);
        let mut report = ImportReport::default();
        self.import_files(&fits, &photos, None, &mut report)?;
        Ok(report)
    }

    /// 고른 날짜의 FIT(시작일 기준)와 그 날짜에 찍은 사진만 넣는다. 나머지는 사유와 함께 `skipped`.
    /// 끝난 뒤 그날 운동이 하나뿐이면, 시간이 안 맞아 남은 그날 사진을 그 운동에 붙인다.
    pub fn import_for_date(
        &mut self,
        date: NaiveDate,
        paths: &[std::path::PathBuf],
    ) -> Result<ImportReport> {
        let (fits, photos) = expand(paths, false);
        let mut report = ImportReport::default();
        let mut keep_fits = Vec::new();
        let mut offset = None;
        for fit in fits {
            // 읽지 못하는 FIT는 그대로 보내 기존 흐름이 실패로 보고하게 한다.
            match std::fs::read(&fit).ok().and_then(|b| parse_fit(&b).ok()) {
                Some(a) if a.header.start_local().date() != date => {
                    report.skipped.push(ImportIssue {
                        file: file_name(&fit),
                        reason: format!("{} 기록이에요", md(a.header.start_local().date())),
                    })
                }
                Some(a) => {
                    offset.get_or_insert(a.header.utc_offset_s);
                    keep_fits.push(fit);
                }
                None => keep_fits.push(fit),
            }
        }
        let day_acts = self.activities_on(date)?;
        let offset = offset
            .or_else(|| day_acts.first().map(|r| r.header.utc_offset_s))
            .unwrap_or_else(|| local_offset_at(Utc::now()));
        let mut keep_photos = Vec::new();
        for photo in photos {
            match read_photo_meta(&photo, offset)
                .ok()
                .and_then(|m| m.taken_at)
            {
                None => report.skipped.push(ImportIssue {
                    file: file_name(&photo),
                    reason: "촬영 날짜가 없어요".into(),
                }),
                Some(t) if local_date(t, offset) != date => report.skipped.push(ImportIssue {
                    file: file_name(&photo),
                    reason: format!("{}에 찍었어요", md(local_date(t, offset))),
                }),
                Some(_) => keep_photos.push(photo),
            }
        }
        // 날짜 검사와 같은 기준 시간대로 사진 시각을 저장한다 (FIT가 중복이어도).
        self.import_files(&keep_fits, &keep_photos, Some(offset), &mut report)?;
        // 파일은 이미 들어갔으니, 붙이기에서 생긴 문제는 보고서에 담고 보고서를 돌려준다.
        if let Err(e) = self.attach_day_photos(date, offset, &mut report) {
            report.failed.push(ImportIssue {
                file: md(date),
                reason: format!("그날 운동에 사진을 붙이지 못했어요: {e}"),
            });
        }
        Ok(report)
    }

    /// 그날 운동이 정확히 하나면, 이번에 안 붙은 그날 사진을 그 운동에 붙인다 (km 지점은 모름).
    fn attach_day_photos(
        &mut self,
        date: NaiveDate,
        offset: i32,
        report: &mut ImportReport,
    ) -> Result<()> {
        let day_acts = self.activities_on(date)?;
        let [only] = day_acts.as_slice() else {
            return Ok(());
        };
        let a = self.load_activity(&only.header.id)?;
        let mut attached = false;
        let mut left = Vec::new();
        for dto in std::mem::take(&mut report.unmatched_photos) {
            let same_day = self
                .store
                .photo(dto.id)?
                .filter(|p| p.taken_at.is_some_and(|t| local_date(t, offset) == date));
            match same_day {
                Some(row) => match self.attach_photo(row, &a, None) {
                    Ok(_) => {
                        report.cards_created += 1;
                        attached = true;
                    }
                    Err(e) => {
                        report.failed.push(ImportIssue {
                            file: dto.file_name.clone(),
                            reason: e.to_string(),
                        });
                        left.push(dto);
                    }
                },
                None => left.push(dto),
            }
        }
        report.unmatched_photos = left;
        if attached {
            self.write_template_post(&a.header.id)?;
        }
        Ok(())
    }

    fn activities_on(&self, date: NaiveDate) -> Result<Vec<ActivityRow>> {
        use workout_core::chrono::Datelike;
        Ok(self
            .store
            .activities_in_month(date.year(), date.month())?
            .into_iter()
            .filter(|r| r.header.start_local().date() == date)
            .collect())
    }

    /// 가져오기 본체: FIT → 남아 있던 사진 다시 맞추기 → 사진 → 글.
    fn import_files(
        &mut self,
        fits: &[std::path::PathBuf],
        photos: &[std::path::PathBuf],
        offset_hint: Option<i32>,
        report: &mut ImportReport,
    ) -> Result<()> {
        let mut cache: Cache = HashMap::new();
        let mut touched: BTreeSet<String> = BTreeSet::new();
        let mut batch: Vec<String> = Vec::new();
        for fit in fits {
            match self.import_fit(fit) {
                Ok(Some(a)) => {
                    report.added_activities.push(a.header.id.clone());
                    touched.insert(a.header.id.clone());
                    batch.push(a.header.id.clone());
                    cache.insert(a.header.id.clone(), a);
                }
                Ok(None) => report.duplicate_files.push(file_name(fit)),
                Err(e) => report.failed.push(ImportIssue {
                    file: file_name(fit),
                    reason: e.to_string(),
                }),
            }
        }
        // 먼저 들어와 있던 매칭 안 된 사진을 새 운동에 다시 붙여 본다.
        let new_acts: Vec<Activity> = batch
            .iter()
            .filter_map(|id| cache.get(id).cloned())
            .collect();
        if !new_acts.is_empty() {
            self.rematch_unmatched(&new_acts, report, &mut touched)?;
        }
        let batch_offset = new_acts
            .first()
            .map(|a| a.header.utc_offset_s)
            .or(offset_hint);
        for photo in photos {
            if let Err(e) = self.import_photo(
                photo,
                &new_acts,
                batch_offset,
                &mut cache,
                report,
                &mut touched,
            ) {
                report.failed.push(ImportIssue {
                    file: file_name(photo),
                    reason: e.to_string(),
                });
            }
        }
        for id in touched {
            if let Err(e) = self.write_template_post(&id) {
                report.failed.push(ImportIssue {
                    file: id,
                    reason: format!("글 작성 실패: {e}"),
                });
            }
        }
        Ok(())
    }

    /// 새 운동이면 복사·저장 후 돌려주고, 이미 있으면 None.
    fn import_fit(&mut self, fit: &Path) -> Result<Option<Activity>> {
        let bytes = std::fs::read(fit)?;
        let a = parse_fit(&bytes)?;
        if self.store.has_activity(&a.header.id)? {
            return Ok(None);
        }
        let rel = unique_rel(
            &self.root,
            &day_dir(a.header.start_local().date()),
            &file_name(fit),
        );
        copy_new(&bytes, &join_rel(&self.root, &rel))?;
        self.store.insert_activity(&ActivityRow {
            header: a.header.clone(),
            fit_path: rel,
            route: simplify_route(&a.samples, ROUTE_MAX_POINTS),
        })?;
        Ok(Some(a))
    }

    fn cached(&self, id: &str, cache: &mut Cache) -> Result<Activity> {
        if let Some(a) = cache.get(id) {
            return Ok(a.clone());
        }
        let a = self.load_activity(id)?;
        cache.insert(id.to_string(), a.clone());
        Ok(a)
    }

    fn import_photo(
        &mut self,
        photo: &Path,
        batch: &[Activity],
        batch_offset: Option<i32>,
        cache: &mut Cache,
        report: &mut ImportReport,
        touched: &mut BTreeSet<String>,
    ) -> Result<()> {
        let bytes = std::fs::read(photo)?;
        let hash = content_hash(&bytes);
        if self.store.photo_by_hash(&hash)?.is_some() {
            report.duplicate_files.push(file_name(photo));
            return Ok(());
        }
        // 시각을 대략 읽어(묶음 운동 오프셋, 없으면 이 PC 시간대) 근처 기존 운동까지 후보로 모은다.
        let rough = read_photo_meta(
            photo,
            batch_offset.unwrap_or_else(|| local_offset_at(Utc::now())),
        )?;
        let guess_offset =
            batch_offset.unwrap_or_else(|| rough.taken_at.map(local_offset_at).unwrap_or(0));
        let meta = read_photo_meta(photo, guess_offset)?;
        let mut candidates: Vec<Activity> = batch.to_vec();
        if let Some(t) = meta.taken_at {
            for row in self.store.activities_near(t, NEAR_HOURS)? {
                if !candidates.iter().any(|c| c.header.id == row.header.id) {
                    candidates.push(self.cached(&row.header.id, cache)?);
                }
            }
        }
        let fallback = candidates
            .first()
            .map(|a| a.header.utc_offset_s)
            .unwrap_or(guess_offset);
        let asg = assign_photo_with_fallback(&candidates, photo, fallback);
        let matched = asg.matched.and_then(|m| {
            candidates
                .iter()
                .find(|a| a.header.id == m.activity_id)
                .map(|a| (a, m))
        });
        let taken_at = if matched.is_some() {
            asg.taken_at
        } else {
            meta.taken_at
        };

        let dir = match (&matched, taken_at) {
            (Some((a, _)), _) => day_dir(a.header.start_local().date()),
            (None, Some(t)) => day_dir(local_date(t, guess_offset)),
            (None, None) => batch
                .first()
                .map(|a| day_dir(a.header.start_local().date()))
                .unwrap_or("files/unsorted".into()),
        };
        let rel = unique_rel(&self.root, &dir, &file_name(photo));
        copy_new(&bytes, &join_rel(&self.root, &rel))?;
        let mut row = PhotoRow {
            id: 0,
            activity_id: None,
            path: rel.clone(),
            taken_at,
            km_mark: None,
            lat: meta.pos.map(|p| p.lat),
            lon: meta.pos.map(|p| p.lon),
            card_path: None,
            content_hash: Some(hash),
        };
        if let Some((a, m)) = &matched {
            row.activity_id = Some(a.header.id.clone());
            row.km_mark = Some(m.km_mark);
            row.card_path = self.make_card(a, &rel, report);
            touched.insert(a.header.id.clone());
        }
        row.id = self.store.insert_photo(&row)?;
        if matched.is_none() {
            report.unmatched_photos.push(self.photo_dto(&row));
        }
        Ok(())
    }

    /// 라이브러리에 남아 있던 매칭 안 된 사진을 새 운동에 맞춰 본다.
    fn rematch_unmatched(
        &mut self,
        new_acts: &[Activity],
        report: &mut ImportReport,
        touched: &mut BTreeSet<String>,
    ) -> Result<()> {
        for photo in self.store.unmatched_photos()? {
            let abs = join_rel(&self.root, &photo.path);
            let Ok(meta) = read_photo_meta(&abs, new_acts[0].header.utc_offset_s) else {
                continue;
            };
            let Some(m) = meta.taken_at.and_then(|t| match_photo(new_acts, t)) else {
                continue;
            };
            let Some(a) = new_acts.iter().find(|a| a.header.id == m.activity_id) else {
                continue;
            };
            let mut row = photo.clone();
            row.taken_at = meta.taken_at;
            match self.attach_photo(row, a, Some(m.km_mark)) {
                Ok(_) => {
                    report.cards_created += 1;
                    touched.insert(a.header.id.clone());
                }
                Err(e) => report.failed.push(ImportIssue {
                    file: photo.path.clone(),
                    reason: e.to_string(),
                }),
            }
        }
        Ok(())
    }

    /// 사진을 운동에 붙인다: 운동 폴더로 안전하게 옮기고(덮어쓰지 않음) 카드를 만든 뒤 DB를 고친다.
    /// 중간에 실패하면 새로 만든 파일을 지우고 원래 상태를 유지한다.
    pub(crate) fn attach_photo(
        &mut self,
        mut photo: PhotoRow,
        a: &Activity,
        km_mark: Option<f64>,
    ) -> Result<PhotoRow> {
        let dir = day_dir(a.header.start_local().date());
        let old_rel = photo.path.clone();
        let old_dir = old_rel
            .rsplit_once('/')
            .map(|(d, _)| d.to_string())
            .unwrap_or_default();
        let moving = old_dir != dir;
        let name = old_rel.rsplit('/').next().unwrap_or(&old_rel).to_string();
        let new_rel = if moving {
            unique_rel(&self.root, &dir, &name)
        } else {
            old_rel.clone()
        };
        if moving {
            copy_new(
                &std::fs::read(join_rel(&self.root, &old_rel))?,
                &join_rel(&self.root, &new_rel),
            )?;
        }
        let mut issues = ImportReport::default();
        let card = self.make_card(a, &new_rel, &mut issues);
        let cleanup = |lib: &Library| {
            if moving {
                let _ = std::fs::remove_file(join_rel(&lib.root, &new_rel));
            }
        };
        let Some(card) = card else {
            cleanup(self);
            let reason = issues.failed.pop().map(|i| i.reason).unwrap_or_default();
            return Err(ServiceError::Failed(format!("카드 생성 실패: {reason}")));
        };
        photo.path = new_rel.clone();
        photo.activity_id = Some(a.header.id.clone());
        photo.km_mark = km_mark;
        photo.card_path = Some(card);
        if let Err(e) = self.store.update_photo(&photo) {
            cleanup(self);
            return Err(e.into());
        }
        if moving {
            let _ = std::fs::remove_file(join_rel(&self.root, &old_rel));
        }
        Ok(photo)
    }

    /// 사진 옆에 `<stem>_card.jpg`를 만든다. 실패는 보고서에 남기고 None.
    pub(crate) fn make_card(
        &self,
        a: &Activity,
        photo_rel: &str,
        report: &mut ImportReport,
    ) -> Option<String> {
        let (parent, name) = photo_rel.rsplit_once('/')?;
        let stem = Path::new(name).file_stem()?.to_string_lossy().into_owned();
        let card_rel = format!("{parent}/{stem}_card.jpg");
        match render_card_to_file(
            &join_rel(&self.root, photo_rel),
            &card_data_with(a, self.card_trim_m()),
            &join_rel(&self.root, &card_rel),
        ) {
            Ok(()) => {
                report.cards_created += 1;
                Some(card_rel)
            }
            Err(e) => {
                report.failed.push(ImportIssue {
                    file: name.to_string(),
                    reason: e.to_string(),
                });
                None
            }
        }
    }

    /// 템플릿으로 글을 (다시) 쓴다. 사용자가 고쳤거나 이미 게시한 글은 건드리지 않는다.
    pub(crate) fn write_template_post(&mut self, activity_id: &str) -> Result<()> {
        let existing = self.store.post(activity_id)?;
        if existing
            .as_ref()
            .is_some_and(|p| p.edited || p.published_at.is_some())
        {
            return Ok(());
        }
        let row = self
            .store
            .activity(activity_id)?
            .ok_or_else(|| ServiceError::NotFound(activity_id.into()))?;
        let post = self.template_post(&row.header, existing)?;
        self.store.upsert_post(&post)?;
        Ok(())
    }

    /// 저장된 장소·경로·날씨와 사진 지점으로 템플릿 글을 만든다 (게시 기록은 유지).
    pub(crate) fn template_post(
        &self,
        header: &ActivityHeader,
        existing: Option<PostRow>,
    ) -> Result<PostRow> {
        let marks: Vec<f64> = self
            .store
            .photos_for(&header.id)?
            .iter()
            .filter_map(|p| p.km_mark)
            .collect();
        let base = existing.unwrap_or_else(|| PostRow {
            activity_id: header.id.clone(),
            body: String::new(),
            hashtags: vec![],
            place: None,
            route_points: vec![],
            weather: None,
            edited: false,
            published_at: None,
            place_user_set: false,
            route_user_set: false,
            weather_user_set: false,
            auto_place: None,
            auto_route_points: vec![],
            auto_weather: None,
        });
        let ctx = PostContext {
            activity: header.clone(),
            place: base.place.clone(),
            route_points: base.route_points.clone(),
            weather: base.weather.clone(),
            photo_km_marks: marks,
        };
        let text = TemplateWriter.write(&ctx)?;
        Ok(PostRow {
            body: text.body,
            hashtags: text.hashtags,
            edited: false,
            ..base
        })
    }
}
