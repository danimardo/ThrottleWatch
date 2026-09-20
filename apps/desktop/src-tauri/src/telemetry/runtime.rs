//! Runs the collector for the whole life of the application: launches the sidecar, speaks the NDJSON
//! protocol of `contracts/ipc-protocol.md` (hello → capabilities → start → samples), feeds the
//! [`LiveState`], and restarts the process with backoff when it dies, stalls or misbehaves.
//!
//! The protocol client only knows a [`CollectorLink`] (send a line, receive a line with a timeout),
//! so it is tested without processes; [`ProcessLink`] is the real transport over the child's stdio.
//! Every number that decides behavior (interval, stall window, invalid-message limit, restart
//! policy) comes from `ruleset-v1`.
#![deny(clippy::unwrap_used, clippy::expect_used)]

use super::live::{CollectorState, LiveState};
use crate::diagnostics::Ruleset;
use crate::ipc::protocol::{MAX_MESSAGE_BYTES, PROTOCOL_VERSION, validate_message};
use crate::ipc::supervisor::{RestartPolicy, spawn_verified};
use serde_json::{Value, json};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The runtime asks for per-core detail: coverage level B needs load per core.
const DETAIL: &str = "per_core";
/// How often a waiting session looks at the stop flag.
const STOP_POLL: Duration = Duration::from_millis(200);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    Collector,
    Catalog,
    Sample,
}

/// Told after every change of the live state (the glue emits the Tauri events from here).
pub trait LiveObserver: Send + Sync {
    fn updated(&self, change: Change, live: &LiveState);
}

#[derive(Debug, PartialEq, Eq)]
pub enum Received {
    Line(String),
    Timeout,
    Eof,
}

pub trait CollectorLink: Send {
    fn send(&mut self, line: &str) -> io::Result<()>;
    fn receive(&mut self, timeout: Duration) -> Received;
}

pub trait CollectorLauncher: Send {
    fn launch(&mut self) -> io::Result<Box<dyn CollectorLink>>;
}

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub interval: Duration,
    pub stall_intervals: u32,
    pub max_invalid: u32,
    pub handshake_timeout: Duration,
    pub restart: RestartPolicy,
}

impl RuntimeConfig {
    pub fn from_ruleset(rules: &Ruleset) -> Option<Self> {
        let interval_ms = rules.parameter("sampling.interval_normal_ms")?;
        let stall = rules.parameter("collector.stall_intervals")?;
        let max_invalid = rules.parameter("collector.max_invalid_messages")?;
        let max_restarts = rules.parameter("collector.max_restarts")?;
        let window_min = rules.parameter("collector.restart_window_min")?;
        let interval = Duration::from_millis(interval_ms as u64);
        Some(Self {
            interval,
            stall_intervals: stall as u32,
            max_invalid: max_invalid as u32,
            // The sidecar must answer `hello` and publish its catalog within the stall window of the
            // slowest interval it may be asked for.
            handshake_timeout: interval.saturating_mul(stall as u32).max(Duration::from_secs(10)),
            restart: RestartPolicy {
                max_restarts: max_restarts as usize,
                window: Duration::from_secs((window_min as u64).saturating_mul(60)),
                ..RestartPolicy::default()
            },
        })
    }

    fn stall_window(&self) -> Duration {
        self.interval.saturating_mul(self.stall_intervals)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum SessionEnd {
    /// The process closed stdout or stopped answering the pipe.
    Eof,
    /// No sample for `stall_intervals` intervals (or no catalog after the handshake window).
    Stalled,
    TooManyInvalid,
    /// The sidecar reported a fatal error.
    Fatal,
    /// The application asked to stop.
    Stopped,
}

fn lock(live: &Mutex<LiveState>) -> MutexGuard<'_, LiveState> {
    live.lock().unwrap_or_else(PoisonError::into_inner)
}

fn epoch_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |elapsed| elapsed.as_millis() as u64)
}

