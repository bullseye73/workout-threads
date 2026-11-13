use workout_core::model::ActivityHeader;

/// 장소명. `estimated`는 화면 표시용(글에는 쓰지 않음).
#[derive(Clone, Debug, PartialEq)]
pub struct PlaceText {
    pub name: String,
    pub estimated: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WeatherText {
    pub description: String,
    pub temp_c: f64,
}

/// 글을 쓰는 데 필요한 재료. 장소·경로·날씨는 비어 있을 수 있다.
#[derive(Clone, Debug)]
pub struct PostContext {
    pub activity: ActivityHeader,
    pub place: Option<PlaceText>,
    /// 경로 중간 지점 이름 (출발·도착 제외).
    pub route_points: Vec<String>,
    pub weather: Option<WeatherText>,
    pub photo_km_marks: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PostText {
    pub body: String,
    pub hashtags: Vec<String>,
}

impl PostText {
    /// 게시용 전체 글: 본문 + 빈 줄 + 해시태그.
    pub fn full_text(&self) -> String {
        if self.hashtags.is_empty() {
            self.body.clone()
        } else {
            format!("{}\n\n{}", self.body, self.hashtags.join(" "))
        }
    }
}
