mod common;
use common::*;
use workout_enrich::claude::*;
use workout_enrich::{EnrichError, HttpResponse};

#[test]
fn request_has_schema_fallback_and_no_coords() {
    let req = build_request(DEFAULT_MODEL, &input_with_route(vec![]));
    assert_eq!(req["model"], "claude-opus-5-5");
    assert_eq!(req["fallbacks"], "default");
    assert_eq!(req["output_config"]["format"]["type"], "json_schema");
    assert_eq!(req["output_config"]["effort"], "medium");
    assert!(req.get("thinking").is_none());
    let s = req.to_string();
    assert!(!s.contains("\"lat") && !s.contains("\"lon") && !s.contains("37.5"));
    assert!(s.contains("반포") && s.contains("16.0"));
}

#[test]
fn request_includes_photos_only_when_given() {
    let none = build_request(DEFAULT_MODEL, &input_with_route(vec![]));
    assert_eq!(none["messages"][0]["content"].as_array().unwrap().len(), 1);
    let one = build_request(DEFAULT_MODEL, &input_with_route(vec![vec![0xFF, 0xD8]]));
    assert_eq!(one["messages"][0]["content"][0]["type"], "image");
    assert_eq!(
        one["messages"][0]["content"][0]["source"]["media_type"],
        "image/jpeg"
    );
}

#[test]
fn parses_ok_and_cleans_hashtags() {
    let t = parse_response(&fixture("claude_ok.json")).unwrap();
    assert!(t.body.starts_with("일요일 오후"));
    assert_eq!(t.hashtags, vec!["#라이딩", "#한강"]);
}

#[test]
fn refusal_truncation_and_bad_text() {
    assert!(
        matches!(parse_response(&fixture("claude_refusal.json")), Err(EnrichError::Refused(Some(c))) if c == "general_harms")
    );
    assert!(matches!(
        parse_response(&fixture("claude_max_tokens.json")),
        Err(EnrichError::Truncated)
    ));
    assert!(matches!(
        parse_response(&fixture("claude_text.json")),
        Err(EnrichError::Parse(_))
    ));
}

#[test]
fn refine_sends_headers_and_maps_401() {
    let http = MockHttp::new(vec![HttpResponse {
        status: 401,
        body: b"{}".to_vec(),
    }]);
    assert!(matches!(
        refine(&http, "k", DEFAULT_MODEL, &input_with_route(vec![])),
        Err(EnrichError::Status(401, _))
    ));
    let h = http.last_headers();
    assert!(h.contains(&("anthropic-version".into(), "2026-10-21".into())));
    assert!(h.contains(&(
        "anthropic-beta".into(),
        "server-side-fallback-2026-07-01".into()
    )));
    assert!(h.contains(&("x-api-key".into(), "k".into())));
}

#[test]
fn refine_ok_roundtrip() {
    let http = MockHttp::new(vec![HttpResponse {
        status: 200,
        body: fixture("claude_ok.json"),
    }]);
    let t = refine(&http, "k", DEFAULT_MODEL, &input_with_route(vec![])).unwrap();
    assert_eq!(t.hashtags[0], "#라이딩");
}

#[test]
fn refine_does_not_send_duplicate_content_type() {
    let http = MockHttp::new(vec![HttpResponse {
        status: 200,
        body: fixture("claude_ok.json"),
    }]);
    refine(&http, "k", DEFAULT_MODEL, &input_with_route(vec![])).unwrap();
    assert!(
        !http
            .last_headers()
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("content-type"))
    );
}

#[test]
fn http_timeouts_short_for_lookups_long_for_claude() {
    let (lookup, claude) = workout_enrich::ReqwestHttp::TIMEOUTS;
    assert!(lookup <= std::time::Duration::from_secs(15));
    assert_eq!(claude, std::time::Duration::from_secs(120));
}
