//! Claude로 게시 글 다듬기. Rust 공식 SDK가 없어 Messages API를 HTTP로 직접 호출한다.
//! 좌표는 보내지 않고, 사진은 호출하는 쪽이 EXIF 없는 축소본으로 넘긴다.
use base64::Engine;
use serde_json::{Value, json};
use workout_core::chrono::{Datelike, Timelike};
use workout_core::format::{hms, pace_per_km, speed_kmh, sport_ko, time_of_day_ko, weekday_ko};
use workout_core::model::Sport;
use workout_post::{PostContext, PostText};

use crate::{EnrichError, Http, check_status};

pub const API_URL: &str = "https://api.anthropic.com/v1/messages";
pub const DEFAULT_MODEL: &str = "claude-opus-5-5";
const API_VERSION: &str = "2026-10-21";
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";

pub struct RefineInput {
    pub ctx: PostContext,
    pub draft: PostText,
    /// EXIF 없는 JPEG 바이트 (사용자가 "사진도 보내기"를 켰을 때만).
    pub photos_jpeg: Vec<Vec<u8>>,
}

pub fn system_prompt() -> &'static str {
    "너는 사용자의 러닝·라이딩 기록을 스레드(Threads)에 올릴 짧은 글로 다듬는다.\n\
     규칙:\n\
     - 말투는 반말체로 끝낸다 (예: \"~했음\", \"~더라\").\n\
     - 순서: 1) 요일·시간대 + 장소 + 거리 + 이모지 한 줄, 2) 지나간 경로 한 줄 (A → B 찍고 복귀), \
     3) 기록 한 줄 (평속 또는 페이스 / 이동 시간 / 평균 심박), 4) 사진 찍은 지점 이야기 한두 줄, \
     5) 날씨·기온 한 줄, 6) 답글을 부르는 질문 한 줄.\n\
     - 주어진 숫자(거리, 시간, 속도, 페이스, 심박, km 지점)는 바꾸지 않는다.\n\
     - 출발·도착 지점 이름, 주소, 좌표는 쓰지 않는다.\n\
     - 주어지지 않은 정보는 지어내지 말고 그 줄을 뺀다.\n\
     - 사진이 있으면 사진에 보이는 장면을 한 줄로 자연스럽게 녹인다.\n\
     - 해시태그는 본문에 넣지 말고 hashtags 배열에 3~7개 넣는다 (종목, 장소, 코스 키워드).\n\
     - 초안의 사실은 유지하되 문장은 자연스럽게 고쳐도 된다."
}

fn facts(ctx: &PostContext) -> Value {
    let h = &ctx.activity;
    let sm = &h.summary;
    let local = h.start_local();
    let pace_or_speed = if h.sport == Sport::Cycling {
        format!("평속 {:.1}km/h", speed_kmh(sm.moving_s, sm.distance_m))
    } else if sm.distance_m > 0.0 {
        format!("페이스 {}/km", pace_per_km(sm.moving_s, sm.distance_m))
    } else {
        String::new()
    };
    json!({
        "sport": sport_ko(h.sport),
        "weekday": weekday_ko(local.date()),
        "time_of_day": time_of_day_ko(local.hour()),
        "date": format!("{}월 {}일", local.month(), local.day()),
        "distance_km": format!("{:.2}", sm.distance_m / 1000.0),
        "moving": hms(sm.moving_s),
        "pace_or_speed": pace_or_speed,
        "avg_hr": sm.avg_hr,
        "place": ctx.place.as_ref().map(|p| p.name.clone()),
        "place_estimated": ctx.place.as_ref().map(|p| p.estimated),
        "route_points": ctx.route_points,
        "weather": ctx.weather.as_ref().map(|w| format!("{} {:.0}도", w.description, w.temp_c)),
        "photo_km_marks": ctx.photo_km_marks.iter().map(|k| format!("{k:.1}km")).collect::<Vec<_>>(),
    })
}

pub fn build_request(model: &str, input: &RefineInput) -> Value {
    let mut content: Vec<Value> = input
        .photos_jpeg
        .iter()
        .map(|b| {
            json!({"type": "image", "source": {"type": "base64", "media_type": "image/jpeg",
                "data": base64::engine::general_purpose::STANDARD.encode(b)}})
        })
        .collect();
    let text = format!(
        "운동 정보:\n{}\n\n템플릿 초안:\n{}\n\n초안 해시태그: {}",
        serde_json::to_string_pretty(&facts(&input.ctx)).unwrap_or_default(),
        input.draft.body,
        input.draft.hashtags.join(" ")
    );
    content.push(json!({"type": "text", "text": text}));
    json!({
        "model": model,
        "max_tokens": 16000,
        "system": system_prompt(),
        "fallbacks": "default",
        "output_config": {
            "effort": "medium",
            "format": {"type": "json_schema", "schema": {
                "type": "object",
                "properties": {"body": {"type": "string"}, "hashtags": {"type": "array", "items": {"type": "string"}}},
                "required": ["body", "hashtags"],
                "additionalProperties": false
            }}
        },
        "messages": [{"role": "user", "content": content}]
    })
}

pub fn parse_response(json: &[u8]) -> Result<PostText, EnrichError> {
    let v: Value = serde_json::from_slice(json).map_err(|e| EnrichError::Parse(e.to_string()))?;
    match v["stop_reason"].as_str() {
        Some("refusal") => {
            return Err(EnrichError::Refused(
                v["stop_details"]["category"].as_str().map(str::to_string),
            ));
        }
        Some("max_tokens") => return Err(EnrichError::Truncated),
        _ => {}
    }
    let text = v["content"]
        .as_array()
        .and_then(|c| c.iter().find(|b| b["type"] == "text"))
        .and_then(|b| b["text"].as_str())
        .ok_or_else(|| EnrichError::Parse("글이 없는 응답".into()))?;
    let out: Value = serde_json::from_str(text).map_err(|e| EnrichError::Parse(e.to_string()))?;
    let body = out["body"]
        .as_str()
        .ok_or_else(|| EnrichError::Parse("본문 없음".into()))?
        .trim()
        .to_string();
    let mut hashtags: Vec<String> = Vec::new();
    for t in out["hashtags"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|t| t.as_str())
    {
        let word: String = t
            .trim()
            .trim_start_matches('#')
            .split_whitespace()
            .collect();
        let tag = format!("#{word}");
        if !word.is_empty() && !hashtags.contains(&tag) {
            hashtags.push(tag);
        }
    }
    Ok(PostText { body, hashtags })
}

/// 다듬은 글을 돌려준다 (저장하지 않음).
pub fn refine(
    http: &dyn Http,
    api_key: &str,
    model: &str,
    input: &RefineInput,
) -> Result<PostText, EnrichError> {
    if api_key.trim().is_empty() {
        return Err(EnrichError::NoKey);
    }
    let headers = [
        ("x-api-key", api_key),
        ("anthropic-version", API_VERSION),
        ("anthropic-beta", FALLBACK_BETA),
    ];
    let res = http.post_json(API_URL, &headers, &build_request(model, input))?;
    if res.status == 401 {
        return Err(EnrichError::Status(401, "API 키를 확인해 주세요".into()));
    }
    check_status(&res)?;
    parse_response(&res.body)
}
