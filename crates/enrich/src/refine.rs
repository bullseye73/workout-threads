//! 글 다듬기 공통 틀: 어느 AI(Gemini, Claude)를 쓰든 같은 지시문·사실 정보·출력 형식을 쓴다.
//! 좌표는 보내지 않는다. 사진은 호출하는 쪽이 EXIF 없는 축소본으로 넘긴다.
use serde_json::{Value, json};
use workout_core::chrono::{Datelike, Timelike};
use workout_core::format::{hms, pace_per_km, speed_kmh, sport_ko, time_of_day_ko, weekday_ko};
use workout_core::model::Sport;
use workout_post::{PostContext, PostText};

use crate::{EnrichError, Http};

pub struct RefineInput {
    pub ctx: PostContext,
    pub draft: PostText,
    /// EXIF 없는 JPEG 바이트 (사용자가 "사진도 보내기"를 켰을 때만).
    pub photos_jpeg: Vec<Vec<u8>>,
}

/// 글 다듬기에 쓸 AI.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Provider {
    Gemini,
    Claude,
}

impl Provider {
    pub const ALL: [Provider; 2] = [Provider::Gemini, Provider::Claude];

    pub fn id(self) -> &'static str {
        match self {
            Provider::Gemini => "gemini",
            Provider::Claude => "claude",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.id() == s)
    }

    /// 키 보관함 이름 (Claude는 3단계부터 쓰던 이름 그대로).
    pub fn key_name(self) -> &'static str {
        match self {
            Provider::Gemini => "gemini_api_key",
            Provider::Claude => "anthropic_api_key",
        }
    }

    /// 설정 테이블의 모델 키.
    pub fn model_setting(self) -> &'static str {
        match self {
            Provider::Gemini => "gemini_model",
            Provider::Claude => "claude_model",
        }
    }

    pub fn default_model(self) -> &'static str {
        match self {
            Provider::Gemini => crate::gemini::DEFAULT_MODEL,
            Provider::Claude => crate::claude::DEFAULT_MODEL,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Provider::Gemini => "Gemini",
            Provider::Claude => "Claude",
        }
    }

    /// 사진을 받는 회사 (안내 문구용).
    pub fn company(self) -> &'static str {
        match self {
            Provider::Gemini => "Google",
            Provider::Claude => "Anthropic",
        }
    }
}

/// 고른 AI로 다듬는다 (저장하지 않음).
pub fn refine_with(
    provider: Provider,
    http: &dyn Http,
    key: &str,
    model: &str,
    input: &RefineInput,
) -> Result<PostText, EnrichError> {
    match provider {
        Provider::Gemini => crate::gemini::refine(http, key, model, input),
        Provider::Claude => crate::claude::refine(http, key, model, input),
    }
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
     - 초안의 사실은 유지하되 문장은 자연스럽게 고쳐도 된다.\n\
     - 결과는 {\"body\": 본문, \"hashtags\": [해시태그]} JSON 하나로만 답한다."
}

/// 보낼 사실 정보 (좌표 없음).
pub fn facts(ctx: &PostContext) -> Value {
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

/// 사용자 메시지: 사실 정보 + 초안 + 해시태그.
pub fn user_text(input: &RefineInput) -> String {
    format!(
        "운동 정보:\n{}\n\n템플릿 초안:\n{}\n\n초안 해시태그: {}",
        serde_json::to_string_pretty(&facts(&input.ctx)).unwrap_or_default(),
        input.draft.body,
        input.draft.hashtags.join(" ")
    )
}

/// 출력 형식 {body, hashtags}.
pub fn output_schema() -> Value {
    json!({
        "type": "object",
        "properties": {"body": {"type": "string"}, "hashtags": {"type": "array", "items": {"type": "string"}}},
        "required": ["body", "hashtags"],
        "additionalProperties": false
    })
}

/// AI가 준 JSON 글을 읽는다. 코드 펜스(```json)를 벗기고, 해시태그를 정리하고, 빈 본문은 거절한다.
pub fn parse_output_json(text: &str) -> Result<PostText, EnrichError> {
    let t = text.trim();
    let t = t
        .strip_prefix("```json")
        .or_else(|| t.strip_prefix("```"))
        .map(|r| r.trim_end().trim_end_matches("```").trim())
        .unwrap_or(t);
    let out: Value = serde_json::from_str(t).map_err(|e| EnrichError::Parse(e.to_string()))?;
    let body = out["body"]
        .as_str()
        .ok_or_else(|| EnrichError::Parse("본문 없음".into()))?
        .trim()
        .to_string();
    if body.is_empty() {
        return Err(EnrichError::Parse("빈 본문".into()));
    }
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