fn envelope(nonce: &str, sequence: u64, message_type: &str, payload: &Value) -> String {
    json!({
        "protocol_version": PROTOCOL_VERSION,
        "session_nonce": nonce,
        "sequence": sequence,
        "timestamp_utc": jiff::Timestamp::now().to_string(),
        "type": message_type,
        "payload": payload,
    })
    .to_string()
}

struct Outbox<'a> {
    link: &'a mut dyn CollectorLink,
    nonce: &'a str,
    sequence: u64,
}

impl Outbox<'_> {
    fn send(&mut self, message_type: &str, payload: &Value) -> io::Result<()> {
        let line = envelope(self.nonce, self.sequence, message_type, payload);
        self.sequence += 1;
        self.link.send(&line)
    }
}

/// One connection to the collector, from `hello` until the process ends or the session is abandoned.
pub fn run_session(
    link: &mut dyn CollectorLink,
    nonce: &str,
    live: &Mutex<LiveState>,
    observer: &dyn LiveObserver,
    config: &RuntimeConfig,
    stop: &AtomicBool,
) -> SessionEnd {
    let mut outbox = Outbox { link, nonce, sequence: 0 };
    let hello = json!({"app_version": env!("CARGO_PKG_VERSION"), "supported_protocols": [PROTOCOL_VERSION]});
    if outbox.send("hello", &hello).is_err() {
        return SessionEnd::Eof;
    }

    let mut previous: Option<u64> = None;
    let mut invalid = 0_u32;
    let mut sent_start = false;
    let mut started = false;
    let mut last_progress = Instant::now();

    loop {
        if stop.load(Ordering::SeqCst) {
            // Best effort: the process is killed when the link is dropped anyway.
            let _ = outbox.send("stop", &json!({}));
            let _ = outbox.send("shutdown", &json!({}));
            return SessionEnd::Stopped;
        }
        let limit = if started { config.stall_window() } else { config.handshake_timeout };
        match outbox.link.receive(STOP_POLL.min(limit)) {
            Received::Eof => return SessionEnd::Eof,
            Received::Timeout => {
                if last_progress.elapsed() >= limit {
                    return SessionEnd::Stalled;
                }
            }
            Received::Line(line) => {
                let validated = validate_message(line.as_bytes(), nonce, previous);
                let Ok(envelope) = validated else {
                    invalid += 1;
                    if invalid >= config.max_invalid {
                        return SessionEnd::TooManyInvalid;
                    }
                    continue;
                };
                previous = Some(envelope.sequence);
                let payload = serde_json::from_str::<Value>(&line)
                    .ok()
                    .and_then(|value| value.get("payload").cloned())
                    .unwrap_or(Value::Null);
                let handled = handle_message(
                    &envelope.message_type,
                    &payload,
                    live,
                    observer,
                    config,
                    &mut outbox,
                    &mut sent_start,
                    &mut started,
                );
                match handled {
                    Handled::Progress => {
                        invalid = 0;
                        last_progress = Instant::now();
                    }
                    Handled::Ignored => invalid = 0,
                    Handled::Invalid => {
                        invalid += 1;
                        if invalid >= config.max_invalid {
                            return SessionEnd::TooManyInvalid;
                        }
                    }
                    Handled::Fatal => return SessionEnd::Fatal,
                    Handled::PeerClosed => return SessionEnd::Eof,
                }
            }
        }
    }
}

enum Handled {
    Progress,
    Ignored,
    Invalid,
    Fatal,
    PeerClosed,
}

