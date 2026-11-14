//! 인터넷을 쓰는 보강 기능: 장소(Nominatim), 날씨(Open-Meteo), Claude 다듬기.
//! HTTP는 `Http` 트레이트 뒤에 두어 테스트는 고정 응답으로 오프라인 실행한다.
pub mod claude;
pub mod gemini;
pub mod geocode;
#[cfg(feature = "mock")]
pub mod mock;
pub mod refine;
pub mod weather;

use std::time::Duration;

/// 외부 서비스에 보내는 앱 식별 문자열 (개인 정보 없음).
pub const USER_AGENT: &str = "workout-app/0.1 (personal desktop app)";

#[derive(Clone, Debug)]
pub struct HttpResponse {
    pub status: u16,
    pub body: Vec<u8>,
}

#[derive(thiserror::Error, Debug)]
pub enum EnrichError {
    #[error("인터넷 연결 실패: {0}")]
    Network(String),
    #[error("서버 오류({0}): {1}")]
    Status(u16, String),
    #[error("응답을 해석하지 못했어요: {0}")]
    Parse(String),
    #[error("AI가 요청을 거절했어요{}", .0.as_ref().map(|c| format!(" ({c})")).unwrap_or_default())]
    Refused(Option<String>),
    #[error("응답이 잘렸어요. 다시 시도해 주세요")]
    Truncated,
    #[error("설정에서 AI API 키를 넣어 주세요")]
    NoKey,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Patch,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Body {
    Empty,
    Json(serde_json::Value),
    /// application/x-www-form-urlencoded
    Form(Vec<(String, String)>),
    Bytes {
        content_type: String,
        data: Vec<u8>,
    },
}

/// 얼마나 기다릴지: 조회(15초) / 긴 작업(Claude·업로드·다운로드, 120초).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeoutClass {
    Lookup,
    Long,
}

#[derive(Clone, Debug)]
pub struct HttpRequest {
    pub method: Method,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Body,
    pub timeout: TimeoutClass,
}

fn owned(headers: &[(&str, &str)]) -> Vec<(String, String)> {
    headers
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

pub trait Http: Send + Sync {
    fn send(&self, req: HttpRequest) -> Result<HttpResponse, EnrichError>;

    fn get(&self, url: &str, headers: &[(&str, &str)]) -> Result<HttpResponse, EnrichError> {
        self.send(HttpRequest {
            method: Method::Get,
            url: url.into(),
            headers: owned(headers),
            body: Body::Empty,
            timeout: TimeoutClass::Lookup,
        })
    }

    fn post_json(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: &serde_json::Value,
    ) -> Result<HttpResponse, EnrichError> {
        self.send(HttpRequest {
            method: Method::Post,
            url: url.into(),
            headers: owned(headers),
            body: Body::Json(body.clone()),
            timeout: TimeoutClass::Long,
        })
    }
}

/// 실제 HTTP (reqwest blocking + rustls). 조회는 짧게, 긴 작업은 길게 기다린다.
pub struct ReqwestHttp {
    lookup: reqwest::blocking::Client,
    long: reqwest::blocking::Client,
}

impl ReqwestHttp {
    /// (조회 제한 시간, 긴 작업 제한 시간).
    pub const TIMEOUTS: (Duration, Duration) = (Duration::from_secs(15), Duration::from_secs(120));
    const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

    pub fn new() -> Self {
        let build = |timeout| {
            reqwest::blocking::Client::builder()
                .user_agent(USER_AGENT)
                .connect_timeout(Self::CONNECT_TIMEOUT)
                .timeout(timeout)
                .build()
                .expect("HTTP 클라이언트")
        };
        ReqwestHttp {
            lookup: build(Self::TIMEOUTS.0),
            long: build(Self::TIMEOUTS.1),
        }
    }
}

impl Default for ReqwestHttp {
    fn default() -> Self {
        Self::new()
    }
}

impl Http for ReqwestHttp {
    fn send(&self, req: HttpRequest) -> Result<HttpResponse, EnrichError> {
        let client = match req.timeout {
            TimeoutClass::Lookup => &self.lookup,
            TimeoutClass::Long => &self.long,
        };
        let mut b = match req.method {
            Method::Get => client.get(&req.url),
            Method::Post => client.post(&req.url),
            Method::Patch => client.patch(&req.url),
        };
        b = match req.body {
            Body::Empty => b,
            Body::Json(v) => b.json(&v),
            Body::Form(f) => b.form(&f),
            Body::Bytes { content_type, data } => b.header("content-type", content_type).body(data),
        };
        for (k, v) in &req.headers {
            let mut value = reqwest::header::HeaderValue::from_str(v)
                .map_err(|e| EnrichError::Network(e.to_string()))?;
            // 키·토큰이 디버그 출력·로그에 나오지 않게.
            value.set_sensitive(
                k.eq_ignore_ascii_case("x-api-key")
                    || k.eq_ignore_ascii_case("x-goog-api-key")
                    || k.eq_ignore_ascii_case("authorization"),
            );
            b = b.header(k.as_str(), value);
        }
        // 오류 문구에 요청 URL(좌표·토큰)이 들어가지 않게 without_url().
        let res = b
            .send()
            .map_err(|e| EnrichError::Network(e.without_url().to_string()))?;
        let status = res.status().as_u16();
        let body = res
            .bytes()
            .map_err(|e| EnrichError::Network(e.without_url().to_string()))?
            .to_vec();
        Ok(HttpResponse { status, body })
    }
}

/// 2xx가 아니면 오류로 바꾼다.
pub fn check_status(r: &HttpResponse) -> Result<(), EnrichError> {
    if (200..300).contains(&r.status) {
        Ok(())
    } else {
        Err(EnrichError::Status(
            r.status,
            String::from_utf8_lossy(&r.body).chars().take(200).collect(),
        ))
    }
}
