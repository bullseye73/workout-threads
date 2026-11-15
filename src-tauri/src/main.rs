// 운동 기록 데스크톱 앱: workout-service의 Library를 Tauri 명령으로 노출한다.
// 가져오기처럼 오래 걸리는 작업이 창을 멈추지 않도록 모든 명령은 백그라운드 스레드에서 실행한다.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;
use workout_service::backup::BackupJob;
use workout_service::*;

type Shared = Arc<Mutex<Library>>;
type Res<T> = std::result::Result<T, String>;

fn lock(shared: &Shared) -> Res<std::sync::MutexGuard<'_, Library>> {
    shared
        .lock()
        .map_err(|_| "라이브러리를 열 수 없음".to_string())
}

/// 라이브러리 작업을 블로킹 스레드에서 실행하고 오류를 화면용 문자열로 바꾼다.
async fn run<T: Send + 'static>(
    lib: State<'_, Shared>,
    f: impl FnOnce(&mut Library) -> workout_service::Result<T> + Send + 'static,
) -> Res<T> {
    let lib = lib.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = lib
            .lock()
            .map_err(|_| "라이브러리를 열 수 없음".to_string())?;
        f(&mut guard).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 올린 파일로 무엇이 만들어질지 (저장 안 함).
#[tauri::command]
async fn plan_upload(
    lib: State<'_, Shared>,
    paths: Vec<String>,
    exclude: Vec<String>,
) -> Res<UploadPlan> {
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    let exclude: Vec<PathBuf> = exclude.into_iter().map(PathBuf::from).collect();
    run(lib, move |l| l.plan_upload(&paths, &exclude)).await
}

/// 올린 파일을 보관함에 저장하고 운동마다 게시물을 만든다.
#[tauri::command]
async fn commit_upload(
    lib: State<'_, Shared>,
    paths: Vec<String>,
    exclude: Vec<String>,
) -> Res<UploadResult> {
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    let exclude: Vec<PathBuf> = exclude.into_iter().map(PathBuf::from).collect();
    run(lib, move |l| l.commit_upload(&paths, &exclude)).await
}

#[tauri::command]
async fn month_view(lib: State<'_, Shared>, year: i32, month: u32) -> Res<MonthView> {
    run(lib, move |l| l.month_view(year, month)).await
}

#[tauri::command]
async fn activity_detail(lib: State<'_, Shared>, id: String) -> Res<ActivityDetail> {
    run(lib, move |l| l.activity_detail(&id)).await
}

#[tauri::command]
async fn post_view(lib: State<'_, Shared>, activity_id: String) -> Res<PostView> {
    run(lib, move |l| l.post_view(&activity_id)).await
}

#[tauri::command]
async fn save_post(
    lib: State<'_, Shared>,
    activity_id: String,
    body: String,
    hashtags: Vec<String>,
) -> Res<PostView> {
    run(lib, move |l| l.save_post(&activity_id, &body, &hashtags)).await
}

#[tauri::command]
async fn regenerate_post(lib: State<'_, Shared>, activity_id: String) -> Res<PostView> {
    run(lib, move |l| l.regenerate_post(&activity_id)).await
}

#[tauri::command]
async fn mark_published(lib: State<'_, Shared>, activity_id: String) -> Res<PostView> {
    run(lib, move |l| l.mark_published(&activity_id)).await
}

#[tauri::command]
async fn assign_photo(lib: State<'_, Shared>, photo_id: i64, activity_id: String) -> Res<PhotoDto> {
    run(lib, move |l| l.assign_photo(photo_id, &activity_id)).await
}

#[tauri::command]
async fn unmatched_photos(lib: State<'_, Shared>) -> Res<Vec<PhotoDto>> {
    run(lib, |l| l.unmatched_photos()).await
}

#[tauri::command]
async fn enrich_activity(lib: State<'_, Shared>, id: String) -> Res<PostView> {
    // 잠금은 준비·저장할 때만 잡고, 인터넷 조회 중에는 풀어 다른 화면이 멈추지 않게 한다.
    let shared = lib.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let job = lock(&shared)?
            .prepare_enrich(&id)
            .map_err(|e| e.to_string())?;
        let fetched = job.run();
        lock(&shared)?
            .apply_enrich(fetched)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn set_place(
    lib: State<'_, Shared>,
    activity_id: String,
    name: Option<String>,
) -> Res<PostView> {
    run(lib, move |l| l.set_place(&activity_id, name)).await
}

#[tauri::command]
async fn set_route_points(
    lib: State<'_, Shared>,
    activity_id: String,
    points: Vec<String>,
) -> Res<PostView> {
    run(lib, move |l| l.set_route_points(&activity_id, points)).await
}

#[tauri::command]
async fn set_weather(
    lib: State<'_, Shared>,
    activity_id: String,
    weather: Option<WeatherDto>,
) -> Res<PostView> {
    run(lib, move |l| l.set_weather(&activity_id, weather)).await
}

#[tauri::command]
async fn refine_post(
    lib: State<'_, Shared>,
    activity_id: String,
    body: String,
    hashtags: Vec<String>,
    include_photos: bool,
) -> Res<RefineDto> {
    // 화면에 있는 글(저장 전 수정 포함)을 다듬는다. Claude 호출 중에는 잠금을 풀어 둔다.
    let shared = lib.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let job = lock(&shared)?
            .prepare_refine(&activity_id, &body, &hashtags, include_photos)
            .map_err(|e| e.to_string())?;
        job.run().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn refine_status(lib: State<'_, Shared>) -> Res<RefineStatusDto> {
    run(lib, |l| l.refine_status()).await
}

#[tauri::command]
async fn set_refine_provider(lib: State<'_, Shared>, provider: String) -> Res<()> {
    run(lib, move |l| l.set_refine_provider(&provider)).await
}

#[tauri::command]
async fn set_refine_key(lib: State<'_, Shared>, provider: String, key: Option<String>) -> Res<()> {
    run(lib, move |l| l.set_refine_key(&provider, key)).await
}

#[tauri::command]
async fn set_refine_model(
    lib: State<'_, Shared>,
    provider: String,
    model: Option<String>,
) -> Res<()> {
    run(lib, move |l| l.set_refine_model(&provider, model)).await
}

#[tauri::command]
async fn set_card_route_trim(lib: State<'_, Shared>, on: bool) -> Res<()> {
    run(lib, move |l| l.set_card_route_trim(on)).await
}

#[tauri::command]
async fn settings(lib: State<'_, Shared>) -> Res<SettingsDto> {
    run(lib, |l| l.settings()).await
}

#[tauri::command]
async fn regenerate_cards(lib: State<'_, Shared>, activity_id: String) -> Res<u32> {
    run(lib, move |l| l.regenerate_cards(&activity_id)).await
}

#[tauri::command]
async fn set_google_client(
    lib: State<'_, Shared>,
    client_id: Option<String>,
    client_secret: Option<String>,
) -> Res<()> {
    run(lib, move |l| l.set_google_client(client_id, client_secret)).await
}

#[tauri::command]
async fn google_client(lib: State<'_, Shared>) -> Res<Option<GoogleClientDto>> {
    run(lib, |l| l.google_client()).await
}

/// 시스템 브라우저로 구글 로그인 화면을 열고, 돌아올 때까지(최대 5분) 잠금 없이 기다린다.
#[tauri::command]
async fn google_login(app: tauri::AppHandle, lib: State<'_, Shared>) -> Res<BackupStatusDto> {
    let shared = lib.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let job = lock(&shared)?.begin_login().map_err(|e| e.to_string())?;
        app.opener()
            .open_url(&job.auth_url, None::<&str>)
            .map_err(|e| e.to_string())?;
        let tokens = job
            .run(std::time::Duration::from_secs(300))
            .map_err(|e| e.to_string())?;
        lock(&shared)?
            .finish_login(tokens)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn google_logout(lib: State<'_, Shared>) -> Res<()> {
    run(lib, |l| l.logout()).await
}

#[tauri::command]
async fn backup_status(lib: State<'_, Shared>) -> Res<BackupStatusDto> {
    run(lib, |l| l.backup_status()).await
}

/// 업로드 중에는 잠금을 풀어 다른 화면이 멈추지 않게 한다.
fn backup_unlocked(shared: &Shared) -> Res<BackupStatusDto> {
    let job: std::result::Result<BackupJob, String> =
        lock(shared)?.prepare_backup().map_err(|e| e.to_string());
    match job {
        Ok(job) => {
            let done = job.run();
            lock(shared)?
                .apply_backup(done, chrono_now())
                .map_err(|e| e.to_string())
        }
        // 로그인 만료 등은 기록하고 상태로 돌려준다.
        Err(_) => lock(shared)?
            .backup_now(chrono_now())
            .map_err(|e| e.to_string()),
    }
}

fn chrono_now() -> workout_service::chrono_utc::DateTime<workout_service::chrono_utc::Utc> {
    workout_service::chrono_utc::Utc::now()
}

#[tauri::command]
async fn backup_now(lib: State<'_, Shared>) -> Res<BackupStatusDto> {
    let shared = lib.inner().clone();
    tauri::async_runtime::spawn_blocking(move || backup_unlocked(&shared))
        .await
        .map_err(|e| e.to_string())?
}

/// 빈 라이브러리일 때만. 드물게 쓰는 작업이라 끝날 때까지 잠금을 잡는다.
#[tauri::command]
async fn restore_from_drive(lib: State<'_, Shared>) -> Res<BackupStatusDto> {
    run(lib, |l| {
        l.restore_from_drive()?;
        l.backup_status()
    })
    .await
}

/// 앱이 켜져 있는 동안: 시작 직후 한 번, 이후 10분마다 백업이 필요한지 확인한다.
fn spawn_backup_loop(app: tauri::AppHandle, shared: Shared) {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(20));
        loop {
            let needed = lock(&shared)
                .ok()
                .and_then(|l| l.needs_backup(chrono_now()).ok())
                .unwrap_or(false);
            if let Some(Ok(status)) = needed.then(|| backup_unlocked(&shared)) {
                let _ = app.emit("backup-status", status);
            }
            std::thread::sleep(std::time::Duration::from_secs(600));
        }
    });
}

#[tauri::command]
async fn set_goals(
    lib: State<'_, Shared>,
    cycling_km: Option<f64>,
    running_km: Option<f64>,
) -> Res<()> {
    run(lib, move |l| l.set_goals(cycling_km, running_km)).await
}

#[tauri::command]
async fn set_garmin_key(lib: State<'_, Shared>, key: Option<String>) -> Res<()> {
    run(lib, move |l| l.set_garmin_key(key)).await
}

#[tauri::command]
async fn test_garmin(lib: State<'_, Shared>) -> Res<u32> {
    run(lib, |l| l.test_garmin()).await
}

#[tauri::command]
async fn set_garmin_auto_check(lib: State<'_, Shared>, on: bool) -> Res<()> {
    run(lib, move |l| l.set_garmin_auto_check(on)).await
}

#[tauri::command]
async fn garmin_activities(
    lib: State<'_, Shared>,
    from: String,
    to: String,
) -> Res<Vec<GarminActivityDto>> {
    use workout_service::chrono_utc::NaiveDate;
    let from = NaiveDate::parse_from_str(&from, "%Y-%m-%d").map_err(|e| e.to_string())?;
    let to = NaiveDate::parse_from_str(&to, "%Y-%m-%d").map_err(|e| e.to_string())?;
    run(lib, move |l| l.garmin_activities(from, to)).await
}

#[tauri::command]
async fn fetch_garmin_fits(lib: State<'_, Shared>, ids: Vec<String>) -> Res<GarminFetchResult> {
    run(lib, move |l| l.fetch_garmin_fits(ids)).await
}

#[tauri::command]
async fn garmin_new_since_last_check(lib: State<'_, Shared>) -> Res<GarminFetchResult> {
    run(lib, |l| l.garmin_new_since_last_check()).await
}

#[tauri::command]
async fn bullsnote_status(lib: State<'_, Shared>) -> Res<BullsNoteStatusDto> {
    run(lib, |l| l.bullsnote_status()).await
}

#[tauri::command]
async fn set_bullsnote_settings(
    lib: State<'_, Shared>,
    enabled: bool,
    vault_path: Option<String>,
    folder_name: Option<String>,
) -> Res<BullsNoteStatusDto> {
    run(lib, move |l| {
        l.set_bullsnote_settings(enabled, vault_path, folder_name)
    })
    .await
}

#[tauri::command]
async fn sync_bullsnote_now(
    lib: State<'_, Shared>,
    year: i32,
    month: u32,
) -> Res<BullsNoteStatusDto> {
    run(lib, move |l| l.sync_bullsnote_month(year, month)).await
}

fn spawn_bullsnote_startup(shared: Shared) {
    use workout_service::chrono_utc::Datelike;
    std::thread::spawn(move || {
        if let Ok(mut guard) = shared.lock()
            && guard.bullsnote_status().map(|s| s.enabled).unwrap_or(false)
        {
            let now = chrono_now().date_naive();
            let _ = guard.sync_bullsnote_month(now.year(), now.month());
        }
    });
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let root = app.path().app_data_dir()?;
            let lib = Library::open(&root).map_err(|e| e.to_string())?;
            let shared: Shared = Arc::new(Mutex::new(lib));
            app.manage::<Shared>(shared.clone());
            spawn_bullsnote_startup(shared.clone());
            spawn_backup_loop(app.handle().clone(), shared);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            plan_upload,
            commit_upload,
            month_view,
            activity_detail,
            post_view,
            save_post,
            regenerate_post,
            mark_published,
            assign_photo,
            unmatched_photos,
            enrich_activity,
            set_place,
            set_route_points,
            set_weather,
            refine_post,
            refine_status,
            set_refine_provider,
            set_refine_key,
            set_refine_model,
            set_card_route_trim,
            settings,
            regenerate_cards,
            set_google_client,
            google_client,
            google_login,
            google_logout,
            backup_status,
            backup_now,
            restore_from_drive,
            set_goals,
            set_garmin_key,
            test_garmin,
            set_garmin_auto_check,
            garmin_activities,
            fetch_garmin_fits,
            garmin_new_since_last_check,
            bullsnote_status,
            set_bullsnote_settings,
            sync_bullsnote_now
        ])
        .run(tauri::generate_context!())
        .expect("앱 실행 실패");
}
