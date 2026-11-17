//! 로컬 라이브러리 저장소 (SQLite).
mod rows;
mod schema;

pub use rows::{ActivityRow, PhotoRow, PostRow, PublishRow};

use rusqlite::{Connection, OptionalExtension, Row, params};
use std::path::Path;
use workout_core::chrono::{DateTime, Duration, TimeZone, Utc};
use workout_core::model::{ActivityHeader, LatLon, Sport, Summary};
use workout_post::{PlaceText, PublishMethod, WeatherText};

#[derive(thiserror::Error, Debug)]
pub enum StoreError {
    #[error("DB 오류: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("데이터 형식 오류: {0}")]
    Json(#[from] serde_json::Error),
}

type Result<T> = std::result::Result<T, StoreError>;

pub struct Store {
    conn: Connection,
}

const ACTIVITY_COLS: &str =
    "id, sport, start_utc, utc_offset_s, distance_m, moving_s, elapsed_s, avg_hr, max_hr,
    avg_speed_mps, ascent_m, avg_cadence, fit_path, route_json";
const PHOTO_COLS: &str =
    "id, activity_id, path, taken_at, km_mark, lat, lon, card_path, content_hash";

fn ts(t: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(t, 0).single().unwrap_or_default()
}

impl Store {
    pub fn open(path: &Path) -> Result<Store> {
        Self::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Store> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Store> {
        conn.pragma_update(None, "foreign_keys", true)?;
        let _ = conn.pragma_update(None, "journal_mode", "WAL");
        let _ = conn.pragma_update(None, "synchronous", "NORMAL");
        schema::migrate(&conn)?;
        Ok(Store { conn })
    }

    pub fn has_activity(&self, id: &str) -> Result<bool> {
        Ok(self
            .conn
            .query_row("SELECT 1 FROM activities WHERE id = ?1", [id], |_| Ok(()))
            .optional()?
            .is_some())
    }

    pub fn insert_activity(&self, row: &ActivityRow) -> Result<()> {
        let h = &row.header;
        let s = &h.summary;
        let route: Vec<[f64; 2]> = row.route.iter().map(|p| [p.lat, p.lon]).collect();
        self.conn.execute(
            "INSERT INTO activities (id, sport, start_utc, start_local, utc_offset_s, distance_m, moving_s, elapsed_s,
             avg_hr, max_hr, avg_speed_mps, ascent_m, avg_cadence, fit_path, route_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                h.id,
                h.sport.to_fit(),
                h.start.timestamp(),
                h.start_local().format("%Y-%m-%d %H:%M:%S").to_string(),
                h.utc_offset_s,
                s.distance_m,
                s.moving_s,
                s.elapsed_s,
                s.avg_hr,
                s.max_hr,
                s.avg_speed_mps,
                s.ascent_m,
                s.avg_cadence,
                row.fit_path,
                serde_json::to_string(&route)?,
            ],
        )?;
        Ok(())
    }

    pub fn activity(&self, id: &str) -> Result<Option<ActivityRow>> {
        let sql = format!("SELECT {ACTIVITY_COLS} FROM activities WHERE id = ?1");
        let raw = self.conn.query_row(&sql, [id], raw_activity).optional()?;
        raw.map(finish_activity).transpose()
    }

    /// 현지 시작 날짜가 그 달인 운동, 시작순.
    pub fn activities_in_month(&self, year: i32, month: u32) -> Result<Vec<ActivityRow>> {
        let sql = format!(
            "SELECT {ACTIVITY_COLS} FROM activities WHERE start_local LIKE ?1 ORDER BY start_utc"
        );
        self.activities_query(&sql, params![format!("{year:04}-{month:02}-%")])
    }

    /// 시작 시각이 at ± hours 안인 운동.
    pub fn activities_near(&self, at: DateTime<Utc>, hours: i64) -> Result<Vec<ActivityRow>> {
        let sql = format!(
            "SELECT {ACTIVITY_COLS} FROM activities WHERE start_utc BETWEEN ?1 AND ?2 ORDER BY start_utc"
        );
        let d = Duration::hours(hours);
        self.activities_query(&sql, params![(at - d).timestamp(), (at + d).timestamp()])
    }

    fn activities_query(&self, sql: &str, p: impl rusqlite::Params) -> Result<Vec<ActivityRow>> {
        let mut stmt = self.conn.prepare(sql)?;
        let raws = stmt
            .query_map(p, raw_activity)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        raws.into_iter().map(finish_activity).collect()
    }

