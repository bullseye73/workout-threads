use chrono::{DateTime, Duration, NaiveDateTime, Utc};

/// 운동 종목. FIT `sport` 코드에서 만든다.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Sport {
    Running,
    Cycling,
    Walking,
    Hiking,
    Other(u8),
}

impl Sport {
    pub fn from_fit(code: u8) -> Sport {
        match code {
            1 => Sport::Running,
            2 => Sport::Cycling,
            11 => Sport::Walking,
            17 => Sport::Hiking,
            c => Sport::Other(c),
        }
    }

    /// [`Sport::from_fit`]의 역함수.
    pub fn to_fit(self) -> u8 {
        match self {
            Sport::Running => 1,
            Sport::Cycling => 2,
            Sport::Walking => 11,
            Sport::Hiking => 17,
            Sport::Other(c) => c,
        }
    }
}

/// 위도·경도 (도 단위).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct LatLon {
    pub lat: f64,
    pub lon: f64,
}

/// FIT record 한 개 (보통 1초 간격).
#[derive(Clone, Debug)]
pub struct Sample {
    pub time: DateTime<Utc>,
    pub pos: Option<LatLon>,
    pub distance_m: Option<f64>,
    pub heart_rate: Option<u8>,
    pub speed_mps: Option<f64>,
    pub altitude_m: Option<f64>,
    pub cadence: Option<u8>,
}

/// 운동 요약 지표. `moving_s`는 FIT total_timer_time.
#[derive(Clone, Debug)]
pub struct Summary {
    pub distance_m: f64,
    pub moving_s: f64,
    pub elapsed_s: f64,
    pub avg_hr: Option<u8>,
    pub max_hr: Option<u8>,
    pub avg_speed_mps: f64,
    pub ascent_m: Option<u16>,
    pub avg_cadence: Option<u8>,
}

/// 시계열 없이 목록·통계에 쓰는 운동 정보. `id` = "{기기시리얼}-{시작 unix초}".
#[derive(Clone, Debug)]
pub struct ActivityHeader {
    pub id: String,
    pub sport: Sport,
    pub start: DateTime<Utc>,
    pub utc_offset_s: i32,
    pub summary: Summary,
}

impl ActivityHeader {
    /// 운동한 곳의 현지 시작 시각.
    pub fn start_local(&self) -> NaiveDateTime {
        (self.start + Duration::seconds(self.utc_offset_s as i64)).naive_utc()
    }

    /// 종료 시각 = 시작 + 경과 시간.
    pub fn end(&self) -> DateTime<Utc> {
        self.start + Duration::milliseconds((self.summary.elapsed_s * 1000.0) as i64)
    }
}

/// 시계열을 포함한 운동 전체.
#[derive(Clone, Debug)]
pub struct Activity {
    pub header: ActivityHeader,
    pub samples: Vec<Sample>,
}
