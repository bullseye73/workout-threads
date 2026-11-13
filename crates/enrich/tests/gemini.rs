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
