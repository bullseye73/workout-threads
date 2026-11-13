mod common;
use common::*;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;
use workout_gdrive::DriveError;
use workout_gdrive::oauth::*;

fn cfg() -> ClientConfig {
    ClientConfig {
        client_id: "cid.apps.googleusercontent.com".into(),
        client_secret: "sec".into(),
    }
}

#[test]
fn pkce_matches_rfc7636_example() {
    let p = Pkce::from_verifier("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk");
    assert_eq!(p.challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    let fresh = Pkce::new();
    assert!(fresh.verifier.len() >= 43 && fresh.verifier.len() <= 128);
    assert!(
        fresh
            .verifier
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-._~".contains(c))
    );
    assert_ne!(Pkce::new().verifier, fresh.verifier);
    assert_ne!(new_state(), new_state());
}

#[test]
fn auth_url_has_required_params() {
    let p = Pkce::from_verifier("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk");
    let u = url::Url::parse(&auth_url(&cfg(), "http://127.0.0.1:5555", &p, "st")).unwrap();
    assert!(u.as_str().starts_with(AUTH_URL));
    let q: std::collections::HashMap<_, _> = u.query_pairs().into_owned().collect();
    assert_eq!(q["client_id"], "cid.apps.googleusercontent.com");
    assert_eq!(q["redirect_uri"], "http://127.0.0.1:5555");
    assert_eq!(q["response_type"], "code");
    assert_eq!(q["scope"], SCOPE);
    assert_eq!(
        q["code_challenge"],
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
    );
    assert_eq!(q["code_challenge_method"], "S256");
    assert_eq!(q["state"], "st");
    assert_eq!(q["access_type"], "offline");
    assert_eq!(q["prompt"], "consent");
    assert!(!q.contains_key("client_secret"));
}

#[test]
fn callback_parsing() {
    assert!(
        matches!(parse_callback("GET /?code=abc&state=s1 HTTP/1.1", "s1"), Ok(Callback::Code(c)) if c == "abc")
    );
    assert!(matches!(
        parse_callback("GET /?error=access_denied&state=s1 HTTP/1.1", "s1"),
        Ok(Callback::Denied(_))
    ));
    assert!(matches!(
        parse_callback("GET /?code=abc&state=other HTTP/1.1", "s1"),
        Err(DriveError::StateMismatch)
    ));
    assert!(parse_callback("GET /favicon.ico HTTP/1.1", "s1").is_err());
}

fn hit(uri: &str, path: &str) -> String {
    let mut s = TcpStream::connect(uri.trim_start_matches("http://")).unwrap();
    s.write_all(format!("GET {path} HTTP/1.1\r\nHost: x\r\n\r\n").as_bytes())
        .unwrap();
    let mut out = String::new();
    let _ = s.read_to_string(&mut out);
    out
}

#[test]
fn loopback_receives_code() {
    let srv = LoopbackServer::bind().unwrap();
    let uri = srv.redirect_uri();
    assert!(uri.starts_with("http://127.0.0.1:"));
    let h = std::thread::spawn(move || hit(&uri, "/?code=xyz&state=st"));
    assert_eq!(srv.wait("st", Duration::from_secs(5)).unwrap(), "xyz");
    assert!(h.join().unwrap().contains("로그인 완료"));
}

#[test]
fn loopback_reports_denied() {
    let srv = LoopbackServer::bind().unwrap();
    let uri = srv.redirect_uri();
    let h = std::thread::spawn(move || hit(&uri, "/?error=access_denied&state=st"));
    assert!(matches!(
        srv.wait("st", Duration::from_secs(5)),
        Err(DriveError::Cancelled)
    ));
    assert!(h.join().unwrap().contains("취소"));
}

#[test]
fn loopback_ignores_stray_requests_then_gets_code() {
    let srv = LoopbackServer::bind().unwrap();
    let uri = srv.redirect_uri();
    let h = std::thread::spawn(move || {
        hit(&uri, "/favicon.ico");
        hit(&uri, "/?code=ok&state=st")
    });
    assert_eq!(srv.wait("st", Duration::from_secs(5)).unwrap(), "ok");
    h.join().unwrap();
}

#[test]
fn loopback_times_out() {
    let srv = LoopbackServer::bind().unwrap();
    assert!(matches!(
        srv.wait("st", Duration::from_millis(200)),
        Err(DriveError::Timeout)
    ));
}

#[test]
fn exchange_sends_pkce_and_parses_tokens() {
    let http = MockHttp::new(vec![resp(200, "token_ok.json")]);
    let p = Pkce::from_verifier("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk");
    let t = exchange_code(
        &http,
        &cfg(),
        "the-code",
        "http://127.0.0.1:5555",
        &p,
        1_000,
    )
    .unwrap();
    assert_eq!(t.access, "ya29.test");
    assert_eq!(t.refresh.as_deref(), Some("1//refresh"));
    assert_eq!(t.expires_at, 1_000 + 3599);
    let f = http.last_form(TOKEN_URL);
    let get = |k: &str| f.iter().find(|(a, _)| a == k).map(|(_, v)| v.clone());
    assert_eq!(get("grant_type").as_deref(), Some("authorization_code"));
    assert_eq!(get("code").as_deref(), Some("the-code"));
    assert_eq!(
        get("code_verifier").as_deref(),
        Some("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk")
    );
    assert_eq!(
        get("redirect_uri").as_deref(),
        Some("http://127.0.0.1:5555")
    );
}

#[test]
fn refresh_keeps_old_refresh_token_and_maps_invalid_grant() {
    let http = MockHttp::new(vec![
        resp(200, "token_refreshed.json"),
        resp(400, "token_invalid_grant.json"),
    ]);
    let t = refresh(&http, &cfg(), "1//refresh", 50).unwrap();
    assert_eq!(t.access, "ya29.new");
    assert_eq!(t.refresh.as_deref(), Some("1//refresh"));
    assert!(matches!(
        refresh(&http, &cfg(), "1//refresh", 50),
        Err(DriveError::NeedLogin)
    ));
}
