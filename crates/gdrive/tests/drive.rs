mod common;
use common::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use workout_enrich::Method;
use workout_gdrive::drive::*;
use workout_gdrive::{DriveError, Tokens};

fn tokens(a: &str) -> Tokens {
    Tokens {
        access: a.into(),
        expires_at: i64::MAX,
        refresh: Some("r".into()),
    }
}

fn drive(http: Arc<MockHttp>) -> HttpDrive {
    HttpDrive::new(http, tokens("t1"), Box::new(|| Err(DriveError::NeedLogin)))
}

fn auth(http: &MockHttp, needle: &str) -> String {
    http.last_headers_for(needle)
        .into_iter()
        .find(|(k, _)| k == "Authorization")
        .map(|(_, v)| v)
        .unwrap_or_default()
}

#[test]
fn find_folder_builds_query() {
    let http = Arc::new(MockHttp::new(vec![
        resp(200, "files_list_folder.json"),
        resp(200, "files_list_empty.json"),
    ]));
    let d = drive(http.clone());
    assert_eq!(
        d.find_folder("WorkoutApp", None).unwrap().as_deref(),
        Some("fold1")
    );
    let u = url::Url::parse(&http.calls()[0]).unwrap();
    let q = u
        .query_pairs()
        .find(|(k, _)| k == "q")
        .unwrap()
        .1
        .into_owned();
    assert!(
        q.contains("name='WorkoutApp'")
            && q.contains("'root' in parents")
            && q.contains("trashed=false")
    );
    assert!(q.contains("application/vnd.google-apps.folder"));
    assert_eq!(auth(&http, "drive/v3/files"), "Bearer t1");
    assert!(d.find_folder("x", Some("p")).unwrap().is_none());
}

#[test]
fn name_with_quote_is_escaped() {
    let http = Arc::new(MockHttp::new(vec![resp(200, "files_list_empty.json")]));
    drive(http.clone()).find_folder("a'b", None).unwrap();
    let u = url::Url::parse(&http.calls()[0]).unwrap();
    assert!(
        u.query_pairs()
            .any(|(k, v)| k == "q" && v.contains(r"name='a\'b'"))
    );
}

#[test]
fn create_folder_posts_metadata() {
    let http = Arc::new(MockHttp::new(vec![resp(200, "file_created.json")]));
    let id = drive(http.clone())
        .create_folder("files", Some("fold1"))
        .unwrap();
    assert_eq!(id, "new1");
    let b = http.last_body("drive/v3/files");
    assert_eq!(b["name"], "files");
    assert_eq!(b["mimeType"], "application/vnd.google-apps.folder");
    assert_eq!(b["parents"][0], "fold1");
}

#[test]
fn upload_new_is_multipart() {
    let http = Arc::new(MockHttp::new(vec![resp(200, "file_created.json")]));
    let id = drive(http.clone())
        .upload_new("fold1", "x.jpg", "image/jpeg", &[0xFF, 0xD8, 0x01])
        .unwrap();
    assert_eq!(id, "new1");
    assert!(
        http.calls()[0].contains("upload/drive/v3/files")
            && http.calls()[0].contains("uploadType=multipart")
    );
    let body = http.last_bytes("upload");
    let text = String::from_utf8_lossy(&body);
    assert!(text.contains("\"name\":\"x.jpg\"") && text.contains("\"parents\":[\"fold1\"]"));
    assert!(body.windows(3).any(|w| w == [0xFF, 0xD8, 0x01]));
}

#[test]
fn update_uses_patch_media() {
    let http = Arc::new(MockHttp::new(vec![resp(200, "file_created.json")]));
    drive(http.clone())
        .update("abc", "application/octet-stream", b"db")
        .unwrap();
    assert!(
        http.calls()[0].contains("upload/drive/v3/files/abc")
            && http.calls()[0].contains("uploadType=media")
    );
    assert_eq!(http.last_method("upload"), Some(Method::Patch));
    assert_eq!(http.last_bytes("upload"), b"db");
}

#[test]
fn list_follows_next_page_token() {
    let http = Arc::new(MockHttp::new(vec![
        resp(200, "files_page1.json"),
        resp(200, "files_page2.json"),
    ]));
    let files = drive(http.clone()).list("fold1").unwrap();
    assert_eq!(files.len(), 2);
    assert!(!files[0].is_folder && files[1].is_folder);
    assert!(http.calls()[1].contains("pageToken=p2"));
}

#[test]
fn download_uses_alt_media() {
    let http = Arc::new(MockHttp::new(vec![HttpResponse {
        status: 200,
        body: b"bytes".to_vec(),
    }]));
    assert_eq!(drive(http.clone()).download("abc").unwrap(), b"bytes");
    assert!(http.calls()[0].contains("files/abc") && http.calls()[0].contains("alt=media"));
}

#[test]
fn retries_once_after_401_with_refreshed_token() {
    let http = Arc::new(MockHttp::new(vec![
        resp(401, "unauthorized.json"),
        resp(200, "files_list_empty.json"),
    ]));
    let n = Arc::new(AtomicUsize::new(0));
    let n2 = n.clone();
    let d = HttpDrive::new(
        http.clone(),
        tokens("old"),
        Box::new(move || {
            n2.fetch_add(1, Ordering::SeqCst);
            Ok(tokens("new"))
        }),
    );
    d.find_folder("WorkoutApp", None).unwrap();
    assert_eq!(n.load(Ordering::SeqCst), 1);
    assert_eq!(auth(&http, "drive/v3/files"), "Bearer new");
    assert_eq!(d.current_tokens().access, "new");
}

#[test]
fn refresh_failure_needs_login() {
    let http = Arc::new(MockHttp::new(vec![resp(401, "unauthorized.json")]));
    assert!(matches!(
        drive(http).find_folder("x", None),
        Err(DriveError::NeedLogin)
    ));
}

#[test]
fn fake_drive_roundtrip_and_failure_injection() {
    let f = FakeDrive::default();
    let root = f.create_folder("WorkoutApp", None).unwrap();
    assert_eq!(
        f.find_folder("WorkoutApp", None).unwrap().as_deref(),
        Some(root.as_str())
    );
    let id = f.upload_new(&root, "a.txt", "text/plain", b"hi").unwrap();
    f.update(&id, "text/plain", b"bye").unwrap();
    assert_eq!(f.download(&id).unwrap(), b"bye");
    assert_eq!(f.list(&root).unwrap().len(), 1);
    f.fail_after(0);
    assert!(f.upload_new(&root, "b.txt", "text/plain", b"x").is_err());
    assert_eq!(f.file_count(), 1);
}
