use std::fmt::Debug;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::panic;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU8, Ordering},
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
const MAX_MSG_CHARS: usize = 256;

/// The five levels of principle XVII, ordered from least to most verbose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "error" => Some(Self::Error),
            "warn" => Some(Self::Warn),
            "info" => Some(Self::Info),
            "debug" => Some(Self::Debug),
            "trace" => Some(Self::Trace),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
            Self::Trace => "trace",
        }
    }

    fn from_tracing(level: &tracing::Level) -> Self {
        match *level {
            tracing::Level::ERROR => Self::Error,
            tracing::Level::WARN => Self::Warn,
            tracing::Level::INFO => Self::Info,
            tracing::Level::DEBUG => Self::Debug,
            tracing::Level::TRACE => Self::Trace,
        }
    }

    const fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Error,
            1 => Self::Warn,
            2 => Self::Info,
            3 => Self::Debug,
            _ => Self::Trace,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildKind {
    /// Debug builds and the CI `e2e` build: never distributed.
    Development,
    Production,
}

impl BuildKind {
    pub const fn current() -> Self {
        if cfg!(any(debug_assertions, feature = "e2e")) {
            Self::Development
        } else {
            Self::Production
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Development => "development",
            Self::Production => "production",
        }
    }
}

/// The level a build logs at with «Registro detallado» off, and whether a development override
/// was present but invalid (the caller reports it once the log exists). Production ignores any
/// override: XV/XVII forbid changing the installed version's level from the environment.
pub fn base_level(build: BuildKind, dev_override: Option<&str>) -> (LogLevel, bool) {
    match build {
        BuildKind::Production => (LogLevel::Info, false),
        BuildKind::Development => match dev_override {
            None => (LogLevel::Debug, false),
            Some(value) => {
                LogLevel::parse(value).map_or((LogLevel::Debug, true), |level| (level, false))
            }
        },
    }
}

/// «Registro detallado» raises the level to `debug` at least; it never reaches `trace` on its own.
pub fn effective_level(base: LogLevel, detailed: bool) -> LogLevel {
    if detailed { base.max(LogLevel::Debug) } else { base }
}

static CURRENT_SESSION: Mutex<Option<String>> = Mutex::new(None);

/// Sets the session every event is correlated with from now on and returns the previous one.
/// Global rather than per thread: the collector, the guided loop and the commands all run on
/// different threads and must share it.
pub fn set_current_session(session_id: Option<String>) -> Option<String> {
    match CURRENT_SESSION.lock() {
        Ok(mut current) => std::mem::replace(&mut *current, session_id),
        Err(poisoned) => std::mem::replace(&mut *poisoned.into_inner(), session_id),
    }
}

pub fn current_session() -> Option<String> {
    CURRENT_SESSION
        .lock()
        .map_or_else(|poisoned| poisoned.into_inner().clone(), |current| current.clone())
}

/// Correlates events with `session_id` until dropped, then restores whatever was current before
/// (a guided test runs inside the passive session and must hand it back).
#[must_use]
pub struct SessionScope {
    previous: Option<String>,
}

pub fn session_scope(session_id: &str) -> SessionScope {
    SessionScope { previous: set_current_session(Some(session_id.to_owned())) }
}

impl Drop for SessionScope {
    fn drop(&mut self) {
        set_current_session(self.previous.take());
    }
}

#[cfg(test)]
static SESSION_TEST_LOCK: Mutex<()> = Mutex::new(());