    pub fn insert_photo(&self, p: &PhotoRow) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO photos (activity_id, path, taken_at, km_mark, lat, lon, card_path, content_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                p.activity_id,
                p.path,
                p.taken_at.map(|t| t.timestamp()),
                p.km_mark,
                p.lat,
                p.lon,
                p.card_path,
                p.content_hash
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn photo(&self, id: i64) -> Result<Option<PhotoRow>> {
        let sql = format!("SELECT {PHOTO_COLS} FROM photos WHERE id = ?1");
        Ok(self.conn.query_row(&sql, [id], photo_row).optional()?)
    }

    pub fn photo_by_path(&self, path: &str) -> Result<Option<PhotoRow>> {
        let sql = format!("SELECT {PHOTO_COLS} FROM photos WHERE path = ?1");
        Ok(self.conn.query_row(&sql, [path], photo_row).optional()?)
    }

    /// 운동에 붙은 사진, 촬영 지점(km) 순 (지점 없는 사진은 뒤).
    /// 해시가 비어 있는 사진 (예전 스키마로 가져온 것).
    pub fn photos_without_hash(&self) -> Result<Vec<PhotoRow>> {
        let sql = format!("SELECT {PHOTO_COLS} FROM photos WHERE content_hash IS NULL ORDER BY id");
        self.photos_query(&sql, [])
    }

    pub fn photo_by_hash(&self, hash: &str) -> Result<Option<PhotoRow>> {
        let sql = format!("SELECT {PHOTO_COLS} FROM photos WHERE content_hash = ?1 LIMIT 1");
        Ok(self.conn.query_row(&sql, [hash], photo_row).optional()?)
    }

    pub fn photos_for(&self, activity_id: &str) -> Result<Vec<PhotoRow>> {
        let sql = format!(
            "SELECT {PHOTO_COLS} FROM photos WHERE activity_id = ?1 ORDER BY km_mark IS NULL, km_mark, id"
        );
        self.photos_query(&sql, [activity_id])
    }

    pub fn unmatched_photos(&self) -> Result<Vec<PhotoRow>> {
        let sql = format!("SELECT {PHOTO_COLS} FROM photos WHERE activity_id IS NULL ORDER BY id");
        self.photos_query(&sql, [])
    }

    fn photos_query(&self, sql: &str, p: impl rusqlite::Params) -> Result<Vec<PhotoRow>> {
        let mut stmt = self.conn.prepare(sql)?;
        Ok(stmt
            .query_map(p, photo_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn update_photo(&self, p: &PhotoRow) -> Result<()> {
        self.conn.execute(
            "UPDATE photos SET activity_id = ?2, path = ?3, taken_at = ?4, km_mark = ?5, lat = ?6, lon = ?7, card_path = ?8,
             content_hash = ?9 WHERE id = ?1",
            params![
                p.id,
                p.activity_id,
                p.path,
                p.taken_at.map(|t| t.timestamp()),
                p.km_mark,
                p.lat,
                p.lon,
                p.card_path,
                p.content_hash
            ],
        )?;
        Ok(())
    }

    pub fn post(&self, activity_id: &str) -> Result<Option<PostRow>> {
        let raw = self
            .conn
            .query_row(
                "SELECT activity_id, body, hashtags_json, place_name, place_estimated, route_points_json,
                 weather_desc, weather_temp, edited, published_at, place_user_set, route_user_set, weather_user_set,
                 auto_place_name, auto_route_json, auto_weather_desc, auto_weather_temp
                 FROM posts WHERE activity_id = ?1",
                [activity_id],
                |r| {
                    Ok(RawPost {
                        activity_id: r.get(0)?,
                        body: r.get(1)?,
                        hashtags_json: r.get(2)?,
                        place_name: r.get(3)?,
                        place_estimated: r.get(4)?,
                        route_json: r.get(5)?,
                        weather_desc: r.get(6)?,
                        weather_temp: r.get(7)?,
                        edited: r.get(8)?,
                        published_at: r.get(9)?,
                        place_user_set: r.get(10)?,
                        route_user_set: r.get(11)?,
                        weather_user_set: r.get(12)?,
                        auto_place_name: r.get(13)?,
                        auto_route_json: r.get(14)?,
                        auto_weather_desc: r.get(15)?,
                        auto_weather_temp: r.get(16)?,
                    })
                },
            )
            .optional()?;
        let Some(r) = raw else { return Ok(None) };
        Ok(Some(PostRow {
            activity_id: r.activity_id,
            body: r.body,
            hashtags: serde_json::from_str(&r.hashtags_json)?,
            place: r.place_name.map(|name| PlaceText {
                name,
                estimated: r.place_estimated,
            }),
            route_points: serde_json::from_str(&r.route_json)?,
            weather: r
                .weather_desc
                .zip(r.weather_temp)
                .map(|(description, temp_c)| WeatherText {
                    description,
                    temp_c,
                }),
            edited: r.edited,
            published_at: r.published_at.map(ts),
            place_user_set: r.place_user_set,
            route_user_set: r.route_user_set,
            weather_user_set: r.weather_user_set,
            auto_place: r.auto_place_name,
            auto_route_points: serde_json::from_str(&r.auto_route_json)?,
            auto_weather: r.auto_weather_desc.zip(r.auto_weather_temp).map(
                |(description, temp_c)| WeatherText {
                    description,
                    temp_c,
                },
            ),
        }))
    }

    pub fn upsert_post(&self, p: &PostRow) -> Result<()> {
        self.conn.execute(
            "INSERT INTO posts (activity_id, body, hashtags_json, place_name, place_estimated, route_points_json,
             weather_desc, weather_temp, edited, published_at, place_user_set, route_user_set, weather_user_set,
             auto_place_name, auto_route_json, auto_weather_desc, auto_weather_temp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
             ON CONFLICT(activity_id) DO UPDATE SET body = excluded.body, hashtags_json = excluded.hashtags_json,
             place_name = excluded.place_name, place_estimated = excluded.place_estimated,
             route_points_json = excluded.route_points_json, weather_desc = excluded.weather_desc,
             weather_temp = excluded.weather_temp, edited = excluded.edited, published_at = excluded.published_at,
             place_user_set = excluded.place_user_set, route_user_set = excluded.route_user_set,
             weather_user_set = excluded.weather_user_set, auto_place_name = excluded.auto_place_name,
             auto_route_json = excluded.auto_route_json, auto_weather_desc = excluded.auto_weather_desc,
             auto_weather_temp = excluded.auto_weather_temp",
            params![
                p.activity_id,
                p.body,
                serde_json::to_string(&p.hashtags)?,
                p.place.as_ref().map(|x| &x.name),
                p.place.as_ref().map(|x| x.estimated).unwrap_or(false),
                serde_json::to_string(&p.route_points)?,
                p.weather.as_ref().map(|w| &w.description),
                p.weather.as_ref().map(|w| w.temp_c),
                p.edited,
                p.published_at.map(|t| t.timestamp()),
                p.place_user_set,
                p.route_user_set,
                p.weather_user_set,
                p.auto_place,
                serde_json::to_string(&p.auto_route_points)?,
                p.auto_weather.as_ref().map(|w| &w.description),
                p.auto_weather.as_ref().map(|w| w.temp_c),
            ],
        )?;
        Ok(())
    }

    /// 백업 기록: 올린 파일의 드라이브 id와 그때의 크기·수정 시각.
    pub fn synced_file(&self, path: &str) -> Result<Option<(String, i64, i64)>> {
        Ok(self
            .conn
            .query_row(
                "SELECT drive_id, size, mtime FROM sync_files WHERE path = ?1",
                [path],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?)
    }

    pub fn put_synced_file(&self, path: &str, drive_id: &str, size: i64, mtime: i64) -> Result<()> {
        self.conn.execute(
            "INSERT INTO sync_files (path, drive_id, size, mtime) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(path) DO UPDATE SET drive_id = excluded.drive_id, size = excluded.size, mtime = excluded.mtime",
            params![path, drive_id, size, mtime],
        )?;
        Ok(())
    }

    pub fn synced_folders(&self) -> Result<Vec<(String, String)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT path, drive_id FROM sync_folders")?;
        Ok(stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn put_synced_folder(&self, path: &str, drive_id: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO sync_folders (path, drive_id) VALUES (?1, ?2)
             ON CONFLICT(path) DO UPDATE SET drive_id = excluded.drive_id",
            params![path, drive_id],
        )?;
        Ok(())
    }

    pub fn sync_value(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM sync_state WHERE key = ?1", [key], |r| {
                r.get(0)
            })
            .optional()?)
    }

    pub fn set_sync_value(&self, key: &str, value: Option<&str>) -> Result<()> {
        match value {
            Some(v) => self.conn.execute(
                "INSERT INTO sync_state (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, v],
            )?,
            None => self.conn.execute("DELETE FROM sync_state WHERE key = ?1", [key])?,
        };
        Ok(())
    }

    /// 백업용 DB 사본: 일관된 스냅샷에서 백업 기록 표를 비운다 (기록이 바뀌어도 사본 내용은 같게).
    pub fn snapshot_to(&self, path: &Path) -> Result<()> {
        let _ = std::fs::remove_file(path);
        self.conn
            .execute("VACUUM INTO ?1", [path.to_string_lossy()])?;
        let snap = Connection::open(path)?;
        snap.execute_batch(
            "DELETE FROM sync_files; DELETE FROM sync_folders; DELETE FROM sync_state; VACUUM;",
        )?;
        Ok(())
    }

    /// 백업 기록을 모두 지운다 (로그아웃·다른 계정).
    pub fn clear_sync(&self) -> Result<()> {
        self.conn.execute_batch(
            "DELETE FROM sync_files; DELETE FROM sync_folders; DELETE FROM sync_state;",
        )?;
        Ok(())
    }

    /// 운동·사진이 하나도 없으면 비어 있는 라이브러리.
    pub fn is_empty(&self) -> Result<bool> {
        let n: i64 = self.conn.query_row(
            "SELECT (SELECT COUNT(*) FROM activities) + (SELECT COUNT(*) FROM photos)",
            [],
            |r| r.get(0),
        )?;
        Ok(n == 0)
    }

    /// 역지오코딩 응답 캐시 (키: "위도,경도" 소수 4자리).
    pub fn geocode_cached(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row(
                "SELECT json FROM geocode_cache WHERE key = ?1",
                [key],
                |r| r.get(0),
            )
            .optional()?)
    }

    pub fn put_geocode(&self, key: &str, json: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO geocode_cache (key, json) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET json = excluded.json",
            params![key, json],
        )?;
        Ok(())
    }

