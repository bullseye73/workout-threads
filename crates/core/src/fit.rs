//! Garmin FIT 활동 파일 → [`Activity`].
use chrono::{DateTime, Local, NaiveDate, Offset, TimeZone, Utc};
use fitparser::de::{DecodeOption, from_bytes_with_options};
use fitparser::profile::MesgNum;
use fitparser::{FitDataRecord, Value};
use std::collections::HashMap;
use std::collections::HashSet;

use crate::CoreError;
use crate::model::*;

const SEMI_TO_DEG: f64 = 180.0 / 2147483648.0;

/// fitparser와 무관한 session 요약. 테스트에서 합성 데이터로도 만든다.
#[derive(Clone, Debug)]
pub struct RawSession {
    pub sport: u8,
    pub start: DateTime<Utc>,
    pub distance_m: f64,
    pub timer_s: f64,
    pub elapsed_s: f64,
    pub avg_hr: Option<u8>,
    pub max_hr: Option<u8>,
    pub avg_speed_mps: Option<f64>,
    pub ascent_m: Option<u16>,
    pub avg_cadence: Option<u8>,
}

/// FIT 바이트를 해석한다. 잘리거나 깨진 파일은 패닉 없이 [`CoreError::Fit`].
pub fn parse_fit(bytes: &[u8]) -> Result<Activity, CoreError> {
    check_complete(bytes)?;
    let opts: HashSet<DecodeOption> = [DecodeOption::ReturnNumericEnumValues]
        .into_iter()
        .collect();
    let records = std::panic::catch_unwind(|| from_bytes_with_options(bytes, &opts))
        .map_err(|_| CoreError::Fit("디코더 내부 오류".into()))?
        .map_err(|e| CoreError::Fit(e.to_string()))?;

    let mut serial = None;
    let mut session = None;
    let mut samples = Vec::new();
    let mut local_offset = None;
    for r in &records {
        let f = fields(r);
        match r.kind() {
            MesgNum::FileId => {
                serial = serial.or(f.get("serial_number").and_then(as_f64).map(|v| v as u32))
            }
            MesgNum::Session if session.is_none() => session = Some(raw_session(&f)?),
            MesgNum::Record => {
                if let Some(s) = sample(&f) {
                    samples.push(s);
                }
            }
            MesgNum::Activity => local_offset = local_offset.or_else(|| activity_offset(&f)),
            _ => {}
        }
    }
    let session = session.ok_or_else(|| CoreError::Fit("session 메시지 없음".into()))?;
    Ok(activity_from_records(
        session,
        samples,
        serial,
        local_offset,
    ))
}

/// 헤더에 적힌 데이터 크기 + CRC 2바이트만큼 있는지 확인한다.
/// fitparser는 메시지 경계에서 잘린 파일을 정상으로 읽어 버리므로 미리 막는다.
fn check_complete(bytes: &[u8]) -> Result<(), CoreError> {
    if bytes.len() < 12 {
        return Err(CoreError::Fit("파일이 너무 짧음".into()));
    }
    let header = bytes[0] as usize;
    let data = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
    if bytes.len() < header + data + 2 {
        return Err(CoreError::Fit("파일이 잘림".into()));
    }
    Ok(())
}

/// 해석된 조각으로 [`Activity`]를 만든다. 오프셋이 없으면 이 컴퓨터의 시간대.
pub fn activity_from_records(
    session: RawSession,
    samples: Vec<Sample>,
    serial: Option<u32>,
    local_offset_s: Option<i32>,
) -> Activity {
    let utc_offset_s = local_offset_s.unwrap_or_else(|| {
        Local
            .offset_from_utc_datetime(&session.start.naive_utc())
            .fix()
            .local_minus_utc()
    });
    let avg_speed_mps = session.avg_speed_mps.unwrap_or(if session.timer_s > 0.0 {
        session.distance_m / session.timer_s
    } else {
        0.0
    });
    Activity {
        header: ActivityHeader {
            id: format!("{}-{}", serial.unwrap_or(0), session.start.timestamp()),
            sport: Sport::from_fit(session.sport),
            start: session.start,
            utc_offset_s,
            summary: Summary {
                distance_m: session.distance_m,
                moving_s: session.timer_s,
                elapsed_s: session.elapsed_s,
                avg_hr: session.avg_hr,
                max_hr: session.max_hr,
                avg_speed_mps,
                ascent_m: session.ascent_m,
                avg_cadence: session.avg_cadence,
            },
        },
        samples,
    }
}