#[allow(clippy::too_many_arguments)]
fn handle_message(
    message_type: &str,
    payload: &Value,
    live: &Mutex<LiveState>,
    observer: &dyn LiveObserver,
    config: &RuntimeConfig,
    outbox: &mut Outbox<'_>,
    sent_start: &mut bool,
    started: &mut bool,
) -> Handled {
    match message_type {
        "hello_ack" => Handled::Progress,
        "capabilities" => {
            let mut guard = lock(live);
            if guard.apply_capabilities(payload).is_err() {
                return Handled::Invalid;
            }
            observer.updated(Change::Catalog, &guard);
            drop(guard);
            if !*sent_start {
                let start =
                    json!({"interval_ms": config.interval.as_millis() as u64, "detail": DETAIL});
                if outbox.send("start", &start).is_err() {
                    return Handled::PeerClosed;
                }
                *sent_start = true;
            }
            Handled::Progress
        }
        "started" => {
            *started = true;
            let mut guard = lock(live);
            guard.set_collector(CollectorState::Running, 0, None);
            observer.updated(Change::Collector, &guard);
            Handled::Progress
        }
        "sample" => {
            let mut guard = lock(live);
            if guard.apply_sample(payload, epoch_ms()).is_err() {
                return Handled::Invalid;
            }
            observer.updated(Change::Sample, &guard);
            Handled::Progress
        }
        "error" => match payload.get("severity").and_then(Value::as_str) {
            Some("fatal") => Handled::Fatal,
            Some("recoverable") => {
                let key = payload.get("message_key").and_then(Value::as_str).map(str::to_owned);
                let mut guard = lock(live);
                let attempt = guard.attempt();
                guard.set_collector(CollectorState::Degraded, attempt, key);
                observer.updated(Change::Collector, &guard);
                Handled::Ignored
            }
            _ => Handled::Ignored,
        },
        "stopped" => Handled::PeerClosed,
        _ => Handled::Ignored,
    }
}

fn session_nonce() -> String {
    use std::hash::{BuildHasher, Hasher, RandomState};
    let nanos =
        SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |elapsed| elapsed.as_nanos() as u64);
    (0..2)
        .map(|salt| {
            let mut hasher = RandomState::new().build_hasher();
            hasher.write_u64(nanos ^ salt);
            format!("{:016x}", hasher.finish())
        })
        .collect()
}

/// Launch → session → restart with backoff, until the application stops the runtime or the restart
/// budget of the window is spent.
pub fn run_forever(
    launcher: &mut dyn CollectorLauncher,
    live: &Mutex<LiveState>,
    observer: &dyn LiveObserver,
    config: &RuntimeConfig,
    stop: &AtomicBool,
) {
    let mut attempts: Vec<Instant> = Vec::new();
    let mut restart_number = 0_u32;
    while !stop.load(Ordering::SeqCst) {
        let state =
            if restart_number == 0 { CollectorState::Starting } else { CollectorState::Restarting };
        set_state(live, observer, state, restart_number, None);
        let end = match launcher.launch() {
            Ok(mut link) => {
                run_session(link.as_mut(), &session_nonce(), live, observer, config, stop)
            }
            Err(_) => {
                // A missing, altered or unstartable sidecar is not transient: retrying changes nothing.
                set_state(
                    live,
                    observer,
                    CollectorState::Failed,
                    restart_number,
                    Some("collector.launch_failed"),
                );
                return;
            }
        };
        if end == SessionEnd::Stopped || stop.load(Ordering::SeqCst) {
            break;
        }
        if !config.restart.can_restart(&mut attempts, Instant::now()) {
            set_state(
                live,
                observer,
                CollectorState::Failed,
                restart_number,
                Some("collector.restart_limit"),
            );
            return;
        }
        set_state(
            live,
            observer,
            CollectorState::Restarting,
            restart_number + 1,
            Some("collector.restarting"),
        );
        sleep_unless_stopped(config.restart.backoff(restart_number as usize), stop);
        restart_number += 1;
    }
    set_state(live, observer, CollectorState::Stopped, restart_number, None);
}

fn set_state(
    live: &Mutex<LiveState>,
    observer: &dyn LiveObserver,
    state: CollectorState,
    attempt: u32,
    message_key: Option<&str>,
) {
    let mut guard = lock(live);
    guard.set_collector(state, attempt, message_key.map(str::to_owned));
    observer.updated(Change::Collector, &guard);
}

