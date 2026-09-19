use std::fmt::Debug;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::panic;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tracing::{Event, Subscriber};
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry::LookupSpan;

const MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;
const MAX_LOG_FILES: usize = 5;
const REDACTED: &str = "[redacted]";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LogError {
    pub code: String,
    pub message_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Map<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LogEvent {
    pub ts: String,
    pub level: String,
    pub component: String,
    pub target: String,
    pub code: String,
    pub msg: String,
    pub session_id: Option<String>,
    pub protocol_version: u64,
    pub fields: Map<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub err: Option<LogError>,
}

#[derive(Debug, Default)]
struct EventFields {
    component: Option<String>,
    code: Option<String>,
    msg: Option<String>,
    session_id: Option<String>,
    protocol_version: Option<u64>,
    fields: Map<String, Value>,
}

impl tracing::field::Visit for EventFields {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn Debug) {
        self.fields.insert(field.name().to_owned(), Value::String(format!("{value:?}")));
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        match field.name() {
            "component" => self.component = Some(value.to_owned()),
            "code" => self.code = Some(value.to_owned()),
            "msg" => self.msg = Some(value.to_owned()),
            "session_id" => self.session_id = Some(value.to_owned()),
            _ => {
                self.fields.insert(field.name().to_owned(), Value::String(value.to_owned()));
            }
        }
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        if field.name() == "protocol_version" {
            self.protocol_version = u64::try_from(value).ok();
        } else {
            self.fields.insert(field.name().to_owned(), Value::from(value));
        }
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        if field.name() == "protocol_version" {
            self.protocol_version = Some(value);
        } else {
            self.fields.insert(field.name().to_owned(), Value::from(value));
        }
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.fields.insert(field.name().to_owned(), Value::from(value));
    }
}

fn is_safe_field(name: &str) -> bool {
    matches!(
        name,
        "attempt"
            | "count"
            | "duration_ms"
            | "dropped"
            | "reason"
            | "sequence"
            | "state"
            | "status"
            | "size_bytes"
    )
}

pub fn redact_fields(fields: &Map<String, Value>) -> Map<String, Value> {
    fields
        .iter()
        .map(|(key, value)| {
            (
                key.clone(),
                if is_safe_field(key) { value.clone() } else { Value::String(REDACTED.to_owned()) },
            )
        })
        .collect()
}

fn now_utc() -> String {
    jiff::Timestamp::now().to_string()
}

fn event_from_tracing(
    metadata: &tracing::Metadata<'_>,
    fields: &mut EventFields,
) -> Option<LogEvent> {
    let code = fields.code.take()?;
    if !code.chars().all(|character| {
        character.is_ascii_uppercase() || character.is_ascii_digit() || character == '_'
    }) {
        return None;
    }
    Some(LogEvent {
        ts: now_utc(),
        level: metadata.level().to_string(),
        component: fields.component.take().unwrap_or_else(|| "core".to_owned()),
        target: metadata.target().to_owned(),
        code,
        msg: fields.msg.take().unwrap_or_else(|| "backend log event".to_owned()),
        session_id: fields.session_id.take(),
        protocol_version: fields.protocol_version.unwrap_or(1),
        fields: redact_fields(&fields.fields),
        err: None,
    })
}

pub struct JsonLogLayer {
    writer: NonBlocking,
    deduplicator: Mutex<Deduplicator>,
}

impl JsonLogLayer {
    fn new(writer: NonBlocking) -> Self {
        Self { writer, deduplicator: Mutex::new(Deduplicator::default()) }
    }
}

#[derive(Default)]
struct Deduplicator {
    pending: Option<(LogEvent, Instant, u64)>,
}

impl Deduplicator {
    fn push(&mut self, event: LogEvent, now: Instant) -> Option<LogEvent> {
        let Some((pending, at, count)) = self.pending.take() else {
            self.pending = Some((event, now, 1));
            return None;
        };
        if pending.code == event.code && now.duration_since(at) < Duration::from_secs(60) {
            self.pending = Some((pending, at, count.saturating_add(1)));
            return None;
        }
        self.pending = Some((event, now, 1));
        Some(with_repeat_count(pending, count))
    }
}

