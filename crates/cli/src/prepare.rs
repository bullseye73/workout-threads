//! `workout-cli prepare <폴더>`: 폴더의 FIT·사진으로 카드와 템플릿 글을 만든다.
use anyhow::{Result, bail};
use std::path::{Path, PathBuf};
use workout_card::{card_data, render_card_to_file};
use workout_core::fit::parse_fit;
use workout_core::matching::assign_photos;
use workout_core::model::Activity;
use workout_core::photo::is_photo_input;
use workout_post::{DraftWriter, PostContext, TemplateWriter};

#[derive(Debug, Default)]
pub struct PrepareReport {
    pub activities: usize,
    pub cards: Vec<PathBuf>,
    pub posts: Vec<PathBuf>,
    pub skipped_fit: Vec<(PathBuf, String)>,
    /// 이미 읽은 운동과 같은 기록(같은 id)이라 건너뛴 FIT.
    pub duplicate_fit: Vec<PathBuf>,
    /// 카드를 만들지 못한 사진과 이유.
    pub failed_cards: Vec<(PathBuf, String)>,
    pub unmatched_photos: Vec<PathBuf>,
}

fn has_ext(p: &Path, ext: &str) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(ext))
}

pub fn prepare(folder: &Path) -> Result<PrepareReport> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(folder)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    entries.sort();
    let mut report = PrepareReport::default();

    let mut activities: Vec<Activity> = Vec::new();
    for fit in entries.iter().filter(|p| has_ext(p, "fit")) {
        match std::fs::read(fit)
            .map_err(|e| e.to_string())
            .and_then(|b| parse_fit(&b).map_err(|e| e.to_string()))
        {
            Ok(a) if activities.iter().any(|b| b.header.id == a.header.id) => {
                report.duplicate_fit.push(fit.clone())
            }
            Ok(a) => activities.push(a),
            Err(e) => report.skipped_fit.push((fit.clone(), e)),
        }
    }
    if activities.is_empty() {
        bail!(
            "{} 안에 읽을 수 있는 .fit 파일이 없습니다.",
            folder.display()
        );
    }
    report.activities = activities.len();
    let photos: Vec<PathBuf> = entries
        .iter()
        .filter(|p| is_photo_input(p))
        .cloned()
        .collect();
    // 운동별 사진 촬영 지점 (km).
    let mut marks: Vec<Vec<f64>> = vec![Vec::new(); activities.len()];
    for asg in assign_photos(&activities, &photos) {
        let photo = &asg.photo;
        let Some(m) = asg.matched else {
            report.unmatched_photos.push(photo.clone());
            continue;
        };
        let idx = activities
            .iter()
            .position(|a| a.header.id == m.activity_id)
            .expect("매칭된 운동");
        let stem = photo.file_stem().unwrap().to_string_lossy();
        let out = photo.with_file_name(format!("{stem}_card.jpg"));
        marks[idx].push(m.km_mark);
        match render_card_to_file(photo, &card_data(&activities[idx]), &out) {
            Ok(()) => report.cards.push(out),
            Err(e) => report.failed_cards.push((photo.clone(), e.to_string())),
        }
    }

    for (a, mut km) in activities.iter().zip(marks) {
        km.sort_by(f64::total_cmp);
        let ctx = PostContext {
            activity: a.header.clone(),
            place: None,
            route_points: vec![],
            weather: None,
            photo_km_marks: km,
        };
        let text = TemplateWriter.write(&ctx)?;
        let hhmm = a.header.start_local().format("%H%M").to_string();
        let out = unique_post_path(folder, &hhmm, &report.posts);
        std::fs::write(&out, text.full_text())?;
        report.posts.push(out);
    }
    Ok(report)
}

/// post_HHMM.txt, 이번 실행에서 이미 쓴 이름이면 post_HHMM_2.txt …
fn unique_post_path(folder: &Path, hhmm: &str, used: &[PathBuf]) -> PathBuf {
    let mut n = 1;
    loop {
        let name = if n == 1 {
            format!("post_{hhmm}.txt")
        } else {
            format!("post_{hhmm}_{n}.txt")
        };
        let p = folder.join(name);
        if !used.contains(&p) {
            return p;
        }
        n += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_minute_posts_get_suffix() {
        let f = Path::new("/x");
        let first = unique_post_path(f, "0925", &[]);
        assert_eq!(first, f.join("post_0925.txt"));
        assert_eq!(
            unique_post_path(f, "0925", std::slice::from_ref(&first)),
            f.join("post_0925_2.txt")
        );
    }
}
