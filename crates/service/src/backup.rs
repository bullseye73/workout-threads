//! 구글 로그인과 드라이브 백업·복원.
//!
//! 백업은 로컬 → 드라이브 단방향이고 어느 쪽도 지우지 않는다. 화면이 멈추지 않도록
//! `prepare_backup`(잠금) → `BackupJob::run`(잠금 없음, 업로드) → `apply_backup`(잠금)으로 나눈다.
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use workout_core::chrono::{DateTime, Duration as ChronoDuration, Utc};
use workout_enrich::Http;
use workout_gdrive::drive::{DriveApi, HttpDrive, RemoteFile};
use workout_gdrive::oauth::{self, ClientConfig, LoopbackServer, Pkce};
use workout_gdrive::{DriveError, Tokens};

use crate::files::{content_hash, join_rel};
use crate::secrets::{GOOGLE_ACCESS, GOOGLE_REFRESH};
use crate::*;

/// 드라이브 연결을 만드는 함수 (실제: HttpDrive, 테스트: FakeDrive).
pub type DriveFactory = Arc<dyn Fn(Tokens, ClientConfig) -> Arc<dyn DriveApi> + Send + Sync>;

const ROOT_FOLDER: &str = "WorkoutApp";
const DB_NAME: &str = "library.db";
const CLIENT_ID_KEY: &str = "google_client_id";
const CLIENT_SECRET_KEY: &str = "google_client_secret";
const LAST_SUCCESS: &str = "last_success";
const LAST_ERROR: &str = "last_error";
const DB_HASH: &str = "db_hash";
const DB_ID: &str = "db_id";
/// 드라이브 백업의 주인 표시 (이 라이브러리의 고유 ID가 들어 있다).
const OWNER_NAME: &str = "owner.txt";
const LIBRARY_ID_KEY: &str = "library_id";
/// 바뀐 것이 없어도 이 간격이 지나면 다시 백업한다.
const BACKUP_EVERY_HOURS: i64 = 24;

pub(crate) fn default_drive_factory(http: Arc<dyn Http>) -> DriveFactory {
    Arc::new(move |tokens: Tokens, cfg: ClientConfig| {
        let h = http.clone();
        let refresh_token = tokens.refresh.clone();
        let refresh = Box::new(move || {
            let rt = refresh_token.clone().ok_or(DriveError::NeedLogin)?;
            oauth::refresh(h.as_ref(), &cfg, &rt, Utc::now().timestamp())
        });
        Arc::new(HttpDrive::new(http.clone(), tokens, refresh)) as Arc<dyn DriveApi>
    })
}

fn content_type(path: &str) -> &'static str {
    match path
        .rsplit('.')
        .next()
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("txt") => "text/plain; charset=utf-8",
        Some("db") => "application/x-sqlite3",
        _ => "application/octet-stream",
    }
}

/// files/ 아래 모든 파일: (상대 경로, 크기, 수정 시각).
fn walk_files(root: &Path) -> Vec<(String, i64, i64)> {
    fn go(dir: &Path, rel: &str, out: &mut Vec<(String, i64, i64)>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        let mut entries: Vec<_> = rd.filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let name = e.file_name().to_string_lossy().into_owned();
            let r = format!("{rel}/{name}");
            let Ok(meta) = e.metadata() else { continue };
            if meta.is_dir() {
                go(&e.path(), &r, out);
            } else {
                let mtime = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                out.push((r, meta.len() as i64, mtime));
            }
        }
    }
    let mut out = Vec::new();
    go(&root.join("files"), "files", &mut out);
    out
}

/// 로그인 작업: 브라우저에서 돌아올 때까지 기다린 뒤 토큰을 받는다 (잠금 없이 실행).
pub struct LoginJob {
    pub auth_url: String,
    server: LoopbackServer,
    pkce: Pkce,
    state: String,
    cfg: ClientConfig,
    http: Arc<dyn Http>,
}

impl LoginJob {
    pub fn run(self, timeout: Duration) -> Result<Tokens> {
        let code = self.server.wait(&self.state, timeout)?;
        Ok(oauth::exchange_code(
            self.http.as_ref(),
            &self.cfg,
            &code,
            &self.server.redirect_uri(),
            &self.pkce,
            Utc::now().timestamp(),
        )?)
    }
}

struct Item {
    rel: String,
    abs: PathBuf,
    size: i64,
    mtime: i64,
    known_id: Option<String>,
}

/// 백업이 진행 중임을 나타낸다. 작업이 끝나면(드롭되면) 풀린다.
struct RunningGuard(Arc<AtomicBool>);