fn with_repeat_count(mut event: LogEvent, count: u64) -> LogEvent {
    if count > 1 {
        event.fields.insert("count".to_owned(), Value::from(count));
    }
    event
}

fn write_event(writer: &NonBlocking, event: &LogEvent) {
    let mut writer = writer.clone();
    if serde_json::to_writer(&mut writer, event).is_ok() {
        let _ = writer.write_all(b"\n");
    }
}

impl<S> Layer<S> for JsonLogLayer
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    fn on_event(&self, event: &Event<'_>, _context: Context<'_, S>) {
        let mut fields = EventFields::default();
        event.record(&mut fields);
        let Some(entry) = event_from_tracing(event.metadata(), &mut fields) else {
            return;
        };
        if let Ok(mut deduplicator) = self.deduplicator.lock()
            && let Some(flushed) = deduplicator.push(entry, Instant::now())
        {
            write_event(&self.writer, &flushed);
        }
    }
}

struct RotatingWriter {
    directory: PathBuf,
    base_name: String,
    file: File,
    bytes: u64,
}

impl RotatingWriter {
    fn open(directory: &Path, base_name: &str) -> io::Result<Self> {
        fs::create_dir_all(directory)?;
        let path = directory.join(base_name);
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        let bytes = file.metadata()?.len();
        Ok(Self { directory: directory.to_owned(), base_name: base_name.to_owned(), file, bytes })
    }

    fn path(&self, suffix: usize) -> PathBuf {
        if suffix == 0 {
            self.directory.join(&self.base_name)
        } else {
            self.directory.join(format!("{}.{}", self.base_name, suffix))
        }
    }

    fn rotate(&mut self) -> io::Result<()> {
        self.file.flush()?;
        for suffix in (1..MAX_LOG_FILES).rev() {
            let from = self.path(suffix - 1);
            let to = self.path(suffix);
            if to.exists() {
                fs::remove_file(&to)?;
            }
            if from.exists() {
                fs::rename(from, to)?;
            }
        }
        self.file = OpenOptions::new().create(true).append(true).open(self.path(0))?;
        self.bytes = 0;
        Ok(())
    }
}

impl Write for RotatingWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.bytes.saturating_add(bytes.len() as u64) > MAX_LOG_BYTES {
            self.rotate()?;
        }
        let written = self.file.write(bytes)?;
        self.bytes = self.bytes.saturating_add(written as u64);
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

pub struct LogGuard {
    _worker: WorkerGuard,
}

pub fn init(directory: impl AsRef<Path>, detailed: bool) -> io::Result<LogGuard> {
    format_human(jiff::Timestamp::now())
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let writer = RotatingWriter::open(directory.as_ref(), "throttlewatch.log")?;
    let (non_blocking, worker) = tracing_appender::non_blocking(writer);
    let layer = JsonLogLayer::new(non_blocking);
    let level = if detailed { tracing::Level::DEBUG } else { tracing::Level::INFO };
    tracing_subscriber::registry()
        .with(layer)
        .with(tracing_subscriber::filter::LevelFilter::from_level(level))
        .try_init()
        .map_err(|error| io::Error::new(io::ErrorKind::AlreadyExists, error.to_string()))?;
    let previous_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        tracing::error!(component = "core", code = "RUST_PANIC", msg = "unhandled Rust panic");
        previous_hook(panic_info);
    }));
    Ok(LogGuard { _worker: worker })
}

pub fn validate_collector_stderr(line: &str) -> Result<LogEvent, String> {
    let event = serde_json::from_str::<LogEvent>(line).map_err(|error| error.to_string())?;
    if event.component != "agent"
        || !event.code.chars().all(|character| {
            character.is_ascii_uppercase() || character.is_ascii_digit() || character == '_'
        })
    {
        return Err("collector log event is outside the contract".to_owned());
    }
    Ok(event)
}

pub fn format_human(timestamp: jiff::Timestamp) -> Result<String, String> {
    let timezone = jiff::tz::TimeZone::get("Europe/Madrid").map_err(|error| error.to_string())?;
    Ok(timestamp.to_zoned(timezone).strftime("%d/%m/%Y %H:%M:%S%.3f %Z").to_string())
}

