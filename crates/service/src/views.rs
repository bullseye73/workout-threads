//! 대시보드와 운동 상세 조회.
use workout_card::card_data;
use workout_core::format::{activity_title, hms, pace_per_km, speed_kmh};
use workout_core::model::{Sample, Sport};
use workout_core::splits::km_splits;
use workout_core::stats::{SportStats, month_stats};

use crate::*;

/// 그래프용 시계열 최대 점 수.
const SERIES_MAX_POINTS: usize = 1000;
/// 예전 합계 목표 (처음 읽을 때 라이딩 목표로 옮긴다).
const OLD_GOAL_KEY: &str = "monthly_goal_km";
const GOAL_CYCLING_KEY: &str = "goal_cycling_km";
const GOAL_RUNNING_KEY: &str = "goal_running_km";

fn sport_summary(s: &SportStats, sport: Sport) -> SportSummaryDto {
    let has = s.count > 0 && s.distance_m > 0.0 && s.moving_s > 0.0;
    SportSummaryDto {
        count: s.count,
        km: s.distance_m / 1000.0,
        moving_s: s.moving_s,
        avg_pace: (has && sport == Sport::Running).then(|| pace_per_km(s.moving_s, s.distance_m)),
        avg_speed_kmh: (has && sport == Sport::Cycling)
            .then(|| speed_kmh(s.moving_s, s.distance_m)),
    }
}

/// 균등 간격으로 최대 max개 인덱스.
fn sample_indices(n: usize, max: usize) -> Vec<usize> {
    if n <= max {
        return (0..n).collect();
    }
    (0..max)
        .map(|i| ((i as f64) * (n - 1) as f64 / (max - 1) as f64).round() as usize)
        .collect()
}

fn series(samples: &[Sample]) -> SeriesDto {
    let idx = sample_indices(samples.len(), SERIES_MAX_POINTS);
    let mut last_d = 0.0;
    let mut out = SeriesDto {
        distance_km: vec![],
        hr: vec![],
        speed_kmh: vec![],
        altitude_m: vec![],
        lat_lon: vec![],
    };
    for i in idx {
        let s = &samples[i];
        last_d = s.distance_m.unwrap_or(last_d);
        out.distance_km.push(last_d / 1000.0);
        out.hr.push(s.heart_rate);
        out.speed_kmh.push(s.speed_mps.map(|v| v * 3.6));
        out.altitude_m.push(s.altitude_m);
        out.lat_lon.push(s.pos.map(|p| [p.lat, p.lon]));
    }
    out
}

impl Library {
    pub fn month_view(&self, year: i32, month: u32) -> Result<MonthView> {
        let rows = self.store.activities_in_month(year, month)?;
        let headers: Vec<_> = rows.iter().map(|r| r.header.clone()).collect();
        let st = month_stats(&headers, year, month);
        let mut days = Vec::new();
        for (date, items) in &st.days {
            let mut out = Vec::new();
            for it in items {
                let h = headers
                    .iter()
                    .find(|h| h.id == it.activity_id)
                    .ok_or_else(|| ServiceError::NotFound(it.activity_id.clone()))?;
                let sm = &h.summary;
                let has_dist = sm.distance_m > 0.0;
                let cards = self.card_paths(&it.activity_id)?;
                out.push(DayItemDto {
                    activity_id: it.activity_id.clone(),
                    sport: it.sport.into(),
                    km: it.distance_m / 1000.0,
                    post_status: self.post_status(&it.activity_id)?,
                    title: activity_title(h.start_local(), h.sport),
                    start_time: h.start_local().format("%H:%M").to_string(),
                    moving_s: sm.moving_s,
                    speed_kmh: (has_dist && h.sport == Sport::Cycling)
                        .then(|| speed_kmh(sm.moving_s, sm.distance_m)),
                    pace: (has_dist && h.sport != Sport::Cycling)
                        .then(|| pace_per_km(sm.moving_s, sm.distance_m)),
                    card_count: cards.len() as u32,
                    cover_card: cards.into_iter().next(),
                });
            }
            days.push(DayDto {
                date: date.format("%Y-%m-%d").to_string(),
                items: out,
            });
        }
        Ok(MonthView {
            year,
            month,
            goal_cycling_km: self.goal(GOAL_CYCLING_KEY)?,
            goal_running_km: self.goal(GOAL_RUNNING_KEY)?,
            total_km: st.total_distance_m / 1000.0,
            total_moving_s: st.total_moving_s,
            running: sport_summary(&st.running, Sport::Running),
            cycling: sport_summary(&st.cycling, Sport::Cycling),
            other_count: st.other.count,
            avg_hr: st.avg_hr,
            weekly: st
                .weekly
                .iter()
                .map(|w| WeekDto {
                    week_start: w.week_start.format("%Y-%m-%d").to_string(),
                    running_km: w.running_m / 1000.0,
                    cycling_km: w.cycling_m / 1000.0,
                })
                .collect(),
            days,
        })
    }