impl Drop for RunningGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

/// 업로드 작업 (잠금 없이 실행).
pub struct BackupJob {
    drive: Arc<dyn DriveApi>,
    items: Vec<Item>,
    snapshot: PathBuf,
    db: Option<(String, Option<String>)>, // (해시, 알려진 id)
    folders: HashMap<String, String>,
    library_id: String,
    listings: HashMap<String, Vec<RemoteFile>>,
    _running: RunningGuard,
}

/// 업로드 결과 (아직 기록 전). 중간에 실패해도 그때까지 올린 것을 담는다.
pub struct BackupDone {
    uploaded: Vec<(String, String, i64, i64)>,
    new_folders: Vec<(String, String)>,
    db: Option<(String, String)>,
    error: Option<DriveError>,
    tokens: Option<Tokens>,
}

fn is_missing(e: &DriveError) -> bool {
    matches!(e, DriveError::NotFound(_) | DriveError::Status(404, _))
}

impl BackupJob {
    fn folder(
        &mut self,
        path: &str,
        new: &mut Vec<(String, String)>,
    ) -> std::result::Result<String, DriveError> {
        if let Some(id) = self.folders.get(path) {
            return Ok(id.clone());
        }
        let (parent_id, name) = if path.is_empty() {
            (None, ROOT_FOLDER.to_string())
        } else {
            let (parent, name) = path.rsplit_once('/').unwrap_or(("", path));
            (Some(self.folder(parent, new)?), name.to_string())
        };
        let id = match self.drive.find_folder(&name, parent_id.as_deref())? {
            Some(id) => id,
            None => self.drive.create_folder(&name, parent_id.as_deref())?,
        };
        self.folders.insert(path.to_string(), id.clone());
        new.push((path.to_string(), id.clone()));
        Ok(id)
    }

    /// 드라이브에서 지워진 폴더를 기억에서 빼고 다시 찾게 한다.
    fn forget_folder(&mut self, path: &str) {
        self.folders
            .retain(|p, _| p != path && !p.starts_with(&format!("{path}/")) && !path.is_empty());
        if path.is_empty() {
            self.folders.clear();
        }
    }

    /// 폴더 안 파일 목록 (한 번만 조회).
    fn listing(&mut self, parent: &str) -> std::result::Result<Vec<RemoteFile>, DriveError> {
        if !self.listings.contains_key(parent) {
            let l = self.drive.list(parent)?;
            self.listings.insert(parent.to_string(), l);
        }
        Ok(self.listings[parent].clone())
    }

    /// 새 파일을 올린다. 같은 이름이 이미 있으면(지난번에 올렸지만 기록 전에 꺼진 경우) 그것을 덮어쓴다.
    fn put_new(
        &mut self,
        dir: &str,
        name: &str,
        data: &[u8],
        new: &mut Vec<(String, String)>,
    ) -> std::result::Result<String, DriveError> {
        for attempt in 0..2 {
            let parent = self.folder(dir, new)?;
            let existing = self
                .listing(&parent)?
                .into_iter()
                .filter(|f| !f.is_folder && f.name == name)
                .max_by(|a, b| a.modified.cmp(&b.modified));
            let r = match existing {
                Some(f) => self
                    .drive
                    .update(&f.id, content_type(name), data)
                    .map(|_| f.id),
                None => self
                    .drive
                    .upload_new(&parent, name, content_type(name), data),
            };
            match r {
                Err(e) if attempt == 0 && is_missing(&e) => {
                    self.forget_folder(dir);
                    self.listings.clear();
                }
                other => return other,
            }
        }
        unreachable!()
    }

    /// 드라이브 루트에 다른 라이브러리의 백업이 있으면 아무것도 올리지 않고 멈춘다.
    fn check_owner(
        &mut self,
        new: &mut Vec<(String, String)>,
    ) -> std::result::Result<(), DriveError> {
        let root = self.folder("", new)?;
        let listing = self.listing(&root)?;
        let has_db = listing.iter().any(|f| !f.is_folder && f.name == DB_NAME);
        let owner = listing
            .iter()
            .find(|f| !f.is_folder && f.name == OWNER_NAME)
            .cloned();
        let mine = match &owner {
            Some(o) => self.drive.download(&o.id)? == self.library_id.as_bytes(),
            None => false,
        };
        if has_db && !mine {
            return Err(DriveError::OtherBackup);
        }
        if !mine {
            let id = self.library_id.clone();
            self.put_new("", OWNER_NAME, id.as_bytes(), new)?;
        }
        Ok(())
    }

