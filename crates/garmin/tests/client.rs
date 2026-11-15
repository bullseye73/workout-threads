use std::collections::VecDeque;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use flate2::Compression;
use flate2::write::DeflateEncoder;
use workout_core::chrono::NaiveDate;
use workout_enrich::{Http, HttpRequest, HttpResponse};
use workout_garmin::{GarminClient, GarminError, KeyTokenProvider, TokenProvider};

struct MockHttp {
    responses: Mutex<VecDeque<std::result::Result<HttpResponse, workout_enrich::EnrichError>>>,
    calls: Mutex<Vec<HttpRequest>>,
}

impl MockHttp {
    fn new(resps: Vec<HttpResponse>) -> Self {
        Self {
            responses: Mutex::new(resps.into_iter().map(Ok).collect()),
            calls: Mutex::new(Vec::new()),
        }
    }

    fn calls(&self) -> Vec<HttpRequest> {
        self.calls.lock().unwrap().clone()
    }
}

impl Http for MockHttp {
    fn send(
        &self,
        req: HttpRequest,
    ) -> std::result::Result<HttpResponse, workout_enrich::EnrichError> {
        self.calls.lock().unwrap().push(req);
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .expect("no more mock responses")
    }
}

struct StaticTokens(String);
impl TokenProvider for StaticTokens {
    fn access_token(&self, _http: &dyn Http) -> workout_garmin::Result<String> {
        Ok(self.0.clone())
    }
    fn refresh_token(&self, _http: &dyn Http) -> workout_garmin::Result<String> {
        Ok(self.0.clone())
    }
}

fn fixture(name: &str) -> Vec<u8> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read(p).unwrap()
}

fn make_zip(filename: &str, content: &[u8]) -> Vec<u8> {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(content).unwrap();
    let compressed = encoder.finish().unwrap();
    let name_bytes = filename.as_bytes();

    let mut out = Vec::new();
    // Local file header
    out.extend_from_slice(b"PK\x03\x04");
    out.extend_from_slice(&20u16.to_le_bytes()); // version needed
    out.extend_from_slice(&0u16.to_le_bytes()); // flags
    out.extend_from_slice(&8u16.to_le_bytes()); // compression: deflate
    out.extend_from_slice(&0u16.to_le_bytes()); // mod time
    out.extend_from_slice(&0u16.to_le_bytes()); // mod date
    out.extend_from_slice(&0u32.to_le_bytes()); // crc32 (unused by our reader)
    out.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
    out.extend_from_slice(&(content.len() as u32).to_le_bytes());
    out.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // extra len
    out.extend_from_slice(name_bytes);
    out.extend_from_slice(&compressed);
    out
}

#[test]
fn lists_activities_from_fixture() {
    let http = Arc::new(MockHttp::new(vec![HttpResponse {
        status: 200,
        body: fixture("activities_ok.json"),
    }]));
    let client = GarminClient::new(http.clone(), Arc::new(StaticTokens("tok".into())));
    let from = NaiveDate::from_ymd_opt(2026, 11, 10).unwrap();
    let to = NaiveDate::from_ymd_opt(2026, 11, 15).unwrap();
    let list = client.list_activities(from, to).unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].id, "1001");
    assert_eq!(list[0].name, "한강 아침 러닝");
    assert_eq!(list[0].sport_type_key, "running");
    assert!((list[0].distance_m - 10250.5).abs() < 0.01);
    assert_eq!(list[1].id, "1002");
    assert_eq!(list[1].sport_type_key, "cycling");

    let calls = http.calls();
    assert!(calls[0].url.contains("startDate=2026-11-10"));
    assert!(calls[0].url.contains("endDate=2026-11-15"));
    assert!(
        calls[0]
            .headers
            .contains(&("Authorization".into(), "Bearer tok".into()))
    );
}

#[test]
fn downloads_fit_from_zip_and_raw() {
    let raw_fit = b".FIT_FAKE_BYTES_12345";
    let zip_bytes = make_zip("1001_ACTIVITY.fit", raw_fit);
    let http = Arc::new(MockHttp::new(vec![
        HttpResponse {
            status: 200,
            body: zip_bytes,
        },
        HttpResponse {
            status: 200,
            body: raw_fit.to_vec(),
        },
    ]));
    let client = GarminClient::new(http, Arc::new(StaticTokens("tok".into())));
    assert_eq!(client.download_fit("1001").unwrap(), raw_fit);
    assert_eq!(client.download_fit("1002").unwrap(), raw_fit);
}

#[test]
fn maps_401_and_429_errors() {
    let http = Arc::new(MockHttp::new(vec![
        HttpResponse {
            status: 401,
            body: b"unauthorized".to_vec(),
        },
        HttpResponse {
            status: 401,
            body: b"still unauthorized".to_vec(),
        },
        HttpResponse {
            status: 429,
            body: b"rate limited".to_vec(),
        },
    ]));
    let client = GarminClient::new(http, Arc::new(StaticTokens("tok".into())));
    let from = NaiveDate::from_ymd_opt(2026, 11, 10).unwrap();
    let to = NaiveDate::from_ymd_opt(2026, 11, 15).unwrap();
    assert!(matches!(
        client.list_activities(from, to),
        Err(GarminError::Unauthorized)
    ));
    assert!(matches!(
        client.list_activities(from, to),
        Err(GarminError::RateLimited)
    ));
}

#[test]
fn refreshes_expired_token_with_api_key() {
    let http = Arc::new(MockHttp::new(vec![
        // 1st call: data endpoint returns 401 with old cached token
        HttpResponse {
            status: 401,
            body: b"expired".to_vec(),
        },
        // 2nd call: token exchange with ApiKey
        HttpResponse {
            status: 200,
            body: fixture("token_ok.json"),
        },
        // 3rd call: retried data endpoint succeeds
        HttpResponse {
            status: 200,
            body: fixture("wellness_ok.json"),
        },
    ]));

    let cache = Arc::new(Mutex::new(Some("old-tok|9999999999".to_string())));
    let cache_r = cache.clone();
    let cache_w = cache.clone();
    let tokens = Arc::new(KeyTokenProvider::new(
        Arc::new(|| Some("my-personal-key".to_string())),
        Arc::new(move || cache_r.lock().unwrap().clone()),
        Arc::new(move |v| *cache_w.lock().unwrap() = v),
    ));

    let client = GarminClient::new(http.clone(), tokens);
    let d = NaiveDate::from_ymd_opt(2026, 11, 15).unwrap();
    let w = client.daily_wellness(d).unwrap();
    assert_eq!(w.total_steps, Some(12450));
    assert_eq!(w.resting_hr, Some(51));
    assert_eq!(w.avg_stress, Some(24));
    assert_eq!(w.body_battery_high, Some(92));
    assert_eq!(w.body_battery_low, Some(38));

    let calls = http.calls();
    assert_eq!(calls.len(), 3);
    assert!(
        calls[1]
            .headers
            .contains(&("Authorization".into(), "ApiKey my-personal-key".into()))
    );
    assert!(calls[2].headers.contains(&(
        "Authorization".into(),
        "Bearer garmin-access-token-123".into()
    )));
    assert!(
        cache
            .lock()
            .unwrap()
            .as_deref()
            .unwrap()
            .starts_with("garmin-access-token-123|")
    );
}