    pub fn activity_detail(&self, id: &str) -> Result<ActivityDetail> {
        let row = self
            .store
            .activity(id)?
            .ok_or_else(|| ServiceError::NotFound(id.into()))?;
        let a = self.load_activity(id)?;
        let h = &a.header;
        let sm = &h.summary;
        let card = card_data(&a);
        let has_dist = sm.distance_m > 0.0;
        let route: Vec<[f64; 2]> = row.route.iter().map(|p| [p.lat, p.lon]).collect();
        Ok(ActivityDetail {
            id: h.id.clone(),
            sport: h.sport.into(),
            title: card.title,
            date: card.date,
            metrics: MetricsDto {
                distance_km: sm.distance_m / 1000.0,
                moving: hms(sm.moving_s),
                elapsed: hms(sm.elapsed_s),
                pace: (has_dist && h.sport != Sport::Cycling)
                    .then(|| pace_per_km(sm.moving_s, sm.distance_m)),
                speed_kmh: (has_dist && h.sport == Sport::Cycling)
                    .then(|| speed_kmh(sm.moving_s, sm.distance_m)),
                avg_hr: sm.avg_hr,
                max_hr: sm.max_hr,
                ascent_m: sm.ascent_m,
                avg_cadence: sm.avg_cadence,
            },
            start: route.first().copied(),
            end: route.last().copied(),
            route,
            photos: self
                .store
                .photos_for(id)?
                .iter()
                .map(|p| self.photo_dto(p))
                .collect(),
            fit_file: self.abs(&row.fit_path),
            folder: row
                .fit_path
                .rsplit_once('/')
                .map(|(d, _)| self.abs(d))
                .unwrap_or_default(),
            series: series(&a.samples),
            splits: km_splits(&a.samples)
                .into_iter()
                .map(|s| SplitDto {
                    km: s.km,
                    duration: hms(s.duration_s),
                    avg_hr: s.avg_hr,
                })
                .collect(),
        })
    }

    /// 라이딩·러닝 월 목표. 0 이하·숫자 아님은 목표 없음.
    pub fn set_goals(&mut self, cycling_km: Option<f64>, running_km: Option<f64>) -> Result<()> {
        let valid = |k: Option<f64>| {
            k.filter(|k| k.is_finite() && *k > 0.0)
                .map(|k| k.to_string())
        };
        self.store
            .set_setting(GOAL_CYCLING_KEY, valid(cycling_km).as_deref())?;
        self.store
            .set_setting(GOAL_RUNNING_KEY, valid(running_km).as_deref())?;
        self.store.set_setting(OLD_GOAL_KEY, None)?;
        Ok(())
    }

    fn goal(&self, key: &str) -> Result<Option<f64>> {
        self.migrate_old_goal()?;
        Ok(self.store.setting(key)?.and_then(|v| v.parse().ok()))
    }

    /// 예전 합계 목표는 사실상 라이딩 목표로 쓰였으므로 라이딩 목표로 옮긴다 (한 번만).
    fn migrate_old_goal(&self) -> Result<()> {
        if let Some(old) = self.store.setting(OLD_GOAL_KEY)? {
            if self.store.setting(GOAL_CYCLING_KEY)?.is_none() {
                self.store.set_setting(GOAL_CYCLING_KEY, Some(&old))?;
            }
            self.store.set_setting(OLD_GOAL_KEY, None)?;
        }
        Ok(())
    }

    pub(crate) fn post_status(&self, activity_id: &str) -> Result<PostStatus> {
        let Some(post) = self.store.post(activity_id)? else {
            return Ok(PostStatus::None);
        };
        if post.published_at.is_some() {
            return Ok(PostStatus::Published);
        }
        let has_card = self
            .store
            .photos_for(activity_id)?
            .iter()
            .any(|p| p.card_path.is_some());
        Ok(if has_card {
            PostStatus::Ready
        } else {
            PostStatus::None
        })
    }
}
