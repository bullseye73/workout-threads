//! 라이브러리 안 파일 이름 정하기와 안전한 복사.
use std::io::Write;
use std::path::{Path, PathBuf};

/// 내용 해시 (FNV-1a 64비트, 버전이 바뀌어도 값이 같다).
pub fn content_hash(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}-{}", bytes.len())
}

fn stem_key(name: &str) -> String {
    let stem = Path::new(name)
        .file_stem()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    stem.strip_suffix("_card")
        .map(str::to_string)
        .unwrap_or(stem)
}

/// `dir` 안에서 이름(확장자 제외, 대소문자 무시, `_card` 제외)이 겹치지 않는 상대 경로.
/// 겹치면 `이름-2.확장자`, `이름-3.확장자` … 를 쓴다. 카드 이름도 함께 겹치지 않게 된다.
pub fn unique_rel(root: &Path, dir: &str, name: &str) -> String {
    let taken: Vec<String> = std::fs::read_dir(root.join(dir))
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| stem_key(&e.file_name().to_string_lossy()))
                .collect()
        })
        .unwrap_or_default();
    let p = Path::new(name);
    let stem = p
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = p
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    let mut n = 1;
    loop {
        let candidate = if n == 1 {
            format!("{stem}{ext}")
        } else {
            format!("{stem}-{n}{ext}")
        };
        if !taken.contains(&stem_key(&candidate)) {
            return format!("{dir}/{candidate}");
        }
        n += 1;
    }
}

/// 라이브러리 상대 경로 → 절대 경로 (구분자는 OS에 맞게).
pub fn join_rel(root: &Path, rel: &str) -> PathBuf {
    rel.split('/')
        .fold(root.to_path_buf(), |p, part| p.join(part))
}

/// 절대 덮어쓰지 않는 복사. 대상이 있으면 오류.
pub fn copy_new(src_bytes: &[u8], dest: &Path) -> std::io::Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dest)?;
    f.write_all(src_bytes)
}
