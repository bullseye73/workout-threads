//! 비밀 값 보관 (Claude API 키, 구글 토큰). 실제 앱은 OS 키 보관함
//! (macOS 키체인, Windows 자격 증명 관리자, Linux Secret Service)을 쓴다.
use std::collections::HashMap;
use std::sync::Mutex;

use crate::{Result, ServiceError};

const SERVICE: &str = "dev.sns-create.workout";
/// Claude API 키 (3단계부터 같은 이름이라 기존 키가 그대로 유지된다).
pub const CLAUDE_KEY: &str = "anthropic_api_key";
pub const GEMINI_KEY: &str = "gemini_api_key";
pub const GOOGLE_REFRESH: &str = "google_refresh_token";
/// "토큰|만료 unix초" 형식.
pub const GOOGLE_ACCESS: &str = "google_access_token";

pub trait SecretStore: Send + Sync {
    fn get(&self, name: &str) -> Option<String>;
    fn set(&self, name: &str, value: Option<&str>) -> Result<()>;
}

/// OS 키 보관함.
pub struct KeyringStore;

impl SecretStore for KeyringStore {
    fn get(&self, name: &str) -> Option<String> {
        keyring::Entry::new(SERVICE, name)
            .ok()?
            .get_password()
            .ok()
            .filter(|k| !k.is_empty())
    }

    fn set(&self, name: &str, value: Option<&str>) -> Result<()> {
        let entry = keyring::Entry::new(SERVICE, name)
            .map_err(|e| ServiceError::Failed(format!("키 보관함 오류: {e}")))?;
        let res = match value {
            Some(v) => entry.set_password(v),
            None => match entry.delete_credential() {
                Err(keyring::Error::NoEntry) => Ok(()),
                other => other,
            },
        };
        res.map_err(|e| ServiceError::Failed(format!("키 보관함 오류: {e}")))
    }
}

/// 메모리 보관 (테스트용).
#[derive(Default)]
pub struct MemoryStore(Mutex<HashMap<String, String>>);

impl SecretStore for MemoryStore {
    fn get(&self, name: &str) -> Option<String> {
        self.0.lock().ok()?.get(name).cloned()
    }

    fn set(&self, name: &str, value: Option<&str>) -> Result<()> {
        let mut m = self
            .0
            .lock()
            .map_err(|_| ServiceError::Failed("잠금 실패".into()))?;
        match value {
            Some(v) => m.insert(name.to_string(), v.to_string()),
            None => m.remove(name),
        };
        Ok(())
    }
}
