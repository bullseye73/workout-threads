mod common;
use common::*;
use workout_enrich::gemini::*;
use workout_enrich::refine::{Provider, refine_with};
use workout_enrich::{EnrichError, HttpResponse};

#[test]
fn request_shape_and_privacy() {
    let req = build_request(&input_with_route(vec![]));
    assert_eq!(
        req["generationConfig"]["responseMimeType"],
        "application/json"
    );
    assert_eq!(
        req["generationConfig"]["responseJsonSchema"]["required"][0],
        "body"
    );
    assert!(
        req["systemInstruction"]["parts"][0]["text"]
            .as_str()
            .unwrap()
            .contains("반말체")
    );
    let parts = req["contents"][0]["parts"].as_array().unwrap();
    assert_eq!(parts.len(), 1);
    let s = req.to_string();
    assert!(!s.contains("\"lat") && !s.contains("\"lon") && !s.contains("37.5"));
    assert!(s.contains("반포") && s.contains("16.0"));
}

#[test]
fn photos_only_when_given() {
    let req = build_request(&input_with_route(vec![vec![0xFF, 0xD8]]));
    assert_eq!(
        req["contents"][0]["parts"][0]["inlineData"]["mimeType"],
        "image/jpeg"
    );
}

#[test]
fn key_goes_in_header_not_url() {
    let http = MockHttp::new(vec![HttpResponse {
        status: 200,
        body: fixture("gemini_ok.json"),
    }]);
    refine(
        &http,
        "SECRET-KEY",
        DEFAULT_MODEL,
        &input_with_route(vec![]),
    )
    .unwrap();
    let url = http.calls()[0].clone();
    assert_eq!(url, endpoint(DEFAULT_MODEL));
    assert!(url.contains("models/gemini-3.8-flash:generateContent") && !url.contains("SECRET-KEY"));
    assert!(
        http.last_headers()
            .contains(&("x-goog-api-key".into(), "SECRET-KEY".into()))
    );
}

#[test]
fn parses_ok_and_cleans_hashtags() {
    let t = parse_response(&fixture("gemini_ok.json")).unwrap();
    assert!(t.body.starts_with("일요일 오후"));
    assert_eq!(t.hashtags, vec!["#라이딩", "#한강"]);
}

#[test]
fn strips_code_fence() {
    assert_eq!(
        parse_response(&fixture("gemini_fenced.json")).unwrap().body,
        "펜스 안 글"
    );
}

#[test]
fn blocked_safety_and_truncated() {
    assert!(
        matches!(parse_response(&fixture("gemini_blocked.json")), Err(EnrichError::Refused(Some(c))) if c == "SAFETY")
    );
    assert!(
        matches!(parse_response(&fixture("gemini_safety.json")), Err(EnrichError::Refused(Some(c))) if c == "SAFETY")
    );
    assert!(matches!(
        parse_response(&fixture("gemini_max_tokens.json")),
        Err(EnrichError::Truncated)
    ));
    assert!(matches!(parse_response(b"{\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"hi\"}]},\"finishReason\":\"STOP\"}]}"), Err(EnrichError::Parse(_))));
}

#[test]
fn forbidden_means_check_key() {
    let http = MockHttp::new(vec![HttpResponse {
        status: 403,
        body: fixture("gemini_forbidden.json"),
    }]);
    let e = refine(&http, "k", DEFAULT_MODEL, &input_with_route(vec![])).unwrap_err();
    assert!(matches!(e, EnrichError::Status(403, ref m) if m.contains("API 키")));
}

#[test]
fn refine_with_routes_by_provider() {
    let http = MockHttp::new(vec![
        HttpResponse {
            status: 200,
            body: fixture("gemini_ok.json"),
        },
        HttpResponse {
            status: 200,
            body: fixture("claude_ok.json"),
        },
    ]);
    refine_with(
        Provider::Gemini,
        &http,
        "k",
        Provider::Gemini.default_model(),
        &input_with_route(vec![]),
    )
    .unwrap();
    refine_with(
        Provider::Claude,
        &http,
        "k",
        Provider::Claude.default_model(),
        &input_with_route(vec![]),
    )
    .unwrap();
    let calls = http.calls();
    assert!(calls[0].contains("generativelanguage.googleapis.com"));
    assert!(calls[1].contains("api.anthropic.com"));
    assert_eq!(Provider::parse("gemini"), Some(Provider::Gemini));
    assert_eq!(Provider::Claude.key_name(), "anthropic_api_key");
    assert_eq!(Provider::Gemini.key_name(), "gemini_api_key");
}

// ---- 리뷰 수정 ----

#[test]
fn messages_do_not_name_claude() {
    let refused = EnrichError::Refused(Some("SAFETY".into())).to_string();
    assert!(
        !refused.contains("Claude") && refused.contains("AI"),
        "{refused}"
    );
    assert!(!EnrichError::NoKey.to_string().contains("Claude"));
}

fn status_msg(status: u16, fixture_name: &str) -> String {
    let http = MockHttp::new(vec![HttpResponse {
        status,
        body: fixture(fixture_name),
    }]);
    match refine(&http, "k", DEFAULT_MODEL, &input_with_route(vec![])).unwrap_err() {
        EnrichError::Status(_, m) => m,
        e => panic!("{e:?}"),
    }
}

#[test]
fn friendly_error_messages() {
    assert!(status_msg(400, "gemini_bad_key.json").contains("API 키"));
    assert!(status_msg(404, "gemini_not_found.json").contains("모델 이름"));
    assert!(status_msg(429, "gemini_quota.json").contains("사용 한도"));
    let other = status_msg(500, "gemini_internal.json");
    assert_eq!(other, "Internal error encountered.");
}

#[test]
fn room_for_thinking_tokens() {
    let req = build_request(&input_with_route(vec![]));
    assert!(req["generationConfig"]["maxOutputTokens"].as_u64().unwrap() >= 16000);
}

#[test]
fn other_finish_reasons_are_refusals() {
    assert!(
        matches!(parse_response(&fixture("gemini_image_safety.json")), Err(EnrichError::Refused(Some(c))) if c == "IMAGE_SAFETY")
    );
    let other = r#"{"candidates":[{"finishReason":"OTHER","content":{"parts":[]}}]}"#.as_bytes();
    assert!(matches!(parse_response(other), Err(EnrichError::Refused(Some(c))) if c == "OTHER"));
}

#[test]
fn tolerant_json_extraction() {
    use workout_enrich::refine::parse_output_json;
    assert_eq!(
        parse_output_json("```JSON\n{\"body\":\"a\",\"hashtags\":[]}\n```")
            .unwrap()
            .body,
        "a"
    );
    assert_eq!(
        parse_output_json("다듬은 결과예요: {\"body\":\"b\",\"hashtags\":[\"x\"]} 끝")
            .unwrap()
            .body,
        "b"
    );
}

#[test]
fn model_names_are_normalized() {
    assert_eq!(
        endpoint("models/gemini-3.8-flash"),
        endpoint("gemini-3.8-flash")
    );
    let http = MockHttp::new(vec![]);
    let e = refine(&http, "k", "gemini/../x?y", &input_with_route(vec![])).unwrap_err();
    assert!(e.to_string().contains("모델 이름"), "{e}");
    assert_eq!(http.calls().len(), 0);
}

#[test]
fn thought_parts_are_skipped() {
    let body = r#"{"candidates":[{"content":{"parts":[{"text":"생각 중...","thought":true},{"text":"{\"body\":\"본문\",\"hashtags\":[]}"}]},"finishReason":"STOP"}]}"#.as_bytes();
    assert_eq!(parse_response(body).unwrap().body, "본문");
}
