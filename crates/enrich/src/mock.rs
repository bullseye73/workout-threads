//! 테스트용 가짜 HTTP (feature "mock"). URL에 들어 있는 문자열별로 응답을 정하고 요청을 기록한다.
use std::sync::Mutex;

use crate::{Body, EnrichError, Http, HttpRequest, HttpResponse, Method, TimeoutClass};

type Route = (String, Vec<HttpResponse>, usize);

pub struct MockHttp {
    routes: Mutex<Vec<Route>>,
    fail_all: bool,
    pub log: Mutex<Vec<HttpRequest>>,
}

impl MockHttp {
    /// 모든 요청에 차례로 응답한다 (다 쓰면 Network 오류).
    pub fn new(responses: Vec<HttpResponse>) -> Self {
        MockHttp {
            routes: Mutex::new(vec![(String::new(), responses, 0)]),
            fail_all: false,
            log: Mutex::new(vec![]),
        }
    }

    /// URL에 `key`가 들어 있으면 그 목록에서 차례로 응답한다 (다 쓰면 마지막 응답을 반복).
    pub fn routed(routes: Vec<(&str, Vec<HttpResponse>)>) -> Self {
        let routes = routes
            .into_iter()
            .map(|(k, r)| (k.to_string(), r, 0))
            .collect();
        MockHttp {
            routes: Mutex::new(routes),
            fail_all: false,
            log: Mutex::new(vec![]),
        }
    }

    /// 모든 요청이 네트워크 오류.
    pub fn failing() -> Self {
        MockHttp {
            routes: Mutex::new(vec![]),
            fail_all: true,
            log: Mutex::new(vec![]),
        }
    }

    pub fn calls(&self) -> Vec<String> {
        self.log
            .lock()
            .unwrap()
            .iter()
            .map(|c| c.url.clone())
            .collect()
    }

    pub fn count(&self, needle: &str) -> usize {
        self.calls().iter().filter(|u| u.contains(needle)).count()
    }

    fn last(&self, needle: &str) -> Option<HttpRequest> {
        self.log
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|c| c.url.contains(needle))
            .cloned()
    }

    pub fn last_headers(&self) -> Vec<(String, String)> {
        self.log
            .lock()
            .unwrap()
            .last()
            .map(|c| c.headers.clone())
            .unwrap_or_default()
    }

    pub fn last_headers_for(&self, needle: &str) -> Vec<(String, String)> {
        self.last(needle).map(|c| c.headers).unwrap_or_default()
    }

    pub fn last_method(&self, needle: &str) -> Option<Method> {
        self.last(needle).map(|c| c.method)
    }

    pub fn last_timeout(&self, needle: &str) -> Option<TimeoutClass> {
        self.last(needle).map(|c| c.timeout)
    }

    pub fn last_body(&self, needle: &str) -> serde_json::Value {
        match self.last(needle).map(|c| c.body) {
            Some(Body::Json(v)) => v,
            _ => serde_json::Value::Null,
        }
    }

    pub fn last_form(&self, needle: &str) -> Vec<(String, String)> {
        match self.last(needle).map(|c| c.body) {
            Some(Body::Form(f)) => f,
            _ => vec![],
        }
    }

    pub fn last_bytes(&self, needle: &str) -> Vec<u8> {
        match self.last(needle).map(|c| c.body) {
            Some(Body::Bytes { data, .. }) => data,
            _ => vec![],
        }
    }
}

impl Http for MockHttp {
    fn send(&self, req: HttpRequest) -> Result<HttpResponse, EnrichError> {
        let url = req.url.clone();
        self.log.lock().unwrap().push(req);
        if self.fail_all {
            return Err(EnrichError::Network("연결 안 됨 (테스트)".into()));
        }
        let mut routes = self.routes.lock().unwrap();
        let sequential = routes.len() == 1 && routes[0].0.is_empty();
        let Some(route) = routes.iter_mut().find(|r| url.contains(&r.0)) else {
            return Err(EnrichError::Network(format!("응답 없음: {url}")));
        };
        if route.2 < route.1.len() {
            route.2 += 1;
            return Ok(route.1[route.2 - 1].clone());
        }
        if sequential || route.1.is_empty() {
            return Err(EnrichError::Network("응답 없음".into()));
        }
        Ok(route.1[route.1.len() - 1].clone())
    }
}
