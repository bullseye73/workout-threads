//! 게시물 작성: 올린 FIT·사진으로 무엇이 만들어질지 계산(미리보기)하고, 같은 계산대로 저장한다.
//! 사진 붙이기 규칙은 `build_plan` 한 곳에만 있다:
//! ① 이번 새 운동과 시간 일치 → ② 보관함 운동과 시간 일치 → ③ 촬영 날짜에 운동이 하나뿐이면 그 운동 → ④ 제외.
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use workout_core::chrono::{DateTime, Datelike, NaiveDate, Utc};
use workout_core::fit::parse_fit;
use workout_core::format::activity_title;
use workout_core::matching::match_photo;
use workout_core::model::{Activity, ActivityHeader};
use workout_core::photo::read_photo_meta;

use workout_store::PhotoRow;

use crate::files::{content_hash, copy_new, join_rel, unique_rel};
use crate::import::{NEAR_HOURS, day_dir, expand, file_name, local_date, local_offset_at};
use crate::*;

pub(crate) const NO_DATE: &str = "촬영 날짜가 없어요";
pub(crate) const NO_TARGET: &str = "붙일 운동이 없어 제외";
pub(crate) const BAD_FIT: &str = "읽을 수 없는 파일";

/// 계획된 사진 한 장의 출처.
#[derive(Clone, Debug)]
pub(crate) enum PhotoSrc {
    /// 새로 올린 파일.
    New(PathBuf),
    /// 보관함에 이미 있는 안 붙은 사진.
    Library(i64),
}

#[derive(Clone, Debug)]
pub(crate) struct Placed {
    pub src: PhotoSrc,
    pub taken_at: DateTime<Utc>,
    pub km_mark: Option<f64>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub hash: Option<String>,
}

/// 미리보기 + 저장에 필요한 원본 데이터.
pub(crate) struct Plan {
    pub view: UploadPlan,
    /// (원본 경로, 파싱한 운동), 시작 시각 순.
    pub new_acts: Vec<(PathBuf, Activity)>,
    /// 운동 id → 붙일 사진.
    pub placements: BTreeMap<String, Vec<Placed>>,
}

