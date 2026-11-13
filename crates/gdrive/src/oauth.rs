//! 설치형 앱 OAuth: 시스템 브라우저 → 127.0.0.1 임시 포트로 돌아옴, PKCE(S256) + state.
use base64::Engine;
use sha2::{Digest, Sha256};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};
use workout_enrich::{Body, Http, HttpRequest, Method, TimeoutClass};

use crate::{DriveError, Tokens};

pub const SCOPE: &str = "https://www.googleapis.com/auth/drive.file";
pub const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

#[derive(Clone, Debug)]
pub struct ClientConfig {
    pub client_id: String,
    pub client_secret: String,
}

fn b64url(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn random_token(n_bytes: usize) -> String {
    let mut b = vec![0u8; n_bytes];
    rand::fill(&mut b[..]);
    b64url(&b)
}

#[derive(Clone, Debug)]
pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

impl Pkce {
    /// 무작위 verifier (48바이트 → 64자).
    pub fn new() -> Self {
        Self::from_verifier(&random_token(48))
    }

    pub fn from_verifier(v: &str) -> Self {
        Pkce {
            verifier: v.to_string(),
            challenge: b64url(&Sha256::digest(v.as_bytes())),
        }
    }
}

impl Default for Pkce {
    fn default() -> Self {
        Self::new()
    }
}

pub fn new_state() -> String {
    random_token(24)
}

pub fn auth_url(cfg: &ClientConfig, redirect: &str, pkce: &Pkce, state: &str) -> String {
    let mut u = url::Url::parse(AUTH_URL).expect("고정 URL");
    u.query_pairs_mut()
        .append_pair("client_id", &cfg.client_id)
        .append_pair("redirect_uri", redirect)
        .append_pair("response_type", "code")
        .append_pair("scope", SCOPE)
        .append_pair("code_challenge", &pkce.challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", state)
        .append_pair("access_type", "offline")
        .append_pair("prompt", "consent");
    u.into()
}

#[derive(Debug, PartialEq)]
pub enum Callback {
    Code(String),
    Denied(String),
}

/// "GET /?code=..&state=.. HTTP/1.1" 한 줄을 해석한다.
pub fn parse_callback(request_line: &str, expected_state: &str) -> Result<Callback, DriveError> {
    let target = request_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| DriveError::Parse("요청 줄 없음".into()))?;
    let u = url::Url::parse(&format!("http://localhost{target}"))
        .map_err(|e| DriveError::Parse(e.to_string()))?;
    let get = |k: &str| {
        u.query_pairs()
            .find(|(a, _)| a == k)
            .map(|(_, v)| v.into_owned())
    };
    let state = get("state").ok_or_else(|| DriveError::Parse("state 없음".into()))?;
    if state != expected_state {
        return Err(DriveError::StateMismatch);
    }
    if let Some(e) = get("error") {
        return Ok(Callback::Denied(e));
    }
    get("code")
        .map(Callback::Code)
        .ok_or_else(|| DriveError::Parse("code 없음".into()))
}

/// 로그인 후 브라우저가 돌아올 127.0.0.1 임시 서버.
pub struct LoopbackServer {
    listener: TcpListener,
}

const PAGE_OK: &str = "로그인 완료. 운동 기록 앱으로 돌아가세요. 이 창은 닫아도 돼요.";
const PAGE_CANCEL: &str = "로그인이 취소됐어요. 앱에서 다시 시도할 수 있어요.";

impl LoopbackServer {
    pub fn bind() -> Result<Self, DriveError> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|e| DriveError::Parse(format!("임시 포트: {e}")))?;
        listener
            .set_nonblocking(true)
            .map_err(|e| DriveError::Parse(e.to_string()))?;
        Ok(LoopbackServer { listener })
    }

    pub fn redirect_uri(&self) -> String {
        format!(
            "http://127.0.0.1:{}",
            self.listener.local_addr().map(|a| a.port()).unwrap_or(0)
        )
    }

    /// 인증 코드가 올 때까지 기다린다. 관계없는 요청(favicon 등)은 무시하고 계속 기다린다.
    pub fn wait(&self, state: &str, timeout: Duration) -> Result<String, DriveError> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            let (stream, _) = match self.listener.accept() {
                Ok(s) => s,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                }
                Err(e) => return Err(DriveError::Parse(e.to_string())),
            };
            let _ = stream.set_nonblocking(false);
            let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
            let mut reader = BufReader::new(&stream);
            let mut line = String::new();
            let _ = reader.read_line(&mut line);
            let result = parse_callback(&line, state);
            let page = match &result {
                Ok(Callback::Code(_)) => PAGE_OK,
                Ok(Callback::Denied(_)) => PAGE_CANCEL,
                Err(_) => "",
            };
            let body =
                format!("<!doctype html><meta charset=utf-8><title>운동 기록</title><p>{page}</p>");
            let mut w = &stream;
            let _ = write!(
                w,
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            match result {
                Ok(Callback::Code(c)) => return Ok(c),
                Ok(Callback::Denied(_)) => return Err(DriveError::Cancelled),
                Err(DriveError::StateMismatch) => return Err(DriveError::StateMismatch),
                Err(_) => continue,
            }
        }
        Err(DriveError::Timeout)
    }
}

