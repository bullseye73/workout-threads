mod common;
use common::*;
use workout_service::secrets::{CLAUDE_KEY, GEMINI_KEY};

#[test]
fn default_provider_is_gemini() {
    let (_t, lib, _h) = temp_library_with_mock();
    let s = lib.refine_status().unwrap();
    assert_eq!(s.provider, "gemini");
    assert_eq!(s.label, "Gemini");
    assert_eq!(s.model, "gemini-3.8-flash");
    assert!(!s.has_key);
    assert_eq!(s.providers.len(), 2);
}

#[test]
fn gemini_key_calls_gemini() {
    workout_testkit::require_samples!();
    let (_t, mut lib, http) = temp_library_with_mock();
    let id = import_1025(&mut lib);
    lib.set_refine_key("gemini", Some("g-key".into())).unwrap();
    let r = lib.refine_post(&id, false).unwrap();
    assert!(r.body.starts_with("일요일 오후"));
    assert_eq!(http.count("generativelanguage.googleapis.com"), 1);
    assert_eq!(http.count("anthropic"), 0);
    assert!(!http.calls().iter().any(|u| u.contains("g-key")));
}

#[test]
fn switching_to_claude_uses_claude_key() {
    workout_testkit::require_samples!();
    let (_t, mut lib, http) = temp_library_with_mock();
    let id = import_1025(&mut lib);
    lib.set_refine_key("gemini", Some("g".into())).unwrap();
    lib.set_refine_key("claude", Some("c".into())).unwrap();
    lib.set_refine_provider("claude").unwrap();
    lib.refine_post(&id, false).unwrap();
    assert_eq!(http.count("api.anthropic.com"), 1);
    assert_eq!(http.count("generativelanguage"), 0);
    assert_eq!(
        lib.secrets_for_tests().get(GEMINI_KEY).as_deref(),
        Some("g")
    );
    assert_eq!(
        lib.secrets_for_tests().get(CLAUDE_KEY).as_deref(),
        Some("c")
    );
}

#[test]
fn missing_key_for_chosen_provider_sends_nothing() {
    workout_testkit::require_samples!();
    let (_t, mut lib, http) = temp_library_with_mock();
    let id = import_1025(&mut lib);
    lib.set_refine_key("claude", Some("c".into())).unwrap(); // Gemini가 기본인데 Claude 키만 있음
    let e = lib.refine_post(&id, false).unwrap_err().to_string();
    assert!(e.contains("Gemini") && e.contains("API 키"), "{e}");
    assert_eq!(http.calls().len(), 0);
}

#[test]
fn per_provider_models_and_status_hide_keys() {
    let (_t, mut lib, _h) = temp_library_with_mock();
    lib.set_refine_model("gemini", Some("gemini-3.5-flash-lite".into()))
        .unwrap();
    lib.set_refine_key("gemini", Some("SECRET-G".into()))
        .unwrap();
    let s = lib.refine_status().unwrap();
    assert_eq!(s.model, "gemini-3.5-flash-lite");
    assert!(s.has_key);
    let claude = s.providers.iter().find(|p| p.id == "claude").unwrap();
    assert_eq!(claude.model, "claude-opus-5-5");
    assert!(!claude.has_key);
    assert!(!serde_json::to_string(&s).unwrap().contains("SECRET-G"));
}

#[test]
fn unknown_provider_is_rejected() {
    let (_t, mut lib, _h) = temp_library_with_mock();
    assert!(lib.set_refine_provider("gpt").is_err());
    assert!(lib.set_refine_key("gpt", Some("x".into())).is_err());
    assert_eq!(lib.refine_status().unwrap().provider, "gemini");
}
