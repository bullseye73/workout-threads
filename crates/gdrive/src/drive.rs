//! 구글 드라이브 v3: 폴더 찾기·만들기, 업로드·덮어쓰기, 목록, 다운로드.
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use workout_enrich::{Body, Http, HttpRequest, HttpResponse, Method, TimeoutClass};

use crate::{DriveError, Tokens};

const API: &str = "https://www.googleapis.com/drive/v3/files";
const UPLOAD: &str = "https://www.googleapis.com/upload/drive/v3/files";
pub const FOLDER_MIME: &str = "application/vnd.google-apps.folder";

#[derive(Clone, Debug, PartialEq)]
pub struct RemoteFile {
    pub id: String,
    pub name: String,
    pub is_folder: bool,
    /// 마지막 수정 시각 (RFC3339, 문자열 비교로 순서가 맞음).
    pub modified: String,
}

pub trait DriveApi: Send + Sync {
    fn find_folder(&self, name: &str, parent: Option<&str>) -> Result<Option<String>, DriveError>;
    fn create_folder(&self, name: &str, parent: Option<&str>) -> Result<String, DriveError>;
    fn upload_new(
        &self,
        parent: &str,
        name: &str,
        content_type: &str,
        data: &[u8],
    ) -> Result<String, DriveError>;
    fn update(&self, id: &str, content_type: &str, data: &[u8]) -> Result<(), DriveError>;
    fn list(&self, parent: &str) -> Result<Vec<RemoteFile>, DriveError>;
    fn download(&self, id: &str) -> Result<Vec<u8>, DriveError>;
    /// 작업 중 갱신된 토큰 (HTTP 구현만 돌려준다).
    fn current_tokens(&self) -> Option<Tokens> {
        None
    }
}

fn escape(name: &str) -> String {
    name.replace('\\', "\\\\").replace('\'', "\\'")
}

type RefreshFn = Box<dyn Fn() -> Result<Tokens, DriveError> + Send + Sync>;

/// 토큰을 들고 Drive v3를 부른다. 401이면 refresh로 한 번 갱신한 뒤 다시 시도한다.
pub struct HttpDrive {
    http: Arc<dyn Http>,
    tokens: Mutex<Tokens>,
    refresh: RefreshFn,
}

impl HttpDrive {
    pub fn new(http: Arc<dyn Http>, tokens: Tokens, refresh: RefreshFn) -> Self {
        HttpDrive {
            http,
            tokens: Mutex::new(tokens),
            refresh,
        }
    }

    /// 갱신됐을 수 있는 현재 토큰 (호출한 쪽이 키 보관함에 다시 저장).
    pub fn current_tokens(&self) -> Tokens {
        self.tokens.lock().unwrap().clone()
    }

    fn call(
        &self,
        method: Method,
        url: String,
        body: Body,
        timeout: TimeoutClass,
    ) -> Result<HttpResponse, DriveError> {
        for attempt in 0..2 {
            let access = self.tokens.lock().unwrap().access.clone();
            let res = self.http.send(HttpRequest {
                method,
                url: url.clone(),
                headers: vec![("Authorization".into(), format!("Bearer {access}"))],
                body: body.clone(),
                timeout,
            })?;
            if res.status == 401 && attempt == 0 {
                let fresh = (self.refresh)()?;
                *self.tokens.lock().unwrap() = fresh;
                continue;
            }
            if res.status == 401 {
                return Err(DriveError::NeedLogin);
            }
            if !(200..300).contains(&res.status) {
                let v: Value = serde_json::from_slice(&res.body).unwrap_or(Value::Null);
                let msg = v["error"]["message"].as_str().unwrap_or("").to_string();
                return Err(DriveError::Status(res.status, msg));
            }
            return Ok(res);
        }
        Err(DriveError::NeedLogin)
    }

    fn json(res: &HttpResponse) -> Result<Value, DriveError> {
        serde_json::from_slice(&res.body).map_err(|e| DriveError::Parse(e.to_string()))
    }

    fn id_of(res: &HttpResponse) -> Result<String, DriveError> {
        Self::json(res)?["id"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| DriveError::Parse("id 없음".into()))
    }

    fn query_url(q: &str, page: Option<&str>) -> String {
        let mut u = url::Url::parse(API).expect("고정 URL");
        u.query_pairs_mut()
            .append_pair("q", q)
            .append_pair(
                "fields",
                "files(id,name,mimeType,modifiedTime),nextPageToken",
            )
            .append_pair("pageSize", "1000")
            .append_pair("spaces", "drive");
        if let Some(p) = page {
            u.query_pairs_mut().append_pair("pageToken", p);
        }
        u.into()
    }
}

impl DriveApi for HttpDrive {
    fn current_tokens(&self) -> Option<Tokens> {
        Some(HttpDrive::current_tokens(self))
    }

