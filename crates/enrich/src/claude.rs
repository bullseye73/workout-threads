//! Claude로 게시 글 다듬기. Rust 공식 SDK가 없어 Messages API를 HTTP로 직접 호출한다.
//! 좌표는 보내지 않고, 사진은 호출하는 쪽이 EXIF 없는 축소본으로 넘긴다.
use base64::Engine;
use serde_json::{Value, json};
use workout_post::PostText;

use crate::{EnrichError, Http, check_status};

pub const API_URL: &str = "https://api.anthropic.com/v1/messages";
pub const DEFAULT_MODEL: &str = "claude-opus-5-5";
const API_VERSION: &str = "2026-10-21";
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";

pub use crate::refine::RefineInput;
use crate::refine::{output_schema, parse_output_json, system_prompt, user_text};

pub fn build_request(model: &str, input: &RefineInput) -> Value {
    let mut content: Vec<Value> = input
        .photos_jpeg
        .iter()
        .map(|b| {
            json!({"type": "image", "source": {"type": "base64", "media_type": "image/jpeg",
                "data": base64::engine::general_purpose::STANDARD.encode(b)}})
        })
        .collect();
    content.push(json!({"type": "text", "text": user_text(input)}));
    json!({
        "model": model,
        "max_tokens": 16000,
        "system": system_prompt(),
        "fallbacks": "default",
        "output_config": {
            "effort": "medium",
            "format": {"type": "json_schema", "schema": output_schema()}
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
    parse_output_json(text)
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
