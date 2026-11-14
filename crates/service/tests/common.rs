#![allow(dead_code)]
use std::path::PathBuf;
use workout_service::Library;

pub fn sample_path(rel: &str) -> PathBuf {
    workout_testkit::sample(rel)
}

/// 한글·공백이 들어간 라이브러리 경로 (Review Focus 5).
pub fn temp_library() -> (tempfile::TempDir, Library) {
    let t = tempfile::tempdir().unwrap();
    let lib = Library::open(&t.path().join("내 라이브러리")).unwrap();
    (t, lib)
}

/// 원본 사진을 dir/name 으로 복사한다.
pub fn copy_as(src: &str, dir: &std::path::Path, name: &str) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let dest = dir.join(name);
    std::fs::copy(sample_path(src), &dest).unwrap();
    dest
}

/// DateTimeOriginal만 있는 EXIF APP1 구간 (FF E1 + 길이 + 내용).
fn exif_app1(local: &str) -> Vec<u8> {
    let mut tiff = Vec::new();
    tiff.extend_from_slice(b"II*\0");
    tiff.extend_from_slice(&8u32.to_le_bytes());
    // IFD0: ExifIFD 포인터 1개
    tiff.extend_from_slice(&1u16.to_le_bytes());
    tiff.extend_from_slice(&0x8769u16.to_le_bytes());
    tiff.extend_from_slice(&4u16.to_le_bytes());
    tiff.extend_from_slice(&1u32.to_le_bytes());
    tiff.extend_from_slice(&26u32.to_le_bytes());
    tiff.extend_from_slice(&0u32.to_le_bytes());
    // ExifIFD: DateTimeOriginal
    tiff.extend_from_slice(&1u16.to_le_bytes());
    tiff.extend_from_slice(&0x9003u16.to_le_bytes());
    tiff.extend_from_slice(&2u16.to_le_bytes());
    tiff.extend_from_slice(&20u32.to_le_bytes());
    tiff.extend_from_slice(&44u32.to_le_bytes());
    tiff.extend_from_slice(&0u32.to_le_bytes());
    tiff.extend_from_slice(local.as_bytes());
    tiff.push(0);
    let mut app1 = b"Exif\0\0".to_vec();
    app1.extend_from_slice(&tiff);
    let mut out = vec![0xFF, 0xE1];
    out.extend_from_slice(&((app1.len() + 2) as u16).to_be_bytes());
    out.extend_from_slice(&app1);
    out
}

/// 시간대 태그 없이 DateTimeOriginal만 있는 최소 JPEG (EXIF만 읽을 수 있음).
pub fn jpeg_with_local_time(path: &std::path::Path, local: &str) {
    let mut out = vec![0xFF, 0xD8];
    out.extend_from_slice(&exif_app1(local));
    out.extend_from_slice(&[0xFF, 0xD9]);
    std::fs::write(path, out).unwrap();
}

/// 샘플 사진(카드를 만들 수 있는 실제 이미지)의 EXIF를 원하는 촬영 시각 하나로 바꿔 저장한다.
pub fn real_jpeg_with_local_time(src: &str, dest: &std::path::Path, local: &str) {
    let b = std::fs::read(sample_path(src)).unwrap();
    let mut out = vec![0xFF, 0xD8];
    out.extend_from_slice(&exif_app1(local));
    let mut i = 2;
    while i + 4 <= b.len() && b[i] == 0xFF && b[i + 1] != 0xDA {
        let len = u16::from_be_bytes([b[i + 2], b[i + 3]]) as usize;
        let seg = &b[i..i + 2 + len];
        let is_exif = b[i + 1] == 0xE1 && seg.get(4..10) == Some(b"Exif\0\0".as_slice());
        if !is_exif {
            out.extend_from_slice(seg);
        }
        i += 2 + len;
    }
    out.extend_from_slice(&b[i..]);
    if let Some(d) = dest.parent() {
        std::fs::create_dir_all(d).unwrap();
    }
    std::fs::write(dest, out).unwrap();
}

use std::sync::Arc;
use workout_enrich::HttpResponse;
pub use workout_enrich::mock::MockHttp;
use workout_service::secrets::MemoryStore;

pub fn enrich_fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../enrich/tests/fixtures"
        ))
        .join(name),
    )
    .unwrap()
}

fn ok(body: Vec<u8>) -> HttpResponse {
    HttpResponse { status: 200, body }
}

/// 장소(공원·동 번갈아), 날씨, Claude 응답을 주는 가짜 HTTP와 라이브러리.
pub fn temp_library_with_mock() -> (tempfile::TempDir, Library, Arc<MockHttp>) {
    let park = enrich_fixture("nominatim_park.json");
    let dong = enrich_fixture("nominatim_dong.json");
    let http = Arc::new(MockHttp::routed(vec![
        (
            "nominatim",
            vec![
                ok(park.clone()),
                ok(dong.clone()),
                ok(park.clone()),
                ok(dong),
                ok(park),
            ],
        ),
        (
            "open-meteo",
            vec![ok(enrich_fixture("openmeteo_hourly.json"))],
        ),
        (
            "api.anthropic.com",
            vec![ok(enrich_fixture("claude_ok.json"))],
        ),
        (
            "generativelanguage.googleapis.com",
            vec![ok(enrich_fixture("gemini_ok.json"))],
        ),
    ]));
    let t = tempfile::tempdir().unwrap();
    let mut lib = Library::open_with(
        &t.path().join("내 라이브러리"),
        http.clone(),
        Arc::new(MemoryStore::default()),
    )
    .unwrap();
    lib.set_nominatim_interval(std::time::Duration::ZERO);
    (t, lib, http)
}

pub fn temp_library_with_failing_http() -> (tempfile::TempDir, Library, Arc<MockHttp>) {
    let http = Arc::new(MockHttp::failing());
    let t = tempfile::tempdir().unwrap();
    let mut lib = Library::open_with(
        &t.path().join("lib"),
        http.clone(),
        Arc::new(MemoryStore::default()),
    )
    .unwrap();
    lib.set_nominatim_interval(std::time::Duration::ZERO);
    (t, lib, http)
}

pub fn import_1025(lib: &mut Library) -> String {
    lib.import_paths(&[sample_path("20261025")])
        .unwrap()
        .added_activities[0]
        .clone()
}

pub use workout_gdrive::Tokens;
pub use workout_gdrive::drive::FakeDrive;

/// 가짜 드라이브에 연결되고 로그인된 라이브러리.
pub fn library_with_drive(root: &std::path::Path, drive: Arc<FakeDrive>) -> Library {
    let mut lib = Library::open_with(
        root,
        Arc::new(MockHttp::failing()),
        Arc::new(MemoryStore::default()),
    )
    .unwrap();
    let d = drive.clone();
    lib.set_drive_factory(Arc::new(move |_t, _c| {
        d.clone() as Arc<dyn workout_gdrive::drive::DriveApi>
    }));
    lib.set_google_client(Some("cid".into()), Some("sec".into()))
        .unwrap();
    lib.finish_login(Tokens {
        access: "ACCESS-SECRET".into(),
        expires_at: i64::MAX,
        refresh: Some("REFRESH-SECRET".into()),
    })
    .unwrap();
    lib
}

pub fn count_files(dir: &std::path::Path) -> usize {
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| {
                    if e.path().is_dir() {
                        count_files(&e.path())
                    } else {
                        1
                    }
                })
                .sum()
        })
        .unwrap_or(0)
}