fn sleep_unless_stopped(total: Duration, stop: &AtomicBool) {
    let deadline = Instant::now() + total;
    while Instant::now() < deadline && !stop.load(Ordering::SeqCst) {
        thread::sleep(STOP_POLL.min(deadline.saturating_duration_since(Instant::now())));
    }
}

/// The real transport: the child's stdin for commands, a reader thread for stdout, a drain thread for
/// stderr. Dropping it kills the process.
pub struct ProcessLink {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: mpsc::Receiver<Option<String>>,
}

impl ProcessLink {
    pub fn from_child(mut child: Child) -> io::Result<Self> {
        let stdin = child.stdin.take();
        let stdout =
            child.stdout.take().ok_or_else(|| io::Error::other("sidecar stdout is unavailable"))?;
        let stderr = child.stderr.take();
        let (sender, lines) = mpsc::channel();
        thread::Builder::new().name("collector-stdout".to_owned()).spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut line = String::new();
                // One byte over the limit tells an oversized message from a full one.
                let read = reader.by_ref().take(MAX_MESSAGE_BYTES as u64 + 1).read_line(&mut line);
                match read {
                    Ok(0) | Err(_) => break,
                    Ok(_) if line.len() > MAX_MESSAGE_BYTES => break, // desynchronized: end the session
                    Ok(_) => {
                        if sender
                            .send(Some(line.trim_end_matches(['\r', '\n']).to_owned()))
                            .is_err()
                        {
                            return;
                        }
                    }
                }
            }
            let _ = sender.send(None);
        })?;
        if let Some(stderr) = stderr {
            thread::Builder::new().name("collector-stderr".to_owned()).spawn(move || {
                for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                    match crate::logging::validate_collector_stderr(&line) {
                        Ok(event) => match event.level.as_str() {
                            "error" => tracing::error!(component = "agent", code = %event.code, msg = %event.msg),
                            "warn" => tracing::warn!(component = "agent", code = %event.code, msg = %event.msg),
                            _ => tracing::debug!(component = "agent", code = %event.code, msg = %event.msg),
                        },
                        Err(_) => tracing::warn!(component = "core", code = "COLLECTOR_STDERR_INVALID", msg = "collector stderr is outside the contract"),
                    }
                }
            })?;
        }
        Ok(Self { child, stdin, lines })
    }
}

impl CollectorLink for ProcessLink {
    fn send(&mut self, line: &str) -> io::Result<()> {
        let stdin =
            self.stdin.as_mut().ok_or_else(|| io::Error::from(io::ErrorKind::BrokenPipe))?;
        stdin.write_all(line.as_bytes())?;
        stdin.write_all(b"\n")?;
        stdin.flush()
    }

    fn receive(&mut self, timeout: Duration) -> Received {
        match self.lines.recv_timeout(timeout) {
            Ok(Some(line)) => Received::Line(line),
            Ok(None) | Err(mpsc::RecvTimeoutError::Disconnected) => Received::Eof,
            Err(mpsc::RecvTimeoutError::Timeout) => Received::Timeout,
        }
    }
}

