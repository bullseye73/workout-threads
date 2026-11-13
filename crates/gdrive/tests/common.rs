#![allow(dead_code, unused_imports)]
pub use workout_enrich::HttpResponse;
pub use workout_enrich::mock::MockHttp;

pub fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/").to_string() + name)
        .unwrap()
}

pub fn resp(status: u16, name: &str) -> HttpResponse {
    HttpResponse {
        status,
        body: fixture(name),
    }
}
