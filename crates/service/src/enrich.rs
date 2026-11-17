//! 장소·경로·날씨 자동 채우기, 사용자가 직접 정한 값, Claude로 다듬기, API 키 관리.
//!
//! 인터넷 조회는 라이브러리 잠금 없이 하도록 세 단계로 나눈다:
//! `prepare_*`(잠금, 필요한 값만 복사) → `Job::run`(잠금 없음, HTTP) → `apply_enrich`(잠금, 다시 확인 후 저장).
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use workout_card::photo_for_upload;
use workout_core::chrono::{Duration as ChronoDuration, Local, NaiveDateTime};
use workout_core::model::LatLon;
use workout_core::privacy::{MAX_PLACE_POINTS, TRIM_M, privacy_points, weather_point};
use workout_enrich::geocode::{
    PlaceSummary, RateLimiter, parse_reverse, reverse_url, summarize_places,
};
use workout_enrich::refine::{Provider, RefineInput, refine_with};
use workout_enrich::weather::{parse_weather, weather_url};
use workout_enrich::{Http, USER_AGENT, check_status};
use workout_post::{PlaceText, PostContext, PostText, WeatherText};
use workout_store::PostRow;

use crate::*;

/// Nominatim 이용 정책: 초당 1건.
pub(crate) const NOMINATIM_INTERVAL: Duration = Duration::from_secs(1);
const PROVIDER_KEY: &str = "refine_provider";
const CARD_TRIM_KEY: &str = "card_route_trim";
/// Claude로 보내는 사진 수와 크기.
const MAX_UPLOAD_PHOTOS: usize = 4;
const UPLOAD_SIDE: u32 = 1024;

fn cache_key(p: LatLon) -> String {
    format!("{:.4},{:.4}", p.lat, p.lon)
}

/// 잠금 없이 실행하는 장소·날씨 조회 작업.
pub struct EnrichJob {
    id: String,
    points: Vec<LatLon>,
    cached: HashMap<String, String>,
    weather: Option<(LatLon, NaiveDateTime)>,
    http: Arc<dyn Http>,
    limiter: Arc<RateLimiter>,
}

/// 조회 결과 (아직 저장 전).
pub struct EnrichFetched {
    id: String,
    places: Option<PlaceSummary>,
    new_cache: Vec<(String, String)>,
    weather: Option<WeatherText>,
    errors: Vec<String>,
}

impl EnrichJob {
    pub fn run(self) -> EnrichFetched {
        let mut errors = Vec::new();
        let mut new_cache = Vec::new();
        let mut results = Vec::new();
        for p in &self.points {
            let key = cache_key(*p);
            // 캐시가 깨졌으면 다시 조회한다.
            if let Some(Ok(r)) = self
                .cached
                .get(&key)
                .map(|json| parse_reverse(json.as_bytes()))
            {
                results.push(r);
                continue;
            }
            self.limiter.wait();
            let got = self
                .http
                .get(&reverse_url(*p), &[("User-Agent", USER_AGENT)])
                .and_then(|res| check_status(&res).map(|_| res))
                .and_then(|res| parse_reverse(&res.body).map(|r| (r, res.body)));
            match got {
                Ok((r, body)) => {
                    new_cache.push((key, String::from_utf8_lossy(&body).into_owned()));
                    results.push(r);
                }
                Err(e) => errors.push(format!("장소 조회: {e}")),
            }
        }
        let places = (!results.is_empty()).then(|| summarize_places(&results));
        let weather = self.weather.and_then(|(wp, mid)| {
            let url = weather_url(wp, mid.date(), Local::now().date_naive());
            let got = self
                .http
                .get(&url, &[("User-Agent", USER_AGENT)])
                .and_then(|res| check_status(&res).map(|_| res))
                .and_then(|res| parse_weather(&res.body, mid));
            match got {
                Ok((description, temp_c)) => Some(WeatherText {
                    description,
                    temp_c,
                }),
                Err(e) => {
                    errors.push(format!("날씨 조회: {e}"));
                    None
                }
            }
        });
        EnrichFetched {
            id: self.id,
            places,
            new_cache,
            weather,
            errors,
        }
    }
}