    pub fn insert_publish(&self, r: &PublishRow) -> Result<()> {
        let method = match r.method {
            PublishMethod::ManualExport => "manual_export",
            PublishMethod::ThreadsApi => "threads_api",
        };
        self.conn.execute(
            "INSERT INTO publish_records (activity_id, method, at, external_id) VALUES (?1, ?2, ?3, ?4)",
            params![r.activity_id, method, r.at.timestamp(), r.external_id],
        )?;
        Ok(())
    }

    pub fn setting(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| {
                r.get(0)
            })
            .optional()?)
    }

    pub fn set_setting(&self, key: &str, value: Option<&str>) -> Result<()> {
        match value {
            Some(v) => self.conn.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, v],
            )?,
            None => self.conn.execute("DELETE FROM settings WHERE key = ?1", [key])?,
        };
        Ok(())
    }
}

/// posts 한 행 (읽기 전용 중간 형태).
struct RawPost {
    activity_id: String,
    body: String,
    hashtags_json: String,
    place_name: Option<String>,
    place_estimated: bool,
    route_json: String,
    weather_desc: Option<String>,
    weather_temp: Option<f64>,
    edited: bool,
    published_at: Option<i64>,
    place_user_set: bool,
    route_user_set: bool,
    weather_user_set: bool,
    auto_place_name: Option<String>,
    auto_route_json: String,
    auto_weather_desc: Option<String>,
    auto_weather_temp: Option<f64>,
}