    fn upload(&mut self, done: &mut BackupDone) -> std::result::Result<(), DriveError> {
        // 이 PC가 아직 DB를 올린 적이 없으면 드라이브 백업의 주인부터 확인한다.
        if self.db.as_ref().is_some_and(|(_, id)| id.is_none()) {
            self.check_owner(&mut done.new_folders)?;
        }
        let items = std::mem::take(&mut self.items);
        for it in items {
            let data = std::fs::read(&it.abs)
                .map_err(|e| DriveError::Parse(format!("{}: {e}", it.rel)))?;
            let (dir, name) = it.rel.rsplit_once('/').unwrap_or(("", &it.rel));
            let updated = match &it.known_id {
                Some(id) => match self.drive.update(id, content_type(&it.rel), &data) {
                    Ok(()) => Some(id.clone()),
                    Err(e) if is_missing(&e) => None, // 드라이브에서 지워짐 → 새로 올림
                    Err(e) => return Err(e),
                },
                None => None,
            };
            let id = match updated {
                Some(id) => id,
                None => self.put_new(dir, name, &data, &mut done.new_folders)?,
            };
            done.uploaded.push((it.rel, id, it.size, it.mtime));
        }
        if let Some((hash, known)) = self.db.take() {
            let data =
                std::fs::read(&self.snapshot).map_err(|e| DriveError::Parse(e.to_string()))?;
            let updated = match &known {
                Some(id) => match self.drive.update(id, content_type(DB_NAME), &data) {
                    Ok(()) => Some(id.clone()),
                    Err(e) if is_missing(&e) => None,
                    Err(e) => return Err(e),
                },
                None => None,
            };
            let id = match updated {
                Some(id) => id,
                None => self.put_new("", DB_NAME, &data, &mut done.new_folders)?,
            };
            done.db = Some((id, hash));
        }
        Ok(())
    }

    pub fn run(mut self) -> BackupDone {
        let mut done = BackupDone {
            uploaded: vec![],
            new_folders: vec![],
            db: None,
            error: None,
            tokens: None,
        };
        if let Err(e) = self.upload(&mut done) {
            done.error = Some(e);
        }
        done.tokens = self.drive.current_tokens();
        let _ = std::fs::remove_file(&self.snapshot);
        done
    }
}

impl Library {
    pub fn set_drive_factory(&mut self, f: DriveFactory) {
        self.drive_factory = f;
    }

    pub fn set_google_client(
        &mut self,
        client_id: Option<String>,
        client_secret: Option<String>,
    ) -> Result<()> {
        // secret을 비워 두면 저장된 secret을 그대로 쓴다. ID를 지우면 둘 다 지운다.
        let clean = |v: Option<String>| v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        let id = clean(client_id);
        self.store.set_setting(CLIENT_ID_KEY, id.as_deref())?;
        match (id, clean(client_secret)) {
            (None, _) => self.store.set_setting(CLIENT_SECRET_KEY, None)?,
            (Some(_), Some(secret)) => self.store.set_setting(CLIENT_SECRET_KEY, Some(&secret))?,
            (Some(_), None) => {}
        }
        Ok(())
    }

    fn client_config(&self) -> Result<Option<ClientConfig>> {
        let id = self.store.setting(CLIENT_ID_KEY)?;
        let secret = self.store.setting(CLIENT_SECRET_KEY)?;
        Ok(id
            .zip(secret)
            .map(|(client_id, client_secret)| ClientConfig {
                client_id,
                client_secret,
            }))
    }

    /// 설정 화면용 (secret 값은 돌려주지 않음).
    pub fn google_client(&self) -> Result<Option<GoogleClientDto>> {
        Ok(self
            .store
            .setting(CLIENT_ID_KEY)?
            .map(|client_id| GoogleClientDto {
                client_id,
                has_secret: self
                    .store
                    .setting(CLIENT_SECRET_KEY)
                    .ok()
                    .flatten()
                    .is_some(),
            }))
    }

    pub fn begin_login(&self) -> Result<LoginJob> {
        let cfg = self.client_config()?.ok_or_else(|| {
            ServiceError::Failed("설정에서 구글 클라이언트 ID와 secret을 먼저 넣어 주세요".into())
        })?;
        let server = LoopbackServer::bind()?;
        let pkce = Pkce::new();
        let state = oauth::new_state();
        let auth_url = oauth::auth_url(&cfg, &server.redirect_uri(), &pkce, &state);
        Ok(LoginJob {
            auth_url,
            server,
            pkce,
            state,
            cfg,
            http: self.http.clone(),
        })
    }