/// 잠금 없이 실행하는 AI 다듬기 작업.
pub struct RefineJob {
    provider: Provider,
    http: Arc<dyn Http>,
    key: String,
    model: String,
    input: RefineInput,
    photo_paths: Vec<PathBuf>,
}

impl RefineJob {
    pub fn run(mut self) -> Result<RefineDto> {
        self.input.photos_jpeg = self
            .photo_paths
            .iter()
            .filter_map(|p| photo_for_upload(p, UPLOAD_SIDE).ok())
            .collect();
        let text = refine_with(
            self.provider,
            self.http.as_ref(),
            &self.key,
            &self.model,
            &self.input,
        )
        .map_err(|e| ServiceError::Failed(e.to_string()))?;
        Ok(RefineDto {
            full_text: text.full_text(),
            body: text.body,
            hashtags: text.hashtags,
        })
    }
}

impl Library {
    fn post_row(&self, id: &str) -> Result<PostRow> {
        let row = self
            .store
            .activity(id)?
            .ok_or_else(|| ServiceError::NotFound(id.into()))?;
        match self.store.post(id)? {
            Some(p) => Ok(p),
            None => self.template_post(&row.header, None),
        }
    }

    /// 값을 바꾼 뒤 저장하고, 사용자가 고친·게시한 글이 아니면 템플릿을 다시 쓴다.
    fn update_post(&mut self, id: &str, f: impl FnOnce(&mut PostRow)) -> Result<PostView> {
        let mut post = self.post_row(id)?;
        f(&mut post);
        self.store.upsert_post(&post)?;
        self.write_template_post(id)?;
        self.post_view(id)
    }

    /// 조회에 필요한 값만 복사한다 (출발·도착 500m 밖 지점, 날씨용 반올림 지점).
    pub fn prepare_enrich(&self, id: &str) -> Result<EnrichJob> {
        let row = self
            .store
            .activity(id)?
            .ok_or_else(|| ServiceError::NotFound(id.into()))?;
        let post = self.post_row(id)?;
        let points = if post.place_user_set && post.route_user_set {
            vec![]
        } else {
            privacy_points(&row.route, TRIM_M, MAX_PLACE_POINTS)
        };
        let mut cached = HashMap::new();
        for p in &points {
            let key = cache_key(*p);
            if let Some(json) = self.store.geocode_cached(&key)? {
                cached.insert(key, json);
            }
        }
        // 운동 중간 시각의 현지 시(時) 날씨.
        let mid = row.header.start_local()
            + ChronoDuration::milliseconds((row.header.summary.elapsed_s * 500.0) as i64);
        let weather = weather_point(&row.route, TRIM_M)
            .filter(|_| !post.weather_user_set)
            .map(|wp| (wp, mid));
        Ok(EnrichJob {
            id: id.into(),
            points,
            cached,
            weather,
            http: self.http.clone(),
            limiter: self.nominatim.clone(),
        })
    }

    /// 조회 결과를 저장한다. 그사이 사용자가 직접 정한 칸은 덮어쓰지 않는다 (자동 값만 갱신).
    pub fn apply_enrich(&mut self, f: EnrichFetched) -> Result<PostView> {
        for (k, v) in &f.new_cache {
            let _ = self.store.put_geocode(k, v);
        }
        let mut post = self.post_row(&f.id)?;
        if let Some(s) = f.places {
            post.auto_place = s.place;
            post.auto_route_points = s.route_points;
            if !post.place_user_set {
                post.place = post.auto_place.clone().map(|name| PlaceText {
                    name,
                    estimated: true,
                });
            }
            if !post.route_user_set {
                post.route_points = post.auto_route_points.clone();
            }
        }
        if let Some(w) = f.weather {
            post.auto_weather = Some(w);
            if !post.weather_user_set {
                post.weather = post.auto_weather.clone();
            }
        }
        self.store.upsert_post(&post)?;
        self.write_template_post(&f.id)?;
        let mut view = self.post_view(&f.id)?;
        view.enrich_error = f.errors.first().cloned();
        Ok(view)
    }

