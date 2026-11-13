//! Gemini로 게시 글 다듬기 (generateContent REST). 키는 URL이 아니라 x-goog-api-key 헤더로 보낸다.
use base64::Engine;
use serde_json::{Value, json};
use workout_post::PostText;

use crate::refine::{RefineInput, output_schema, parse_output_json, system_prompt, user_text};
use crate::{EnrichError, Http, check_status};

pub const DEFAULT_MODEL: &str = "gemini-3.8-flash";
const BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";

pub fn endpoint(model: &str) -> String {
    format!("{BASE}/{model}:generateContent")
}

pub fn build_request(input: &RefineInput) -> Value {
    let mut parts: Vec<Value> = input
        .photos_jpeg
        .iter()
        .map(|b| {
            json!({"inlineData": {"mimeType": "image/jpeg",
                "data": base64::engine::general_purpose::STANDARD.encode(b)}})
        })
        .collect();
    parts.push(json!({"text": user_text(input)}));
    json!({
        "systemInstruction": {"parts": [{"text": system_prompt()}]},
        "contents": [{"role": "user", "parts": parts}],
        "generationConfig": {
            "responseMimeType": "application/json",
            "responseJsonSchema": output_schema(),
            "maxOutputTokens": 8192
        }
    })
}

/// 차단·잘림을 구분해 글을 읽는다.
pub fn parse_response(json: &[u8]) -> Result<PostText, EnrichError> {
    let v: Value = serde_json::from_slice(json).map_err(|e| EnrichError::Parse(e.to_string()))?;
    if let Some(reason) = v["promptFeedback"]["blockReason"].as_str() {
        return Err(EnrichError::Refused(Some(reason.to_string())));
    }
    let cand = v["candidates"]
        .get(0)
        .ok_or_else(|| EnrichError::Parse("후보 없음".into()))?;
    match cand["finishReason"].as_str() {
        Some("MAX_TOKENS") => return Err(EnrichError::Truncated),
        Some(r @ ("SAFETY" | "RECITATION" | "PROHIBITED_CONTENT" | "BLOCKLIST" | "SPII")) => {
            return Err(EnrichError::Refused(Some(r.to_string())));
        }
        _ => {}
    }
    let text: String = cand["content"]["parts"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| p["text"].as_str())
        .collect();
    if text.trim().is_empty() {
        return Err(EnrichError::Parse("글이 없는 응답".into()));
    }
    parse_output_json(&text)
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
    let res = http.post_json(
        &endpoint(model),
        &[("x-goog-api-key", api_key)],
        &build_request(input),
    )?;
    if res.status == 401 || res.status == 403 {
        return Err(EnrichError::Status(
            res.status,
            "API 키를 확인해 주세요".into(),
        ));
    }
    check_status(&res)?;
    parse_response(&res.body)
}
