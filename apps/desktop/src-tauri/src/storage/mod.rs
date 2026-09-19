#![deny(clippy::unwrap_used, clippy::expect_used)]

use rusqlite::{Connection, OptionalExtension, Result, params};
use std::path::Path;
use std::sync::Mutex;

const SCHEMA_VERSION: i64 = 2;

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
        assert_eq!(storage.schema_version()?, 2);
        assert!(storage.foreign_keys_enabled()?);
        assert!(storage.advanced_access_enabled()?);
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
}
