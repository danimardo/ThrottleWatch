#![deny(clippy::unwrap_used, clippy::expect_used)]

use rusqlite::{Connection, OptionalExtension, Result, params};
use rusqlite_migration::{M, Migrations};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::export::{
    CoverageChange, DiagnosticReport, ExportBundle, ExportEvent, ExportSample, ExportSession,
};

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

#[derive(Debug, Clone, PartialEq)]
pub struct StoredSampleValue {
    pub sensor_id: String,
    pub value: Option<f64>,
    pub boolean: Option<bool>,
    pub quality: String,
}

/// One recorded frame's readings, keyed by sensor id (`(sensor_id, value_real, value_bool)`,
/// matching however many sensors published on it).
#[derive(Debug, Clone, PartialEq)]
pub struct StoredFrame {
    pub sequence: i64,
    pub monotonic_ms: i64,
    pub values: Vec<(String, Option<f64>, Option<bool>)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisSourceEvent {
    pub id: String,
    pub kind: String,
    pub start_ms: i64,
    pub end_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSummary {
    pub session_id: String,
    pub kind: String,
    pub status: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub coverage_tier: String,
    pub is_reference: bool,
    pub frame_count: i64,
    pub report_classification: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredReport {
    pub session_id: String,
    pub schema_version: i64,
    pub payload_json: String,
    pub frozen_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExportSnapshot {
    pub session: ExportSession,
    pub samples: Vec<ExportSample>,
    pub events: Vec<ExportEvent>,
    pub report: Option<DiagnosticReport>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StorageUsage {
    pub database_bytes: u64,
    pub session_count: u64,
}

pub struct AppState {
    pub storage: Mutex<Storage>,
}

impl AppState {
    pub fn new(storage: Storage) -> Self {
        Self { storage: Mutex::new(storage) }
    }
}

/// Ordered schema migrations (constitution: `rusqlite_migration`). The number of applied
/// migrations is `PRAGMA user_version`; a new schema change is a new entry at the end.
static MIGRATION_LIST: &[M<'static>] = &[
    M::up(
        "             CREATE TABLE IF NOT EXISTS cpu_device (
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
             );",
    ),
    M::up(
        "CREATE TABLE IF NOT EXISTS preference (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                 );",
    ),
    M::up(
        "CREATE TABLE IF NOT EXISTS onboarding_state (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    flow_version INTEGER NOT NULL,
                    last_slide INTEGER NOT NULL,
                    status TEXT NOT NULL
                 );
                 INSERT OR IGNORE INTO onboarding_state (id, flow_version, last_slide, status)
                     VALUES (1, 1, 1, 'pending');",
    ),
    M::up(
        "ALTER TABLE onboarding_state ADD COLUMN completed_at TEXT;
                 ALTER TABLE onboarding_state ADD COLUMN last_seen_notice_version INTEGER NOT NULL DEFAULT 0;",
    ),
    M::up(
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
                     VALUES ('main', 0, 0, 1100, 760, 0, '1970-01-01T00:00:00Z');",
    ),
    M::up(
        "CREATE TABLE IF NOT EXISTS guided_checkpoint (
                    session_id TEXT PRIMARY KEY,
                    phase TEXT NOT NULL,
                    profile TEXT NOT NULL,
                    elapsed_ms INTEGER NOT NULL CHECK (elapsed_ms >= 0),
                    reason TEXT,
                    updated_at TEXT NOT NULL
                 );",
    ),
    M::up(
        "CREATE TABLE IF NOT EXISTS coverage_change (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    session_id TEXT NOT NULL REFERENCES monitoring_session(id) ON DELETE CASCADE,
                    at INTEGER NOT NULL CHECK (at >= 0),
                    from_tier TEXT NOT NULL CHECK (from_tier IN ('A', 'B', 'C')),
                    to_tier TEXT NOT NULL CHECK (to_tier IN ('A', 'B', 'C')),
                    reason TEXT NOT NULL
                 );
                 CREATE INDEX IF NOT EXISTS idx_coverage_change_session_at
                     ON coverage_change (session_id, at);",
    ),
    M::up(
        "CREATE TABLE IF NOT EXISTS user_preference (
                    key TEXT PRIMARY KEY,
                    typed_value TEXT NOT NULL,
                    schema_version INTEGER NOT NULL,
                    updated_at TEXT NOT NULL
                 );",
    ),
    M::up(
        "CREATE TABLE IF NOT EXISTS update_state (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    payload_json TEXT NOT NULL
                 );",
    ),
];
static MIGRATIONS: Migrations<'static> = Migrations::from_slice(MIGRATION_LIST);

