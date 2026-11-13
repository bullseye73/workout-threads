//! 월간 대시보드 통계.
use chrono::{Datelike, Duration, NaiveDate};
use std::collections::BTreeMap;

use crate::model::{ActivityHeader, Sport};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SportStats {
    pub count: u32,
    pub distance_m: f64,
    pub moving_s: f64,
}

/// 월요일 시작 주. 그 달에 속한 운동만 더한다.
#[derive(Clone, Debug, PartialEq)]
pub struct WeekBucket {
    pub week_start: NaiveDate,
    pub running_m: f64,
    pub cycling_m: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DayItem {
    pub activity_id: String,
    pub sport: Sport,
    pub distance_m: f64,
}

#[derive(Clone, Debug)]
pub struct MonthStats {
    pub year: i32,
    pub month: u32,
    /// 모든 종목 합.
    pub total_distance_m: f64,
    pub total_moving_s: f64,
    pub running: SportStats,
    pub cycling: SportStats,
    /// 걷기·하이킹·기타.
    pub other: SportStats,
    /// 이동 시간 가중 평균 심박.
    pub avg_hr: Option<f64>,
    /// 그 달과 겹치는 모든 주 (월요일 시작).
    pub weekly: Vec<WeekBucket>,
    pub days: BTreeMap<NaiveDate, Vec<DayItem>>,
}

fn monday_of(d: NaiveDate) -> NaiveDate {
    d - Duration::days(d.weekday().num_days_from_monday() as i64)
}

/// 운동은 현지 시작 날짜 기준으로 달·날짜에 귀속한다.
pub fn month_stats(headers: &[ActivityHeader], year: i32, month: u32) -> MonthStats {
    let first = NaiveDate::from_ymd_opt(year, month, 1).expect("유효한 연·월");
    let last = first.checked_add_months(chrono::Months::new(1)).unwrap() - Duration::days(1);
    let mut weekly = Vec::new();
    let mut w = monday_of(first);
    while w <= last {
        weekly.push(WeekBucket {
            week_start: w,
            running_m: 0.0,
            cycling_m: 0.0,
        });
        w += Duration::days(7);
    }
    let mut s = MonthStats {
        year,
        month,
        total_distance_m: 0.0,
        total_moving_s: 0.0,
        running: SportStats::default(),
        cycling: SportStats::default(),
        other: SportStats::default(),
        avg_hr: None,
        weekly,
        days: BTreeMap::new(),
    };
    let (mut hr_weighted, mut hr_secs) = (0.0, 0.0);
    for h in headers {
        let day = h.start_local().date();
        if day < first || day > last {
            continue;
        }
        let sm = &h.summary;
        s.total_distance_m += sm.distance_m;
        s.total_moving_s += sm.moving_s;
        let bucket = match h.sport {
            Sport::Running => &mut s.running,
            Sport::Cycling => &mut s.cycling,
            _ => &mut s.other,
        };
        bucket.count += 1;
        bucket.distance_m += sm.distance_m;
        bucket.moving_s += sm.moving_s;
        if let Some(hr) = sm.avg_hr {
            hr_weighted += hr as f64 * sm.moving_s;
            hr_secs += sm.moving_s;
        }
        if let Some(wb) = s
            .weekly
            .iter_mut()
            .find(|wb| wb.week_start == monday_of(day))
        {
            match h.sport {
                Sport::Running => wb.running_m += sm.distance_m,
                Sport::Cycling => wb.cycling_m += sm.distance_m,
                _ => {}
            }
        }
        s.days.entry(day).or_default().push(DayItem {
            activity_id: h.id.clone(),
            sport: h.sport,
            distance_m: sm.distance_m,
        });
    }
    s.avg_hr = (hr_secs > 0.0).then(|| hr_weighted / hr_secs);
    s
}
