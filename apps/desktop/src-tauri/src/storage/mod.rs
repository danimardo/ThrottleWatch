#![deny(clippy::unwrap_used, clippy::expect_used)]

use rusqlite::{Connection, OptionalExtension, Result, params};
use std::path::Path;
use std::sync::Mutex;

const SCHEMA_VERSION: i64 = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnboardingStatus {
    Pending,
    Completed,
    Skipped,
}

impl OnboardingStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Completed => "completed",
            Self::Skipped => "skipped",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "completed" => Some(Self::Completed),
            "skipped" => Some(Self::Skipped),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnboardingState {
    pub flow_version: i64,
    pub last_slide: i64,
    pub status: OnboardingStatus,
    pub completed_at: Option<String>,
    pub last_seen_notice_version: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowState {
    pub restored_x: i64,
    pub restored_y: i64,
    pub restored_width: i64,
    pub restored_height: i64,
    pub maximized: bool,
    pub display_fingerprint: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuidedCheckpoint {
    pub session_id: String,
    pub phase: String,
    pub profile: String,
    pub elapsed_ms: i64,
    pub reason: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnalysisSourcePoint {
    pub sensor_id: String,
    pub monotonic_ms: i64,
    pub value: Option<f64>,
    pub quality: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisSourceEvent {
    pub id: String,
    pub kind: String,
    pub start_ms: i64,
    pub end_ms: i64,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            restored_x: 0,
            restored_y: 0,
            restored_width: 1100,
            restored_height: 760,
            maximized: false,
            display_fingerprint: None,
            updated_at: "1970-01-01T00:00:00Z".to_owned(),
        }
    }
}

impl Default for OnboardingState {
    fn default() -> Self {
        Self {
            flow_version: 1,
            last_slide: 1,
            status: OnboardingStatus::Pending,
            completed_at: None,
            last_seen_notice_version: 0,
        }
    }
}

pub struct Storage {
    connection: Connection,
}

pub struct AppState {
    pub storage: Mutex<Storage>,
}

impl AppState {
    pub fn new(storage: Storage) -> Self {
        Self { storage: Mutex::new(storage) }
    }
}

impl Storage {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::from_connection(Connection::open(path)?)
    }

    pub fn in_memory() -> Result<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(connection: Connection) -> Result<Self> {
        connection.pragma_update(None, "foreign_keys", "ON")?;
        let storage = Self { connection };
        storage.migrate()?;
        Ok(storage)
    }

    fn migrate(&self) -> Result<()> {
        self.connection.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL);
             INSERT INTO schema_version (version)
                 SELECT 0 WHERE NOT EXISTS (SELECT 1 FROM schema_version);
             CREATE TABLE IF NOT EXISTS cpu_device (
                 id TEXT PRIMARY KEY, vendor TEXT NOT NULL, display_name TEXT NOT NULL,
                 logical_processors INTEGER NOT NULL, physical_cores INTEGER, hybrid INTEGER NOT NULL,
                 fingerprint_version INTEGER NOT NULL, first_seen_at TEXT NOT NULL, last_seen_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS monitoring_session (
                 id TEXT PRIMARY KEY, cpu_id TEXT NOT NULL REFERENCES cpu_device(id),
                 kind TEXT NOT NULL, status TEXT NOT NULL, started_at TEXT NOT NULL, ended_at TEXT,
                 monotonic_duration_ms INTEGER, protocol_version INTEGER NOT NULL, ruleset_version TEXT NOT NULL,
                 incomplete_reason TEXT, is_reference INTEGER NOT NULL DEFAULT 0, coverage_tier TEXT NOT NULL,
                 split_reason TEXT
             );
             CREATE TABLE IF NOT EXISTS sample_frame (
                 session_id TEXT NOT NULL REFERENCES monitoring_session(id) ON DELETE CASCADE,
                 sequence INTEGER NOT NULL, monotonic_ms INTEGER NOT NULL, duration_ms INTEGER NOT NULL,
                 quality TEXT NOT NULL, PRIMARY KEY (session_id, sequence)
             );
             CREATE TABLE IF NOT EXISTS sample_value (
                 session_id TEXT NOT NULL, sequence INTEGER NOT NULL, sensor_id TEXT NOT NULL,
                 value_real REAL, value_bool INTEGER, quality TEXT NOT NULL,
                 FOREIGN KEY (session_id, sequence)
                     REFERENCES sample_frame(session_id, sequence) ON DELETE CASCADE
             );
             CREATE TABLE IF NOT EXISTS limit_event (
                 id TEXT PRIMARY KEY, session_id TEXT NOT NULL REFERENCES monitoring_session(id) ON DELETE CASCADE,
                 kind TEXT NOT NULL, start_sequence INTEGER NOT NULL, end_sequence INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS diagnostic_report (
                 session_id TEXT PRIMARY KEY REFERENCES monitoring_session(id) ON DELETE CASCADE,
                 schema_version INTEGER NOT NULL, payload_json TEXT NOT NULL, frozen_at TEXT
             );
             UPDATE schema_version SET version = 1 WHERE version = 0;",
        )?;
        let mut version: i64 =
            self.connection
                .query_row("SELECT version FROM schema_version", [], |row| row.get(0))?;
        if version == 1 {
            self.connection.execute_batch(
                "CREATE TABLE IF NOT EXISTS preference (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                 );
                 UPDATE schema_version SET version = 2 WHERE version = 1;",
            )?;
            version = 2;
        }
        if version == 2 {
            self.connection.execute_batch(
                "CREATE TABLE IF NOT EXISTS onboarding_state (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    flow_version INTEGER NOT NULL,
                    last_slide INTEGER NOT NULL,
                    status TEXT NOT NULL
                 );
                 INSERT OR IGNORE INTO onboarding_state (id, flow_version, last_slide, status)
                     VALUES (1, 1, 1, 'pending');
                 UPDATE schema_version SET version = 3 WHERE version = 2;",
            )?;
            version = 3;
        }
        if version == 3 {
            self.connection.execute_batch(
                "ALTER TABLE onboarding_state ADD COLUMN completed_at TEXT;
                 ALTER TABLE onboarding_state ADD COLUMN last_seen_notice_version INTEGER NOT NULL DEFAULT 0;
                 UPDATE schema_version SET version = 4 WHERE version = 3;",
            )?;
            version = 4;
        }
        if version == 4 {
            self.connection.execute_batch(
                "CREATE TABLE IF NOT EXISTS window_state (
                    window_key TEXT PRIMARY KEY,
                    restored_x INTEGER NOT NULL,
                    restored_y INTEGER NOT NULL,
                    restored_width INTEGER NOT NULL,
                    restored_height INTEGER NOT NULL,
                    maximized INTEGER NOT NULL,
                    display_fingerprint TEXT,
                    updated_at TEXT NOT NULL
                 );
                 INSERT OR IGNORE INTO window_state
                     (window_key, restored_x, restored_y, restored_width, restored_height, maximized, updated_at)
                     VALUES ('main', 0, 0, 1100, 760, 0, '1970-01-01T00:00:00Z');
                 UPDATE schema_version SET version = 5 WHERE version = 4;",
            )?;
            version = 5;
        }
        if version == 5 {
            self.connection.execute_batch(
                "CREATE TABLE IF NOT EXISTS guided_checkpoint (
                    session_id TEXT PRIMARY KEY,
                    phase TEXT NOT NULL,
                    profile TEXT NOT NULL,
                    elapsed_ms INTEGER NOT NULL CHECK (elapsed_ms >= 0),
                    reason TEXT,
                    updated_at TEXT NOT NULL
                 );
                 UPDATE schema_version SET version = 6 WHERE version = 5;",
            )?;
            version = 6;
        }
        if version != SCHEMA_VERSION {
            return Err(rusqlite::Error::InvalidQuery);
        }
        Ok(())
    }

    pub fn schema_version(&self) -> Result<i64> {
        self.connection.query_row("SELECT version FROM schema_version", [], |row| row.get(0))
    }

    pub fn create_cpu(&self, id: &str, vendor: &str, display_name: &str) -> Result<()> {
        self.connection.execute(
            "INSERT INTO cpu_device
             (id, vendor, display_name, logical_processors, hybrid, fingerprint_version, first_seen_at, last_seen_at)
             VALUES (?1, ?2, ?3, 1, 0, 1, '1970-01-01T00:00:00Z', '1970-01-01T00:00:00Z')",
            params![id, vendor, display_name],
        )?;
        Ok(())
    }

    pub fn create_session(&self, id: &str, cpu_id: &str, split_reason: &str) -> Result<()> {
        self.connection.execute(
            "INSERT INTO monitoring_session
             (id, cpu_id, kind, status, started_at, protocol_version, ruleset_version, coverage_tier, split_reason)
             VALUES (?1, ?2, 'passive', 'running', '1970-01-01T00:00:00Z', 1, 'ruleset-v1', 'C', ?3)",
            params![id, cpu_id, split_reason],
        )?;
        Ok(())
    }

    pub fn insert_frame(&self, session_id: &str, sequence: i64, monotonic_ms: i64) -> Result<()> {
        self.connection.execute(
            "INSERT INTO sample_frame (session_id, sequence, monotonic_ms, duration_ms, quality)
             VALUES (?1, ?2, ?3, 0, 'complete')",
            params![session_id, sequence, monotonic_ms],
        )?;
        Ok(())
    }

    pub fn delete_session(&self, session_id: &str) -> Result<bool> {
        Ok(self
            .connection
            .execute("DELETE FROM monitoring_session WHERE id = ?1", params![session_id])?
            == 1)
    }

    pub fn frame_count(&self, session_id: &str) -> Result<i64> {
        self.connection.query_row(
            "SELECT COUNT(*) FROM sample_frame WHERE session_id = ?1",
            params![session_id],
            |row| row.get(0),
        )
    }

    pub fn session_exists(&self, session_id: &str) -> Result<bool> {
        self.connection
            .query_row(
                "SELECT 1 FROM monitoring_session WHERE id = ?1",
                params![session_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map(|value| value.is_some())
    }

    pub fn advanced_access_enabled(&self) -> Result<bool> {
        let value: Option<String> = self
            .connection
            .query_row(
                "SELECT value FROM preference WHERE key = 'advanced_access_enabled'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        Ok(value.as_deref() != Some("false"))
    }

    pub fn set_advanced_access_enabled(&self, enabled: bool) -> Result<()> {
        self.connection.execute(
            "INSERT INTO preference (key, value) VALUES ('advanced_access_enabled', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![if enabled { "true" } else { "false" }],
        )?;
        Ok(())
    }

    pub fn onboarding_state(&self) -> Result<OnboardingState> {
        let row = self.connection.query_row(
            "SELECT flow_version, last_slide, status, completed_at, last_seen_notice_version
             FROM onboarding_state WHERE id = 1",
            [],
            |row| {
                let flow_version: i64 = row.get(0)?;
                let last_slide: i64 = row.get(1)?;
                let status: String = row.get(2)?;
                let completed_at: Option<String> = row.get(3)?;
                let last_seen_notice_version: i64 = row.get(4)?;
                Ok((flow_version, last_slide, status, completed_at, last_seen_notice_version))
            },
        )?;
        let (flow_version, last_slide, status, completed_at, last_seen_notice_version) = row;
        if flow_version < 1 || !(1..=5).contains(&last_slide) || last_seen_notice_version < 0 {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let status = OnboardingStatus::parse(&status).ok_or(rusqlite::Error::InvalidQuery)?;
        if matches!(status, OnboardingStatus::Pending) && completed_at.is_some() {
            return Err(rusqlite::Error::InvalidQuery);
        }
        if !matches!(status, OnboardingStatus::Pending) && completed_at.is_none() {
            return Err(rusqlite::Error::InvalidQuery);
        }
        Ok(OnboardingState {
            flow_version,
            last_slide,
            status,
            completed_at,
            last_seen_notice_version,
        })
    }

    pub fn set_onboarding_state(&self, state: OnboardingState) -> Result<()> {
        if state.flow_version < 1 || !(1..=5).contains(&state.last_slide) {
            return Err(rusqlite::Error::InvalidQuery);
        }
        if state.last_seen_notice_version < 0 {
            return Err(rusqlite::Error::InvalidQuery);
        }
        if matches!(state.status, OnboardingStatus::Pending) && state.completed_at.is_some() {
            return Err(rusqlite::Error::InvalidQuery);
        }
        if !matches!(state.status, OnboardingStatus::Pending) && state.completed_at.is_none() {
            return Err(rusqlite::Error::InvalidQuery);
        }
        self.connection.execute(
            "INSERT INTO onboarding_state
                 (id, flow_version, last_slide, status, completed_at, last_seen_notice_version)
             VALUES (1, ?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET
                 flow_version = excluded.flow_version,
                 last_slide = excluded.last_slide,
                 status = excluded.status,
                 completed_at = excluded.completed_at,
                 last_seen_notice_version = excluded.last_seen_notice_version",
            params![
                state.flow_version,
                state.last_slide,
                state.status.as_str(),
                state.completed_at,
                state.last_seen_notice_version
            ],
        )?;
        Ok(())
    }

    pub fn window_state(&self) -> Result<WindowState> {
        let row = self.connection.query_row(
            "SELECT restored_x, restored_y, restored_width, restored_height, maximized,
                    display_fingerprint, updated_at
             FROM window_state WHERE window_key = 'main'",
            [],
            |row| {
                Ok(WindowState {
                    restored_x: row.get(0)?,
                    restored_y: row.get(1)?,
                    restored_width: row.get(2)?,
                    restored_height: row.get(3)?,
                    maximized: row.get::<_, i64>(4)? != 0,
                    display_fingerprint: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        )?;
        if row.restored_width < 480 || row.restored_height < 500 || row.updated_at.is_empty() {
            return Err(rusqlite::Error::InvalidQuery);
        }
        Ok(row)
    }

    pub fn set_window_state(&self, state: WindowState) -> Result<()> {
        if state.restored_width < 480 || state.restored_height < 500 || state.updated_at.is_empty()
        {
            return Err(rusqlite::Error::InvalidQuery);
        }
        self.connection.execute(
            "INSERT INTO window_state
                 (window_key, restored_x, restored_y, restored_width, restored_height, maximized,
                  display_fingerprint, updated_at)
             VALUES ('main', ?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(window_key) DO UPDATE SET
                 restored_x = excluded.restored_x,
                 restored_y = excluded.restored_y,
                 restored_width = excluded.restored_width,
                 restored_height = excluded.restored_height,
                 maximized = excluded.maximized,
                 display_fingerprint = excluded.display_fingerprint,
                 updated_at = excluded.updated_at",
            params![
                state.restored_x,
                state.restored_y,
                state.restored_width,
                state.restored_height,
                if state.maximized { 1 } else { 0 },
                state.display_fingerprint,
                state.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn save_guided_checkpoint(&self, checkpoint: &GuidedCheckpoint) -> Result<()> {
        if checkpoint.session_id.is_empty()
            || checkpoint.phase.is_empty()
            || checkpoint.profile.is_empty()
            || checkpoint.elapsed_ms < 0
            || checkpoint.updated_at.is_empty()
        {
            return Err(rusqlite::Error::InvalidQuery);
        }
        self.connection.execute(
            "INSERT INTO guided_checkpoint
                 (session_id, phase, profile, elapsed_ms, reason, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(session_id) DO UPDATE SET
                 phase = excluded.phase,
                 profile = excluded.profile,
                 elapsed_ms = excluded.elapsed_ms,
                 reason = excluded.reason,
                 updated_at = excluded.updated_at",
            params![
                checkpoint.session_id,
                checkpoint.phase,
                checkpoint.profile,
                checkpoint.elapsed_ms,
                checkpoint.reason,
                checkpoint.updated_at
            ],
        )?;
        Ok(())
    }

    pub fn guided_checkpoint(&self, session_id: &str) -> Result<Option<GuidedCheckpoint>> {
        self.connection
            .query_row(
                "SELECT session_id, phase, profile, elapsed_ms, reason, updated_at
                 FROM guided_checkpoint WHERE session_id = ?1",
                params![session_id],
                |row| {
                    Ok(GuidedCheckpoint {
                        session_id: row.get(0)?,
                        phase: row.get(1)?,
                        profile: row.get(2)?,
                        elapsed_ms: row.get(3)?,
                        reason: row.get(4)?,
                        updated_at: row.get(5)?,
                    })
                },
            )
            .optional()
    }

    pub fn analysis_points(
        &self,
        session_id: &str,
        start_ms: i64,
        end_ms: i64,
    ) -> Result<Vec<AnalysisSourcePoint>> {
        let mut statement = self.connection.prepare(
            "SELECT value.sensor_id, frame.monotonic_ms, value.value_real, value.quality
             FROM sample_value AS value
             JOIN sample_frame AS frame
               ON frame.session_id = value.session_id AND frame.sequence = value.sequence
             WHERE value.session_id = ?1 AND frame.monotonic_ms BETWEEN ?2 AND ?3
             ORDER BY frame.monotonic_ms, value.sensor_id",
        )?;
        let rows = statement.query_map(params![session_id, start_ms, end_ms], |row| {
            Ok(AnalysisSourcePoint {
                sensor_id: row.get(0)?,
                monotonic_ms: row.get(1)?,
                value: row.get(2)?,
                quality: row.get(3)?,
            })
        })?;
        rows.collect()
    }

    pub fn analysis_events(
        &self,
        session_id: &str,
        start_ms: i64,
        end_ms: i64,
    ) -> Result<Vec<AnalysisSourceEvent>> {
        let mut statement = self.connection.prepare(
            "SELECT event.id, event.kind, start_frame.monotonic_ms, end_frame.monotonic_ms
             FROM limit_event AS event
             JOIN sample_frame AS start_frame
               ON start_frame.session_id = event.session_id AND start_frame.sequence = event.start_sequence
             JOIN sample_frame AS end_frame
               ON end_frame.session_id = event.session_id AND end_frame.sequence = event.end_sequence
             WHERE event.session_id = ?1
               AND end_frame.monotonic_ms >= ?2
               AND start_frame.monotonic_ms <= ?3
             ORDER BY start_frame.monotonic_ms",
        )?;
        let rows = statement.query_map(params![session_id, start_ms, end_ms], |row| {
            Ok(AnalysisSourceEvent {
                id: row.get(0)?,
                kind: row.get(1)?,
                start_ms: row.get(2)?,
                end_ms: row.get(3)?,
            })
        })?;
        rows.collect()
    }

    #[cfg(test)]
    fn foreign_keys_enabled(&self) -> Result<bool> {
        self.connection
            .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))
            .map(|value| value == 1)
    }
}

#[cfg(test)]
mod tests {
    use super::Storage;

    #[test]
    fn migrates_schema_and_enables_foreign_keys() -> rusqlite::Result<()> {
        let storage = Storage::in_memory()?;
        assert_eq!(storage.schema_version()?, 6);
        assert!(storage.foreign_keys_enabled()?);
        assert!(storage.advanced_access_enabled()?);
        assert_eq!(storage.onboarding_state()?, super::OnboardingState::default());
        Ok(())
    }

    #[test]
    fn persists_onboarding_progress_and_status() -> rusqlite::Result<()> {
        let storage = Storage::in_memory()?;
        let state = super::OnboardingState {
            flow_version: 2,
            last_slide: 4,
            status: super::OnboardingStatus::Skipped,
            completed_at: Some("2026-09-19T12:00:00Z".to_owned()),
            last_seen_notice_version: 3,
        };
        storage.set_onboarding_state(state.clone())?;
        assert_eq!(storage.onboarding_state()?, state);
        Ok(())
    }

    #[test]
    fn rejects_invalid_onboarding_progress_and_corrupt_status() -> rusqlite::Result<()> {
        let storage = Storage::in_memory()?;
        assert!(
            storage
                .set_onboarding_state(super::OnboardingState {
                    flow_version: 1,
                    last_slide: 6,
                    status: super::OnboardingStatus::Pending,
                    completed_at: None,
                    last_seen_notice_version: 0,
                })
                .is_err()
        );
        storage
            .connection
            .execute("UPDATE onboarding_state SET status = 'corrupt' WHERE id = 1", [])?;
        assert!(storage.onboarding_state().is_err());
        Ok(())
    }

    #[test]
    fn persists_advanced_access_preference() -> rusqlite::Result<()> {
        let storage = Storage::in_memory()?;
        storage.set_advanced_access_enabled(false)?;
        assert!(!storage.advanced_access_enabled()?);
        storage.set_advanced_access_enabled(true)?;
        assert!(storage.advanced_access_enabled()?);
        Ok(())
    }

    #[test]
    fn persists_and_validates_window_state() -> rusqlite::Result<()> {
        let storage = Storage::in_memory()?;
        assert_eq!(storage.window_state()?, super::WindowState::default());
        let state = super::WindowState {
            restored_x: 100,
            restored_y: 120,
            restored_width: 840,
            restored_height: 600,
            maximized: true,
            display_fingerprint: Some("display-a".to_owned()),
            updated_at: "2026-09-19T18:00:00Z".to_owned(),
        };
        storage.set_window_state(state.clone())?;
        assert_eq!(storage.window_state()?, state);
        assert!(
            storage.set_window_state(super::WindowState { restored_width: 479, ..state }).is_err()
        );
        Ok(())
    }

    #[test]
    fn deleting_session_cascades_frames() -> rusqlite::Result<()> {
        let storage = Storage::in_memory()?;
        storage.create_cpu("cpu-1", "unknown", "Test CPU")?;
        storage.create_session("session-1", "cpu-1", "start")?;
        storage.insert_frame("session-1", 1, 1000)?;
        storage.connection.execute(
            "INSERT INTO sample_value (session_id, sequence, sensor_id, value_real, quality)
             VALUES ('session-1', 1, 'sensor-1', 42.0, 'direct')",
            [],
        )?;
        storage.connection.execute(
            "INSERT INTO limit_event (id, session_id, kind, start_sequence, end_sequence)
             VALUES ('event-1', 'session-1', 'thermal', 1, 1)",
            [],
        )?;
        storage.connection.execute(
            "INSERT INTO diagnostic_report (session_id, schema_version, payload_json)
             VALUES ('session-1', 1, '{}')",
            [],
        )?;
        assert_eq!(storage.frame_count("session-1")?, 1);
        assert!(storage.delete_session("session-1")?);
        assert_eq!(storage.frame_count("session-1")?, 0);
        for table in ["sample_value", "limit_event", "diagnostic_report"] {
            let count: i64 = storage.connection.query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE session_id = 'session-1'"),
                [],
                |row| row.get(0),
            )?;
            assert_eq!(count, 0, "cascade failed for {table}");
        }
        assert!(!storage.session_exists("session-1")?);
        Ok(())
    }

    #[test]
    fn persists_partial_guided_state_for_resume_or_incomplete_report() -> rusqlite::Result<()> {
        let storage = Storage::in_memory()?;
        let checkpoint = super::GuidedCheckpoint {
            session_id: "guided-1".to_owned(),
            phase: "steady_load".to_owned(),
            profile: "standard".to_owned(),
            elapsed_ms: 12_345,
            reason: None,
            updated_at: "2026-09-20T10:00:00Z".to_owned(),
        };
        storage.save_guided_checkpoint(&checkpoint)?;
        assert_eq!(storage.guided_checkpoint("guided-1")?, Some(checkpoint));
        Ok(())
    }

    #[test]
    fn reads_analysis_points_and_event_edges_from_sqlite() -> rusqlite::Result<()> {
        let storage = Storage::in_memory()?;
        storage.create_cpu("cpu-1", "unknown", "Test CPU")?;
        storage.create_session("session-1", "cpu-1", "start")?;
        storage.insert_frame("session-1", 1, 1000)?;
        storage.insert_frame("session-1", 2, 2000)?;
        storage.connection.execute(
            "INSERT INTO sample_value (session_id, sequence, sensor_id, value_real, quality)
             VALUES ('session-1', 1, 'temperature', 60.0, 'complete'),
                    ('session-1', 2, 'temperature', 70.0, 'reduced')",
            [],
        )?;
        storage.connection.execute(
            "INSERT INTO limit_event (id, session_id, kind, start_sequence, end_sequence)
             VALUES ('event-1', 'session-1', 'thermal', 1, 2)",
            [],
        )?;
        assert_eq!(storage.analysis_points("session-1", 0, 3000)?.len(), 2);
        let events = storage.analysis_events("session-1", 0, 3000)?;
        assert_eq!(events[0].start_ms, 1000);
        assert_eq!(events[0].end_ms, 2000);
        Ok(())
    }
}