    fn find_folder(&self, name: &str, parent: Option<&str>) -> Result<Option<String>, DriveError> {
        let q = format!(
            "name='{}' and mimeType='{FOLDER_MIME}' and '{}' in parents and trashed=false",
            escape(name),
            escape(parent.unwrap_or("root"))
        );
        let res = self.call(
            Method::Get,
            Self::query_url(&q, None),
            Body::Empty,
            TimeoutClass::Lookup,
        )?;
        Ok(Self::json(&res)?["files"]
            .as_array()
            .and_then(|f| f.first())
            .and_then(|f| f["id"].as_str())
            .map(str::to_string))
    }

    fn create_folder(&self, name: &str, parent: Option<&str>) -> Result<String, DriveError> {
        let meta =
            json!({"name": name, "mimeType": FOLDER_MIME, "parents": [parent.unwrap_or("root")]});
        let res = self.call(
            Method::Post,
            format!("{API}?fields=id"),
            Body::Json(meta),
            TimeoutClass::Lookup,
        )?;
        Self::id_of(&res)
    }

    fn upload_new(
        &self,
        parent: &str,
        name: &str,
        content_type: &str,
        data: &[u8],
    ) -> Result<String, DriveError> {
        let boundary = "workout_app_boundary_7f3a";
        let meta = json!({"name": name, "parents": [parent]}).to_string();
        let mut body = format!(
            "--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n{meta}\r\n--{boundary}\r\nContent-Type: {content_type}\r\n\r\n"
        )
        .into_bytes();
        body.extend_from_slice(data);
        body.extend_from_slice(format!("\r\n--{boundary}--").as_bytes());
        let res = self.call(
            Method::Post,
            format!("{UPLOAD}?uploadType=multipart&fields=id"),
            Body::Bytes {
                content_type: format!("multipart/related; boundary={boundary}"),
                data: body,
            },
            TimeoutClass::Long,
        )?;
        Self::id_of(&res)
    }

    fn update(&self, id: &str, content_type: &str, data: &[u8]) -> Result<(), DriveError> {
        self.call(
            Method::Patch,
            format!("{UPLOAD}/{id}?uploadType=media&fields=id"),
            Body::Bytes {
                content_type: content_type.into(),
                data: data.to_vec(),
            },
            TimeoutClass::Long,
        )?;
        Ok(())
    }

    fn list(&self, parent: &str) -> Result<Vec<RemoteFile>, DriveError> {
        let q = format!("'{}' in parents and trashed=false", escape(parent));
        let mut out = Vec::new();
        let mut page: Option<String> = None;
        loop {
            let res = self.call(
                Method::Get,
                Self::query_url(&q, page.as_deref()),
                Body::Empty,
                TimeoutClass::Lookup,
            )?;
            let v = Self::json(&res)?;
            for f in v["files"].as_array().into_iter().flatten() {
                out.push(RemoteFile {
                    id: f["id"].as_str().unwrap_or_default().to_string(),
                    name: f["name"].as_str().unwrap_or_default().to_string(),
                    is_folder: f["mimeType"] == FOLDER_MIME,
                    modified: f["modifiedTime"].as_str().unwrap_or_default().to_string(),
                });
            }
            match v["nextPageToken"].as_str() {
                Some(p) => page = Some(p.to_string()),
                None => return Ok(out),
            }
        }
    }

    fn download(&self, id: &str) -> Result<Vec<u8>, DriveError> {
        Ok(self
            .call(
                Method::Get,
                format!("{API}/{id}?alt=media"),
                Body::Empty,
                TimeoutClass::Long,
            )?
            .body)
    }
}

#[cfg(feature = "fake")]
pub use fake::FakeDrive;

#[cfg(feature = "fake")]
mod fake {
    use super::*;
    use std::collections::BTreeMap;

    #[derive(Clone)]
    struct Node {
        name: String,
        parent: String,
        folder: bool,
        data: Vec<u8>,
        modified: u64,
    }

    /// 메모리 드라이브 (테스트용). `fail_after(n)`이면 n번 쓴 뒤부터 쓰기·읽기가 실패한다.
    #[derive(Default)]
    pub struct FakeDrive {
        nodes: Mutex<BTreeMap<String, Node>>,
        next: Mutex<u64>,
        budget: Mutex<Option<usize>>,
        need_login: Mutex<bool>,
    }

    impl FakeDrive {
        pub fn fail_after(&self, n: usize) {
            *self.budget.lock().unwrap() = Some(n);
        }

        /// 로그인 만료 흉내.
        pub fn need_login(&self, on: bool) {
            *self.need_login.lock().unwrap() = on;
        }

        pub fn heal(&self) {
            *self.budget.lock().unwrap() = None;
        }

        /// 폴더가 아닌 파일 수.
        pub fn file_count(&self) -> usize {
            self.nodes
                .lock()
                .unwrap()
                .values()
                .filter(|n| !n.folder)
                .count()
        }

        /// "WorkoutApp/files/20261025/x.jpg" 형태의 전체 경로 목록 (파일만).
        pub fn paths(&self) -> Vec<String> {
            let nodes = self.nodes.lock().unwrap();
            let path_of = |id: &str| {
                let mut parts = vec![];
                let mut cur = id.to_string();
                while let Some(n) = nodes.get(&cur) {
                    parts.push(n.name.clone());
                    cur = n.parent.clone();
                }
                parts.reverse();
                parts.join("/")
            };
            let mut v: Vec<String> = nodes
                .iter()
                .filter(|(_, n)| !n.folder)
                .map(|(id, _)| path_of(id))
                .collect();
            v.sort();
            v
        }