fn iso(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

impl Library {
    /// 저장하지 않고 무엇이 만들어질지 돌려준다.
    /// `exclude`: 사용자가 미리보기에서 뺀 파일 (폴더로 올린 것 중 일부를 뺄 수 있게).
    pub fn plan_upload(&self, paths: &[PathBuf], exclude: &[PathBuf]) -> Result<UploadPlan> {
        Ok(self.build_plan(paths, exclude)?.view)
    }

    /// 미리보기와 같은 계산으로 저장한다: FIT마다 운동·게시물 하나, 사진은 계획된 운동에 붙인다.
    /// 하나가 실패해도 나머지는 계속하고 `failed`에 담는다. 원본 파일은 건드리지 않는다.
    pub fn commit_upload(
        &mut self,
        paths: &[PathBuf],
        exclude: &[PathBuf],
    ) -> Result<UploadResult> {
        let plan = self.build_plan(paths, exclude)?;
        let mut r = UploadResult {
            excluded: plan.view.excluded.clone(),
            ..Default::default()
        };
        let mut ready: HashMap<String, Activity> = HashMap::new();
        for (fit, a) in &plan.new_acts {
            match self.import_fit(fit) {
                Ok(_) => {
                    r.created.push(a.header.id.clone());
                    ready.insert(a.header.id.clone(), a.clone());
                }
                Err(e) => r.failed.push(ImportIssue {
                    file: file_name(fit),
                    reason: e.to_string(),
                }),
            }
        }
        let mut touched: BTreeSet<String> = BTreeSet::new();
        for (id, photos) in &plan.placements {
            let a = match ready.get(id) {
                Some(a) => a.clone(),
                None if plan.new_acts.iter().any(|(_, n)| &n.header.id == id) => continue, // 운동 저장 실패
                None => match self.load_activity(id) {
                    Ok(a) => a,
                    Err(e) => {
                        r.failed.push(ImportIssue {
                            file: id.clone(),
                            reason: e.to_string(),
                        });
                        continue;
                    }
                },
            };
            for p in photos {
                let done = match &p.src {
                    PhotoSrc::New(path) => self.store_new_photo(path, &a, p, &mut r),
                    PhotoSrc::Library(pid) => match self.store.photo(*pid)? {
                        Some(row) => self.attach_photo(row, &a, p.km_mark).map(|_| true),
                        None => Ok(false),
                    },
                };
                match done {
                    Ok(true) => {
                        r.cards_created += 1;
                        touched.insert(id.clone());
                    }
                    Ok(false) => {}
                    Err(e) => r.failed.push(ImportIssue {
                        file: id.clone(),
                        reason: e.to_string(),
                    }),
                }
            }
        }
        r.updated = plan
            .view
            .existing
            .iter()
            .map(|e| e.activity_id.clone())
            .filter(|id| touched.contains(id))
            .collect();
        for id in r
            .created
            .iter()
            .chain(&r.updated)
            .cloned()
            .collect::<Vec<_>>()
        {
            if let Err(e) = self.write_template_post(&id) {
                r.failed.push(ImportIssue {
                    file: id,
                    reason: format!("글 작성 실패: {e}"),
                });
            }
        }
        Ok(r)
    }

    /// 새 사진을 운동 날짜 폴더에 복사하고 카드를 만든 뒤 저장한다. 카드를 만들었으면 true.
    fn store_new_photo(
        &mut self,
        src: &Path,
        a: &Activity,
        p: &Placed,
        r: &mut UploadResult,
    ) -> Result<bool> {
        let bytes = std::fs::read(src)?;
        let rel = unique_rel(
            &self.root,
            &day_dir(a.header.start_local().date()),
            &file_name(src),
        );
        copy_new(&bytes, &join_rel(&self.root, &rel))?;
        let mut issues = ImportReport::default();
        let card = self.make_card(a, &rel, &mut issues);
        r.failed.extend(issues.failed);
        let made = card.is_some();
        self.store.insert_photo(&PhotoRow {
            id: 0,
            activity_id: Some(a.header.id.clone()),
            path: rel,
            taken_at: Some(p.taken_at),
            km_mark: p.km_mark,
            lat: p.lat,
            lon: p.lon,
            card_path: card,
            content_hash: p.hash.clone(),
        })?;
        Ok(made)
    }

    pub(crate) fn build_plan(&self, paths: &[PathBuf], exclude: &[PathBuf]) -> Result<Plan> {
        let (mut fits, mut photos) = expand(paths, true);
        fits.retain(|p| !exclude.contains(p));
        photos.retain(|p| !exclude.contains(p));
        let mut view = UploadPlan::default();
        let mut new_acts: Vec<(PathBuf, Activity)> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();

        // FIT: 운동 id로 중복을 걸러 새 운동 / 이미 있는 운동으로 나눈다.
        for fit in fits {
            let Some(a) = std::fs::read(&fit).ok().and_then(|b| parse_fit(&b).ok()) else {
                view.excluded.push(ImportIssue {
                    file: file_name(&fit),
                    reason: BAD_FIT.into(),
                });
                continue;
            };
            if !seen.insert(a.header.id.clone()) {
                continue;
            }
            if self.store.has_activity(&a.header.id)? {
                self.existing_target(&mut view, &a.header.id)?.uploaded_fit = true;
            } else {
                new_acts.push((fit, a));
            }
        }
        new_acts.sort_by_key(|(_, a)| a.header.start);

        let mut lib_cache: HashMap<String, Activity> = HashMap::new();
        let mut placements: BTreeMap<String, Vec<Placed>> = BTreeMap::new();
        let mut handled_library: HashSet<i64> = HashSet::new();
        let batch_offset = new_acts.first().map(|(_, a)| a.header.utc_offset_s);
        let new_only: Vec<Activity> = new_acts.iter().map(|(_, a)| a.clone()).collect();

        // 올린 사진.
        for photo in photos {
            let bytes = std::fs::read(&photo)?;
            let hash = content_hash(&bytes);
            if let Some(row) = self.store.photo_by_hash(&hash)? {
                if row.activity_id.is_some() {
                    view.duplicates.push(file_name(&photo));
                    continue;
                }
                // 보관함에 안 붙은 채 있던 같은 사진: 보관함 사진으로 붙여 본다.
                handled_library.insert(row.id);
                let Some(t) = row.taken_at else {
                    view.duplicates.push(file_name(&photo));
                    continue;
                };
                let offset = batch_offset.unwrap_or_else(|| local_offset_at(t));
                match self.assign(t, offset, &new_only, true, &mut lib_cache)? {
                    Some((id, km)) => self.place(
                        &mut view,
                        &mut placements,
                        &new_acts,
                        &photo,
                        id,
                        Placed {
                            src: PhotoSrc::Library(row.id),
                            taken_at: t,
                            km_mark: km,
                            lat: row.lat,
                            lon: row.lon,
                            hash: None,
                        },
                    )?,
                    None => view.duplicates.push(file_name(&photo)),
                }
                continue;
            }
            // 시각을 대략 읽어(이번 운동 오프셋, 없으면 이 PC 시간대) 다시 읽는다 — 가져오기와 같은 추정.
            let rough = read_photo_meta(
                &photo,
                batch_offset.unwrap_or_else(|| local_offset_at(Utc::now())),
            )?;
            let offset =
                batch_offset.unwrap_or_else(|| rough.taken_at.map(local_offset_at).unwrap_or(0));
            let meta = read_photo_meta(&photo, offset)?;
            let Some(t) = meta.taken_at else {
                view.excluded.push(ImportIssue {
                    file: file_name(&photo),
                    reason: NO_DATE.into(),
                });
                continue;
            };
            match self.assign(t, offset, &new_only, true, &mut lib_cache)? {
                Some((id, km)) => self.place(
                    &mut view,
                    &mut placements,
                    &new_acts,
                    &photo,
                    id,
                    Placed {
                        src: PhotoSrc::New(photo.clone()),
                        taken_at: t,
                        km_mark: km,
                        lat: meta.pos.map(|p| p.lat),
                        lon: meta.pos.map(|p| p.lon),
                        hash: Some(hash),
                    },
                )?,
                None => view.excluded.push(ImportIssue {
                    file: file_name(&photo),
                    reason: NO_TARGET.into(),
                }),
            }
        }

        // 보관함에 남아 있던 안 붙은 사진: 새 운동에만 붙여 본다(제외하지 않음).
        if !new_acts.is_empty() {
            for row in self.store.unmatched_photos()? {
                if handled_library.contains(&row.id) {
                    continue;
                }
                let offset = batch_offset.unwrap_or(0);
                let abs = crate::files::join_rel(&self.root, &row.path);
                let Some(t) = read_photo_meta(&abs, offset).ok().and_then(|m| m.taken_at) else {
                    continue;
                };
                let Some((id, km)) = self.assign(t, offset, &new_only, false, &mut lib_cache)?
                else {
                    continue;
                };
                if let Some(i) = new_acts.iter().position(|(_, a)| a.header.id == id) {
                    view.new_activities_mut(&new_acts)[i].library_photos += 1;
                    placements.entry(id).or_default().push(Placed {
                        src: PhotoSrc::Library(row.id),
                        taken_at: t,
                        km_mark: km,
                        lat: row.lat,
                        lon: row.lon,
                        hash: None,
                    });
                }
            }
        }

        view.new_activities_mut(&new_acts);
        view.existing
            .sort_by(|a, b| a.date.cmp(&b.date).then(a.activity_id.cmp(&b.activity_id)));
        Ok(Plan {
            view,
            new_acts,
            placements,
        })
    }

    /// 사진 한 장을 붙일 운동과 km 지점. `use_library`가 false면 이번 새 운동에만.
    fn assign(
        &self,
        t: DateTime<Utc>,
        offset: i32,
        new_acts: &[Activity],
        use_library: bool,
        cache: &mut HashMap<String, Activity>,
    ) -> Result<Option<(String, Option<f64>)>> {
        // ① 이번 새 운동과 시간 일치.
        if let Some(m) = match_photo(new_acts, t) {
            return Ok(Some((m.activity_id, Some(m.km_mark))));
        }
        // ② 보관함 운동과 시간 일치.
        if use_library {
            let mut near = Vec::new();
            for row in self.store.activities_near(t, NEAR_HOURS)? {
                if !cache.contains_key(&row.header.id) {
                    cache.insert(row.header.id.clone(), self.load_activity(&row.header.id)?);
                }
                near.push(cache[&row.header.id].clone());
            }
            if let Some(m) = match_photo(&near, t) {
                return Ok(Some((m.activity_id, Some(m.km_mark))));
            }
        }
        // ③ 촬영 날짜(그날 운동의 시간대 기준)에 운동이 하나뿐이면 그 운동.
        let date = local_date(t, offset);
        let mut ids: Vec<String> = new_acts
            .iter()
            .filter(|a| a.header.start_local().date() == date)
            .map(|a| a.header.id.clone())
            .collect();
        for row in self.store.activities_in_month(date.year(), date.month())? {
            if row.header.start_local().date() == date && !ids.contains(&row.header.id) {
                ids.push(row.header.id);
            }
        }
        let only = match ids.as_slice() {
            [id] => Some(id.clone()),
            _ => None,
        };
        Ok(only
            .filter(|id| use_library || new_acts.iter().any(|a| &a.header.id == id))
            .map(|id| (id, None)))
    }

    /// 계획에 사진을 놓고, 미리보기 목록에도 보인다.
    #[allow(clippy::too_many_arguments)]
    fn place(
        &self,
        view: &mut UploadPlan,
        placements: &mut BTreeMap<String, Vec<Placed>>,
        new_acts: &[(PathBuf, Activity)],
        shown_path: &Path,
        id: String,
        placed: Placed,
    ) -> Result<()> {
        let shown = PlannedPhoto {
            path: shown_path.to_string_lossy().into_owned(),
            file_name: file_name(shown_path),
            km_mark: placed.km_mark,
        };
        if let Some(i) = new_acts.iter().position(|(_, a)| a.header.id == id) {
            view.new_activities_mut(new_acts)[i].photos.push(shown);
        } else {
            self.existing_target(view, &id)?.photos.push(shown);
        }
        placements.entry(id).or_default().push(placed);
        Ok(())
    }

    /// 미리보기의 "이미 있는 운동" 항목 (없으면 만든다).
    fn existing_target<'a>(
        &self,
        view: &'a mut UploadPlan,
        id: &str,
    ) -> Result<&'a mut ExistingTarget> {
        if let Some(i) = view.existing.iter().position(|e| e.activity_id == id) {
            return Ok(&mut view.existing[i]);
        }
        let row = self
            .store
            .activity(id)?
            .ok_or_else(|| ServiceError::NotFound(id.into()))?;
        let post_locked = self
            .store
            .post(id)?
            .is_some_and(|p| p.edited || p.published_at.is_some());
        view.existing.push(ExistingTarget {
            activity_id: id.into(),
            date: iso(row.header.start_local().date()),
            title: activity_title(row.header.start_local(), row.header.sport),
            sport: row.header.sport.into(),
            km: row.header.summary.distance_m / 1000.0,
            uploaded_fit: false,
            photos: vec![],
            post_locked,
        });
        Ok(view.existing.last_mut().expect("방금 넣음"))
    }
}

fn planned(fit: &Path, h: &ActivityHeader) -> PlannedActivity {
    PlannedActivity {
        source: fit.to_string_lossy().into_owned(),
        activity_id: h.id.clone(),
        date: iso(h.start_local().date()),
        title: activity_title(h.start_local(), h.sport),
        sport: h.sport.into(),
        km: h.summary.distance_m / 1000.0,
        photos: vec![],
        library_photos: 0,
    }
}

impl UploadPlan {
    /// 새 운동 목록을 `new_acts` 순서로 맞춰 두고 돌려준다 (사진을 붙이기 전에 항목이 있어야 하므로).
    fn new_activities_mut(
        &mut self,
        new_acts: &[(PathBuf, Activity)],
    ) -> &mut Vec<PlannedActivity> {
        if self.new_activities.len() != new_acts.len() {
            self.new_activities = new_acts
                .iter()
                .map(|(p, a)| planned(p, &a.header))
                .collect();
        }
        &mut self.new_activities
    }
}
