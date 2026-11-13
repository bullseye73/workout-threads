//! 게시 준비: 글 조회·저장·다시 쓰기, 게시 완료, 사진 수동 지정.
use workout_core::chrono::Utc;
use workout_post::{ManualExport, PostDraft, PostText, PublishMethod, Publisher};
use workout_store::PublishRow;

use crate::*;

impl Library {
    pub fn post_view(&self, activity_id: &str) -> Result<PostView> {
        let row = self
            .store
            .activity(activity_id)?
            .ok_or_else(|| ServiceError::NotFound(activity_id.into()))?;
        let post = match self.store.post(activity_id)? {
            Some(p) => p,
            None => self.template_post(&row.header, None)?,
        };
        let cards: Vec<String> = self
            .store
            .photos_for(activity_id)?
            .iter()
            .filter_map(|p| p.card_path.as_deref().map(|c| self.abs(c)))
            .collect();
        let folder = row
            .fit_path
            .rsplit_once('/')
            .map(|(d, _)| self.abs(d))
            .unwrap_or_default();
        let text = PostText {
            body: post.body.clone(),
            hashtags: post.hashtags.clone(),
        };
        Ok(PostView {
            activity_id: activity_id.into(),
            full_text: text.full_text(),
            body: post.body,
            hashtags: post.hashtags,
            place: post.place.map(|p| PlaceDto {
                name: p.name,
                estimated: p.estimated,
            }),
            card_paths: cards,
            folder,
            status: self.post_status(activity_id)?,
            edited: post.edited,
            route_points: post.route_points,
            weather: post.weather.map(|w| WeatherDto {
                description: w.description,
                temp_c: w.temp_c,
            }),
            place_user_set: post.place_user_set,
            route_user_set: post.route_user_set,
            weather_user_set: post.weather_user_set,
            enrich_error: None,
        })
    }

    /// 사용자가 고친 글 저장. 이후 자동으로 다시 쓰지 않는다.
    pub fn save_post(
        &mut self,
        activity_id: &str,
        body: &str,
        hashtags: &[String],
    ) -> Result<PostView> {
        let row = self
            .store
            .activity(activity_id)?
            .ok_or_else(|| ServiceError::NotFound(activity_id.into()))?;
        let mut post = match self.store.post(activity_id)? {
            Some(p) => p,
            None => self.template_post(&row.header, None)?,
        };
        post.body = body.into();
        post.hashtags = hashtags.to_vec();
        post.edited = true;
        self.store.upsert_post(&post)?;
        self.post_view(activity_id)
    }

    /// 고친 내용을 버리고 템플릿으로 다시 쓴다.
    pub fn regenerate_post(&mut self, activity_id: &str) -> Result<PostView> {
        let row = self
            .store
            .activity(activity_id)?
            .ok_or_else(|| ServiceError::NotFound(activity_id.into()))?;
        let post = self.template_post(&row.header, self.store.post(activity_id)?)?;
        self.store.upsert_post(&post)?;
        self.post_view(activity_id)
    }

    /// 수동 게시 완료: 운동 폴더에 post.txt를 쓰고 기록을 남긴다.
    pub fn mark_published(&mut self, activity_id: &str) -> Result<PostView> {
        let view = self.post_view(activity_id)?;
        let draft = PostDraft {
            activity_id: activity_id.into(),
            text: PostText {
                body: view.body.clone(),
                hashtags: view.hashtags.clone(),
            },
            card_paths: vec![],
        };
        let receipt = ManualExport {
            out_dir: view.folder.clone().into(),
        }
        .publish(&draft)?;
        self.store.insert_publish(&PublishRow {
            activity_id: activity_id.into(),
            method: PublishMethod::ManualExport,
            at: receipt.at,
            external_id: receipt.external_id,
        })?;
        let row = self
            .store
            .activity(activity_id)?
            .ok_or_else(|| ServiceError::NotFound(activity_id.into()))?;
        let mut post = match self.store.post(activity_id)? {
            Some(p) => p,
            None => self.template_post(&row.header, None)?,
        };
        post.published_at = Some(Utc::now());
        self.store.upsert_post(&post)?;
        self.post_view(activity_id)
    }

    /// 매칭 안 된 사진을 운동에 붙인다: 운동 폴더로 옮기고 카드를 만든다 (기존 파일은 덮어쓰지 않음).
    pub fn assign_photo(&mut self, photo_id: i64, activity_id: &str) -> Result<PhotoDto> {
        let photo = self
            .store
            .photo(photo_id)?
            .ok_or_else(|| ServiceError::NotFound(photo_id.to_string()))?;
        let activity = self.load_activity(activity_id)?;
        let photo = self.attach_photo(photo, &activity, None)?;
        self.write_template_post(activity_id)?;
        Ok(self.photo_dto(&photo))
    }

    /// 아직 운동에 붙지 않은 사진.
    pub fn unmatched_photos(&self) -> Result<Vec<PhotoDto>> {
        Ok(self
            .store
            .unmatched_photos()?
            .iter()
            .map(|p| self.photo_dto(p))
            .collect())
    }
}
