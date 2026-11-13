//! 데스크톱 앱이 쓰는 라이브러리 파사드: 가져오기, 조회, 게시 준비.
//! Tauri와 무관하게 테스트할 수 있다.
pub mod backup;
mod dto;
mod enrich;
pub use enrich::{EnrichFetched, EnrichJob, RefineJob};
mod files;
mod import;
mod posts;
pub mod secrets;
mod views;

pub use dto::*;
/// Tauri 껍데기가 같은 chrono를 쓰도록 다시 내보낸다.
pub use workout_core::chrono as chrono_utc;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use workout_card::CardError;
use workout_core::CoreError;
use workout_core::fit::parse_fit;
use workout_core::model::Activity;
use workout_enrich::geocode::RateLimiter;
use workout_enrich::{Http, ReqwestHttp};
use workout_post::PostError;
use workout_store::{PhotoRow, Store, StoreError};

#[derive(thiserror::Error, Debug)]
pub enum ServiceError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error(transparent)]
    Card(#[from] CardError),
    #[error(transparent)]
    Post(#[from] PostError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("찾을 수 없음: {0}")]
    NotFound(String),
    #[error("{0}")]
    Failed(String),
    #[error(transparent)]
    Drive(#[from] workout_gdrive::DriveError),
}

pub type Result<T> = std::result::Result<T, ServiceError>;

/// 라이브러리 폴더 하나 (`library.db` + `files/`).
pub struct Library {
    root: PathBuf,
    store: Store,
    http: Arc<dyn Http>,
    secrets: Arc<dyn secrets::SecretStore>,
    nominatim: Arc<RateLimiter>,
    drive_factory: backup::DriveFactory,
    backup_running: Arc<std::sync::atomic::AtomicBool>,
}

impl Library {
    /// 실제 HTTP와 OS 키 보관함을 쓰는 라이브러리.
    pub fn open(root: &Path) -> Result<Library> {
        Self::open_with(
            root,
            Arc::new(ReqwestHttp::new()),
            Arc::new(secrets::KeyringStore),
        )
    }

    /// HTTP와 비밀 저장소를 바꿔 끼울 수 있는 생성자 (테스트·개발 도구용).
    pub fn open_with(
        root: &Path,
        http: Arc<dyn Http>,
        secrets: Arc<dyn secrets::SecretStore>,
    ) -> Result<Library> {
        std::fs::create_dir_all(root.join("files"))?;
        let store = Store::open(&root.join("library.db"))?;
        let lib = Library {
            root: root.to_path_buf(),
            store,
            drive_factory: backup::default_drive_factory(http.clone()),
            backup_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            http,
            secrets,
            nominatim: Arc::new(RateLimiter::new(enrich::NOMINATIM_INTERVAL)),
        };
        lib.backfill_hashes()?;
        Ok(lib)
    }

    /// 예전에 가져와 해시가 없는 사진에 내용 해시를 채운다 (중복 판단용).
    fn backfill_hashes(&self) -> Result<()> {
        for mut p in self.store.photos_without_hash()? {
            if let Ok(bytes) = std::fs::read(files::join_rel(&self.root, &p.path)) {
                p.content_hash = Some(files::content_hash(&bytes));
                self.store.update_photo(&p)?;
            }
        }
        Ok(())
    }

    /// Nominatim 호출 간격 (기본 1초, 테스트에서만 줄인다).
    pub fn set_nominatim_interval(&mut self, d: Duration) {
        self.nominatim = Arc::new(RateLimiter::new(d));
    }

    #[doc(hidden)]
    pub fn secrets_for_tests(&self) -> &dyn secrets::SecretStore {
        self.secrets.as_ref()
    }

    #[doc(hidden)]
    pub fn store_for_tests(&self) -> &Store {
        &self.store
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 라이브러리 상대 경로 → 절대 경로 문자열.
    pub(crate) fn abs(&self, rel: &str) -> String {
        files::join_rel(&self.root, rel)
            .to_string_lossy()
            .into_owned()
    }

    /// 저장된 원본 FIT를 다시 읽어 시계열까지 포함한 운동을 돌려준다.
    pub(crate) fn load_activity(&self, id: &str) -> Result<Activity> {
        let row = self
            .store
            .activity(id)?
            .ok_or_else(|| ServiceError::NotFound(id.into()))?;
        Ok(parse_fit(&std::fs::read(files::join_rel(
            &self.root,
            &row.fit_path,
        ))?)?)
    }

    pub(crate) fn photo_dto(&self, p: &PhotoRow) -> PhotoDto {
        PhotoDto {
            id: p.id,
            file_name: Path::new(&p.path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            path: self.abs(&p.path),
            card_path: p.card_path.as_deref().map(|c| self.abs(c)),
            taken_at: p.taken_at.map(|t| t.to_rfc3339()),
            km_mark: p.km_mark,
            lat: p.lat,
            lon: p.lon,
        }
    }
}