fn token_request(
    http: &dyn Http,
    form: Vec<(String, String)>,
) -> Result<serde_json::Value, DriveError> {
    let res = http.send(HttpRequest {
        method: Method::Post,
        url: TOKEN_URL.into(),
        headers: vec![],
        body: Body::Form(form),
        timeout: TimeoutClass::Lookup,
    })?;
    let v: serde_json::Value =
        serde_json::from_slice(&res.body).map_err(|e| DriveError::Parse(e.to_string()))?;
    if v["error"] == "invalid_grant" {
        return Err(DriveError::NeedLogin);
    }
    if !(200..300).contains(&res.status) {
        let msg = v["error_description"]
            .as_str()
            .or(v["error"].as_str())
            .unwrap_or("")
            .to_string();
        return Err(DriveError::Status(res.status, msg));
    }
    Ok(v)
}

fn tokens_from(
    v: &serde_json::Value,
    now: i64,
    old_refresh: Option<&str>,
) -> Result<Tokens, DriveError> {
    let access = v["access_token"]
        .as_str()
        .ok_or_else(|| DriveError::Parse("access_token 없음".into()))?;
    Ok(Tokens {
        access: access.to_string(),
        expires_at: now + v["expires_in"].as_i64().unwrap_or(3600),
        refresh: v["refresh_token"]
            .as_str()
            .map(str::to_string)
            .or(old_refresh.map(str::to_string)),
    })
}

fn kv(k: &str, v: &str) -> (String, String) {
    (k.to_string(), v.to_string())
}

pub fn exchange_code(
    http: &dyn Http,
    cfg: &ClientConfig,
    code: &str,
    redirect: &str,
    pkce: &Pkce,
    now: i64,
) -> Result<Tokens, DriveError> {
    let v = token_request(
        http,
        vec![
            kv("grant_type", "authorization_code"),
            kv("code", code),
            kv("redirect_uri", redirect),
            kv("client_id", &cfg.client_id),
            kv("client_secret", &cfg.client_secret),
            kv("code_verifier", &pkce.verifier),
        ],
    )?;
    tokens_from(&v, now, None)
}

/// 리프레시 토큰으로 새 액세스 토큰을 받는다. 만료·취소되었으면 NeedLogin.
pub fn refresh(
    http: &dyn Http,
    cfg: &ClientConfig,
    refresh_token: &str,
    now: i64,
) -> Result<Tokens, DriveError> {
    let v = token_request(
        http,
        vec![
            kv("grant_type", "refresh_token"),
            kv("refresh_token", refresh_token),
            kv("client_id", &cfg.client_id),
            kv("client_secret", &cfg.client_secret),
        ],
    )?;
    tokens_from(&v, now, Some(refresh_token))
}