/// Tests that set or read the global session take this, so parallel tests do not see each
/// other's session.
#[cfg(test)]
pub(crate) fn session_test_lock() -> std::sync::MutexGuard<'static, ()> {
    SESSION_TEST_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The events `emit` logs, exactly as they would reach the file (redaction, default component,
/// current session): what the diagnosis scenarios of XVII assert on, from any module's tests.
#[cfg(test)]
pub(crate) fn capture_events(emit: impl FnOnce()) -> Vec<LogEvent> {
    struct Capture(Arc<Mutex<Vec<LogEvent>>>);
    impl<S> Layer<S> for Capture
    where
        S: Subscriber + for<'span> LookupSpan<'span>,
    {
        fn on_event(&self, event: &Event<'_>, _context: Context<'_, S>) {
            let mut fields = EventFields::default();
            event.record(&mut fields);
            if let Some(entry) = event_from_tracing(event.metadata(), &mut fields, "core")
                && let Ok(mut captured) = self.0.lock()
            {
                captured.push(entry);
            }
        }
    }
    let captured = Arc::new(Mutex::new(Vec::new()));
    let subscriber = tracing_subscriber::registry().with(Capture(Arc::clone(&captured)));
    tracing::subscriber::with_default(subscriber, emit);
    captured.lock().map(|events| events.clone()).unwrap_or_default()
}

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
    // `%value` (Display) and `?value` (Debug) both dispatch here, never to `record_str`, even for
    // a reserved name like `msg` or `code` — the widespread `msg = %format!(...)` call sites (the
    // collector's own launch failures, every forwarded sensor-agent/UI message) would otherwise
    // land in the generic `fields` map and be wiped by `redact_fields`, regardless of how
    // detailed logging was set. Route the reserved names the same way `record_str` does.
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn Debug) {
        let formatted = format!("{value:?}");
        match field.name() {
            "component" => self.component = Some(formatted),
            "code" => self.code = Some(formatted),
            "msg" => self.msg = Some(formatted),
            "target" => self.target = Some(formatted),
            "session_id" => self.session_id = Some(formatted),
            name => {
                self.fields.insert(name.to_owned(), Value::String(formatted));
            }
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        match field.name() {
            "component" => self.component = Some(value.to_owned()),
            "code" => self.code = Some(value.to_owned()),
            "msg" => self.msg = Some(value.to_owned()),
            "target" => self.target = Some(value.to_owned()),
            "session_id" => self.session_id = Some(value.to_owned()),
            // `__emit` carries the wrapper's structured fields as one JSON object: `tracing` needs
            // field names fixed at compile time, the wrapper's are the caller's.
            "fields_json" => {
                if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(value) {
                    for (name, value) in map {
                        match (name.as_str(), value) {
                            ("target", Value::String(target)) => self.target = Some(target),
                            (_, value) => {
                                self.fields.insert(name, value);
                            }
                        }
                    }
                }
            }
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
        "action"
            | "attempt"
            | "build"
            | "check"
            | "classification"
            | "count"
            | "elevated"
            | "enabled"
            | "from"
            | "kind"
            | "profile"
            | "require_ac"
            | "skip_rest"
            | "to"
            | "detailed"
            | "duration_ms"
            | "dropped"
            | "level"
            | "original_level"
            | "phase"
            | "reason"
            | "sequence"
            | "state"
            | "status"
            | "size_bytes"
            | "tier"
            | "version"
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

fn is_event_code(code: &str) -> bool {
    !code.is_empty()
        && code.chars().all(|character| {
            character.is_ascii_uppercase() || character.is_ascii_digit() || character == '_'
        })
}

/// Events the application itself emits. A third-party crate's `tracing` output carries no code
/// and is not ours to report; ours without a code is a defect that must still reach the file.
fn is_own_target(target: &str) -> bool {
    target.starts_with("throttlewatch") || target == "ui"
}

fn bounded_message(message: String) -> String {
    let message = if message.trim().is_empty() { "(no message)".to_owned() } else { message };
    if message.chars().count() <= MAX_MSG_CHARS {
        return message;
    }
    message.chars().take(MAX_MSG_CHARS).collect()
}

fn event_from_tracing(
    metadata: &tracing::Metadata<'_>,
    fields: &mut EventFields,
    default_component: &str,
) -> Option<LogEvent> {
    let level = LogLevel::from_tracing(metadata.level());
    let mut entry_fields = redact_fields(&fields.fields);
    let (level, code) = match fields.code.take().filter(|code| is_event_code(code)) {
        Some(code) => (level, code),
        None if is_own_target(metadata.target()) => {
            entry_fields.insert("original_level".to_owned(), Value::from(level.as_str()));
            (LogLevel::Error, "LOG_EVENT_WITHOUT_CODE".to_owned())
        }
        None => return None,
    };
    Some(LogEvent {
        ts: now_utc(),
        level: level.as_str().to_owned(),
        component: fields
            .component
            .take()
            .filter(|component| !component.is_empty())
            .unwrap_or_else(|| default_component.to_owned()),
        target: fields.target.take().unwrap_or_else(|| metadata.target().to_owned()),
        code,
        msg: bounded_message(fields.msg.take().unwrap_or_default()),
        session_id: fields.session_id.take().filter(|id| !id.is_empty()).or_else(current_session),
        protocol_version: fields.protocol_version.unwrap_or(1),
        fields: entry_fields,
        err: None,
    })
}

/// `LOG_EVENTS_DROPPED` when the non-blocking writer's lossy channel dropped lines since the
/// last check; the event itself is written straight to the writer, never through `tracing`.
fn dropped_events_entry(reported: usize, dropped_now: usize) -> Option<LogEvent> {
    let newly_dropped = dropped_now.checked_sub(reported).filter(|count| *count > 0)?;
    Some(LogEvent {
        ts: now_utc(),
        level: LogLevel::Warn.as_str().to_owned(),
        component: "core".to_owned(),
        target: "throttlewatch_lib::logging".to_owned(),
        code: "LOG_EVENTS_DROPPED".to_owned(),
        msg: "the log writer was saturated and dropped events".to_owned(),
        session_id: current_session(),
        protocol_version: 1,
        fields: Map::from_iter([("dropped".to_owned(), Value::from(newly_dropped as u64))]),
        err: None,
    })
}

/// How long repeats of the same code are folded into one entry, and therefore how long a quiet
/// application may hold its last entry before [`flush_aged`] writes it out anyway.
const DEDUPLICATION_WINDOW: Duration = Duration::from_secs(60);

pub struct JsonLogLayer {
    writer: NonBlocking,
    deduplicator: Arc<Mutex<Deduplicator>>,
    control: LogControl,
    default_component: &'static str,
}

/// Writes the pending entry when it is older than the deduplication window. Without this the
/// deduplicator only ever emits an entry when a *different* one displaces it, so a healthy, quiet
/// application writes nothing at all and the last thing that happened — usually the interesting
/// one — never reaches disk (T184).
fn flush_aged(writer: &NonBlocking, deduplicator: &Mutex<Deduplicator>, now: Instant) {
    if let Ok(mut guard) = deduplicator.lock()
        && let Some(event) = guard.take_aged(now, DEDUPLICATION_WINDOW)
    {
        write_event(writer, &event);
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
        if pending.code == event.code && now.duration_since(at) < DEDUPLICATION_WINDOW {
            self.pending = Some((pending, at, count.saturating_add(1)));
            return None;
        }
        self.pending = Some((event, now, 1));
        Some(with_repeat_count(pending, count))
    }

    /// The pending entry once it has outlived the window, so a quiet application still writes it.
    fn take_aged(&mut self, now: Instant, window: Duration) -> Option<LogEvent> {
        let (_, at, _) = self.pending.as_ref()?;
        if now.duration_since(*at) < window {
            return None;
        }
        let (event, _, count) = self.pending.take()?;
        Some(with_repeat_count(event, count))
    }

    /// The pending entry whatever its age, for shutdown: otherwise it dies with the process.
    fn take_pending(&mut self) -> Option<LogEvent> {
        let (event, _, count) = self.pending.take()?;
        Some(with_repeat_count(event, count))
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
        if !self.control.enabled(LogLevel::from_tracing(event.metadata().level())) {
            return;
        }
        let mut fields = EventFields::default();
        event.record(&mut fields);
        let Some(entry) = event_from_tracing(event.metadata(), &mut fields, self.default_component)
        else {
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
    writer: NonBlocking,
    deduplicator: Arc<Mutex<Deduplicator>>,
    _worker: WorkerGuard,
    control: LogControl,
}

impl Drop for LogGuard {
    /// T184: whatever is still pending dies with the process otherwise — and that is precisely
    /// the last thing the application said before closing. Runs before `_worker` is dropped
    /// (fields drop after `Drop::drop`), so the entry still reaches the writer.
    fn drop(&mut self) {
        if let Ok(mut guard) = self.deduplicator.lock()
            && let Some(event) = guard.take_pending()
        {
            write_event(&self.writer, &event);
        }
    }
}

#[derive(Clone)]
pub struct LogControl {
    base: Arc<AtomicU8>,
    detailed: Arc<AtomicBool>,
    detailed_until: Arc<Mutex<Option<String>>>,
}

impl LogControl {
    pub fn new(base: LogLevel) -> Self {
        Self {
            base: Arc::new(AtomicU8::new(base as u8)),
            detailed: Arc::new(AtomicBool::new(false)),
            detailed_until: Arc::new(Mutex::new(None)),
        }
    }

    pub fn base_level(&self) -> LogLevel {
        LogLevel::from_u8(self.base.load(Ordering::Relaxed))
    }

    pub fn effective_level(&self) -> LogLevel {
        effective_level(self.base_level(), self.is_detailed())
    }

    pub fn enabled(&self, level: LogLevel) -> bool {
        level <= self.effective_level()
    }

    pub fn set_detailed(&self, enabled: bool) {
        let was = self.is_detailed();
        self.detailed.store(enabled, Ordering::Relaxed);
        if let Ok(mut until) = self.detailed_until.lock() {
            *until = None;
        }
        report_detailed_change(was, self.is_detailed());
    }

    pub fn set_detailed_until(&self, until: Option<String>) {
        let was = self.is_detailed();
        self.detailed.store(until.is_some(), Ordering::Relaxed);
        if let Ok(mut deadline) = self.detailed_until.lock() {
            *deadline = until;
        }
        report_detailed_change(was, self.is_detailed());
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

fn report_detailed_change(was: bool, now: bool) {
    match (was, now) {
        (false, true) => {
            crate::log_info!(
                "LOG_DETAILED_ENABLED",
                "detailed logging switched on",
                detailed = true
            )
        }
        (true, false) => {
            crate::log_info!(
                "LOG_DETAILED_DISABLED",
                "detailed logging switched off",
                detailed = false
            )
        }
        _ => {}
    }
}

/// The JSON-lines file layer and the guard that flushes it, without installing anything
/// globally: `init` installs it for the process, tests scope it with `with_default`.
fn open_log(
    directory: &Path,
    file_name: &str,
    base: LogLevel,
    default_component: &'static str,
) -> io::Result<(JsonLogLayer, LogGuard)> {
    format_human(jiff::Timestamp::now())
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let writer = RotatingWriter::open(directory, file_name)?;
    let (non_blocking, worker) = tracing_appender::non_blocking(writer);
    let control = LogControl::new(base);
    let deduplicator = Arc::new(Mutex::new(Deduplicator::default()));
    let layer = JsonLogLayer {
        writer: non_blocking.clone(),
        deduplicator: Arc::clone(&deduplicator),
        control: control.clone(),
        default_component,
    };
    Ok((layer, LogGuard { writer: non_blocking, deduplicator, _worker: worker, control }))
}

#[cfg(test)]
pub(crate) fn scoped_file_subscriber(
    directory: &Path,
    file_name: &str,
    base: LogLevel,
    default_component: &'static str,
) -> io::Result<(impl Subscriber + Send + Sync + 'static, LogGuard)> {
    let (layer, guard) = open_log(directory, file_name, base, default_component)?;
    Ok((tracing_subscriber::registry().with(layer), guard))
}

/// The application's log: `throttlewatch.log`, events default to `component: "core"`.
pub fn init(directory: impl AsRef<Path>, base: LogLevel) -> io::Result<LogGuard> {
    init_with(directory.as_ref(), "throttlewatch.log", base, "core")
}

/// The elevated launcher's own log (XVII): its typical failure is precisely not reaching the
/// application, so it cannot report through it.
pub fn init_launcher(directory: impl AsRef<Path>, base: LogLevel) -> io::Result<LogGuard> {
    init_with(directory.as_ref(), "throttlewatch-launcher.log", base, "launcher")
}

fn init_with(
    directory: &Path,
    file_name: &str,
    base: LogLevel,
    default_component: &'static str,
) -> io::Result<LogGuard> {
    let (layer, guard) = open_log(directory, file_name, base, default_component)?;
    tracing_subscriber::registry()
        .with(layer)
        .try_init()
        .map_err(|error| io::Error::new(io::ErrorKind::AlreadyExists, error.to_string()))?;
    // T184: without this the deduplicator only emits an entry when a different one displaces it,
    // so a quiet application never writes anything. The process ends when it ends; this thread
    // does not need to be joined, it only has to keep the tail of the log moving.
    {
        let writer = guard.writer.clone();
        let pending = Arc::clone(&guard.deduplicator);
        let dropped = guard.writer.error_counter();
        std::thread::Builder::new().name("log-flush".to_owned()).spawn(move || {
            let mut reported = 0_usize;
            loop {
                std::thread::sleep(DEDUPLICATION_WINDOW);
                flush_aged(&writer, &pending, Instant::now());
                let dropped_now = dropped.dropped_lines();
                if let Some(event) = dropped_events_entry(reported, dropped_now) {
                    write_event(&writer, &event);
                    reported = dropped_now;
                }
            }
        })?;
    }
    let previous_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        crate::log_error!("RUST_PANIC", "unhandled Rust panic");
        previous_hook(panic_info);
    }));
    Ok(guard)
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

/// `dd/MM/yyyy HH:mm:ss,SSS` in `Europe/Madrid`, with the zone abbreviation (CET/CEST) so the
/// repeated hour of the autumn change stays unambiguous.
pub fn format_human(timestamp: jiff::Timestamp) -> Result<String, String> {
    let timezone = jiff::tz::TimeZone::get("Europe/Madrid").map_err(|error| error.to_string())?;
    Ok(timestamp.to_zoned(timezone).strftime("%d/%m/%Y %H:%M:%S,%3f %Z").to_string())
}

/// The only way application code logs (principle XVII). Every form takes the event code and
/// the message first, so an event without either does not compile:
///
/// ```ignore
/// log_warn!("CODE", "message");
/// log_warn!("CODE", format!("with {detail}"), field = value, other = %display, third = ?debug);
/// log_warn!(component: "guided", "CODE", "message", field = value);
/// log_warn!(session: &session_id, "CODE", "message");
/// log_warn!(component: "agent", session: &session_id, "CODE", "message");
/// ```
///
/// `component` defaults to the log's own (`core`, or `launcher` in the launcher's file) and
/// `session_id` to the session set with [`set_current_session`]/[`session_scope`]. Field names
/// outside the redaction allow-list reach the file as `[redacted]`; `code` and `msg` never do.
/// A structured field value for the wrapper macros' `name = value` form.
pub trait LogValue {
    fn to_log_value(&self) -> Value;
}

impl<T: LogValue + ?Sized> LogValue for &T {
    fn to_log_value(&self) -> Value {
        (**self).to_log_value()
    }
}

impl<T: LogValue> LogValue for Option<T> {
    fn to_log_value(&self) -> Value {
        self.as_ref().map_or(Value::Null, LogValue::to_log_value)
    }
}

impl LogValue for str {
    fn to_log_value(&self) -> Value {
        Value::from(self)
    }
}

impl LogValue for String {
    fn to_log_value(&self) -> Value {
        Value::from(self.as_str())
    }
}

macro_rules! impl_log_value {
    ($($ty:ty),+) => {
        $(impl LogValue for $ty {
            fn to_log_value(&self) -> Value {
                Value::from(*self)
            }
        })+
    };
}

impl_log_value!(bool, i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, f32, f64);

const EMIT_FIELDS: &[&str] = &["component", "session_id", "code", "target", "msg", "fields_json"];

/// One static callsite per level, built by hand exactly as `tracing::event!` builds its own:
/// Clippy reports the `static`s a `tracing::event!` expansion creates at crate level, so no
/// `#[allow]` anywhere in this module could exempt a use of the macro from `disallowed-macros`,
/// and the wrapper would have been the one module unable to comply with it.
macro_rules! emit_callsite {
    ($callsite:ident, $metadata:ident, $level:expr) => {
        static $callsite: tracing::callsite::DefaultCallsite =
            tracing::callsite::DefaultCallsite::new(&$metadata);
        static $metadata: tracing::Metadata<'static> = tracing::Metadata::new(
            "application event",
            module_path!(),
            $level,
            Some(file!()),
            Some(line!()),
            Some(module_path!()),
            tracing::field::FieldSet::new(EMIT_FIELDS, tracing::callsite::Identifier(&$callsite)),
            tracing::metadata::Kind::EVENT,
        );
    };
}

emit_callsite!(ERROR_CALLSITE, ERROR_METADATA, tracing::Level::ERROR);
emit_callsite!(WARN_CALLSITE, WARN_METADATA, tracing::Level::WARN);
emit_callsite!(INFO_CALLSITE, INFO_METADATA, tracing::Level::INFO);
emit_callsite!(DEBUG_CALLSITE, DEBUG_METADATA, tracing::Level::DEBUG);
emit_callsite!(TRACE_CALLSITE, TRACE_METADATA, tracing::Level::TRACE);

/// The single place application events enter `tracing`; the `log_*!` macros only gather the
/// arguments and call this.
#[doc(hidden)]
pub fn __emit(
    level: tracing::Level,
    target: &str,
    component: Option<&str>,
    session: Option<&str>,
    code: &str,
    msg: &str,
    fields: Map<String, Value>,
) {
    let (callsite, metadata): (&'static tracing::callsite::DefaultCallsite, _) =
        if level == tracing::Level::ERROR {
            (&ERROR_CALLSITE, &ERROR_METADATA)
        } else if level == tracing::Level::WARN {
            (&WARN_CALLSITE, &WARN_METADATA)
        } else if level == tracing::Level::INFO {
            (&INFO_CALLSITE, &INFO_METADATA)
        } else if level == tracing::Level::DEBUG {
            (&DEBUG_CALLSITE, &DEBUG_METADATA)
        } else {
            (&TRACE_CALLSITE, &TRACE_METADATA)
        };
    let _ = callsite.interest();
    let fields_json =
        if fields.is_empty() { String::new() } else { Value::Object(fields).to_string() };
    let component = component.unwrap_or("");
    let session = session.unwrap_or("");
    let set = metadata.fields();
    let (
        Some(component_field),
        Some(session_field),
        Some(code_field),
        Some(target_field),
        Some(msg_field),
        Some(fields_field),
    ) = (
        set.field("component"),
        set.field("session_id"),
        set.field("code"),
        set.field("target"),
        set.field("msg"),
        set.field("fields_json"),
    )
    else {
        return;
    };
    let fields_json = fields_json.as_str();
    let values: [(&tracing::field::Field, Option<&dyn tracing::field::Value>); 6] = [
        (&component_field, Some(&component)),
        (&session_field, Some(&session)),
        (&code_field, Some(&code)),
        (&target_field, Some(&target)),
        (&msg_field, Some(&msg)),
        (&fields_field, Some(&fields_json)),
    ];
    tracing::Event::dispatch(metadata, &set.value_set(&values));
}

#[doc(hidden)]
#[macro_export]
macro_rules! __tw_fields {
    ($map:ident $(,)?) => {};
    ($map:ident, $name:ident = % $value:expr $(, $($rest:tt)*)?) => {
        $map.insert(
            ::std::stringify!($name).to_owned(),
            ::serde_json::Value::String(::std::format!("{}", $value)),
        );
        $( $crate::__tw_fields!($map, $($rest)*); )?
    };
    ($map:ident, $name:ident = ? $value:expr $(, $($rest:tt)*)?) => {
        $map.insert(
            ::std::stringify!($name).to_owned(),
            ::serde_json::Value::String(::std::format!("{:?}", $value)),
        );
        $( $crate::__tw_fields!($map, $($rest)*); )?
    };
    ($map:ident, $name:ident = $value:expr $(, $($rest:tt)*)?) => {
        $map.insert(
            ::std::stringify!($name).to_owned(),
            $crate::logging::LogValue::to_log_value(&$value),
        );
        $( $crate::__tw_fields!($map, $($rest)*); )?
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __tw_log {
    (@emit $level:expr, $component:expr, $session:expr, $code:expr, $msg:expr $(, $($fields:tt)+)?) => {{
        #[allow(unused_mut)]
        let mut __tw_map = ::serde_json::Map::new();
        $( $crate::__tw_fields!(__tw_map, $($fields)+); )?
        $crate::logging::__emit(
            $level,
            ::std::module_path!(),
            $component,
            $session,
            ::std::convert::AsRef::<str>::as_ref(&$code),
            ::std::convert::AsRef::<str>::as_ref(&$msg),
            __tw_map,
        );
    }};
    ($level:expr, component: $component:expr, session: $session:expr, $code:expr, $msg:expr $(, $($fields:tt)+)?) => {
        $crate::__tw_log!(@emit $level, ::std::option::Option::Some($component), ::std::option::Option::Some(::std::convert::AsRef::<str>::as_ref(&$session)), $code, $msg $(, $($fields)+)?)
    };
    ($level:expr, component: $component:expr, $code:expr, $msg:expr $(, $($fields:tt)+)?) => {
        $crate::__tw_log!(@emit $level, ::std::option::Option::Some($component), ::std::option::Option::None, $code, $msg $(, $($fields)+)?)
    };
    ($level:expr, session: $session:expr, $code:expr, $msg:expr $(, $($fields:tt)+)?) => {
        $crate::__tw_log!(@emit $level, ::std::option::Option::None, ::std::option::Option::Some(::std::convert::AsRef::<str>::as_ref(&$session)), $code, $msg $(, $($fields)+)?)
    };
    ($level:expr, $code:expr, $msg:expr $(, $($fields:tt)+)?) => {
        $crate::__tw_log!(@emit $level, ::std::option::Option::None, ::std::option::Option::None, $code, $msg $(, $($fields)+)?)
    };
}

#[macro_export]
macro_rules! log_trace {
    ($($args:tt)+) => { $crate::__tw_log!(::tracing::Level::TRACE, $($args)+) };
}

#[macro_export]
macro_rules! log_debug {
    ($($args:tt)+) => { $crate::__tw_log!(::tracing::Level::DEBUG, $($args)+) };
}

#[macro_export]
macro_rules! log_info {
    ($($args:tt)+) => { $crate::__tw_log!(::tracing::Level::INFO, $($args)+) };
}

#[macro_export]
macro_rules! log_warn {
    ($($args:tt)+) => { $crate::__tw_log!(::tracing::Level::WARN, $($args)+) };
}

#[macro_export]
macro_rules! log_error {
    ($($args:tt)+) => { $crate::__tw_log!(::tracing::Level::ERROR, $($args)+) };
}

#[cfg(test)]
mod tests {
    /// An event the wrapper would never produce (no code, a third-party target, the `?` form),
    /// built with the same low-level API as `__emit`: the `tracing` macros are off limits here
    /// too, since Clippy reports their callsite statics at crate level.
    macro_rules! raw_event {
        ($level:expr, $target:literal, [$($name:literal => $value:expr),* $(,)?]) => {{
            static CALLSITE: tracing::callsite::DefaultCallsite =
                tracing::callsite::DefaultCallsite::new(&META);
            static META: tracing::Metadata<'static> = tracing::Metadata::new(
                "raw test event",
                $target,
                $level,
                None,
                None,
                None,
                tracing::field::FieldSet::new(
                    &[$($name),*],
                    tracing::callsite::Identifier(&CALLSITE),
                ),
                tracing::metadata::Kind::EVENT,
            );
            let _ = CALLSITE.interest();
            let set = META.fields();
            tracing::Event::dispatch(
                &META,
                &set.value_set(&[$((
                    &set.field($name).unwrap_or_else(|| panic!("field {}", $name)),
                    Some(&$value as &dyn tracing::field::Value),
                )),*]),
            );
        }};
    }

    use super::{
        BuildKind, Deduplicator, EventFields, LogControl, LogEvent, LogLevel, base_level,
        clear_directory, directory_bytes, dropped_events_entry, effective_level,
        event_from_tracing, format_human, redact_fields, scoped_file_subscriber, session_scope,
        session_test_lock, set_current_session, validate_collector_stderr,
    };
    use serde_json::{Map, Value};
    use std::fs::{self, File};
    use std::io::Write;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};
    use tracing::{Event, Subscriber};
    use tracing_subscriber::layer::{Context, Layer, SubscriberExt};
    use tracing_subscriber::registry::LookupSpan;

    fn sample_event(code: &str) -> LogEvent {
        LogEvent {
            ts: "2026-09-26T00:00:00Z".to_owned(),
            level: "WARN".to_owned(),
            component: "core".to_owned(),
            target: "test".to_owned(),
            code: code.to_owned(),
            msg: "m".to_owned(),
            session_id: None,
            protocol_version: 1,
            fields: Map::new(),
            err: None,
        }
    }

    /// T184: the bug that made a healthy application write an empty log. `push` only ever returns
    /// an entry when a *different* one displaces it, so a single event stayed pending forever and
    /// was lost when the process ended.
    #[test]
    fn a_lone_event_is_still_written_once_it_outlives_the_window() {
        let mut deduplicator = Deduplicator::default();
        let start = Instant::now();
        assert!(
            deduplicator.push(sample_event("ALONE"), start).is_none(),
            "the first event is held back, waiting to fold repeats into it"
        );
        assert!(
            deduplicator.take_aged(start, Duration::from_secs(60)).is_none(),
            "not yet: inside the window it may still gather repeats"
        );
        let aged = deduplicator
            .take_aged(start + Duration::from_secs(61), Duration::from_secs(60))
            .unwrap_or_else(|| panic!("an event older than the window must be written"));
        assert_eq!(aged.code, "ALONE");
        assert!(deduplicator.take_aged(start + Duration::from_secs(120), Duration::ZERO).is_none());
    }

    #[test]
    fn shutdown_writes_whatever_was_still_pending() {
        let mut deduplicator = Deduplicator::default();
        let start = Instant::now();
        deduplicator.push(sample_event("LAST_WORDS"), start);
        let pending = deduplicator
            .take_pending()
            .unwrap_or_else(|| panic!("closing must not swallow the last entry"));
        assert_eq!(pending.code, "LAST_WORDS");
        assert!(deduplicator.take_pending().is_none());
    }

    #[test]
    fn repeats_are_still_folded_and_counted_when_flushed_by_age() {
        let mut deduplicator = Deduplicator::default();
        let start = Instant::now();
        deduplicator.push(sample_event("SAME"), start);
        deduplicator.push(sample_event("SAME"), start + Duration::from_secs(1));
        deduplicator.push(sample_event("SAME"), start + Duration::from_secs(2));
        let folded = deduplicator
            .take_aged(start + Duration::from_secs(61), Duration::from_secs(60))
            .unwrap_or_else(|| panic!("the folded entry must come out"));
        assert_eq!(folded.fields.get("count").and_then(Value::as_u64), Some(3));
    }

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

    struct Capture(Arc<Mutex<Vec<LogEvent>>>);

    impl<S> Layer<S> for Capture
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
            let mut fields = EventFields::default();
            event.record(&mut fields);
            if let Some(built) = event_from_tracing(event.metadata(), &mut fields, "core")
                && let Ok(mut captured) = self.0.lock()
            {
                captured.push(built);
            }
        }
    }

    /// The bug this guards: `msg = %format!(...)` (Display) — how every collector-launch failure
    /// and every forwarded sensor-agent/UI message is logged — used to be recorded as a generic
    /// field named `msg` inside `fields`, not the top-level, never-redacted `msg`. It survived
    /// only as `"[redacted]"`, in the log file, at any logging level.
    fn capture(emit: impl FnOnce()) -> Vec<LogEvent> {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let subscriber = tracing_subscriber::registry().with(Capture(Arc::clone(&captured)));
        tracing::subscriber::with_default(subscriber, emit);
        captured.lock().unwrap_or_else(|error| panic!("{error}")).clone()
    }

    #[test]
    fn an_own_event_without_a_code_is_reported_instead_of_dropped() {
        let _session = session_test_lock();
        let events = capture(|| {
            raw_event!(tracing::Level::WARN, "throttlewatch_lib::logging::tests", [
                "msg" => "session recording failed",
            ])
        });
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].code, "LOG_EVENT_WITHOUT_CODE");
        assert_eq!(events[0].level, "error");
        assert_eq!(events[0].msg, "session recording failed");
        assert_eq!(events[0].fields["original_level"], Value::from("warn"));
    }

    #[test]
    fn an_invalid_code_is_reported_like_a_missing_one() {
        let _session = session_test_lock();
        let events = capture(|| {
            raw_event!(tracing::Level::INFO, "throttlewatch_lib::logging::tests", [
                "code" => "not-a-code",
                "msg" => "m",
            ])
        });
        assert_eq!(events[0].code, "LOG_EVENT_WITHOUT_CODE");
    }

    #[test]
    fn third_party_events_without_a_code_stay_out_of_the_file() {
        let _session = session_test_lock();
        let events =
            capture(|| raw_event!(tracing::Level::WARN, "some_dependency", ["message" => "noise"]));
        assert!(events.is_empty());
    }

    #[test]
    fn the_wrapper_forms_all_reach_the_file_with_their_component_and_session() {
        let _session = session_test_lock();
        set_current_session(None);
        let events = capture(|| {
            crate::log_warn!("PLAIN", "plain");
            crate::log_warn!(component: "guided", "WITH_COMPONENT", format!("n={}", 2), count = 2_u64);
            crate::log_warn!(session: "explicit", "WITH_SESSION", "s");
            let _scope = session_scope("scoped");
            crate::log_warn!("FROM_SCOPE", "s");
        });
        let codes: Vec<&str> = events.iter().map(|event| event.code.as_str()).collect();
        assert_eq!(codes, ["PLAIN", "WITH_COMPONENT", "WITH_SESSION", "FROM_SCOPE"]);
        assert_eq!(events[0].component, "core");
        assert_eq!(events[0].session_id, None);
        assert_eq!(events[1].component, "guided");
        assert_eq!(events[1].msg, "n=2");
        assert_eq!(events[1].fields["count"], Value::from(2));
        assert_eq!(events[2].session_id.as_deref(), Some("explicit"));
        assert_eq!(events[3].session_id.as_deref(), Some("scoped"));
        assert_eq!(super::current_session(), None, "the scope hands the previous session back");
    }

    #[test]
    fn a_long_message_is_cut_to_the_schema_limit() {
        let _session = session_test_lock();
        let events = capture(|| crate::log_warn!("LONG", "x".repeat(400)));
        assert_eq!(events[0].msg.chars().count(), 256);
    }

    #[test]
    fn development_starts_at_debug_and_production_at_info() {
        assert_eq!(base_level(BuildKind::Development, None), (LogLevel::Debug, false));
        assert_eq!(base_level(BuildKind::Production, None), (LogLevel::Info, false));
        assert_eq!(base_level(BuildKind::Development, Some("trace")), (LogLevel::Trace, false));
        assert_eq!(base_level(BuildKind::Development, Some(" WARN ")), (LogLevel::Warn, false));
        assert_eq!(base_level(BuildKind::Development, Some("loud")), (LogLevel::Debug, true));
        assert_eq!(
            base_level(BuildKind::Production, Some("trace")),
            (LogLevel::Info, false),
            "the installed version ignores any override"
        );
    }

    #[test]
    fn detailed_logging_raises_to_debug_and_never_reaches_trace_by_itself() {
        assert_eq!(effective_level(LogLevel::Info, true), LogLevel::Debug);
        assert_eq!(effective_level(LogLevel::Info, false), LogLevel::Info);
        assert_eq!(effective_level(LogLevel::Warn, true), LogLevel::Debug);
        assert_eq!(effective_level(LogLevel::Trace, true), LogLevel::Trace);
        assert_eq!(effective_level(LogLevel::Debug, false), LogLevel::Debug);
    }

    #[test]
    fn the_control_gates_each_level_by_the_effective_one() {
        let _session = session_test_lock();
        let production = LogControl::new(LogLevel::Info);
        assert!(production.enabled(LogLevel::Info));
        assert!(!production.enabled(LogLevel::Debug));
        production.set_detailed(true);
        assert!(production.enabled(LogLevel::Debug));
        assert!(!production.enabled(LogLevel::Trace));
        let development = LogControl::new(LogLevel::Debug);
        assert!(development.enabled(LogLevel::Debug));
        assert!(!development.enabled(LogLevel::Trace));
    }

    #[test]
    fn switching_detailed_logging_is_itself_logged() {
        let _session = session_test_lock();
        let events = capture(|| {
            let control = LogControl::new(LogLevel::Info);
            control.set_detailed_until(Some("2999-01-01T00:00:00Z".to_owned()));
            control.set_detailed_until(Some("2999-01-02T00:00:00Z".to_owned()));
            control.set_detailed_until(None);
        });
        let codes: Vec<&str> = events.iter().map(|event| event.code.as_str()).collect();
        assert_eq!(codes, ["LOG_DETAILED_ENABLED", "LOG_DETAILED_DISABLED"]);
    }

    #[test]
    fn dropped_lines_are_reported_once_per_increase() {
        assert!(dropped_events_entry(0, 0).is_none());
        let event = dropped_events_entry(2, 7).unwrap_or_else(|| panic!("5 new drops"));
        assert_eq!(event.code, "LOG_EVENTS_DROPPED");
        assert_eq!(event.fields["dropped"], Value::from(5));
        assert!(dropped_events_entry(7, 7).is_none());
    }

    fn at(timestamp: &str) -> String {
        let parsed = timestamp
            .parse::<jiff::Timestamp>()
            .unwrap_or_else(|error| panic!("{timestamp}: {error}"));
        format_human(parsed).unwrap_or_else(|error| panic!("{timestamp}: {error}"))
    }

    /// The two instants a naive formatter gets wrong: spring skips 02:00–03:00, autumn repeats
    /// 02:00–03:00, and only the zone abbreviation tells the two 02:xx apart.
    #[test]
    fn formats_the_exact_daylight_saving_transitions() {
        assert_eq!(at("2026-03-29T00:59:59.999Z"), "29/03/2026 01:59:59,999 CET");
        assert_eq!(at("2026-03-29T01:00:00Z"), "29/03/2026 03:00:00,000 CEST");
        assert_eq!(at("2026-10-25T00:59:59.999Z"), "25/10/2026 02:59:59,999 CEST");
        assert_eq!(at("2026-10-25T01:00:00Z"), "25/10/2026 02:00:00,000 CET");
    }

    fn golden_path() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../packages/trace-fixtures/logs/log-events.golden.jsonl")
    }

    fn normalized(line: &str) -> Value {
        let mut value: Value =
            serde_json::from_str(line).unwrap_or_else(|error| panic!("{line}: {error}"));
        let ts = value["ts"].as_str().unwrap_or_default().to_owned();
        assert!(ts.parse::<jiff::Timestamp>().is_ok(), "ts must be an RFC 3339 instant: {ts}");
        value["ts"] = Value::from("2026-01-01T00:00:00Z");
        value
    }

    /// T-LOG-004: the real layer, the real rotating file, read back from disk. Guards every way a
    /// message has been lost before: `%`/`?` forms, a missing code, redaction eating `msg`.
    #[test]
    fn a_real_log_file_keeps_code_message_and_session_and_matches_the_golden() -> std::io::Result<()>
    {
        let _session = session_test_lock();
        set_current_session(None);
        let directory =
            std::env::temp_dir().join(format!("throttlewatch-log-e2e-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        let (subscriber, guard) =
            scoped_file_subscriber(&directory, "throttlewatch.log", LogLevel::Debug, "core")?;
        tracing::subscriber::with_default(subscriber, || {
            crate::log_warn!("E2E_LITERAL", "a literal message");
            crate::log_info!("E2E_DISPLAY", format!("a formatted message about {}", "the pipe"));
            crate::log_warn!(
                "E2E_FIELDS",
                "only allow-listed fields keep their value",
                count = 3_u64,
                user_name = "Ada"
            );
            crate::log_debug!(component: "agent", "E2E_COMPONENT", "an explicit component");
            {
                let _scope = session_scope("session-1234");
                crate::log_info!("E2E_SESSION", "correlated with the current session");
            }
            crate::log_info!(session: "session-explicit", "E2E_EXPLICIT_SESSION", "explicit");
            crate::log_trace!("E2E_TRACE_HIDDEN", "trace is above the debug base level");
            raw_event!(tracing::Level::WARN, "throttlewatch_lib::logging::tests", [
                "code" => "E2E_DEBUG_FORM",
                "msg" => tracing::field::debug("debug formatted"),
            ]);
            raw_event!(tracing::Level::WARN, "throttlewatch_lib::logging::tests", [
                "msg" => "an own event without a code",
            ]);
            raw_event!(tracing::Level::WARN, "some_dependency", ["message" => "third-party noise"]);
        });
        drop(guard);
        let written = fs::read_to_string(directory.join("throttlewatch.log"))?;
        fs::remove_dir_all(&directory)?;
        let actual: Vec<Value> = written.lines().map(normalized).collect();
        let golden_text = fs::read_to_string(golden_path()).unwrap_or_default();
        let golden: Vec<Value> =
            golden_text.lines().filter(|line| !line.trim().is_empty()).map(normalized).collect();
        let rendered: Vec<String> = actual.iter().map(Value::to_string).collect();
        assert_eq!(actual, golden, "actual lines:\n{}", rendered.join("\n"));
        Ok(())
    }

    #[test]
    fn a_percent_formatted_msg_field_lands_in_the_top_level_msg_not_the_redacted_fields() {
        let _session = session_test_lock();
        let captured = Arc::new(Mutex::new(Vec::new()));
        let subscriber = tracing_subscriber::registry().with(Capture(Arc::clone(&captured)));
        tracing::subscriber::with_default(subscriber, || {
            raw_event!(tracing::Level::WARN, "throttlewatch_lib::logging::tests", [
                "component" => "core",
                "code" => "SOMETHING_FAILED",
                "msg" => tracing::field::display(format!("path was {}", "C:\\secret\\place")),
            ]);
        });
        let events = captured.lock().unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].msg, "path was C:\\secret\\place");
        assert!(events[0].fields.get("msg").is_none());
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
        let _session = session_test_lock();
        let control = LogControl::new(LogLevel::Info);
        assert!(!control.is_detailed());
        control.set_detailed(true);
        assert!(control.is_detailed());
        control.set_detailed(false);
        assert!(!control.is_detailed());
    }

    #[test]
    fn detailed_logging_expires_while_the_process_remains_open() {
        let _session = session_test_lock();
        let control = LogControl::new(LogLevel::Info);
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