impl Storage {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::from_connection(Connection::open(path)?)
    }

    pub fn in_memory() -> Result<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    /// FR-075: a database that fails to open because it is corrupt — not merely inaccessible —
    /// is moved aside instead of overwritten in place ("sin intervención destructiva automática
    /// sobre los datos existentes"), and a fresh one takes over so the app can still start.
    /// `now` names the backup file (`<original>.corrupt-<fecha>`); the caller decides what to
    /// tell the person and whether to offer exporting the moved file.
    pub fn open_recovering_corruption(
        path: impl AsRef<Path>,
        now: &str,
    ) -> Result<(Self, Option<PathBuf>)> {
        let path = path.as_ref();
        match Self::open(path) {
            Ok(storage) => Ok((storage, None)),
            Err(error) if is_corruption(&error) => {
                let backup = corrupt_backup_path(path, now);
                std::fs::rename(path, &backup).map_err(|_| rusqlite::Error::InvalidQuery)?;
                Ok((Self::open(path)?, Some(backup)))
            }
            Err(error) => Err(error),
        }
    }

    fn from_connection(connection: Connection) -> Result<Self> {
        connection.pragma_update(None, "foreign_keys", "ON")?;
        let mut storage = Self { connection };
        storage.migrate()?;
        Ok(storage)
    }

    /// Databases created before `rusqlite_migration` tracked their version in a `schema_version`
    /// table: carry that number over to `user_version` (the migrations are numbered the same) and
    /// drop the table, so an existing installation upgrades instead of being migrated twice.
    fn adopt_legacy_schema_version(&self) -> Result<()> {
        let has_table: i64 = self.connection.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'schema_version'",
            [],
            |row| row.get(0),
        )?;
        if has_table == 0 {
            return Ok(());
        }
        let legacy: i64 = self.connection.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )?;
        let current: i64 =
            self.connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if current == 0 && legacy > 0 {
            self.connection.pragma_update(None, "user_version", legacy)?;
        }
        self.connection.execute_batch("DROP TABLE schema_version;")
    }

    fn migrate(&mut self) -> Result<()> {
        self.connection.pragma_update(None, "journal_mode", "WAL")?;
        self.adopt_legacy_schema_version()?;
        MIGRATIONS.to_latest(&mut self.connection).map_err(|_| rusqlite::Error::InvalidQuery)?;
        self.ensure_default_preferences()?;
        self.ensure_update_state()?;
        Ok(())
    }

    fn ensure_default_preferences(&self) -> Result<()> {
        let now = jiff::Timestamp::now().to_string();
        for (key, value) in crate::preferences::default_values() {
            let typed_value =
                serde_json::to_string(&value).map_err(|_| rusqlite::Error::InvalidQuery)?;
            self.connection.execute(
                "INSERT OR IGNORE INTO user_preference (key, typed_value, schema_version, updated_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![key, typed_value, crate::preferences::SCHEMA_VERSION, now],
            )?;
        }
        Ok(())
    }

    fn ensure_update_state(&self) -> Result<()> {
        let exists: i64 = self.connection.query_row(
            "SELECT COUNT(*) FROM update_state WHERE id = 1",
            [],
            |row| row.get(0),
        )?;
        if exists == 0 {
            let payload = serde_json::to_string(&crate::updater::UpdateMachine::default())
                .map_err(|_| rusqlite::Error::InvalidQuery)?;
            self.connection.execute(
                "INSERT INTO update_state (id, payload_json) VALUES (1, ?1)",
                params![payload],
            )?;
        }
        Ok(())
    }

    pub fn schema_version(&self) -> Result<i64> {
        self.connection.pragma_query_value(None, "user_version", |row| row.get(0))
    }

    pub fn updater_state(&self) -> Result<crate::updater::UpdateMachine> {
        let payload: String = self.connection.query_row(
            "SELECT payload_json FROM update_state WHERE id = 1",
            [],
            |row| row.get(0),
        )?;
        serde_json::from_str(&payload).map_err(|_| rusqlite::Error::InvalidQuery)
    }

    pub fn set_updater_state(&self, state: &crate::updater::UpdateMachine) -> Result<()> {
        let payload = serde_json::to_string(state).map_err(|_| rusqlite::Error::InvalidQuery)?;
        self.connection
            .execute("UPDATE update_state SET payload_json = ?1 WHERE id = 1", params![payload])?;
        Ok(())
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

    /// Stable identifier of a processor model: the same CPU is one row, whichever session sees it.
    pub fn cpu_id(vendor: &str, display_name: &str, logical_processors: u32) -> String {
        let digest = crate::ipc::supervisor::sha256_hex(
            format!("{vendor}|{display_name}|{logical_processors}").as_bytes(),
        );
        format!("cpu-{}", &digest[..16])
    }

    /// Records the processor the collector reported (once per model) and refreshes when it was
    /// last seen.
    pub fn upsert_cpu(
        &self,
        id: &str,
        vendor: &str,
        display_name: &str,
        logical_processors: i64,
        hybrid: bool,
        now: &str,
    ) -> Result<()> {
        self.connection.execute(
            "INSERT INTO cpu_device
             (id, vendor, display_name, logical_processors, hybrid, fingerprint_version, first_seen_at, last_seen_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?6)
             ON CONFLICT(id) DO UPDATE SET
                 display_name = excluded.display_name,
                 logical_processors = excluded.logical_processors,
                 hybrid = excluded.hybrid,
                 last_seen_at = excluded.last_seen_at",
            params![id, vendor, display_name, logical_processors, i64::from(hybrid), now],
        )?;
        Ok(())
    }

    pub fn create_session(
        &self,
        id: &str,
        cpu_id: &str,
        split_reason: &str,
        started_at: &str,
    ) -> Result<()> {
        self.connection.execute(
            "INSERT INTO monitoring_session
             (id, cpu_id, kind, status, started_at, protocol_version, ruleset_version, coverage_tier, split_reason)
             VALUES (?1, ?2, 'passive', 'running', ?4, 1, 'ruleset-v1', 'C', ?3)",
            params![id, cpu_id, split_reason, started_at],
        )?;
        Ok(())
    }

    pub fn create_guided_session(&self, id: &str, cpu_id: &str, started_at: &str) -> Result<()> {
        self.connection.execute(
            "INSERT INTO monitoring_session
                 (id, cpu_id, kind, status, started_at, protocol_version, ruleset_version, coverage_tier, split_reason)
             VALUES (?1, ?2, 'guided', 'running', ?3, 1, 'ruleset-v1', 'C', 'guided')",
            params![id, cpu_id, started_at],
        )?;
        Ok(())
    }

    /// Closes a running session (`completed` or `aborted`) with its end, its monotonic duration
    /// and, when it did not finish, why. Returns whether a running session was closed: a session
    /// that already ended is never rewritten.
    pub fn finish_session(
        &self,
        id: &str,
        status: &str,
        ended_at: &str,
        duration_ms: i64,
        incomplete_reason: Option<&str>,
    ) -> Result<bool> {
        let changed = self.connection.execute(
            "UPDATE monitoring_session
             SET status = ?2, ended_at = ?3, monotonic_duration_ms = ?4, incomplete_reason = ?5
             WHERE id = ?1 AND status = 'running'",
            params![id, status, ended_at, duration_ms.max(0), incomplete_reason],
        )?;
        Ok(changed == 1)
    }

    pub fn latest_session_id(&self) -> Result<Option<String>> {
        self.connection
            .query_row(
                "SELECT id FROM monitoring_session
                 ORDER BY rowid DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()
    }

    pub fn active_session_id(&self) -> Result<Option<String>> {
        self.connection
            .query_row(
                "SELECT id FROM monitoring_session
                 WHERE status = 'running' ORDER BY rowid DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()
    }

    pub fn list_sessions(&self, limit: u32) -> Result<Vec<SessionSummary>> {
        let bounded_limit = i64::from(limit.clamp(1, 100));
        let mut statement = self.connection.prepare(
            "SELECT session.id, session.kind, session.status, session.started_at, session.ended_at,
                    session.monotonic_duration_ms, session.coverage_tier, session.is_reference,
                    (SELECT COUNT(*) FROM sample_frame AS frame WHERE frame.session_id = session.id),
                    report.payload_json
             FROM monitoring_session AS session
             LEFT JOIN diagnostic_report AS report ON report.session_id = session.id
             WHERE session.kind IN ('passive', 'guided', 'imported')
             ORDER BY session.rowid DESC LIMIT ?1",
        )?;
        let rows = statement.query_map([bounded_limit], |row| {
            let payload: Option<String> = row.get(9)?;
            let report_classification = payload.and_then(|value| {
                serde_json::from_str::<serde_json::Value>(&value).ok().and_then(|json| {
                    json.get("classification").and_then(|item| item.as_str()).map(str::to_owned)
                })
            });
            Ok(SessionSummary {
                session_id: row.get(0)?,
                kind: row.get(1)?,
                status: row.get(2)?,
                started_at: row.get(3)?,
                ended_at: row.get(4)?,
                duration_ms: row.get(5)?,
                coverage_tier: row.get(6)?,
                is_reference: row.get::<_, i64>(7)? != 0,
                frame_count: row.get(8)?,
                report_classification,
            })
        })?;
        rows.collect()
    }

    pub fn session_status(&self, session_id: &str) -> Result<Option<String>> {
        self.connection
            .query_row(
                "SELECT status FROM monitoring_session WHERE id = ?1",
                params![session_id],
                |row| row.get(0),
            )
            .optional()
    }

    pub fn record_coverage_change(
        &self,
        session_id: &str,
        at: i64,
        from_tier: &str,
        to_tier: &str,
        reason: &str,
    ) -> Result<()> {
        self.connection.execute(
            "INSERT INTO coverage_change (session_id, at, from_tier, to_tier, reason)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![session_id, at.max(0), from_tier, to_tier, reason],
        )?;
        Ok(())
    }

    pub fn set_session_coverage_tier(&self, session_id: &str, tier: &str) -> Result<bool> {
        let changed = self.connection.execute(
            "UPDATE monitoring_session SET coverage_tier = ?2 WHERE id = ?1 AND status IN ('running', 'preparing')",
            params![session_id, tier],
        )?;
        Ok(changed == 1)
    }

    pub fn coverage_changes(&self, session_id: &str) -> Result<Vec<CoverageChange>> {
        let mut statement = self.connection.prepare(
            "SELECT at, from_tier, to_tier, reason
             FROM coverage_change WHERE session_id = ?1 ORDER BY at, id",
        )?;
        let rows = statement.query_map(params![session_id], |row| {
            Ok(CoverageChange {
                at: row.get(0)?,
                from_tier: row.get(1)?,
                to_tier: row.get(2)?,
                reason: row.get(3)?,
            })
        })?;
        rows.collect()
    }

    pub fn set_session_reference(&self, session_id: &str, is_reference: bool) -> Result<bool> {
        let changed = self.connection.execute(
            "UPDATE monitoring_session SET is_reference = ?2
             WHERE id = ?1 AND kind = 'guided' AND status = 'completed'",
            params![session_id, if is_reference { 1 } else { 0 }],
        )?;
        Ok(changed == 1)
    }

    pub fn stored_report(&self, session_id: &str) -> Result<Option<StoredReport>> {
        self.connection
            .query_row(
                "SELECT session_id, schema_version, payload_json, frozen_at
                 FROM diagnostic_report WHERE session_id = ?1",
                params![session_id],
                |row| {
                    Ok(StoredReport {
                        session_id: row.get(0)?,
                        schema_version: row.get(1)?,
                        payload_json: row.get(2)?,
                        frozen_at: row.get(3)?,
                    })
                },
            )
            .optional()
    }

    /// Freezes the report of a session. The class, severity, per-class durations and analysed
    /// interval come from the engine's `verdict` for the whole session; without one (nothing was
    /// ever evaluated) the report says `indeterminate` instead of inventing «normal».
    pub fn freeze_report(
        &self,
        session_id: &str,
        verdict: Option<&crate::diagnostics::events::SessionClassification>,
    ) -> Result<Option<DiagnosticReport>> {
        let summary =
            self.list_sessions(100)?.into_iter().find(|session| session.session_id == session_id);
        let Some(summary) = summary else {
            return Ok(None);
        };
        let (start_ms, end_ms): (Option<i64>, Option<i64>) = self.connection.query_row(
            "SELECT MIN(monotonic_ms), MAX(monotonic_ms) FROM sample_frame WHERE session_id = ?1",
            params![session_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let start_ms = start_ms.unwrap_or(0);
        let end_ms = end_ms.unwrap_or(start_ms);
        let report_events = self
            .analysis_events(session_id, start_ms, end_ms)?
            .into_iter()
            .map(|event| crate::export::ExportEvent {
                kind: event.kind,
                start_ms: event.start_ms,
                end_ms: event.end_ms,
                severity: None,
            })
            .collect::<Vec<_>>();
        let (classification, severity, duration_by_class_ms, analyzed_start_ms, analyzed_end_ms) =
            match verdict {
                Some(verdict) => (
                    verdict.classification.key().to_owned(),
                    verdict.severity.map(|value| value.key().to_owned()),
                    verdict
                        .class_durations_ms
                        .iter()
                        .map(|(class, duration)| {
                            (class.clone(), i64::try_from(*duration).unwrap_or(i64::MAX))
                        })
                        .collect(),
                    verdict
                        .analyzed_from_ms
                        .and_then(|value| i64::try_from(value).ok())
                        .unwrap_or(start_ms),
                    verdict
                        .analyzed_to_ms
                        .and_then(|value| i64::try_from(value).ok())
                        .unwrap_or(end_ms),
                ),
                None => (
                    "indeterminate".to_owned(),
                    None,
                    std::collections::BTreeMap::new(),
                    start_ms,
                    end_ms,
                ),
            };
        let confidence = match (classification.as_str(), summary.coverage_tier.as_str()) {
            ("indeterminate", _) => "low",
            (_, "A") => "high",
            (_, "B") => "medium",
            _ => "low",
        };
        let rules = [("ruleset_version".to_owned(), "ruleset-v1".to_owned())].into_iter().collect();
        let report = DiagnosticReport {
            classification,
            severity,
            coverage_tier: summary.coverage_tier,
            confidence: confidence.to_owned(),
            analyzed_start_ms,
            analyzed_end_ms,
            duration_by_class_ms,
            cooling_potential: None,
            guided_result: None,
            ruleset_version: "ruleset-v1".to_owned(),
            rules,
            coverage_history: self.coverage_changes(session_id)?,
            events: report_events,
        };
        let payload = serde_json::to_string(&report).map_err(|_| rusqlite::Error::InvalidQuery)?;
        self.connection.execute(
            "INSERT INTO diagnostic_report (session_id, schema_version, payload_json, frozen_at)
             VALUES (?1, 1, ?2, COALESCE(
                 (SELECT ended_at FROM monitoring_session WHERE id = ?1),
                 (SELECT started_at FROM monitoring_session WHERE id = ?1)
             ))
             ON CONFLICT(session_id) DO UPDATE SET
                 schema_version = excluded.schema_version,
                 payload_json = excluded.payload_json,
                 frozen_at = excluded.frozen_at",
            params![session_id, payload],
        )?;
        Ok(Some(report))
    }

    /// Closes the sessions a crash, a kill or a power cut left `running`: they end at their last
    /// recorded frame, as `aborted` with `app_closed_unexpectedly`. Without this such a session
    /// would stay active for ever and block deleting data or resetting the application.
    pub fn abort_orphaned_sessions(&self) -> Result<usize> {
        let mut statement = self.connection.prepare(
            "SELECT session.id, session.started_at,
                    COALESCE((SELECT MAX(frame.monotonic_ms) FROM sample_frame AS frame
                              WHERE frame.session_id = session.id), 0)
             FROM monitoring_session AS session WHERE session.status = 'running'",
        )?;
        let orphans: Vec<(String, String, i64)> = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<Result<_>>()?;
        drop(statement);
        let mut closed = 0;
        for (id, started_at, last_frame_ms) in orphans {
            let ended_at = started_at
                .parse::<jiff::Timestamp>()
                .ok()
                .and_then(|started| {
                    started.checked_add(jiff::SignedDuration::from_millis(last_frame_ms)).ok()
                })
                .map_or(started_at, |ended| ended.to_string());
            if self.finish_session(
                &id,
                "aborted",
                &ended_at,
                last_frame_ms,
                Some("app_closed_unexpectedly"),
            )? {
                // Nothing was evaluated to a conclusion: the report says so.
                self.freeze_report(&id, None)?;
                closed += 1;
            }
        }
        Ok(closed)
    }

    /// Persists one limitation of a session, between two of its recorded frames.
    pub fn insert_limit_event(
        &self,
        session_id: &str,
        kind: &str,
        start_sequence: i64,
        end_sequence: i64,
    ) -> Result<()> {
        self.connection.execute(
            "INSERT INTO limit_event (id, session_id, kind, start_sequence, end_sequence)
             VALUES (?1 || '-e' || (SELECT COUNT(*) + 1 FROM limit_event WHERE session_id = ?1),
                     ?1, ?2, ?3, ?4)",
            params![session_id, kind, start_sequence, end_sequence],
        )?;
        Ok(())
    }

    pub fn session_started_at(&self, session_id: &str) -> Result<Option<String>> {
        self.connection
            .query_row(
                "SELECT started_at FROM monitoring_session WHERE id = ?1",
                params![session_id],
                |row| row.get(0),
            )
            .optional()
    }

    /// `(kind, coverage_tier)`, for the reevaluate-only-imported guard and the reevaluated
    /// report's tier (unaffected by which ruleset judges the samples).
    pub fn session_kind_and_tier(&self, session_id: &str) -> Result<Option<(String, String)>> {
        self.connection
            .query_row(
                "SELECT kind, coverage_tier FROM monitoring_session WHERE id = ?1",
                params![session_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
    }

    /// Every stored frame of a session, each with its own sensor readings, in order. What
    /// [`crate::telemetry::reevaluate`] replays through the engine with today's ruleset (FR-073).
    pub fn session_frames(&self, session_id: &str) -> Result<Vec<StoredFrame>> {
        let mut frames_statement = self.connection.prepare(
            "SELECT sequence, monotonic_ms FROM sample_frame WHERE session_id = ?1 ORDER BY sequence",
        )?;
        let mut frames: Vec<StoredFrame> = frames_statement
            .query_map(params![session_id], |row| {
                Ok(StoredFrame {
                    sequence: row.get(0)?,
                    monotonic_ms: row.get(1)?,
                    values: Vec::new(),
                })
            })?
            .collect::<Result<_>>()?;
        drop(frames_statement);

        let mut values_statement = self.connection.prepare(
            "SELECT sequence, sensor_id, value_real, value_bool
             FROM sample_value WHERE session_id = ?1 ORDER BY sequence",
        )?;
        let rows: Vec<(i64, String, Option<f64>, Option<i64>)> = values_statement
            .query_map(params![session_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })?
            .collect::<Result<_>>()?;
        drop(values_statement);

        type ValuesBySequence =
            std::collections::BTreeMap<i64, Vec<(String, Option<f64>, Option<bool>)>>;
        let mut by_sequence: ValuesBySequence = std::collections::BTreeMap::new();
        for (sequence, sensor_id, value_real, value_bool) in rows {
            by_sequence.entry(sequence).or_default().push((
                sensor_id,
                value_real,
                value_bool.map(|value| value != 0),
            ));
        }
        for frame in &mut frames {
            if let Some(values) = by_sequence.remove(&frame.sequence) {
                frame.values = values;
            }
        }
        Ok(frames)
    }

    pub fn export_snapshot(
        &self,
        session_id: &str,
        range: Option<(i64, i64)>,
    ) -> Result<Option<ExportSnapshot>> {
        let transaction = self.connection.unchecked_transaction()?;
        let session = transaction
            .query_row(
                "SELECT session.id, session.status, session.started_at, session.ended_at,
                        session.monotonic_duration_ms, session.coverage_tier, session.ruleset_version,
                        cpu.vendor, cpu.display_name, cpu.logical_processors, cpu.physical_cores
                 FROM monitoring_session AS session
                 JOIN cpu_device AS cpu ON cpu.id = session.cpu_id
                 WHERE session.id = ?1",
                params![session_id],
                |row| {
                    let logical: i64 = row.get(9)?;
                    let physical: Option<i64> = row.get(10)?;
                    Ok(ExportSession {
                        session_id: row.get(0)?,
                        status: row.get(1)?,
                        started_at: row.get(2)?,
                        ended_at: row.get(3)?,
                        duration_ms: row.get(4)?,
                        coverage_tier: row.get(5)?,
                        cpu_vendor: row.get(7)?,
                        cpu_model: row.get(8)?,
                        topology: format!(
                            "{}t/{}c",
                            logical,
                            physical.unwrap_or(logical)
                        ),
                        ruleset_version: row.get(6)?,
                    })
                },
            )
            .optional()?;
        let Some(session) = session else {
            return Ok(None);
        };

        let (start_ms, end_ms) = range.unwrap_or((i64::MIN, i64::MAX));
        let mut samples_statement = transaction.prepare(
            "SELECT frame.monotonic_ms, value.sensor_id, value.value_real, value.value_bool, value.quality
             FROM sample_value AS value
             JOIN sample_frame AS frame
               ON frame.session_id = value.session_id AND frame.sequence = value.sequence
             WHERE value.session_id = ?1 AND frame.monotonic_ms BETWEEN ?2 AND ?3
             ORDER BY frame.monotonic_ms, value.sensor_id",
        )?;
        let samples = samples_statement
            .query_map(params![session_id, start_ms, end_ms], |row| {
                let value: Option<f64> = row.get(2)?;
                let boolean: Option<i64> = row.get(3)?;
                let boolean = boolean.map(|value| value != 0);
                let sensor_id: String = row.get(1)?;
                let (metric, scope) = crate::telemetry::catalog::known_sensor(&sensor_id)
                    .unwrap_or(("unknown", "unknown"));
                Ok(ExportSample {
                    timestamp_utc: session.started_at.clone(),
                    monotonic_ms: row.get(0)?,
                    sensor_id,
                    metric: metric.to_owned(),
                    scope: scope.to_owned(),
                    value,
                    boolean,
                    status: if value.is_some() || boolean.is_some() { "ok" } else { "missing" }
                        .to_owned(),
                    quality: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        let mut events_statement = transaction.prepare(
            "SELECT event.kind, start_frame.monotonic_ms, end_frame.monotonic_ms
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
        let events = events_statement
            .query_map(params![session_id, start_ms, end_ms], |row| {
                Ok(ExportEvent {
                    kind: row.get(0)?,
                    start_ms: row.get(1)?,
                    end_ms: row.get(2)?,
                    severity: None,
                })
            })?
            .collect::<Result<Vec<_>>>()?;
        drop(events_statement);
        drop(samples_statement);

        let report = transaction
            .query_row(
                "SELECT payload_json FROM diagnostic_report WHERE session_id = ?1",
                params![session_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .map(|payload| {
                serde_json::from_str(&payload).map_err(|_| rusqlite::Error::InvalidQuery)
            })
            .transpose()?;
        transaction.commit()?;
        Ok(Some(ExportSnapshot { session, samples, events, report }))
    }

    pub fn import_export_bundle(&self, bundle: &ExportBundle) -> Result<String> {
        bundle.validate().map_err(|_| rusqlite::Error::InvalidQuery)?;
        let transaction = self.connection.unchecked_transaction()?;
        let cpu_id = "imported-cpu";
        transaction.execute(
            "INSERT OR IGNORE INTO cpu_device
             (id, vendor, display_name, logical_processors, physical_cores, hybrid, fingerprint_version, first_seen_at, last_seen_at)
             VALUES (?1, ?2, ?3, 1, 1, 0, 1, ?4, ?4)",
            params![cpu_id, bundle.session.cpu_vendor, bundle.session.cpu_model, bundle.session.started_at],
        )?;

        let base_id = format!("imported-{}", bundle.session.session_id);
        let mut imported_id = base_id.clone();
        let mut suffix = 2;
        while transaction
            .query_row(
                "SELECT 1 FROM monitoring_session WHERE id = ?1",
                params![imported_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
            .is_some()
        {
            imported_id = format!("{base_id}-{suffix}");
            suffix += 1;
        }
        transaction.execute(
            "INSERT INTO monitoring_session
             (id, cpu_id, kind, status, started_at, ended_at, monotonic_duration_ms,
              protocol_version, ruleset_version, coverage_tier, split_reason)
             VALUES (?1, ?2, 'imported', 'imported', ?3, ?4, ?5, 1, ?6, ?7, 'import')",
            params![
                imported_id,
                cpu_id,
                bundle.session.started_at,
                bundle.session.ended_at,
                bundle.session.duration_ms,
                bundle.session.ruleset_version,
                bundle.session.coverage_tier
            ],
        )?;
        // `ExportSample` carries no frame id: samples that shared a frame share its
        // `monotonic_ms` (the export query orders by it), which is how they are grouped back.
        // One `ExportSample` per frame — the previous shape here — silently turned every real,
        // multi-sensor frame into one single-sensor frame per sensor on import, which broke
        // `session_frames`-based replay (T077's reevaluation) on anything but a single-sensor
        // fixture: fixed together with reevaluation, not before it was needed.
        let mut frames_by_ms: std::collections::BTreeMap<i64, Vec<&ExportSample>> =
            std::collections::BTreeMap::new();
        for sample in &bundle.samples {
            frames_by_ms.entry(sample.monotonic_ms).or_default().push(sample);
        }
        for (sequence, (monotonic_ms, values)) in frames_by_ms.into_iter().enumerate() {
            let sequence = i64::try_from(sequence).map_err(|_| rusqlite::Error::InvalidQuery)?;
            transaction.execute(
                "INSERT INTO sample_frame (session_id, sequence, monotonic_ms, duration_ms, quality)
                 VALUES (?1, ?2, ?3, 0, 'complete')",
                params![imported_id, sequence, monotonic_ms],
            )?;
            for sample in values {
                transaction.execute(
                    "INSERT INTO sample_value (session_id, sequence, sensor_id, value_real, value_bool, quality)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        imported_id,
                        sequence,
                        sample.sensor_id,
                        sample.value,
                        sample.boolean.map(i64::from),
                        sample.quality
                    ],
                )?;
            }
        }
        if let Some(report) = &bundle.report {
            let payload =
                serde_json::to_string(report).map_err(|_| rusqlite::Error::InvalidQuery)?;
            transaction.execute(
                "INSERT INTO diagnostic_report (session_id, schema_version, payload_json, frozen_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![imported_id, bundle.schema_version, payload, bundle.session.ended_at],
            )?;
            for change in &report.coverage_history {
                transaction.execute(
                    "INSERT INTO coverage_change (session_id, at, from_tier, to_tier, reason)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        imported_id,
                        change.at,
                        change.from_tier,
                        change.to_tier,
                        change.reason
                    ],
                )?;
            }
        }
        transaction.commit()?;
        Ok(imported_id)
    }

    pub fn insert_sample(
        &self,
        session_id: &str,
        sequence: i64,
        monotonic_ms: i64,
        duration_ms: i64,
        values: &[StoredSampleValue],
    ) -> Result<()> {
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute(
            "INSERT INTO sample_frame (session_id, sequence, monotonic_ms, duration_ms, quality)
             VALUES (?1, ?2, ?3, ?4, 'complete')",
            params![session_id, sequence, monotonic_ms, duration_ms.max(0)],
        )?;
        for value in values {
            transaction.execute(
                "INSERT INTO sample_value
                     (session_id, sequence, sensor_id, value_real, value_bool, quality)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    session_id,
                    sequence,
                    value.sensor_id,
                    value.value,
                    value.boolean.map(i64::from),
                    value.quality
                ],
            )?;
        }
        transaction.commit()
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

    /// Removes user-generated data while keeping the typed preferences intact. FR-050: also
    /// keeps the onboarding state and the window geometry — neither is "monitoring data", and
    /// wiping the onboarding row would make the next `onboarding_state()` read fail outright
    /// (`query_row` on an empty table errors, it does not fall back to a default).
    pub fn clear_data_preserving_preferences(&self) -> Result<()> {
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute_batch(
            "DELETE FROM monitoring_session;
             DELETE FROM cpu_device;
             DELETE FROM coverage_change;
             DELETE FROM guided_checkpoint;
             DELETE FROM preference;",
        )?;
        transaction.commit()
    }

    /// Removes all local data and restores factory preference values in one SQLite transaction.
    /// FR-051: this must "provocar un primer inicio limpio" — the onboarding and window-geometry
    /// rows are re-seeded with their factory defaults (the same ones the migrations insert on a
    /// brand new database), not just deleted, for the same reason as above.
    pub fn reset_all(&self) -> Result<()> {
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute_batch(
            "DELETE FROM monitoring_session;
             DELETE FROM cpu_device;
             DELETE FROM coverage_change;
             DELETE FROM guided_checkpoint;
             DELETE FROM window_state;
             DELETE FROM onboarding_state;
             DELETE FROM preference;
             DELETE FROM user_preference;
             DELETE FROM update_state;",
        )?;
        for (key, value) in crate::preferences::default_values() {
            let typed_value =
                serde_json::to_string(&value).map_err(|_| rusqlite::Error::InvalidQuery)?;
            transaction.execute(
                "INSERT INTO user_preference (key, typed_value, schema_version, updated_at)
                 VALUES (?1, ?2, ?3, '1970-01-01T00:00:00Z')",
                params![key, typed_value, crate::preferences::SCHEMA_VERSION],
            )?;
        }
        transaction.execute(
            "INSERT INTO onboarding_state (id, flow_version, last_slide, status)
             VALUES (1, 1, 1, 'pending')",
            [],
        )?;
        transaction.execute(
            "INSERT INTO window_state
                 (window_key, restored_x, restored_y, restored_width, restored_height,
                  maximized, updated_at)
             VALUES ('main', 0, 0, 1100, 760, 0, '1970-01-01T00:00:00Z')",
            [],
        )?;
        let payload = serde_json::to_string(&crate::updater::UpdateMachine::default())
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        transaction.execute(
            "INSERT INTO update_state (id, payload_json) VALUES (1, ?1)",
            params![payload],
        )?;
        transaction.commit()
    }

    pub fn usage(&self) -> Result<StorageUsage> {
        let page_count: i64 =
            self.connection.query_row("PRAGMA page_count", [], |row| row.get(0))?;
        let page_size: i64 = self.connection.query_row("PRAGMA page_size", [], |row| row.get(0))?;
        let session_count: i64 =
            self.connection
                .query_row("SELECT COUNT(*) FROM monitoring_session", [], |row| row.get(0))?;
        Ok(StorageUsage {
            database_bytes: u64::try_from(page_count.max(0))
                .unwrap_or(0)
                .saturating_mul(u64::try_from(page_size.max(0)).unwrap_or(0)),
            session_count: u64::try_from(session_count.max(0)).unwrap_or(0),
        })
    }

    /// Removes sessions according to the user-selected retention policy.
    /// The operation is transactional and never touches user preferences.
    pub fn prune_sessions(&mut self, retention: &str, now: &str) -> Result<u64> {
        let transaction = self.connection.unchecked_transaction()?;
        let deleted = match retention {
            "session" => transaction.execute(
                "DELETE FROM monitoring_session WHERE status NOT IN ('running', 'preparing')",
                [],
            )?,
            "1d" | "7d" | "30d" => {
                let days = match retention {
                    "1d" => 1,
                    "7d" => 7,
                    "30d" => 30,
                    _ => 0,
                };
                transaction.execute(
                    "DELETE FROM monitoring_session
                     WHERE status NOT IN ('running', 'preparing')
                       AND ended_at IS NOT NULL
                       AND ended_at < datetime(?1, ?2)",
                    params![now, format!("-{days} days")],
                )?
            }
            _ => 0,
        };
        transaction.commit()?;
        if deleted > 0 {
            // Limited, non-blocking maintenance (SQLite's own recommendation for a long-lived
            // connection): refreshes the query planner's statistics after a prune actually
            // changed the data's shape. Not a `VACUUM` — this never rewrites the whole file.
            let _ = self.connection.execute_batch("PRAGMA optimize;");
        }
        Ok(deleted as u64)
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

    pub fn user_preferences(&self) -> Result<BTreeMap<String, Value>> {
        let mut statement =
            self.connection.prepare("SELECT key, typed_value FROM user_preference ORDER BY key")?;
        let rows = statement.query_map([], |row| {
            let key: String = row.get(0)?;
            let typed_value: String = row.get(1)?;
            let value = serde_json::from_str(&typed_value).map_err(|_| {
                rusqlite::Error::FromSqlConversionFailure(
                    typed_value.len(),
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "invalid stored preference",
                    )),
                )
            })?;
            Ok((key, value))
        })?;
        rows.collect()
    }

    pub fn set_user_preferences(&self, values: &BTreeMap<String, Value>) -> Result<()> {
        let transaction = self.connection.unchecked_transaction()?;
        let now = jiff::Timestamp::now().to_string();
        for (key, value) in values {
            let typed_value =
                serde_json::to_string(value).map_err(|_| rusqlite::Error::InvalidQuery)?;
            transaction.execute(
                "INSERT INTO user_preference (key, typed_value, schema_version, updated_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(key) DO UPDATE SET
                    typed_value = excluded.typed_value,
                    schema_version = excluded.schema_version,
                    updated_at = excluded.updated_at",
                params![key, typed_value, crate::preferences::SCHEMA_VERSION, now],
            )?;
        }
        transaction.commit()
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

/// Distinguishes "the file is corrupt" from any other reason `Connection::open`/migration can
/// fail (a locked file, a permissions error, a disk-full write while migrating): only the former
/// may move the file aside. `SQLITE_NOTADB` covers a file that is not a database at all (empty,
/// truncated, or garbage), which `sqlite3_open` also reports this way.
fn is_corruption(error: &rusqlite::Error) -> bool {
    matches!(
        error.sqlite_error_code(),
        Some(rusqlite::ErrorCode::DatabaseCorrupt) | Some(rusqlite::ErrorCode::NotADatabase)
    )
}

/// `<original file name>.corrupt-<fecha>` next to the original, filesystem-safe on Windows (no
/// `:`) and collision-resistant enough for something that only happens once per corruption.
fn corrupt_backup_path(path: &Path, now: &str) -> PathBuf {
    let stamp: String =
        now.chars().map(|character| if character == ':' { '-' } else { character }).collect();
    let file_name = path.file_name().map(|name| name.to_string_lossy().into_owned());
    let mut backup = path.to_path_buf();
    backup.set_file_name(match file_name {
        Some(name) => format!("{name}.corrupt-{stamp}"),
        None => format!("database.corrupt-{stamp}"),
    });
    backup
}

#[cfg(test)]
mod tests {
    use super::{MIGRATION_LIST, MIGRATIONS, Storage};

    #[test]
    fn migrates_schema_and_enables_foreign_keys() -> rusqlite::Result<()> {
        let storage = Storage::in_memory()?;
        assert_eq!(storage.schema_version()?, MIGRATION_LIST.len() as i64);
        assert!(storage.foreign_keys_enabled()?);
        assert!(storage.advanced_access_enabled()?);
        assert_eq!(storage.onboarding_state()?, super::OnboardingState::default());
        Ok(())
    }

    /// A private scratch file per test, cleaned up on drop — no new dependency (`tempfile`)
    /// for what `std::env::temp_dir()` and a unique name already do.
    struct ScratchFile {
        path: std::path::PathBuf,
    }

    impl ScratchFile {
        fn named(label: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "throttlewatch-test-{label}-{}-{}.db",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|elapsed| elapsed.as_nanos())
                    .unwrap_or_default()
            ));
            Self { path }
        }
    }

    impl Drop for ScratchFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
            let _ = std::fs::remove_file(super::corrupt_backup_path(
                &self.path,
                "2026-09-21T12:00:00Z",
            ));
        }
    }

    #[test]
    fn a_corrupt_database_is_moved_aside_and_a_fresh_one_takes_its_place() {
        let scratch = ScratchFile::named("corrupt");
        let garbage = b"not a sqlite database, just garbage bytes";
        std::fs::write(&scratch.path, garbage)
            .unwrap_or_else(|error| panic!("write garbage: {error}"));

        let (storage, recovered) =
            Storage::open_recovering_corruption(&scratch.path, "2026-09-21T12:00:00Z")
                .unwrap_or_else(|error| panic!("open_recovering_corruption: {error}"));

        let backup = recovered.unwrap_or_else(|| panic!("expected a recovered backup path"));
        assert_ne!(backup, scratch.path, "the backup must not overwrite the fresh database");
        assert_eq!(
            std::fs::read(&backup).unwrap_or_else(|error| panic!("read backup: {error}")),
            garbage,
            "the original bytes must survive untouched in the moved-aside file"
        );
        assert_eq!(
            storage.schema_version().unwrap_or_else(|error| panic!("schema_version: {error}")),
            MIGRATION_LIST.len() as i64,
            "the fresh database at the original path must be fully migrated, not empty"
        );
    }

    #[test]
    fn a_healthy_database_is_never_moved_aside() {
        let scratch = ScratchFile::named("healthy");
        Storage::open(&scratch.path).unwrap_or_else(|error| panic!("initial open: {error}"));

        let (_storage, recovered) =
            Storage::open_recovering_corruption(&scratch.path, "2026-09-21T12:00:00Z")
                .unwrap_or_else(|error| panic!("open_recovering_corruption: {error}"));

        assert!(recovered.is_none(), "a database that opens fine must never be touched");
    }

    #[test]
    fn the_corrupt_backup_name_has_no_colons_even_from_an_iso_timestamp() {
        let backup = super::corrupt_backup_path(
            std::path::Path::new("C:/data/throttlewatch.db"),
            "2026-09-21T12:00:00Z",
        );
        let name = backup.file_name().and_then(|name| name.to_str()).unwrap_or_default();
        assert!(!name.contains(':'), "{name} must be a valid Windows file name");
        assert_eq!(name, "throttlewatch.db.corrupt-2026-09-21T12-00-00Z");
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
    fn persists_typed_versioned_user_preferences() -> rusqlite::Result<()> {
        let storage = Storage::in_memory()?;
        let mut values = storage.user_preferences()?;
        values.insert("appearance.theme".to_owned(), serde_json::json!("dark"));
        storage.set_user_preferences(&values)?;
        assert_eq!(storage.user_preferences()?["appearance.theme"], serde_json::json!("dark"));
        assert_eq!(values.len(), 20);
        Ok(())
    }

    #[test]
    fn persists_updater_state_across_storage_reads() -> rusqlite::Result<()> {
        let storage = Storage::in_memory()?;
        let mut state = crate::updater::UpdateMachine::new(true);
        state
            .begin_check("2026-09-20T08:00:00Z", true)
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        state
            .finish_check(Some(("1.2.0".to_owned(), "signed release".to_owned())))
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        storage.set_updater_state(&state)?;
        assert_eq!(storage.updater_state()?, state);
        Ok(())
    }

    #[test]
    fn clears_data_without_touching_typed_preferences() -> rusqlite::Result<()> {
        let storage = Storage::in_memory()?;
        let mut values = storage.user_preferences()?;
        values.insert("appearance.theme".to_owned(), serde_json::json!("dark"));
        storage.set_user_preferences(&values)?;
        storage.create_cpu("cpu-1", "unknown", "Test CPU")?;
        storage.create_session("session-1", "cpu-1", "start", "2026-09-20T12:00:00Z")?;
        let onboarding = super::OnboardingState {
            flow_version: 1,
            last_slide: 4,
            status: super::OnboardingStatus::Skipped,
            completed_at: Some("2026-09-19T12:00:00Z".to_owned()),
            last_seen_notice_version: 2,
        };
        storage.set_onboarding_state(onboarding.clone())?;
        storage.clear_data_preserving_preferences()?;
        assert!(!storage.session_exists("session-1")?);
        assert_eq!(storage.user_preferences()?["appearance.theme"], serde_json::json!("dark"));
        // FR-050: "conservando preferencias y estado del onboarding" — deleting monitoring data
        // must not send a finished onboarding back to its first slide, nor error out because the
        // row disappeared (`onboarding_state()` has no empty-table fallback).
        assert_eq!(storage.onboarding_state()?, onboarding);
        Ok(())
    }

    #[test]
    fn reset_restores_factory_preferences_in_the_same_database() -> rusqlite::Result<()> {
        let storage = Storage::in_memory()?;
        let mut values = storage.user_preferences()?;
        values.insert("appearance.theme".to_owned(), serde_json::json!("dark"));
        storage.set_user_preferences(&values)?;
        storage.set_advanced_access_enabled(false)?;
        storage.set_onboarding_state(super::OnboardingState {
            flow_version: 1,
            last_slide: 5,
            status: super::OnboardingStatus::Completed,
            completed_at: Some("2026-09-19T12:00:00Z".to_owned()),
            last_seen_notice_version: 3,
        })?;
        storage.reset_all()?;
        assert_eq!(storage.user_preferences()?["appearance.theme"], serde_json::json!("system"));
        assert!(storage.advanced_access_enabled()?);
        assert_eq!(storage.usage()?.session_count, 0);
        // FR-051: "provocar un primer inicio limpio" — a factory reset must leave onboarding and
        // window geometry in their original migration-seeded state, readable without error, not
        // just gone (a missing row makes `onboarding_state()`/`window_state()` fail, which would
        // have broken the very first screen the app shows after a reset).
        assert_eq!(storage.onboarding_state()?, super::OnboardingState::default());
        let window = storage.window_state()?;
        assert_eq!(
            (window.restored_width, window.restored_height, window.maximized),
            (1100, 760, false)
        );
        Ok(())
    }

    #[test]
    fn retention_prunes_completed_sessions_but_keeps_preferences_and_active_work()
    -> rusqlite::Result<()> {
        let mut storage = Storage::in_memory()?;
        storage.create_cpu("cpu-1", "unknown", "Test CPU")?;
        storage.create_session("old", "cpu-1", "start", "2026-08-01T00:00:00Z")?;
        storage.create_session("active", "cpu-1", "start", "2026-09-20T12:00:00Z")?;
        storage.connection.execute(
            "UPDATE monitoring_session SET status = 'completed', ended_at = '2026-09-01T00:00:00Z' WHERE id = 'old'",
            [],
        )?;
        let deleted = storage.prune_sessions("7d", "2026-09-20T00:00:00Z")?;
        assert_eq!(deleted, 1);
        assert!(!storage.session_exists("old")?);
        assert!(storage.session_exists("active")?);
        assert!(storage.user_preferences()?.contains_key("history.retention"));
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
        storage.create_session("session-1", "cpu-1", "start", "2026-09-20T12:00:00Z")?;
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
        storage.create_session("session-1", "cpu-1", "start", "2026-09-20T12:00:00Z")?;
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

    #[test]
    fn writes_guided_samples_checkpoints_and_resolves_latest_session() -> rusqlite::Result<()> {
        let storage = Storage::in_memory()?;
        storage.upsert_cpu("cpu-1", "unknown", "Test CPU", 12, false, "2026-09-20T12:00:00Z")?;
        storage.create_guided_session("guided-1", "cpu-1", "2026-09-20T12:00:00Z")?;
        storage.insert_sample(
            "guided-1",
            0,
            1_000,
            1_000,
            &[super::StoredSampleValue {
                sensor_id: "cpu.package.temp".to_owned(),
                value: Some(65.0),
                boolean: None,
                quality: "direct".to_owned(),
            }],
        )?;
        assert_eq!(storage.latest_session_id()?.as_deref(), Some("guided-1"));
        assert_eq!(storage.analysis_points("guided-1", 0, 2_000)?.len(), 1);
        storage.save_guided_checkpoint(&super::GuidedCheckpoint {
            session_id: "guided-1".to_owned(),
            phase: "warming".to_owned(),
            profile: "standard".to_owned(),
            elapsed_ms: 1_000,
            reason: None,
            updated_at: "2026-09-20T12:00:00Z".to_owned(),
        })?;
        assert_eq!(
            storage.guided_checkpoint("guided-1")?.map(|value| value.phase),
            Some("warming".to_owned())
        );
        Ok(())
    }

    #[test]
    fn the_migration_list_is_valid_and_a_new_database_reaches_the_latest_version()
    -> rusqlite::Result<()> {
        MIGRATIONS.validate().map_err(|_| rusqlite::Error::InvalidQuery)?;
        assert_eq!(MIGRATION_LIST.len(), 9);
        assert_eq!(Storage::in_memory()?.schema_version()?, 9);
        Ok(())
    }

    #[test]
    fn a_database_from_before_rusqlite_migration_is_adopted_not_migrated_twice()
    -> rusqlite::Result<()> {
        // A version-8 database of the old hand-rolled scheme: `schema_version` table, no
        // `update_state`, an existing user preference that must survive.
        let storage = Storage::in_memory()?;
        let mut values = storage.user_preferences()?;
        values.insert("appearance.theme".to_owned(), serde_json::json!("dark"));
        storage.set_user_preferences(&values)?;
        let connection = storage.connection;
        connection.execute_batch(
            "DROP TABLE update_state;
             PRAGMA user_version = 0;
             CREATE TABLE schema_version (version INTEGER NOT NULL);
             INSERT INTO schema_version (version) VALUES (8);",
        )?;

        let upgraded = Storage::from_connection(connection)?;

        assert_eq!(upgraded.schema_version()?, 9);
        assert_eq!(
            upgraded.user_preferences()?.get("appearance.theme"),
            Some(&serde_json::json!("dark"))
        );
        assert!(upgraded.updater_state().is_ok(), "the missing table was created by migration 9");
        let legacy_tables: i64 = upgraded.connection.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name = 'schema_version'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(legacy_tables, 0);
        Ok(())
    }

    #[test]
    fn a_frozen_report_carries_the_engines_verdict_and_the_persisted_events() -> rusqlite::Result<()>
    {
        use crate::diagnostics::classifier::{Classification, Severity};
        use crate::diagnostics::events::SessionClassification;
        let storage = Storage::in_memory()?;
        storage.upsert_cpu("cpu-1", "amd", "Test CPU", 12, false, "2026-09-21T08:00:00Z")?;
        storage.create_session("passive-1", "cpu-1", "start", "2026-09-21T08:00:00Z")?;
        for sequence in 0..=4_i64 {
            storage.insert_frame("passive-1", sequence, sequence * 30_000)?;
        }
        storage.insert_limit_event("passive-1", "thermal", 1, 3)?;
        storage.insert_limit_event("passive-1", "power", 3, 4)?;
        assert!(storage.set_session_coverage_tier("passive-1", "A")?);
        let verdict = SessionClassification {
            classification: Classification::ThermalConfirmed,
            severity: Some(Severity::BelowBase),
            class_durations_ms: [("thermal_confirmed".to_owned(), 90_000_u64)]
                .into_iter()
                .collect(),
            analyzed_from_ms: Some(30_000),
            analyzed_to_ms: Some(120_000),
        };

        let report = storage
            .freeze_report("passive-1", Some(&verdict))?
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;

        assert_eq!(report.classification, "thermal_confirmed");
        assert_eq!(report.severity.as_deref(), Some("below_base"));
        assert_eq!(report.confidence, "high");
        assert_eq!(report.duration_by_class_ms.get("thermal_confirmed"), Some(&90_000));
        assert_eq!((report.analyzed_start_ms, report.analyzed_end_ms), (30_000, 120_000));
        assert_eq!(report.events.len(), 2);
        assert_eq!(report.events[0].kind, "thermal");
        assert_eq!((report.events[0].start_ms, report.events[0].end_ms), (30_000, 90_000));
        assert_eq!(
            storage.list_sessions(10)?[0].report_classification.as_deref(),
            Some("thermal_confirmed")
        );
        Ok(())
    }

    #[test]
    fn a_session_start_can_be_read_back_to_place_events_relative_to_it() -> rusqlite::Result<()> {
        let storage = Storage::in_memory()?;
        storage.upsert_cpu("cpu-1", "amd", "Test CPU", 12, false, "2026-09-21T08:00:00Z")?;
        storage.create_session("passive-1", "cpu-1", "start", "2026-09-21T08:00:00Z")?;
        assert_eq!(
            storage.session_started_at("passive-1")?.as_deref(),
            Some("2026-09-21T08:00:00Z")
        );
        assert_eq!(storage.session_started_at("missing")?, None);
        Ok(())
    }

    #[test]
    fn sessions_left_running_by_a_crash_are_closed_at_their_last_frame() -> rusqlite::Result<()> {
        let storage = Storage::in_memory()?;
        storage.upsert_cpu("cpu-1", "amd", "Test CPU", 12, false, "2026-09-21T08:00:00Z")?;
        storage.create_session("orphan", "cpu-1", "start", "2026-09-21T08:00:00Z")?;
        storage.insert_frame("orphan", 0, 0)?;
        storage.insert_frame("orphan", 1, 90_000)?;
        storage.create_session("done", "cpu-1", "gap", "2026-09-21T07:00:00Z")?;
        assert!(storage.finish_session(
            "done",
            "completed",
            "2026-09-21T07:10:00Z",
            600_000,
            None
        )?);

        assert_eq!(storage.abort_orphaned_sessions()?, 1);

        assert_eq!(storage.active_session_id()?, None, "nothing blocks deleting or resetting");
        let sessions = storage.list_sessions(10)?;
        let orphan = sessions.iter().find(|session| session.session_id == "orphan");
        let orphan = orphan.unwrap_or_else(|| panic!("the orphan session is still listed"));
        assert_eq!(orphan.status, "aborted");
        assert_eq!(orphan.ended_at.as_deref(), Some("2026-09-21T08:01:30Z"));
        assert_eq!(orphan.duration_ms, Some(90_000));
        assert_eq!(orphan.report_classification.as_deref(), Some("indeterminate"));
        let done = sessions.iter().find(|session| session.session_id == "done");
        assert_eq!(done.map(|session| session.status.as_str()), Some("completed"));
        assert_eq!(storage.abort_orphaned_sessions()?, 0, "nothing left to close");
        Ok(())
    }

    #[test]
    fn a_session_keeps_its_real_start_and_finishing_it_frees_the_active_slot()
    -> rusqlite::Result<()> {
        let storage = Storage::in_memory()?;
        storage.upsert_cpu("cpu-1", "amd", "Test CPU", 12, false, "2026-09-21T08:00:00Z")?;
        storage.create_guided_session("guided-1", "cpu-1", "2026-09-21T08:00:00Z")?;
        assert_eq!(storage.active_session_id()?.as_deref(), Some("guided-1"));
        assert_eq!(storage.list_sessions(10)?[0].started_at, "2026-09-21T08:00:00Z");

        assert!(storage.finish_session(
            "guided-1",
            "aborted",
            "2026-09-21T08:03:00Z",
            180_000,
            Some("guided.user_requested"),
        )?);

        assert_eq!(storage.active_session_id()?, None);
        let summary = &storage.list_sessions(10)?[0];
        assert_eq!(summary.status, "aborted");
        assert_eq!(summary.ended_at.as_deref(), Some("2026-09-21T08:03:00Z"));
        assert_eq!(summary.duration_ms, Some(180_000));
        Ok(())
    }

    #[test]
    fn a_finished_session_is_never_rewritten() -> rusqlite::Result<()> {
        let storage = Storage::in_memory()?;
        storage.upsert_cpu("cpu-1", "amd", "Test CPU", 12, false, "2026-09-21T08:00:00Z")?;
        storage.create_guided_session("guided-1", "cpu-1", "2026-09-21T08:00:00Z")?;
        assert!(storage.finish_session(
            "guided-1",
            "completed",
            "2026-09-21T08:06:00Z",
            360_000,
            None
        )?);
        assert!(!storage.finish_session(
            "guided-1",
            "aborted",
            "2026-09-21T09:00:00Z",
            1,
            Some("late")
        )?);
        assert_eq!(storage.list_sessions(10)?[0].status, "completed");
        assert!(!storage.finish_session(
            "missing",
            "completed",
            "2026-09-21T08:06:00Z",
            0,
            None
        )?);
        Ok(())
    }

    #[test]
    fn the_cpu_id_is_stable_and_distinguishes_models() {
        let ryzen = Storage::cpu_id("amd", "Ryzen 5 2600X", 12);
        assert_eq!(ryzen, Storage::cpu_id("amd", "Ryzen 5 2600X", 12));
        assert_ne!(ryzen, Storage::cpu_id("amd", "Ryzen 5 2600", 12));
        assert!(ryzen.starts_with("cpu-") && ryzen.len() == 20);
    }

    #[test]
    fn the_cpu_is_recorded_once_and_its_last_seen_time_refreshes() -> rusqlite::Result<()> {
        let storage = Storage::in_memory()?;
        storage.upsert_cpu("cpu-1", "amd", "Ryzen", 12, false, "2026-09-21T08:00:00Z")?;
        storage.upsert_cpu("cpu-1", "amd", "Ryzen", 12, false, "2026-09-22T08:00:00Z")?;
        let (count, first, last): (i64, String, String) = storage.connection.query_row(
            "SELECT COUNT(*), MIN(first_seen_at), MIN(last_seen_at) FROM cpu_device",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        assert_eq!(count, 1);
        assert_eq!(first, "2026-09-21T08:00:00Z");
        assert_eq!(last, "2026-09-22T08:00:00Z");
        Ok(())
    }

    #[test]
    fn lists_sessions_and_only_completed_guided_sessions_can_be_references() -> rusqlite::Result<()>
    {
        let storage = Storage::in_memory()?;
        storage.upsert_cpu("cpu-1", "unknown", "Test CPU", 12, false, "2026-09-20T12:00:00Z")?;
        storage.create_guided_session("guided-1", "cpu-1", "2026-09-20T12:00:00Z")?;
        let sessions = storage.list_sessions(10)?;
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].status, "running");
        assert!(!storage.set_session_reference("guided-1", true)?);
        storage.connection.execute(
            "UPDATE monitoring_session SET status = 'completed' WHERE id = 'guided-1'",
            [],
        )?;
        assert!(storage.set_session_reference("guided-1", true)?);
        assert!(storage.list_sessions(10)?[0].is_reference);
        Ok(())
    }

    #[test]
    fn exports_a_consistent_snapshot_and_replays_it_as_imported_data() -> rusqlite::Result<()> {
        let storage = Storage::in_memory()?;
        storage.upsert_cpu("cpu-1", "unknown", "Test CPU", 12, false, "2026-09-20T12:00:00Z")?;
        storage.create_guided_session("guided-1", "cpu-1", "2026-09-20T12:00:00Z")?;
        storage.record_coverage_change("guided-1", 1_000, "A", "B", "provider_denied")?;
        storage.insert_sample(
            "guided-1",
            0,
            1_000,
            1_000,
            &[
                super::StoredSampleValue {
                    sensor_id: "cpu.package.temp".to_owned(),
                    value: Some(65.0),
                    boolean: None,
                    quality: "direct".to_owned(),
                },
                super::StoredSampleValue {
                    sensor_id: "msr/thermal_flag".to_owned(),
                    value: None,
                    boolean: Some(true),
                    quality: "direct".to_owned(),
                },
            ],
        )?;
        let report =
            storage.freeze_report("guided-1", None)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        assert_eq!(
            report.classification, "indeterminate",
            "a session nobody evaluated must not be reported as normal"
        );
        assert_eq!(report.confidence, "low");
        let snapshot = storage
            .export_snapshot("guided-1", Some((0, 2_000)))?
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        let bundle = crate::export::bundle_from_snapshot(&snapshot);
        assert_eq!(bundle.samples.len(), 2);
        let temperature = bundle
            .samples
            .iter()
            .find(|sample| sample.sensor_id == "cpu.package.temp")
            .unwrap_or_else(|| panic!("temperature sample"));
        assert_eq!(
            (temperature.metric.as_str(), temperature.scope.as_str()),
            ("temperature", "package")
        );
        let flag = bundle
            .samples
            .iter()
            .find(|sample| sample.sensor_id == "msr/thermal_flag")
            .unwrap_or_else(|| panic!("flag sample"));
        assert_eq!((flag.metric.as_str(), flag.scope.as_str()), ("thermal_flag", "package"));
        assert_eq!(flag.boolean, Some(true));
        assert_eq!(flag.status, "ok", "a boolean-only row is not reported as missing");
        assert!(bundle.report.is_some());
        let imported_id = storage.import_export_bundle(&bundle)?;
        assert!(imported_id.starts_with("imported-guided-1"));
        assert_eq!(storage.frame_count(&imported_id)?, 1);
        assert_eq!(storage.list_sessions(10)?[0].kind, "imported");
        assert_eq!(storage.coverage_changes(&imported_id)?.len(), 1);
        let replayed = storage.session_frames(&imported_id)?;
        let replayed_flag = replayed[0]
            .values
            .iter()
            .find(|(id, _, _)| id == "msr/thermal_flag")
            .unwrap_or_else(|| panic!("the boolean flag must survive the import"));
        assert_eq!(replayed_flag.2, Some(true));
        Ok(())
    }

    #[test]
    fn persists_coverage_history_and_cascades_it_with_the_session() -> rusqlite::Result<()> {
        let storage = Storage::in_memory()?;
        storage.create_cpu("cpu-1", "unknown", "Test CPU")?;
        storage.create_session("session-1", "cpu-1", "start", "2026-09-20T12:00:00Z")?;
        assert!(storage.set_session_coverage_tier("session-1", "B")?);
        assert_eq!(storage.list_sessions(10)?[0].coverage_tier, "B");
        storage.record_coverage_change("session-1", 500, "A", "B", "service_stopped")?;
        storage.record_coverage_change("session-1", 900, "B", "C", "provider_error")?;
        assert_eq!(storage.coverage_changes("session-1")?.len(), 2);
        assert!(storage.delete_session("session-1")?);
        assert!(storage.coverage_changes("session-1")?.is_empty());
        Ok(())
    }
}
