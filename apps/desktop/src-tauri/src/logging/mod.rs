use std::fmt::Debug;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::panic;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
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

pub fn directory_bytes(directory: impl AsRef<Path>) -> io::Result<u64> {
    let mut total = 0_u64;
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error),
    };
    for entry in entries {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            total = total.saturating_add(entry.metadata()?.len());
        }
    }
    Ok(total)
}

pub fn clear_directory(directory: impl AsRef<Path>) -> io::Result<()> {
    let directory = directory.as_ref();
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    for entry in entries {
        let entry = entry?;
        if entry.file_type()?.is_file()
            && entry.file_name().to_string_lossy().starts_with("throttlewatch")
        {
            fs::remove_file(entry.path())?;
        }
    }
    Ok(())
}

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
    target: Option<String>,
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
            "target" => self.target = Some(value.to_owned()),
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
        target: fields.target.take().unwrap_or_else(|| metadata.target().to_owned()),
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
    control: LogControl,
}

impl JsonLogLayer {
    fn new(writer: NonBlocking, control: LogControl) -> Self {
        Self { writer, deduplicator: Mutex::new(Deduplicator::default()), control }
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
        if matches!(*event.metadata().level(), tracing::Level::DEBUG | tracing::Level::TRACE)
            && !self.control.is_detailed()
        {
            return;
        }
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
    control: LogControl,
}

#[derive(Clone)]
pub struct LogControl {
    detailed: Arc<AtomicBool>,
    detailed_until: Arc<Mutex<Option<String>>>,
}

impl LogControl {
    pub fn set_detailed(&self, enabled: bool) {
        self.detailed.store(enabled, Ordering::Relaxed);
        if let Ok(mut until) = self.detailed_until.lock() {
            *until = None;
        }
    }

    pub fn set_detailed_until(&self, until: Option<String>) {
        self.detailed.store(until.is_some(), Ordering::Relaxed);
        if let Ok(mut deadline) = self.detailed_until.lock() {
            *deadline = until;
        }
    }

    pub fn is_detailed(&self) -> bool {
        if !self.detailed.load(Ordering::Relaxed) {
            return false;
        }
        let Ok(until) = self.detailed_until.lock() else { return false };
        until.as_deref().is_none_or(|value| {
            value.parse::<jiff::Timestamp>().is_ok_and(|deadline| deadline > jiff::Timestamp::now())
        })
    }
}

pub fn init(directory: impl AsRef<Path>, detailed: bool) -> io::Result<LogGuard> {
    format_human(jiff::Timestamp::now())
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let writer = RotatingWriter::open(directory.as_ref(), "throttlewatch.log")?;
    let (non_blocking, worker) = tracing_appender::non_blocking(writer);
    let control = LogControl {
        detailed: Arc::new(AtomicBool::new(detailed)),
        detailed_until: Arc::new(Mutex::new(None)),
    };
    let layer = JsonLogLayer::new(non_blocking, control.clone());
    tracing_subscriber::registry()
        .with(layer)
        .try_init()
        .map_err(|error| io::Error::new(io::ErrorKind::AlreadyExists, error.to_string()))?;
    let previous_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        tracing::error!(component = "core", code = "RUST_PANIC", msg = "unhandled Rust panic");
        previous_hook(panic_info);
    }));
    Ok(LogGuard { _worker: worker, control })
}

impl LogGuard {
    pub fn control(&self) -> LogControl {
        self.control.clone()
    }
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
    use super::{
        Deduplicator, LogControl, LogEvent, clear_directory, directory_bytes, format_human,
        redact_fields, validate_collector_stderr,
    };
    use serde_json::{Map, Value};
    use std::fs::{self, File};
    use std::io::Write;
    use std::sync::{Arc, Mutex, atomic::AtomicBool};
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

    #[test]
    fn detailed_logging_control_can_be_changed_without_restarting_the_layer() {
        let control = LogControl {
            detailed: Arc::new(AtomicBool::new(false)),
            detailed_until: Arc::new(Mutex::new(None)),
        };
        assert!(!control.is_detailed());
        control.set_detailed(true);
        assert!(control.is_detailed());
        control.set_detailed(false);
        assert!(!control.is_detailed());
    }

    #[test]
    fn detailed_logging_expires_while_the_process_remains_open() {
        let control = LogControl {
            detailed: Arc::new(AtomicBool::new(false)),
            detailed_until: Arc::new(Mutex::new(None)),
        };
        control.set_detailed_until(Some("2020-01-01T00:00:00Z".to_owned()));
        assert!(!control.is_detailed());
    }

    #[test]
    fn measures_and_clears_only_throttlewatch_logs() -> std::io::Result<()> {
        let directory =
            std::env::temp_dir().join(format!("throttlewatch-log-test-{}", std::process::id()));
        fs::create_dir_all(&directory)?;
        let log = directory.join("throttlewatch.log");
        let unrelated = directory.join("keep.txt");
        let mut file = File::create(&log)?;
        file.write_all(b"event")?;
        drop(file);
        File::create(&unrelated)?;
        assert_eq!(directory_bytes(&directory)?, 5);
        clear_directory(&directory)?;
        assert!(!log.exists());
        assert!(unrelated.exists());
        fs::remove_file(unrelated)?;
        fs::remove_dir(directory)?;
        Ok(())
    }
}