fn fields(r: &FitDataRecord) -> HashMap<&str, &Value> {
    r.fields().iter().map(|f| (f.name(), f.value())).collect()
}

fn as_f64(v: &&Value) -> Option<f64> {
    Some(match v {
        Value::Enum(x) | Value::UInt8(x) | Value::UInt8z(x) | Value::Byte(x) => *x as f64,
        Value::SInt8(x) => *x as f64,
        Value::SInt16(x) => *x as f64,
        Value::UInt16(x) | Value::UInt16z(x) => *x as f64,
        Value::SInt32(x) => *x as f64,
        Value::UInt32(x) | Value::UInt32z(x) => *x as f64,
        Value::SInt64(x) => *x as f64,
        Value::UInt64(x) | Value::UInt64z(x) => *x as f64,
        Value::Float32(x) => *x as f64,
        Value::Float64(x) => *x,
        _ => return None,
    })
}

fn as_time(v: &&Value) -> Option<DateTime<Utc>> {
    match v {
        Value::Timestamp(t) => Some(t.with_timezone(&Utc)),
        _ => None,
    }
}

fn num(f: &HashMap<&str, &Value>, keys: &[&str]) -> Option<f64> {
    keys.iter().find_map(|k| f.get(k).and_then(as_f64))
}

fn raw_session(f: &HashMap<&str, &Value>) -> Result<RawSession, CoreError> {
    let start = f
        .get("start_time")
        .or(f.get("timestamp"))
        .and_then(as_time)
        .ok_or_else(|| CoreError::Fit("session 시작 시각 없음".into()))?;
    let timer_s = num(f, &["total_timer_time"]).unwrap_or(0.0);
    Ok(RawSession {
        sport: num(f, &["sport"]).unwrap_or(0.0) as u8,
        start,
        distance_m: num(f, &["total_distance"]).unwrap_or(0.0),
        timer_s,
        elapsed_s: num(f, &["total_elapsed_time"]).unwrap_or(timer_s),
        avg_hr: num(f, &["avg_heart_rate"]).map(|v| v as u8),
        max_hr: num(f, &["max_heart_rate"]).map(|v| v as u8),
        avg_speed_mps: num(f, &["enhanced_avg_speed", "avg_speed"]),
        ascent_m: num(f, &["total_ascent"]).map(|v| v as u16),
        avg_cadence: num(f, &["avg_cadence", "avg_running_cadence"]).map(|v| v as u8),
    })
}

fn sample(f: &HashMap<&str, &Value>) -> Option<Sample> {
    let time = f.get("timestamp").and_then(as_time)?;
    let pos = match (num(f, &["position_lat"]), num(f, &["position_long"])) {
        (Some(la), Some(lo)) => Some(LatLon {
            lat: la * SEMI_TO_DEG,
            lon: lo * SEMI_TO_DEG,
        }),
        _ => None,
    };
    Some(Sample {
        time,
        pos,
        distance_m: num(f, &["distance"]),
        heart_rate: num(f, &["heart_rate"]).map(|v| v as u8),
        speed_mps: num(f, &["enhanced_speed", "speed"]),
        altitude_m: num(f, &["enhanced_altitude", "altitude"]),
        cadence: num(f, &["cadence"]).map(|v| v as u8),
    })
}

/// activity 메시지의 local_timestamp − timestamp.
/// fitparser는 local_timestamp를 *이 컴퓨터의* 시간대로 해석하므로 그만큼 되돌린다.
fn activity_offset(f: &HashMap<&str, &Value>) -> Option<i32> {
    let utc = f.get("timestamp").and_then(as_time)?;
    let local = f.get("local_timestamp").and_then(as_time)?;
    let fit_ref = NaiveDate::from_ymd_opt(1989, 12, 31)?.and_hms_opt(0, 0, 0)?;
    let machine = Local
        .offset_from_local_datetime(&fit_ref)
        .single()?
        .fix()
        .local_minus_utc();
    Some((local - utc).num_seconds() as i32 + machine)
}
