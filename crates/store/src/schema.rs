//! 스키마 마이그레이션 (PRAGMA user_version).
use rusqlite::Connection;

const V1: &str = "
CREATE TABLE activities (
    id TEXT PRIMARY KEY, sport INTEGER NOT NULL, start_utc INTEGER NOT NULL, start_local TEXT NOT NULL,
    utc_offset_s INTEGER NOT NULL, distance_m REAL NOT NULL, moving_s REAL NOT NULL, elapsed_s REAL NOT NULL,
    avg_hr INTEGER, max_hr INTEGER, avg_speed_mps REAL NOT NULL, ascent_m INTEGER, avg_cadence INTEGER,
    fit_path TEXT NOT NULL, route_json TEXT NOT NULL);
CREATE INDEX activities_start_local ON activities(start_local);
CREATE TABLE photos (
    id INTEGER PRIMARY KEY, activity_id TEXT REFERENCES activities(id), path TEXT NOT NULL UNIQUE,
    taken_at INTEGER, km_mark REAL, lat REAL, lon REAL, card_path TEXT);
CREATE TABLE posts (
    activity_id TEXT PRIMARY KEY REFERENCES activities(id), body TEXT NOT NULL, hashtags_json TEXT NOT NULL,
    place_name TEXT, place_estimated INTEGER NOT NULL DEFAULT 0, route_points_json TEXT NOT NULL,
    weather_desc TEXT, weather_temp REAL, edited INTEGER NOT NULL, published_at INTEGER);
CREATE TABLE publish_records (
    id INTEGER PRIMARY KEY, activity_id TEXT NOT NULL REFERENCES activities(id), method TEXT NOT NULL,
    at INTEGER NOT NULL, external_id TEXT);
CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
";

const V2: &str = "
ALTER TABLE photos ADD COLUMN content_hash TEXT;
CREATE INDEX photos_content_hash ON photos(content_hash);
";

const V3: &str = "
ALTER TABLE posts ADD COLUMN place_user_set INTEGER NOT NULL DEFAULT 0;
ALTER TABLE posts ADD COLUMN route_user_set INTEGER NOT NULL DEFAULT 0;
ALTER TABLE posts ADD COLUMN weather_user_set INTEGER NOT NULL DEFAULT 0;
CREATE TABLE geocode_cache (key TEXT PRIMARY KEY, json TEXT NOT NULL);
";

/// 마지막으로 자동 조회한 값 (직접 정한 값을 지우면 이것으로 돌아간다).
const V4: &str = "
ALTER TABLE posts ADD COLUMN auto_place_name TEXT;
ALTER TABLE posts ADD COLUMN auto_route_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE posts ADD COLUMN auto_weather_desc TEXT;
ALTER TABLE posts ADD COLUMN auto_weather_temp REAL;
";

/// 드라이브 백업 기록.
const V5: &str = "
CREATE TABLE sync_files (path TEXT PRIMARY KEY, drive_id TEXT NOT NULL, size INTEGER NOT NULL, mtime INTEGER NOT NULL);
CREATE TABLE sync_folders (path TEXT PRIMARY KEY, drive_id TEXT NOT NULL);
CREATE TABLE sync_state (key TEXT PRIMARY KEY, value TEXT NOT NULL);
";

pub fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if version < 1 {
        conn.execute_batch(V1)?;
        conn.pragma_update(None, "user_version", 1)?;
    }
    if version < 2 {
        conn.execute_batch(V2)?;
        conn.pragma_update(None, "user_version", 2)?;
    }
    if version < 3 {
        conn.execute_batch(V3)?;
        conn.pragma_update(None, "user_version", 3)?;
    }
    if version < 4 {
        conn.execute_batch(V4)?;
        conn.pragma_update(None, "user_version", 4)?;
    }
    if version < 5 {
        conn.execute_batch(V5)?;
        conn.pragma_update(None, "user_version", 5)?;
    }
    Ok(())
}
