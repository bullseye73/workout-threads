mod common;
use common::*;
use workout_service::secrets::*;

#[test]
fn named_secrets_are_separate() {
    let s = MemoryStore::default();
    s.set(CLAUDE_KEY, Some("c")).unwrap();
    s.set(GOOGLE_REFRESH, Some("r")).unwrap();
    assert_eq!(s.get(CLAUDE_KEY).as_deref(), Some("c"));
    assert_eq!(s.get(GOOGLE_REFRESH).as_deref(), Some("r"));
    s.set(CLAUDE_KEY, None).unwrap();
    assert!(s.get(CLAUDE_KEY).is_none());
    assert_eq!(s.get(GOOGLE_REFRESH).as_deref(), Some("r"));
}

#[test]
fn claude_key_does_not_touch_google_tokens() {
    let (_t, mut lib, _h) = temp_library_with_mock();
    lib.set_claude_key(Some("k".into())).unwrap();
    assert!(lib.claude_status().unwrap().has_key);
    assert!(lib.secrets_for_tests().get(GOOGLE_REFRESH).is_none());
    assert_eq!(
        lib.secrets_for_tests().get(CLAUDE_KEY).as_deref(),
        Some("k")
    );
}
