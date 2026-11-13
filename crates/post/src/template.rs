//! 초안 A 형식 템플릿 (반말체, 출발·도착 지점 이름 없음).
use workout_core::chrono::{Datelike, Timelike};
use workout_core::format::{hms, pace_per_km, speed_kmh, time_of_day_ko, weekday_ko};
use workout_core::model::Sport;

use crate::PostError;
use crate::context::{PostContext, PostText};

/// 글 작성기. 지금은 템플릿, 3단계에서 Claude 다듬기가 추가된다.
pub trait DraftWriter {
    fn write(&self, ctx: &PostContext) -> Result<PostText, PostError>;
}

pub struct TemplateWriter;

const Q_CYCLING: &[&str] = &["다들 이번 주말 어디로 달렸음?", "라이딩 코스 추천 좀 해줘"];
const Q_RUNNING: &[&str] = &["다들 이번 주 몇 km 뛰었음?", "러닝 코스 추천해줄 사람?"];
const Q_OTHER: &[&str] = &["다들 오늘 운동했음?"];

impl DraftWriter for TemplateWriter {
    fn write(&self, ctx: &PostContext) -> Result<PostText, PostError> {
        let h = &ctx.activity;
        let sm = &h.summary;
        let local = h.start_local();
        let km = sm.distance_m / 1000.0;
        let (dist, emoji) = match h.sport {
            Sport::Cycling => (format!("{}km", km.round() as i64), "🚲"),
            Sport::Running => (format!("{km:.1}km"), "🏃"),
            _ => (format!("{km:.1}km"), "👟"),
        };
        let place = ctx
            .place
            .as_ref()
            .map(|p| format!("{} ", p.name))
            .unwrap_or_default();
        let mut lines = vec![format!(
            "{} {} {place}{dist} {emoji}",
            weekday_ko(local.date()),
            time_of_day_ko(local.hour())
        )];
        if !ctx.route_points.is_empty() {
            lines.push(format!("{} 찍고 복귀", ctx.route_points.join(" → ")));
        }
        let mut rec = vec![if h.sport == Sport::Cycling {
            format!("평속 {:.1}km/h", speed_kmh(sm.moving_s, sm.distance_m))
        } else {
            format!("페이스 {}/km", pace_per_km(sm.moving_s, sm.distance_m))
        }];
        rec.push(format!("이동 {}", hms(sm.moving_s)));
        if let Some(hr) = sm.avg_hr {
            rec.push(format!("평균 심박 {hr}"));
        }
        lines.push(rec.join(" / "));
        if !ctx.photo_km_marks.is_empty() {
            let marks: Vec<String> = ctx
                .photo_km_marks
                .iter()
                .map(|k| format!("{k:.1}km"))
                .collect();
            lines.push(format!("사진은 {} 지점에서 찍었음", marks.join(", ")));
        }
        if let Some(w) = &ctx.weather {
            lines.push(format!("날씨 {}, {:.0}도", w.description, w.temp_c));
        }
        let qs = match h.sport {
            Sport::Cycling => Q_CYCLING,
            Sport::Running => Q_RUNNING,
            _ => Q_OTHER,
        };
        lines.push(qs[local.ordinal() as usize % qs.len()].to_string());
        Ok(PostText {
            body: lines.join("\n"),
            hashtags: hashtags(ctx),
        })
    }
}

fn hashtags(ctx: &PostContext) -> Vec<String> {
    let base: &[&str] = match ctx.activity.sport {
        Sport::Running => &["러닝", "러닝기록"],
        Sport::Cycling => &["라이딩", "자전거"],
        _ => &["운동기록"],
    };
    let words = base
        .iter()
        .map(|s| s.to_string())
        .chain(ctx.place.iter().map(|p| p.name.clone()))
        .chain(ctx.route_points.iter().cloned());
    let mut out: Vec<String> = Vec::new();
    for w in words {
        let tag = format!("#{}", w.split_whitespace().collect::<String>());
        if tag.len() > 1 && !out.contains(&tag) {
            out.push(tag);
        }
    }
    out
}