#[macro_export]
macro_rules! log_trace {
    ($code:expr, $msg:expr) => {
        tracing::trace!(component = "core", code = $code, msg = $msg);
    };
}

#[macro_export]
macro_rules! log_debug {
    ($code:expr, $msg:expr) => {
        tracing::debug!(component = "core", code = $code, msg = $msg);
    };
}

#[macro_export]
macro_rules! log_info {
    ($code:expr, $msg:expr) => {
        tracing::info!(component = "core", code = $code, msg = $msg);
    };
}

#[macro_export]
macro_rules! log_warn {
    ($code:expr, $msg:expr) => {
        tracing::warn!(component = "core", code = $code, msg = $msg);
    };
}

#[macro_export]
macro_rules! log_error {
    ($code:expr, $msg:expr) => {
        tracing::error!(component = "core", code = $code, msg = $msg);
    };
}

#[cfg(test)]
mod tests {
    use super::{Deduplicator, LogEvent, format_human, redact_fields, validate_collector_stderr};
    use serde_json::{Map, Value};
    use std::time::{Duration, Instant};

    #[test]
    fn redacts_fields_outside_the_allowlist() {
        let fields = Map::from_iter([
            ("sequence".to_owned(), Value::from(4)),
            ("user_name".to_owned(), Value::from("Ada")),
        ]);
        let redacted = redact_fields(&fields);
        assert_eq!(redacted["sequence"], Value::from(4));
        assert_eq!(redacted["user_name"], Value::from("[redacted]"));
    }

    #[test]
    fn validates_agent_events_from_stderr() {
        let event = LogEvent {
            ts: "2026-09-19T10:00:00.000Z".to_owned(),
            level: "error".to_owned(),
            component: "agent".to_owned(),
            target: "collector".to_owned(),
            code: "SENSOR_FAILED".to_owned(),
            msg: "sensor failed".to_owned(),
            session_id: None,
            protocol_version: 1,
            fields: Map::new(),
            err: None,
        };
        let raw = serde_json::to_string(&event).unwrap_or_default();
        assert_eq!(validate_collector_stderr(&raw), Ok(event));
        assert!(validate_collector_stderr(&raw.replace("SENSOR_FAILED", "bad-code")).is_err());
    }

    #[test]
    fn formats_madrid_with_milliseconds_and_zone() {
        let winter = "2026-01-15T12:00:00Z"
            .parse::<jiff::Timestamp>()
            .unwrap_or_else(|error| panic!("winter timestamp: {error}"));
        let summer = "2026-07-15T12:00:00Z"
            .parse::<jiff::Timestamp>()
            .unwrap_or_else(|error| panic!("summer timestamp: {error}"));
        let winter_text =
            format_human(winter).unwrap_or_else(|error| panic!("winter format: {error}"));
        let summer_text =
            format_human(summer).unwrap_or_else(|error| panic!("summer format: {error}"));
        assert!(winter_text.contains("15/01/2026") && winter_text.contains("CET"));
        assert!(summer_text.contains("15/07/2026") && summer_text.contains("CEST"));
    }

    #[test]
    fn groups_the_same_code_only_within_sixty_seconds() {
        let mut deduplicator = Deduplicator::default();
        let start = Instant::now();
        let event = || LogEvent {
            ts: "2026-09-19T10:00:00.000Z".to_owned(),
            level: "warn".to_owned(),
            component: "core".to_owned(),
            target: "collector".to_owned(),
            code: "COLLECTOR_RESTART".to_owned(),
            msg: "restart".to_owned(),
            session_id: None,
            protocol_version: 1,
            fields: Map::new(),
            err: None,
        };
        assert!(deduplicator.push(event(), start).is_none());
        assert!(deduplicator.push(event(), start + Duration::from_secs(10)).is_none());
        let flushed = deduplicator
            .push(event(), start + Duration::from_secs(61))
            .unwrap_or_else(|| panic!("the repeated event must flush"));
        assert_eq!(flushed.fields["count"], Value::from(2));
    }
}
