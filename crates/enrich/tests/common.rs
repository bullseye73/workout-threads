#![allow(dead_code, unused_imports)]
pub use workout_enrich::mock::MockHttp;

pub fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/").to_string() + name)
        .unwrap()
}
