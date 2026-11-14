//! Gemini로 게시 글 다듬기 (generateContent REST). 키는 URL이 아니라 x-goog-api-key 헤더로 보낸다.
use base64::Engine;
use serde_json::{Value, json};
use workout_post::PostText;

use crate::refine::{RefineInput, output_schema, parse_output_json, system_prompt, user_text};
use crate::{EnrichError, Http};

pub const DEFAULT_MODEL: &str = "gemini-3.8-flash";
const BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";

/// "models/…" 접두어는 떼어 낸다.
fn clean_model(model: &str) -> &str {
    model.trim().strip_prefix("models/").unwrap_or(model.trim())
}

fn valid_model(model: &str) -> bool {
    !model.is_empty()
        && model
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
}

pub fn endpoint(model: &str) -> String {
    format!("{BASE}/{}:generateContent", clean_model(model))
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
            "maxOutputTokens": 16384
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
    let reason = cand["finishReason"].as_str().unwrap_or("STOP");
    if reason == "MAX_TOKENS" {
        return Err(EnrichError::Truncated);
    }
    // 생각(thought) 부분은 빼고 답만 모은다.
    let text: String = cand["content"]["parts"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|p| p["thought"] != true)
        .filter_map(|p| p["text"].as_str())
        .collect();
    if text.trim().is_empty() {
        // STOP이 아닌 이유로 글 없이 끝났으면 (SAFETY, IMAGE_SAFETY, OTHER …) 거절로 알린다.
        return Err(if reason == "STOP" {
            EnrichError::Parse("글이 없는 응답".into())
        } else {
            EnrichError::Refused(Some(reason.to_string()))
        });
    }
    if reason != "STOP" {
        return Err(EnrichError::Refused(Some(reason.to_string())));
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
    let model = clean_model(model);
    if !valid_model(model) {
        return Err(EnrichError::Parse(format!(
            "모델 이름을 확인해 주세요 ({model})"
        )));
    }
    let res = http.post_json(
        &endpoint(model),
        &[("x-goog-api-key", api_key)],
        &build_request(input),
    )?;
    if !(200..300).contains(&res.status) {
        return Err(EnrichError::Status(
            res.status,
            error_message(res.status, &res.body, model),
        ));
    }
    parse_response(&res.body)
}

/// 오류 응답을 사람이 읽을 문구로.
fn error_message(status: u16, body: &[u8], model: &str) -> String {
    let v: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
    let raw = String::from_utf8_lossy(body);
    match status {
        401 | 403 => "API 키를 확인해 주세요".into(),
        400 if raw.contains("API_KEY_INVALID") || raw.contains("API key not valid") => {
            "API 키를 확인해 주세요".into()
        }
        404 => format!("모델 이름을 확인해 주세요 ({model})"),
        429 => "사용 한도를 넘었어요. 잠시 후 다시 시도해 주세요".into(),
        _ => v["error"]["message"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| format!("서버 오류({status})")),
    }
}