        fn spend(&self) -> Result<(), DriveError> {
            if *self.need_login.lock().unwrap() {
                return Err(DriveError::NeedLogin);
            }
            let mut b = self.budget.lock().unwrap();
            match b.as_mut() {
                Some(0) => Err(DriveError::Http(workout_enrich::EnrichError::Network(
                    "연결 끊김 (테스트)".into(),
                ))),
                Some(n) => {
                    *n -= 1;
                    Ok(())
                }
                None => Ok(()),
            }
        }

        fn insert(&self, mut node: Node) -> String {
            let mut next = self.next.lock().unwrap();
            *next += 1;
            let id = format!("f{next}");
            if node.modified == 0 {
                node.modified = 1000 + *next;
            }
            self.nodes.lock().unwrap().insert(id.clone(), node);
            id
        }

        fn id_named(&self, name: &str) -> Option<String> {
            self.nodes
                .lock()
                .unwrap()
                .iter()
                .find(|(_, n)| !n.folder && n.name == name)
                .map(|(id, _)| id.clone())
        }

        /// WorkoutApp 바로 아래에 더 오래된 파일을 하나 더 둔다 (중복 백업 흉내).
        pub fn add_root_file_before(&self, name: &str, data: &[u8]) {
            let root = self
                .find_folder("WorkoutApp", None)
                .ok()
                .flatten()
                .expect("WorkoutApp 폴더");
            self.insert(Node {
                name: name.into(),
                parent: root,
                folder: false,
                data: data.to_vec(),
                modified: 1,
            });
        }

        /// 이름으로 찾은 파일 내용을 바꾼다 (손상 흉내).
        pub fn overwrite_named(&self, name: &str, data: &[u8]) {
            let id = self.id_named(name).expect("파일");
            self.nodes.lock().unwrap().get_mut(&id).unwrap().data = data.to_vec();
        }

        /// 사용자가 드라이브에서 지운 것처럼.
        pub fn delete_named(&self, name: &str) {
            let id = self.id_named(name).expect("파일");
            self.nodes.lock().unwrap().remove(&id);
        }
    }

    impl DriveApi for FakeDrive {
        fn find_folder(
            &self,
            name: &str,
            parent: Option<&str>,
        ) -> Result<Option<String>, DriveError> {
            let p = parent.unwrap_or("root");
            Ok(self
                .nodes
                .lock()
                .unwrap()
                .iter()
                .find(|(_, n)| n.folder && n.name == name && n.parent == p)
                .map(|(id, _)| id.clone()))
        }

        fn create_folder(&self, name: &str, parent: Option<&str>) -> Result<String, DriveError> {
            self.spend()?;
            Ok(self.insert(Node {
                name: name.into(),
                parent: parent.unwrap_or("root").into(),
                folder: true,
                data: vec![],
                modified: 0,
            }))
        }

        fn upload_new(
            &self,
            parent: &str,
            name: &str,
            _ct: &str,
            data: &[u8],
        ) -> Result<String, DriveError> {
            self.spend()?;
            let parent_ok = self
                .nodes
                .lock()
                .unwrap()
                .get(parent)
                .is_some_and(|n| n.folder);
            if !parent_ok {
                return Err(DriveError::NotFound(parent.into()));
            }
            Ok(self.insert(Node {
                name: name.into(),
                parent: parent.into(),
                folder: false,
                data: data.to_vec(),
                modified: 0,
            }))
        }

        fn update(&self, id: &str, _ct: &str, data: &[u8]) -> Result<(), DriveError> {
            self.spend()?;
            let mut nodes = self.nodes.lock().unwrap();
            let n = nodes
                .get_mut(id)
                .ok_or_else(|| DriveError::NotFound(id.into()))?;
            n.data = data.to_vec();
            drop(nodes);
            let mut next = self.next.lock().unwrap();
            *next += 1;
            if let Some(n) = self.nodes.lock().unwrap().get_mut(id) {
                n.modified = 1000 + *next;
            }
            Ok(())
        }

        fn list(&self, parent: &str) -> Result<Vec<RemoteFile>, DriveError> {
            Ok(self
                .nodes
                .lock()
                .unwrap()
                .iter()
                .filter(|(_, n)| n.parent == parent)
                .map(|(id, n)| RemoteFile {
                    id: id.clone(),
                    name: n.name.clone(),
                    is_folder: n.folder,
                    modified: format!("{:012}", n.modified),
                })
                .collect())
        }

        fn download(&self, id: &str) -> Result<Vec<u8>, DriveError> {
            self.spend()?;
            self.nodes
                .lock()
                .unwrap()
                .get(id)
                .map(|n| n.data.clone())
                .ok_or_else(|| DriveError::NotFound(id.into()))
        }
    }
}
