mod common;
use common::*;
use workout_enrich::*;

#[test]
fn send_form_is_recorded() {
    let http = MockHttp::new(vec![HttpResponse {
        status: 200,
        body: b"{}".to_vec(),
    }]);
    http.send(HttpRequest {
        method: Method::Post,
        url: "https://example.test/token".into(),
        headers: vec![],
        body: Body::Form(vec![("grant_type".into(), "refresh_token".into())]),
        timeout: TimeoutClass::Lookup,
    })
    .unwrap();
    assert_eq!(
        http.last_form("token"),
        vec![("grant_type".to_string(), "refresh_token".to_string())]
    );
    assert_eq!(http.last_method("token"), Some(Method::Post));
}

#[test]
fn get_and_post_json_use_send_defaults() {
    let ok = || HttpResponse {
        status: 200,
        body: b"{}".to_vec(),
    };
    let http = MockHttp::new(vec![ok(), ok()]);
    http.get("https://a.test/x", &[]).unwrap();
    http.post_json("https://b.test/y", &[], &serde_json::json!({"k": 1}))
        .unwrap();
    assert_eq!(http.last_method("a.test"), Some(Method::Get));
    assert_eq!(http.last_timeout("a.test"), Some(TimeoutClass::Lookup));
    assert_eq!(http.last_method("b.test"), Some(Method::Post));
    assert_eq!(http.last_timeout("b.test"), Some(TimeoutClass::Long));
    assert_eq!(http.last_body("b.test")["k"], 1);
}

#[test]
fn bytes_body_is_recorded() {
    let http = MockHttp::new(vec![HttpResponse {
        status: 200,
        body: vec![],
    }]);
    http.send(HttpRequest {
        method: Method::Patch,
        url: "https://up.test/f".into(),
        headers: vec![("Authorization".into(), "Bearer t".into())],
        body: Body::Bytes {
            content_type: "image/jpeg".into(),
            data: vec![1, 2, 3],
        },
        timeout: TimeoutClass::Long,
    })
    .unwrap();
    assert_eq!(http.last_bytes("up.test"), vec![1, 2, 3]);
    assert_eq!(http.last_method("up.test"), Some(Method::Patch));
}