impl Drop for ProcessLink {
    fn drop(&mut self) {
        self.stdin.take(); // EOF on stdin is the sidecar's orderly shutdown
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The shipped sidecar: started only if its SHA-256 equals the one the signed release manifest lists.
pub struct SidecarLauncher {
    pub executable: PathBuf,
    pub expected_sha256: String,
}

impl CollectorLauncher for SidecarLauncher {
    fn launch(&mut self) -> io::Result<Box<dyn CollectorLink>> {
        let child = spawn_verified(&self.executable, &self.expected_sha256)?;
        Ok(Box::new(ProcessLink::from_child(child)?))
    }
}

/// Any program that speaks the protocol (the fake collector of the `e2e` build). Not for release builds.
pub struct CommandLauncher {
    pub program: PathBuf,
    pub args: Vec<String>,
}

impl CollectorLauncher for CommandLauncher {
    fn launch(&mut self) -> io::Result<Box<dyn CollectorLink>> {
        let child = Command::new(&self.program)
            .args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        Ok(Box::new(ProcessLink::from_child(child)?))
    }
}

/// Owns the thread that runs the collector; stopping it ends the session and kills the process.
pub struct CollectorRuntime {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl CollectorRuntime {
    pub fn start(
        mut launcher: Box<dyn CollectorLauncher>,
        live: Arc<Mutex<LiveState>>,
        observer: Arc<dyn LiveObserver>,
        config: RuntimeConfig,
    ) -> io::Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let handle =
            thread::Builder::new().name("collector-runtime".to_owned()).spawn(move || {
                run_forever(launcher.as_mut(), &live, observer.as_ref(), &config, &thread_stop);
            })?;
        Ok(Self { stop, handle: Some(handle) })
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for CollectorRuntime {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    const NONCE: &str = "test-nonce";

    #[derive(Default)]
    struct Recorder {
        changes: Mutex<Vec<Change>>,
    }

    impl LiveObserver for Recorder {
        fn updated(&self, change: Change, _live: &LiveState) {
            lock_vec(&self.changes).push(change);
        }
    }

    fn lock_vec<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
        mutex.lock().unwrap_or_else(PoisonError::into_inner)
    }

    struct Scripted {
        inbound: VecDeque<Received>,
        sent: Vec<Value>,
        on_empty: fn() -> Received,
    }

    impl Scripted {
        fn new(messages: Vec<Received>, on_empty: fn() -> Received) -> Self {
            Self { inbound: messages.into(), sent: Vec::new(), on_empty }
        }
    }

    impl CollectorLink for Scripted {
        fn send(&mut self, line: &str) -> io::Result<()> {
            self.sent.push(serde_json::from_str(line).map_err(io::Error::other)?);
            Ok(())
        }

        fn receive(&mut self, timeout: Duration) -> Received {
            match self.inbound.pop_front() {
                Some(Received::Timeout) => {
                    thread::sleep(timeout);
                    Received::Timeout
                }
                Some(other) => other,
                None => {
                    let next = (self.on_empty)();
                    if next == Received::Timeout {
                        thread::sleep(timeout);
                    }
                    next
                }
            }
        }
    }

    fn config() -> RuntimeConfig {
        RuntimeConfig {
            interval: Duration::from_millis(20),
            stall_intervals: 3,
            max_invalid: 3,
            handshake_timeout: Duration::from_millis(400),
            restart: RestartPolicy {
                max_restarts: 2,
                window: Duration::from_secs(60),
                initial_backoff: Duration::from_millis(1),
                max_backoff: Duration::from_millis(2),
            },
        }
    }

    fn message(sequence: u64, message_type: &str, payload: Value) -> Received {
        Received::Line(envelope(NONCE, sequence, message_type, &payload))
    }

    fn capabilities() -> Value {
        json!({
            "cpu": {"vendor": "amd", "display_name": "Test CPU", "logical_processors": 4, "hybrid": false, "virtualized": false},
            "groups": [{"id": "all", "kind": "homogeneous", "logical_count": 4}],
            "sensors": [
                {"id": "cpu.package.load", "source_id": "l", "source_name": "CPU Total", "metric": "load", "scope": "package", "unit": "percent", "quality": "direct"},
                {"id": "cpu.core.1.load", "source_id": "c", "source_name": "CPU Core #1", "metric": "load", "scope": "core", "scope_ref": "1", "unit": "percent", "quality": "direct"}
            ]
        })
    }

    fn sample(load: f64) -> Value {
        json!({"monotonic_ms": 1, "duration_ms": 1, "values": [
            {"sensor_id": "cpu.package.load", "number": load, "status": "ok"},
            {"sensor_id": "cpu.core.1.load", "number": load, "status": "ok"}
        ]})
    }

    fn good_start() -> Vec<Received> {
        vec![
            message(
                0,
                "hello_ack",
                json!({"agent_version": "t", "selected_protocol": 1, "runtime": ".NET", "low_level_access": {"state": "missing"}}),
            ),
            message(1, "capabilities", capabilities()),
            message(2, "started", json!({"interval_ms": 20, "detail": "per_core"})),
        ]
    }

    fn run(link: &mut Scripted, stop: &AtomicBool) -> (SessionEnd, Mutex<LiveState>, Recorder) {
        let live = Mutex::new(LiveState::new());
        let observer = Recorder::default();
        let end = run_session(link, NONCE, &live, &observer, &config(), stop);
        (end, live, observer)
    }

    #[test]
    fn handshake_catalog_start_and_samples_feed_the_live_state() {
        let mut inbound = good_start();
        inbound.push(message(3, "sample", sample(40.0)));
        inbound.push(message(4, "sample", sample(60.0)));
        inbound.push(Received::Eof);
        let mut link = Scripted::new(inbound, || Received::Eof);

        let (end, live, observer) = run(&mut link, &AtomicBool::new(false));

        assert_eq!(end, SessionEnd::Eof);
        let state = lock(&live);
        assert_eq!(state.collector(), CollectorState::Running);
        assert_eq!(state.cpu().map(|cpu| cpu.display_name.as_str()), Some("Test CPU"));
        assert_eq!(state.snapshot_input().and_then(|input| input.load_percent), Some(60.0));
        assert_eq!(state.cores().len(), 1);
        // hello first, then `start` asking for per-core detail at the ruleset interval
        assert_eq!(link.sent[0]["type"], "hello");
        assert_eq!(link.sent[1]["type"], "start");
        assert_eq!(link.sent[1]["payload"]["detail"], "per_core");
        assert_eq!(link.sent[1]["payload"]["interval_ms"], 20);
        assert_eq!(link.sent[1]["sequence"], 1);
        let changes = lock_vec(&observer.changes);
        assert!(changes.contains(&Change::Catalog) && changes.contains(&Change::Sample));
    }

    #[test]
    fn a_silent_collector_is_declared_stalled_after_the_stall_window() {
        let mut inbound = good_start();
        inbound.push(Received::Timeout);
        let mut link = Scripted::new(inbound, || Received::Timeout);

        let (end, _live, _observer) = run(&mut link, &AtomicBool::new(false));

        assert_eq!(end, SessionEnd::Stalled);
    }

    #[test]
    fn no_catalog_within_the_handshake_window_is_a_stall() {
        let mut link = Scripted::new(vec![], || Received::Timeout);

        let (end, live, _observer) = run(&mut link, &AtomicBool::new(false));

        assert_eq!(end, SessionEnd::Stalled);
        assert!(!lock(&live).has_data());
    }

    #[test]
    fn three_consecutive_invalid_messages_end_the_session_without_touching_the_state() {
        let mut inbound = good_start();
        inbound.push(Received::Line("not json".to_owned()));
        inbound.push(Received::Line(envelope("other-nonce", 3, "sample", &sample(1.0))));
        inbound.push(message(9, "sample", sample(1.0))); // sequence gap
        let mut link = Scripted::new(inbound, || Received::Timeout);

        let (end, live, _observer) = run(&mut link, &AtomicBool::new(false));

        assert_eq!(end, SessionEnd::TooManyInvalid);
        assert!(lock(&live).snapshot_input().is_none());
    }

    #[test]
    fn a_valid_message_resets_the_invalid_counter() {
        let mut inbound = good_start();
        inbound.push(Received::Line("garbage".to_owned()));
        inbound.push(Received::Line("garbage".to_owned()));
        inbound.push(message(3, "sample", sample(5.0)));
        inbound.push(Received::Line("garbage".to_owned()));
        inbound.push(Received::Eof);
        let mut link = Scripted::new(inbound, || Received::Eof);

        let (end, live, _observer) = run(&mut link, &AtomicBool::new(false));

        assert_eq!(end, SessionEnd::Eof);
        assert_eq!(lock(&live).snapshot_input().and_then(|input| input.load_percent), Some(5.0));
    }

    #[test]
    fn a_fatal_error_from_the_sidecar_ends_the_session_and_a_recoverable_one_degrades() {
        let mut inbound = good_start();
        inbound.push(message(3, "error", json!({"code": "SENSOR_ENUMERATION_FAILED", "severity": "recoverable", "message_key": "collector.sensor_enumeration_failed"})));
        inbound.push(message(
            4,
            "error",
            json!({"code": "BOOM", "severity": "fatal", "message_key": "collector.fatal"}),
        ));
        let mut link = Scripted::new(inbound, || Received::Timeout);

        let (end, live, _observer) = run(&mut link, &AtomicBool::new(false));

        assert_eq!(end, SessionEnd::Fatal);
        assert_eq!(lock(&live).collector(), CollectorState::Degraded);
        assert_eq!(lock(&live).message_key(), Some("collector.sensor_enumeration_failed"));
    }

    #[test]
    fn the_stop_flag_asks_the_sidecar_to_stop_and_ends_the_session() {
        let mut link = Scripted::new(good_start(), || Received::Timeout);
        let stop = AtomicBool::new(true);

        let (end, _live, _observer) = run(&mut link, &stop);

        assert_eq!(end, SessionEnd::Stopped);
        let types: Vec<&str> =
            link.sent.iter().filter_map(|message| message["type"].as_str()).collect();
        assert_eq!(types, vec!["hello", "stop", "shutdown"]);
    }

    struct Launcher {
        links: VecDeque<io::Result<Box<dyn CollectorLink>>>,
        launched: usize,
    }

    impl CollectorLauncher for Launcher {
        fn launch(&mut self) -> io::Result<Box<dyn CollectorLink>> {
            self.launched += 1;
            self.links.pop_front().unwrap_or_else(|| Err(io::Error::other("no more links")))
        }
    }

    fn dying_link() -> io::Result<Box<dyn CollectorLink>> {
        Ok(Box::new(Scripted::new(vec![Received::Eof], || Received::Eof)))
    }

    fn drive(launcher: &mut Launcher) -> (Mutex<LiveState>, Recorder) {
        let live = Mutex::new(LiveState::new());
        let observer = Recorder::default();
        run_forever(launcher, &live, &observer, &config(), &AtomicBool::new(false));
        (live, observer)
    }

    #[test]
    fn a_dying_collector_is_restarted_within_the_budget_and_then_declared_failed() {
        let mut launcher = Launcher { links: (0..10).map(|_| dying_link()).collect(), launched: 0 };

        let (live, observer) = drive(&mut launcher);

        // first launch + max_restarts (2) restarts
        assert_eq!(launcher.launched, 3);
        let state = lock(&live);
        assert_eq!(state.collector(), CollectorState::Failed);
        assert_eq!(state.message_key(), Some("collector.restart_limit"));
        let restarting = lock_vec(&observer.changes)
            .iter()
            .filter(|change| **change == Change::Collector)
            .count();
        assert!(restarting >= 5);
    }

    #[test]
    fn a_launch_failure_is_final_and_reported_without_retrying() {
        let mut launcher = Launcher {
            links: VecDeque::from([Err(io::Error::from(io::ErrorKind::NotFound))]),
            launched: 0,
        };

        let (live, _observer) = drive(&mut launcher);

        assert_eq!(launcher.launched, 1);
        assert_eq!(lock(&live).collector(), CollectorState::Failed);
        assert_eq!(lock(&live).message_key(), Some("collector.launch_failed"));
    }

    #[test]
    fn the_config_reads_every_number_from_the_ruleset() {
        let rules = Ruleset::v1().unwrap_or_else(|error| panic!("ruleset: {error}"));

        let config = RuntimeConfig::from_ruleset(&rules).unwrap_or_else(|| panic!("config"));

        assert_eq!(config.interval, Duration::from_millis(1000));
        assert_eq!(config.stall_intervals, 3);
        assert_eq!(config.max_invalid, 3);
        assert_eq!(config.restart.max_restarts, 3);
        assert_eq!(config.restart.window, Duration::from_secs(600));
    }
}