type RawActivity = (ActivityHeader, String, String);

fn raw_activity(r: &Row) -> rusqlite::Result<RawActivity> {
    let header = ActivityHeader {
        id: r.get(0)?,
        sport: Sport::from_fit(r.get(1)?),
        start: ts(r.get(2)?),
        utc_offset_s: r.get(3)?,
        summary: Summary {
            distance_m: r.get(4)?,
            moving_s: r.get(5)?,
            elapsed_s: r.get(6)?,
            avg_hr: r.get(7)?,
            max_hr: r.get(8)?,
            avg_speed_mps: r.get(9)?,
            ascent_m: r.get(10)?,
            avg_cadence: r.get(11)?,
        },
    };
    Ok((header, r.get(12)?, r.get(13)?))
}

fn finish_activity((header, fit_path, route_json): RawActivity) -> Result<ActivityRow> {
    let pts: Vec<[f64; 2]> = serde_json::from_str(&route_json)?;
    Ok(ActivityRow {
        header,
        fit_path,
        route: pts
            .into_iter()
            .map(|[lat, lon]| LatLon { lat, lon })
            .collect(),
    })
}

fn photo_row(r: &Row) -> rusqlite::Result<PhotoRow> {
    Ok(PhotoRow {
        id: r.get(0)?,
        activity_id: r.get(1)?,
        path: r.get(2)?,
        taken_at: r.get::<_, Option<i64>>(3)?.map(ts),
        km_mark: r.get(4)?,
        lat: r.get(5)?,
        lon: r.get(6)?,
        card_path: r.get(7)?,
        content_hash: r.get(8)?,
    })
}
