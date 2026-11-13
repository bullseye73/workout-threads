//! km 구간 기록.
use crate::model::Sample;

#[derive(Clone, Debug, PartialEq)]
pub struct Split {
    /// 몇 번째 km (1부터). 마지막 미만 구간도 다음 번호로 포함.
    pub km: u32,
    pub duration_s: f64,
    pub avg_hr: Option<u8>,
}

pub fn km_splits(samples: &[Sample]) -> Vec<Split> {
    let pts: Vec<&Sample> = samples.iter().filter(|s| s.distance_m.is_some()).collect();
    let Some(first) = pts.first() else {
        return vec![];
    };
    let mut out = Vec::new();
    let (mut start_t, mut hr_sum, mut hr_n, mut km) = (first.time, 0u64, 0u64, 1u32);
    let mut flush = |end_t: chrono::DateTime<chrono::Utc>,
                     km: u32,
                     hr_sum: u64,
                     hr_n: u64,
                     start_t: chrono::DateTime<chrono::Utc>| {
        out.push(Split {
            km,
            duration_s: (end_t - start_t).num_milliseconds() as f64 / 1000.0,
            avg_hr: (hr_n > 0).then(|| (hr_sum / hr_n) as u8),
        })
    };
    for s in &pts {
        if let Some(hr) = s.heart_rate {
            hr_sum += hr as u64;
            hr_n += 1;
        }
        if s.distance_m.unwrap() >= km as f64 * 1000.0 {
            flush(s.time, km, hr_sum, hr_n, start_t);
            start_t = s.time;
            hr_sum = 0;
            hr_n = 0;
            km += 1;
        }
    }
    let last = pts.last().unwrap();
    if last.distance_m.unwrap() > (km - 1) as f64 * 1000.0 && last.time > start_t {
        flush(last.time, km, hr_sum, hr_n, start_t);
    }
    out
}