    fn save_tokens(&self, t: &Tokens) -> Result<()> {
        if let Some(r) = &t.refresh {
            self.secrets.set(GOOGLE_REFRESH, Some(r))?;
        }
        self.secrets.set(
            GOOGLE_ACCESS,
            Some(&format!("{}|{}", t.access, t.expires_at)),
        )
    }

    fn load_tokens(&self) -> Option<Tokens> {
        let refresh = self.secrets.get(GOOGLE_REFRESH)?;
        let (access, exp) = self
            .secrets
            .get(GOOGLE_ACCESS)
            .and_then(|a| {
                a.split_once('|')
                    .map(|(t, e)| (t.to_string(), e.parse().unwrap_or(0)))
            })
            .unwrap_or_default();
        Some(Tokens {
            access,
            expires_at: exp,
            refresh: Some(refresh),
        })
    }

    pub fn finish_login(&mut self, tokens: Tokens) -> Result<BackupStatusDto> {
        self.save_tokens(&tokens)?;
        self.store.set_sync_value(LAST_ERROR, None)?;
        self.backup_status()
    }

    /// 로그아웃하면 백업 기록(드라이브 id)도 지운다. 다른 계정으로 로그인해도 꼬이지 않게.
    pub fn logout(&mut self) -> Result<()> {
        self.secrets.set(GOOGLE_REFRESH, None)?;
        self.secrets.set(GOOGLE_ACCESS, None)?;
        self.store.clear_sync()?;
        Ok(())
    }

    fn drive(&self) -> Result<Arc<dyn DriveApi>> {
        let cfg = self
            .client_config()?
            .ok_or_else(|| ServiceError::Failed("구글 클라이언트가 설정되지 않았어요".into()))?;
        let tokens = self
            .load_tokens()
            .ok_or(ServiceError::Drive(DriveError::NeedLogin))?;
        Ok((self.drive_factory)(tokens, cfg))
    }

    fn pending_items(&self) -> Result<Vec<Item>> {
        let mut out = Vec::new();
        for (rel, size, mtime) in walk_files(&self.root) {
            let known = self.store.synced_file(&rel)?;
            if known
                .as_ref()
                .is_some_and(|(_, s, m)| *s == size && *m == mtime)
            {
                continue;
            }
            out.push(Item {
                abs: join_rel(&self.root, &rel),
                rel,
                size,
                mtime,
                known_id: known.map(|k| k.0),
            });
        }
        Ok(out)
    }

    /// DB 사본을 만들고 그 해시를 돌려준다. 동시에 여러 사본이 생겨도 겹치지 않게 이름을 매번 다르게.
    fn snapshot(&self) -> Result<(PathBuf, String)> {
        let dir = self.root.join(".backup");
        std::fs::create_dir_all(&dir)?;
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        let path = dir.join(format!("library-{}-{nanos}.db", std::process::id()));
        self.store.snapshot_to(&path)?;
        let hash = content_hash(&std::fs::read(&path)?);
        Ok((path, hash))
    }

    /// 이 라이브러리의 고유 ID (드라이브 백업의 주인 확인용). 처음 부를 때 만든다.
    fn library_id(&self) -> Result<String> {
        if let Some(id) = self.store.setting(LIBRARY_ID_KEY)? {
            return Ok(id);
        }
        let id = oauth::new_state();
        self.store.set_setting(LIBRARY_ID_KEY, Some(&id))?;
        Ok(id)
    }

    pub fn backup_status(&self) -> Result<BackupStatusDto> {
        let signed_in = self.secrets.get(GOOGLE_REFRESH).is_some();
        Ok(BackupStatusDto {
            configured: self.client_config()?.is_some(),
            signed_in,
            last_success: self.store.sync_value(LAST_SUCCESS)?,
            last_error: self.store.sync_value(LAST_ERROR)?,
            pending: self.pending_items()?.len() as u32,
            can_restore: self.store.is_empty()?,
        })
    }

