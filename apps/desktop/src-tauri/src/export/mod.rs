#![deny(clippy::unwrap_used, clippy::expect_used)]

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};

pub const EXPORT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Csv,
    Json,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExportScope {
    Session { session_id: String },
    Report { session_id: String },
    Range { session_id: String, start_ms: i64, end_ms: i64 },
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ExportSample {
    pub timestamp_utc: String,
    pub monotonic_ms: i64,
    pub sensor_id: String,
    pub metric: String,
    pub scope: String,
    pub value: Option<f64>,
    /// A limit-reason flag has no numeric `value`; kept separate rather than encoded as 0.0/1.0,
    /// which a reader could mistake for a real measurement.
    #[serde(default)]
    pub boolean: Option<bool>,
    pub status: String,
    pub quality: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ExportEvent {
    pub kind: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub severity: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct CoverageChange {
    pub at: i64,
    pub from_tier: String,
    pub to_tier: String,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct DiagnosticReport {
    pub classification: String,
    pub severity: Option<String>,
    pub coverage_tier: String,
    pub confidence: String,
    pub analyzed_start_ms: i64,
    pub analyzed_end_ms: i64,
    pub duration_by_class_ms: std::collections::BTreeMap<String, i64>,
    pub cooling_potential: Option<CoolingPotential>,
    pub guided_result: Option<GuidedResult>,
    pub ruleset_version: String,
    #[serde(default)]
    pub rules: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub coverage_history: Vec<CoverageChange>,
    pub events: Vec<ExportEvent>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct CoolingPotential {
    pub low_percent: u8,
    pub high_percent: u8,
    pub method: String,
    pub inputs: std::collections::BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct GuidedResult {
    pub initial_ops_per_second: f64,
    pub sustained_ops_per_second: f64,
    pub relative_percent: Option<f64>,
    pub method: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ExportSession {
    pub session_id: String,
    pub status: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub coverage_tier: String,
    pub cpu_vendor: String,
    pub cpu_model: String,
    pub topology: String,
    pub ruleset_version: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ExportBundle {
    pub schema_version: u32,
    pub kind: String,
    pub session: ExportSession,
    pub samples: Vec<ExportSample>,
    pub events: Vec<ExportEvent>,
    pub report: Option<DiagnosticReport>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ExportPreview {
    pub format: ExportFormat,
    pub included_fields: Vec<String>,
    pub excluded_fields: Vec<String>,
    pub estimated_bytes: usize,
    pub proposed_file_name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ReevaluatedReport {
    pub original: DiagnosticReport,
    pub reevaluated: DiagnosticReport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportError {
    InvalidDocument,
    SchemaTooNew { version: u32 },
}

impl ExportBundle {
    pub fn validate(&self) -> Result<(), ImportError> {
        if self.schema_version == 0 || self.schema_version > EXPORT_SCHEMA_VERSION {
            return Err(ImportError::SchemaTooNew { version: self.schema_version });
        }
        if self.kind != "session" || self.session.session_id.is_empty() {
            return Err(ImportError::InvalidDocument);
        }
        Ok(())
    }

    pub fn to_json_bytes(&self, anonymize: bool) -> Result<Vec<u8>, serde_json::Error> {
        let value = serde_json::to_value(self)?;
        let value = if anonymize { anonymize_document(&value) } else { value };
        serde_json::to_vec_pretty(&value)
    }
}

pub fn import_bundle(bytes: &[u8]) -> Result<ExportBundle, ImportError> {
    let mut value: Value =
        serde_json::from_slice(bytes).map_err(|_| ImportError::InvalidDocument)?;
    let version =
        value.get("schema_version").and_then(Value::as_u64).ok_or(ImportError::InvalidDocument)?;
    let version = u32::try_from(version).map_err(|_| ImportError::InvalidDocument)?;
    if version > EXPORT_SCHEMA_VERSION {
        return Err(ImportError::SchemaTooNew { version });
    }
    if version == 0 {
        value
            .as_object_mut()
            .ok_or(ImportError::InvalidDocument)?
            .insert("schema_version".to_owned(), Value::from(EXPORT_SCHEMA_VERSION));
    }
    let bundle: ExportBundle =
        serde_json::from_value(value).map_err(|_| ImportError::InvalidDocument)?;
    bundle.validate()?;
    Ok(bundle)
}

pub fn reevaluate_report(
    original: &DiagnosticReport,
    current: DiagnosticReport,
) -> ReevaluatedReport {
    ReevaluatedReport { original: original.clone(), reevaluated: current }
}

pub fn bundle_from_snapshot(snapshot: &crate::storage::ExportSnapshot) -> ExportBundle {
    ExportBundle {
        schema_version: EXPORT_SCHEMA_VERSION,
        kind: "session".to_owned(),
        session: snapshot.session.clone(),
        samples: snapshot.samples.clone(),
        events: snapshot.events.clone(),
        report: snapshot.report.clone(),
    }
}

pub fn preview_export(
    scope: &ExportScope,
    format: ExportFormat,
    anonymize: bool,
    estimated_bytes: usize,
) -> ExportPreview {
    let kind = match scope {
        ExportScope::Session { .. } => "session",
        ExportScope::Report { .. } => "report",
        ExportScope::Range { .. } => "range",
    };
    let extension = match format {
        ExportFormat::Csv => "csv",
        ExportFormat::Json => "json",
    };
    let mut excluded_fields = Vec::new();
    if anonymize {
        excluded_fields.extend(
            [
                "hostname",
                "windows_user",
                "serial_numbers",
                "mac_addresses",
                "local_paths",
                "monitor_fingerprints",
                "power_plan_guid",
            ]
            .into_iter()
            .map(str::to_owned),
        );
    }
    ExportPreview {
        format,
        included_fields: match format {
            ExportFormat::Csv => vec![
                "timestamp_utc",
                "monotonic_ms",
                "sensor_id",
                "metric",
                "scope",
                "value",
                "status",
                "quality",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
            ExportFormat::Json => vec![
                "classification",
                "severity",
                "coverage_tier",
                "confidence",
                "events",
                "cooling_potential",
                "guided_result",
                "ruleset_version",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        },
        excluded_fields,
        estimated_bytes,
        proposed_file_name: format!("throttlewatch_{kind}_export.{extension}"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteOutcome {
    Written,
    /// Stopped partway through, checked every [`CANCEL_CHECK_ROWS`] rows: cheap enough not to
    /// slow the write down, frequent enough that a cancelled multi-day CSV stops in well under a
    /// second. The caller discards whatever was written.
    Cancelled,
}

const CANCEL_CHECK_ROWS: usize = 512;

pub fn write_csv<W: Write>(
    writer: &mut W,
    rows: impl IntoIterator<Item = ExportSample>,
) -> io::Result<()> {
    match write_csv_checked(writer, rows, None) {
        Ok(WriteOutcome::Written) => Ok(()),
        // No cancellation token was given, so `Cancelled` cannot happen; kept exhaustive so a
        // future variant is not silently swallowed here.
        Ok(WriteOutcome::Cancelled) => Ok(()),
        Err(error) => Err(error),
    }
}

/// Like [`write_csv`], but stops early when `cancelled` turns true (T172's export cancellation).
/// JSON export has no equivalent: [`bundle_from_snapshot`]/[`ExportBundle::to_json_bytes`] build
/// one in-memory document, so JSON export can only be cancelled before or after that step, not
/// mid-write; documented at the call site rather than pretended away.
pub fn write_csv_checked<W: Write>(
    writer: &mut W,
    rows: impl IntoIterator<Item = ExportSample>,
    cancelled: Option<&AtomicBool>,
) -> io::Result<WriteOutcome> {
    writer.write_all(&[0xEF, 0xBB, 0xBF])?;
    writer
        .write_all(b"timestamp_utc,monotonic_ms,sensor_id,metric,scope,value,status,quality\r\n")?;
    for (index, row) in rows.into_iter().enumerate() {
        if index.is_multiple_of(CANCEL_CHECK_ROWS)
            && cancelled.is_some_and(|flag| flag.load(Ordering::Relaxed))
        {
            return Ok(WriteOutcome::Cancelled);
        }
        let cell = row.value.map(|value| value.to_string()).or_else(|| {
            row.boolean.map(|value| if value { "true".to_owned() } else { "false".to_owned() })
        });
        let fields = [
            row.timestamp_utc,
            row.monotonic_ms.to_string(),
            row.sensor_id,
            row.metric,
            row.scope,
            cell.unwrap_or_default(),
            row.status,
            row.quality,
        ];
        let line = fields.into_iter().map(csv_cell).collect::<Vec<_>>().join(",");
        writer.write_all(line.as_bytes())?;
        writer.write_all(b"\r\n")?;
    }
    Ok(WriteOutcome::Written)
}

fn csv_cell(value: String) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\r') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value
    }
}

const SAFE_KEYS: &[&str] = &[
    "schema_version",
    "kind",
    "session",
    "session_id",
    "status",
    "started_at",
    "ended_at",
    "duration_ms",
    "coverage_tier",
    "cpu_vendor",
    "cpu_model",
    "topology",
    "ruleset_version",
    "samples",
    "timestamp_utc",
    "monotonic_ms",
    "sensor_id",
    "metric",
    "scope",
    "value",
    "quality",
    "events",
    "start_ms",
    "end_ms",
    "severity",
    "report",
    "classification",
    "confidence",
    "analyzed_start_ms",
    "analyzed_end_ms",
    "duration_by_class_ms",
    "cooling_potential",
    "low_percent",
    "high_percent",
    "method",
    "inputs",
    "guided_result",
    "initial_ops_per_second",
    "sustained_ops_per_second",
    "relative_percent",
    "rules",
    "coverage_history",
];

pub fn anonymize_document(value: &Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut result = Map::new();
            for (key, value) in object {
                if SAFE_KEYS.contains(&key.as_str()) {
                    result.insert(key.clone(), anonymize_document(value));
                }
            }
            Value::Object(result)
        }
        Value::Array(values) => Value::Array(values.iter().map(anonymize_document).collect()),
        primitive => primitive.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn sample() -> ExportSample {
        ExportSample {
            timestamp_utc: "2026-09-20T12:00:00Z".to_owned(),
            monotonic_ms: 100,
            sensor_id: "cpu.package.temp".to_owned(),
            metric: "temperature".to_owned(),
            scope: "package".to_owned(),
            value: Some(65.5),
            boolean: None,
            status: "ok".to_owned(),
            quality: "direct".to_owned(),
        }
    }

    fn bundle() -> ExportBundle {
        ExportBundle {
            schema_version: EXPORT_SCHEMA_VERSION,
            kind: "session".to_owned(),
            session: ExportSession {
                session_id: "session-1".to_owned(),
                status: "completed".to_owned(),
                started_at: "2026-09-20T12:00:00Z".to_owned(),
                ended_at: Some("2026-09-20T12:01:00Z".to_owned()),
                duration_ms: Some(60_000),
                coverage_tier: "B".to_owned(),
                cpu_vendor: "amd".to_owned(),
                cpu_model: "Ryzen 5".to_owned(),
                topology: "8t/4c".to_owned(),
                ruleset_version: "ruleset-v1".to_owned(),
            },
            samples: vec![sample()],
            events: Vec::new(),
            report: None,
        }
    }

    #[test]
    fn csv_is_long_utf8_bom_and_escapes_cells() -> io::Result<()> {
        let mut output = Vec::new();
        let mut row = sample();
        row.sensor_id = "sensor,\"one\"".to_owned();
        write_csv(&mut output, [row])?;
        let text = String::from_utf8(output).map_err(|_| io::Error::other("invalid UTF-8"))?;
        assert!(text.starts_with('\u{feff}'));
        assert!(text.contains("\"sensor,\"\"one\"\"\""));
        assert!(
            text.contains("timestamp_utc,monotonic_ms,sensor_id,metric,scope,value,status,quality")
        );
        Ok(())
    }

    #[test]
    fn a_cancelled_write_stops_within_the_check_interval_and_reports_it() {
        let mut output = Vec::new();
        let cancelled = AtomicBool::new(false);
        let rows = (0..5_000_i64).map(|index| {
            if index == 10 {
                cancelled.store(true, Ordering::SeqCst);
            }
            let mut row = sample();
            row.monotonic_ms = index;
            row
        });
        let outcome = write_csv_checked(&mut output, rows, Some(&cancelled))
            .unwrap_or_else(|error| panic!("write: {error}"));
        assert_eq!(outcome, WriteOutcome::Cancelled);
        // Stopped at the next check point (every CANCEL_CHECK_ROWS rows), not at row 5000.
        assert!(output.len() < 200_000, "wrote {} bytes, expected an early stop", output.len());
    }

    #[test]
    fn without_a_cancellation_token_every_row_is_written() {
        let mut output = Vec::new();
        let outcome = write_csv_checked(&mut output, (0..10).map(|_| sample()), None)
            .unwrap_or_else(|error| panic!("write: {error}"));
        assert_eq!(outcome, WriteOutcome::Written);
        let newline = b'\n';
        assert_eq!(output.iter().filter(|byte| **byte == newline).count(), 11, "header + 10 rows");
    }

    #[test]
    fn anonymization_is_allowlist_based_and_drops_identifiers_at_any_depth() {
        let input = serde_json::json!({
            "schema_version": 1,
            "session": {"session_id": "session-1", "cpu_model": "Ryzen 5", "hostname": "SECRET-HOST"},
            "report": {"classification": "normal", "user": "SECRET-USER", "nested": {"serial": "SECRET-SERIAL"}},
            "local_path": "C:\\Users\\secret\\report.json"
        });
        let output = anonymize_document(&input).to_string();
        for secret in ["SECRET-HOST", "SECRET-USER", "SECRET-SERIAL", "C:\\\\Users"] {
            assert!(!output.contains(secret), "leaked {secret}");
        }
        assert!(output.contains("Ryzen 5"));
    }

    proptest! {
        #[test]
        fn seeded_identifiers_never_survive_at_any_nested_position(depth in 0usize..8) {
            let mut value = serde_json::json!({"classification": "normal"});
            for _ in 0..depth {
                value = serde_json::json!({
                    "nested": value,
                    "hostname": "SECRET-HOST",
                    "windows_user": "SECRET-USER",
                    "local_path": "SECRET-PATH"
                });
            }
            let output = anonymize_document(&value).to_string();
            prop_assert!(!output.contains("SECRET-HOST"));
            prop_assert!(!output.contains("SECRET-USER"));
            prop_assert!(!output.contains("SECRET-PATH"));
        }
    }

    #[test]
    fn json_round_trip_preserves_export_dto() -> Result<(), Box<dyn std::error::Error>> {
        let original = bundle();
        let bytes = original.to_json_bytes(false)?;
        let restored = import_bundle(&bytes).map_err(|_| "import failed")?;
        assert_eq!(restored, original);
        Ok(())
    }

    #[test]
    fn migrates_schema_zero_and_keeps_original_when_reevaluating()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut legacy = serde_json::to_value(bundle())?;
        legacy["schema_version"] = Value::from(0);
        let bytes = serde_json::to_vec(&legacy)?;
        assert_eq!(
            import_bundle(&bytes).map_err(|_| "migration failed")?.schema_version,
            EXPORT_SCHEMA_VERSION
        );

        let original = DiagnosticReport {
            classification: "thermal_probable".to_owned(),
            severity: Some("below_base".to_owned()),
            coverage_tier: "A".to_owned(),
            confidence: "high".to_owned(),
            analyzed_start_ms: 0,
            analyzed_end_ms: 1_000,
            duration_by_class_ms: [("thermal_probable".to_owned(), 1_000)].into_iter().collect(),
            cooling_potential: None,
            guided_result: None,
            ruleset_version: "ruleset-v1".to_owned(),
            rules: [("thermal_margin_c".to_owned(), "10".to_owned())].into_iter().collect(),
            coverage_history: Vec::new(),
            events: Vec::new(),
        };
        let mut current = original.clone();
        current.classification = "normal".to_owned();
        let reevaluated = reevaluate_report(&original, current);
        assert_eq!(reevaluated.original.classification, "thermal_probable");
        assert_eq!(reevaluated.reevaluated.classification, "normal");
        Ok(())
    }

    #[test]
    fn future_schema_is_rejected_without_touching_storage() {
        let bytes = br#"{"schema_version":2}"#;
        assert_eq!(
            import_bundle(bytes),
            Err(ImportError::SchemaTooNew { version: EXPORT_SCHEMA_VERSION + 1 })
        );
    }

    #[test]
    fn preview_reports_anonymization_and_stable_filename() {
        let preview = preview_export(
            &ExportScope::Session { session_id: "session-1".to_owned() },
            ExportFormat::Json,
            true,
            128,
        );
        assert_eq!(preview.proposed_file_name, "throttlewatch_session_export.json");
        assert!(preview.excluded_fields.iter().any(|field| field == "local_paths"));
    }
}