    /// 준비 → 조회 → 저장을 한 번에 (잠금을 따로 다루지 않는 곳에서 쓴다).
    pub fn enrich_activity(&mut self, id: &str) -> Result<PostView> {
        let fetched = self.prepare_enrich(id)?.run();
        self.apply_enrich(fetched)
    }

    /// 장소를 직접 정한다. None이면 마지막 자동 값으로 되돌린다.
    pub fn set_place(&mut self, id: &str, name: Option<String>) -> Result<PostView> {
        let name = name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty());
        self.update_post(id, |p| {
            p.place_user_set = name.is_some();
            p.place = match name {
                Some(name) => Some(PlaceText {
                    name,
                    estimated: false,
                }),
                None => p.auto_place.clone().map(|name| PlaceText {
                    name,
                    estimated: true,
                }),
            };
        })
    }

    /// 지나간 동네를 직접 정한다. 빈 목록이면 마지막 자동 값으로 되돌린다.
    pub fn set_route_points(&mut self, id: &str, points: Vec<String>) -> Result<PostView> {
        let points: Vec<String> = points
            .into_iter()
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
            .collect();
        self.update_post(id, |p| {
            p.route_user_set = !points.is_empty();
            p.route_points = if points.is_empty() {
                p.auto_route_points.clone()
            } else {
                points
            };
        })
    }

    /// 날씨를 직접 정한다. None이면 마지막 자동 값으로 되돌린다.
    pub fn set_weather(&mut self, id: &str, weather: Option<WeatherDto>) -> Result<PostView> {
        self.update_post(id, |p| {
            p.weather_user_set = weather.is_some();
            p.weather = match weather {
                Some(w) => Some(WeatherText {
                    description: w.description,
                    temp_c: w.temp_c,
                }),
                None => p.auto_weather.clone(),
            };
        })
    }

    /// 다듬기 준비: 화면에 있는 글(저장 안 된 수정 포함)과 운동 정보. 좌표는 넣지 않는다.
    pub fn prepare_refine(
        &self,
        id: &str,
        body: &str,
        hashtags: &[String],
        include_photos: bool,
    ) -> Result<RefineJob> {
        let provider = self.refine_provider()?;
        let key = self.secrets.get(provider.key_name()).ok_or_else(|| {
            ServiceError::Failed(format!(
                "설정에서 {} API 키를 넣어 주세요",
                provider.label()
            ))
        })?;
        let row = self
            .store
            .activity(id)?
            .ok_or_else(|| ServiceError::NotFound(id.into()))?;
        let post = self.post_row(id)?;
        let photos = self.store.photos_for(id)?;
        let photo_paths = if include_photos {
            photos
                .iter()
                .take(MAX_UPLOAD_PHOTOS)
                .map(|p| files::join_rel(&self.root, &p.path))
                .collect()
        } else {
            vec![]
        };
        let input = RefineInput {
            ctx: PostContext {
                activity: row.header,
                place: post.place.clone(),
                route_points: post.route_points.clone(),
                weather: post.weather.clone(),
                photo_km_marks: photos.iter().filter_map(|p| p.km_mark).collect(),
            },
            draft: PostText {
                body: body.to_string(),
                hashtags: hashtags.to_vec(),
            },
            photos_jpeg: vec![],
        };
        Ok(RefineJob {
            provider,
            http: self.http.clone(),
            key,
            model: self.refine_model(provider)?,
            input,
            photo_paths,
        })
    }

    /// 저장된 글로 다듬은 글을 돌려준다. 저장하지 않는다.
    pub fn refine_post(&self, id: &str, include_photos: bool) -> Result<RefineDto> {
        let post = self.post_row(id)?;
        self.prepare_refine(id, &post.body, &post.hashtags, include_photos)?
            .run()
    }

    fn provider_arg(id: &str) -> Result<Provider> {
        Provider::parse(id).ok_or_else(|| ServiceError::Failed(format!("알 수 없는 AI: {id}")))
    }

    /// 고른 AI. 아직 고른 적이 없으면 Gemini가 기본이지만,
    /// 예전 버전에서 Claude 키만 넣어 둔 사용자는 Claude를 그대로 쓴다.
    fn refine_provider(&self) -> Result<Provider> {
        if let Some(p) = self
            .store
            .setting(PROVIDER_KEY)?
            .and_then(|p| Provider::parse(&p))
        {
            return Ok(p);
        }
        let has = |p: Provider| self.secrets.get(p.key_name()).is_some();
        Ok(if has(Provider::Claude) && !has(Provider::Gemini) {
            Provider::Claude
        } else {
            Provider::Gemini
        })
    }

    fn refine_model(&self, p: Provider) -> Result<String> {
        Ok(self
            .store
            .setting(p.model_setting())?
            .unwrap_or_else(|| p.default_model().to_string()))
    }

    /// 글 다듬기에 쓸 AI를 고른다 ("gemini" | "claude").
    pub fn set_refine_provider(&mut self, provider: &str) -> Result<()> {
        let p = Self::provider_arg(provider)?;
        self.store.set_setting(PROVIDER_KEY, Some(p.id()))?;
        Ok(())
    }

    /// 그 AI의 API 키를 키 보관함에 저장한다 (None이면 지움).
    pub fn set_refine_key(&mut self, provider: &str, key: Option<String>) -> Result<()> {
        let p = Self::provider_arg(provider)?;
        let key = key.map(|k| k.trim().to_string()).filter(|k| !k.is_empty());
        self.secrets.set(p.key_name(), key.as_deref())
    }

    pub fn set_refine_model(&mut self, provider: &str, model: Option<String>) -> Result<()> {
        let p = Self::provider_arg(provider)?;
        let model = model
            .map(|m| m.trim().to_string())
            .filter(|m| !m.is_empty());
        self.store
            .set_setting(p.model_setting(), model.as_deref())?;
        Ok(())
    }

    /// 고른 AI와 각 AI의 키 유무·모델 (키 값은 절대 돌려주지 않음).
    pub fn refine_status(&self) -> Result<RefineStatusDto> {
        let current = self.refine_provider()?;
        let mut providers = Vec::new();
        for p in Provider::ALL {
            providers.push(ProviderDto {
                id: p.id().into(),
                label: p.label().into(),
                company: p.company().into(),
                has_key: self.secrets.get(p.key_name()).is_some(),
                model: self.refine_model(p)?,
            });
        }
        let cur = providers
            .iter()
            .find(|d| d.id == current.id())
            .cloned()
            .expect("고른 AI");
        Ok(RefineStatusDto {
            provider: cur.id,
            label: cur.label,
            company: cur.company,
            has_key: cur.has_key,
            model: cur.model,
            providers,
        })
    }

    pub(crate) fn card_trim_m(&self) -> f64 {
        match self.store.setting(CARD_TRIM_KEY) {
            Ok(Some(_)) => TRIM_M,
            _ => 0.0,
        }
    }

    pub fn set_card_route_trim(&mut self, on: bool) -> Result<()> {
        let v = TRIM_M.to_string();
        self.store
            .set_setting(CARD_TRIM_KEY, on.then_some(v.as_str()))?;
        Ok(())
    }

    pub fn settings(&self) -> Result<SettingsDto> {
        Ok(SettingsDto {
            card_route_trim: self.card_trim_m() > 0.0,
            garmin_connected: self.has_garmin_key(),
            garmin_auto_check: self.garmin_auto_check(),
        })
    }

    /// 이 운동의 대표 카드를 지금 설정으로 다시 그린다. 다시 그린 장수를 돌려준다.
    pub fn regenerate_cards(&mut self, id: &str) -> Result<u32> {
        let activity = self.load_activity(id)?;
        let mut report = ImportReport::default();
        for (idx, photo) in self.store.photos_for(id)?.into_iter().enumerate() {
            if photo.card_path.is_some() {
                if idx == 0 {
                    self.make_card(&activity, &photo.path, &mut report);
                } else {
                    self.make_optimized_photo(&photo.path, &mut report);
                }
            }
        }
        if let Some(issue) = report.failed.first() {
            return Err(ServiceError::Failed(format!(
                "카드 다시 만들기 실패: {}",
                issue.reason
            )));
        }
        Ok(report.cards_created)
    }
}