    /// 로그인돼 있고 라이브러리가 비어 있지 않으며, 바뀐 파일·DB가 있거나 마지막 백업 뒤 24시간이 지났으면 true.
    /// (빈 라이브러리는 백업하지 않는다: 새 PC가 복원 전에 빈 DB를 올리는 것을 막는다.)
    pub fn needs_backup(&self, now: DateTime<Utc>) -> Result<bool> {
        if self.secrets.get(GOOGLE_REFRESH).is_none()
            || self.client_config()?.is_none()
            || self.store.is_empty()?
        {
            return Ok(false);
        }
        if self.backup_running.load(Ordering::SeqCst) {
            return Ok(false);
        }
        if !self.pending_items()?.is_empty() {
            return Ok(true);
        }
        let last = self
            .store
            .sync_value(LAST_SUCCESS)?
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|d| d.with_timezone(&Utc));
        if last.is_none_or(|l| now - l >= ChronoDuration::hours(BACKUP_EVERY_HOURS)) {
            return Ok(true);
        }
        let (path, hash) = self.snapshot()?;
        let _ = std::fs::remove_file(path);
        Ok(self.store.sync_value(DB_HASH)?.as_deref() != Some(hash.as_str()))
    }

    pub fn prepare_backup(&self) -> Result<BackupJob> {
        if self.store.is_empty()? {
            return Err(ServiceError::Failed(
                "빈 라이브러리는 백업하지 않아요".into(),
            ));
        }
        if self
            .backup_running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(ServiceError::Failed("이미 백업 중이에요".into()));
        }
        let running = RunningGuard(self.backup_running.clone());
        // ID는 DB 사본보다 먼저 만든다 (사본을 만든 뒤 DB가 바뀌면 다음에 또 백업하게 됨).
        let library_id = self.library_id()?;
        let drive = self.drive()?;
        let items = self.pending_items()?;
        let (snapshot, hash) = self.snapshot()?;
        let db_id = self.store.sync_value(DB_ID)?;
        let db = (self.store.sync_value(DB_HASH)?.as_deref() != Some(hash.as_str())
            || db_id.is_none())
        .then_some((hash, db_id));
        let folders = self.store.synced_folders()?.into_iter().collect();
        Ok(BackupJob {
            drive,
            items,
            snapshot,
            db,
            folders,
            library_id,
            listings: HashMap::new(),
            _running: running,
        })
    }

    pub fn apply_backup(
        &mut self,
        done: BackupDone,
        now: DateTime<Utc>,
    ) -> Result<BackupStatusDto> {
        // 백업 도중 로그아웃했다면 결과를 기록하지 않는다 (로그아웃이 기록을 지웠으므로).
        let still_signed_in = self.secrets.get(GOOGLE_REFRESH).is_some();
        if still_signed_in {
            for (path, id) in &done.new_folders {
                self.store.put_synced_folder(path, id)?;
            }
            for (rel, id, size, mtime) in &done.uploaded {
                self.store.put_synced_file(rel, id, *size, *mtime)?;
            }
            if let Some((id, hash)) = &done.db {
                self.store.set_sync_value(DB_ID, Some(id))?;
                self.store.set_sync_value(DB_HASH, Some(hash))?;
            }
        }
        match done.error {
            Some(DriveError::NeedLogin) => {
                self.logout()?;
                self.store
                    .set_sync_value(LAST_ERROR, Some(&DriveError::NeedLogin.to_string()))?;
            }
            Some(e) => self
                .store
                .set_sync_value(LAST_ERROR, Some(&e.to_string()))?,
            None if still_signed_in => {
                if let Some(t) = &done.tokens {
                    self.save_tokens(t)?;
                }
                self.store
                    .set_sync_value(LAST_SUCCESS, Some(&now.to_rfc3339()))?;
                self.store.set_sync_value(LAST_ERROR, None)?;
            }
            None => {}
        }
        self.backup_status()
    }

    /// 준비 → 업로드 → 기록을 한 번에. 빈 라이브러리면 아무것도 하지 않는다.
    pub fn backup_now(&mut self, now: DateTime<Utc>) -> Result<BackupStatusDto> {
        if self.store.is_empty()? {
            return self.backup_status();
        }
        let done = match self.prepare_backup() {
            Ok(job) => job.run(),
            Err(ServiceError::Drive(e)) => BackupDone {
                uploaded: vec![],
                new_folders: vec![],
                db: None,
                error: Some(e),
                tokens: None,
            },
            Err(e) => return Err(e),
        };
        self.apply_backup(done, now)
    }

    /// 빈 라이브러리에만: 드라이브의 WorkoutApp을 내려받는다.
    /// 실패하면 내려받은 파일을 지우고 원래(빈) DB를 그대로 쓴다.
    pub fn restore_from_drive(&mut self) -> Result<()> {
        if !self.store.is_empty()? {
            return Err(DriveError::NotEmpty.into());
        }
        let drive = self.drive()?;
        let mut written: Vec<PathBuf> = Vec::new();
        let result = self
            .download_tree(drive.as_ref(), &mut written)
            .and_then(|(db, files, folders)| self.install_restored(db, files, folders));
        if result.is_err() {
            for p in &written {
                let _ = std::fs::remove_file(p);
            }
            if let Err(ServiceError::Drive(DriveError::NeedLogin)) = &result {
                self.logout()?;
            }
        }
        result
    }

    #[allow(clippy::type_complexity)]
    fn download_tree(
        &self,
        drive: &dyn DriveApi,
        written: &mut Vec<PathBuf>,
    ) -> Result<(Vec<u8>, Vec<(String, String)>, Vec<(String, String)>)> {
        let root = drive
            .find_folder(ROOT_FOLDER, None)?
            .ok_or_else(|| DriveError::NotFound(ROOT_FOLDER.into()))?;
        let mut folders = vec![(String::new(), root.clone())];
        let mut files = Vec::new();
        let mut db: Option<RemoteFile> = None;
        let mut queue = vec![(String::new(), root)];
        while let Some((path, id)) = queue.pop() {
            for f in drive.list(&id)? {
                let rel = if path.is_empty() {
                    f.name.clone()
                } else {
                    format!("{path}/{}", f.name)
                };
                if f.is_folder {
                    folders.push((rel.clone(), f.id.clone()));
                    queue.push((rel, f.id));
                } else if path.is_empty() && f.name == DB_NAME {
                    // DB가 여러 개면 가장 최근 것.
                    if db.as_ref().is_none_or(|d| f.modified > d.modified) {
                        db = Some(f);
                    }
                } else if rel.starts_with("files/") && safe_rel(&rel) {
                    let data = drive.download(&f.id)?;
                    let dest = join_rel(&self.root, &rel);
                    if let Some(parent) = dest.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::write(&dest, data)?;
                    written.push(dest);
                    files.push((rel, f.id));
                }
            }
        }
        let db = db.ok_or_else(|| DriveError::NotFound(DB_NAME.into()))?;
        let bytes = drive.download(&db.id)?;
        files.push((DB_NAME.to_string(), db.id));
        Ok((bytes, files, folders))
    }

    /// 내려받은 DB를 먼저 임시 파일로 열어 확인한 뒤 바꿔 넣는다. 실패하면 원래 DB를 다시 연다.
    fn install_restored(
        &mut self,
        db_bytes: Vec<u8>,
        files: Vec<(String, String)>,
        folders: Vec<(String, String)>,
    ) -> Result<()> {
        let db_path = self.root.join(DB_NAME);
        let tmp = self.root.join("library.db.restore");
        let checked = std::fs::write(&tmp, &db_bytes)
            .map_err(ServiceError::from)
            .and_then(|_| {
                workout_store::Store::open(&tmp)
                    .map(drop)
                    .map_err(ServiceError::from)
            });
        if let Err(e) = checked {
            let _ = std::fs::remove_file(&tmp);
            return Err(e);
        }
        // 지금 DB 연결을 닫고(Windows 파일 잠금) 바꿔 넣는다.
        self.store = workout_store::Store::open_in_memory()?;
        let swapped = std::fs::rename(&tmp, &db_path);
        self.store = workout_store::Store::open(&db_path)?;
        if let Err(e) = swapped {
            let _ = std::fs::remove_file(&tmp);
            return Err(e.into());
        }
        for (path, id) in folders {
            self.store.put_synced_folder(&path, &id)?;
        }
        let local: HashMap<String, (i64, i64)> = walk_files(&self.root)
            .into_iter()
            .map(|(r, s, m)| (r, (s, m)))
            .collect();
        for (rel, id) in files {
            if rel == DB_NAME {
                self.store.set_sync_value(DB_ID, Some(&id))?;
            } else if let Some((size, mtime)) = local.get(&rel) {
                self.store.put_synced_file(&rel, &id, *size, *mtime)?;
            }
        }
        let (snap, hash) = self.snapshot()?;
        let _ = std::fs::remove_file(snap);
        self.store.set_sync_value(DB_HASH, Some(&hash))?;
        Ok(())
    }
}

/// 드라이브의 파일 이름을 로컬 경로로 쓸 때 상위 폴더로 벗어나지 않게.
fn safe_rel(rel: &str) -> bool {
    rel.split('/')
        .all(|p| !p.is_empty() && p != "." && p != ".." && !p.contains(['\\', ':']))
}
