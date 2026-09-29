use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Child;
use std::thread;
use std::time::Duration;

pub const ELEVATED_TASK_NAME: &str = "ThrottleWatch\\SidecarElevated";
pub const ELEVATED_PIPE_PREFIX: &str = r"\\.\pipe\ThrottleWatch.ElevatedSidecar.";
const FILE_FLAG_FIRST_PIPE_INSTANCE: u32 = 0x0008_0000;
#[cfg(windows)]
use super::supervisor::hidden_command;
const RELEASE_MANIFEST_NAME: &str = "release-manifest.json";
const RELEASE_SIGNATURE_NAME: &str = "release-manifest.json.minisig";

/// Prefixes a sidecar stderr line forwarded over the control pipe so [`super::runtime::PipeLink`]
/// can tell it apart from a protocol line without a second pipe: a NDJSON protocol line always
/// starts with `{`, never with a NUL byte.
pub const STDERR_MARKER: &str = "\u{0}stderr\u{0}";

/// One protocol line the local reader accepted; caps how much a single desynchronized line can
/// grow before the forwarder gives up, the same limit [`crate::ipc::protocol::MAX_MESSAGE_BYTES`]
/// enforces on the unelevated path.
const MAX_LINE_BYTES: u64 = crate::ipc::protocol::MAX_MESSAGE_BYTES as u64;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ElevatedLaunchPayload {
    pub sidecar_path: PathBuf,
    pub sidecar_sha256: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct EmptyPayload {}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
enum ElevatedCommand {
    StartSession(ElevatedLaunchPayload),
    Heartbeat(EmptyPayload),
    RecheckCoverage(EmptyPayload),
    StopSession(EmptyPayload),
}

fn parse_command(value: &serde_json::Value) -> Result<ElevatedCommand, String> {
    let message_type = value
        .get("type")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "missing elevated command".to_owned())?;
    let payload = value.get("payload").cloned().unwrap_or_else(|| serde_json::json!({}));
    match message_type {
        "elevated_start" => serde_json::from_value(payload)
            .map(ElevatedCommand::StartSession)
            .map_err(|_| "invalid launch payload".to_owned()),
        "elevated_heartbeat" => serde_json::from_value(payload)
            .map(ElevatedCommand::Heartbeat)
            .map_err(|_| "invalid heartbeat payload".to_owned()),
        "elevated_recheck_coverage" => serde_json::from_value(payload)
            .map(ElevatedCommand::RecheckCoverage)
            .map_err(|_| "invalid coverage payload".to_owned()),
        "elevated_stop_session" => serde_json::from_value(payload)
            .map(ElevatedCommand::StopSession)
            .map_err(|_| "invalid stop payload".to_owned()),
        _ => Err("unknown elevated command".to_owned()),
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ElevatedLaunchRequest {
    pub protocol_version: u64,
    pub session_nonce: String,
    pub sequence: u64,
    pub timestamp_utc: String,
    pub message_type: &'static str,
    pub payload: ElevatedLaunchPayload,
}

pub fn validate_launch_request(
    raw: &[u8],
    expected_nonce: &str,
    previous_sequence: Option<u64>,
    expected_path: &Path,
    expected_sha256: &str,
) -> Result<ElevatedLaunchRequest, String> {
    let envelope = crate::ipc::protocol::validate_message(raw, expected_nonce, previous_sequence)
        .map_err(|error| error.to_string())?;
    if envelope.message_type != "elevated_start" {
        return Err("unexpected elevated launcher message".to_owned());
    }
    let value: serde_json::Value =
        serde_json::from_slice(raw).map_err(|_| "invalid JSON".to_owned())?;
    let ElevatedCommand::StartSession(parsed) = parse_command(&value)? else {
        return Err("only StartSession is valid during launcher handshake".to_owned());
    };
    if parsed.sidecar_path != expected_path
        || !parsed.sidecar_sha256.eq_ignore_ascii_case(expected_sha256)
    {
        return Err("sidecar path or hash does not match the installed manifest".to_owned());
    }
    let timestamp_utc = value
        .get("timestamp_utc")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "missing timestamp".to_owned())?
        .to_owned();
    Ok(ElevatedLaunchRequest {
        protocol_version: crate::ipc::protocol::PROTOCOL_VERSION,
        session_nonce: expected_nonce.to_owned(),
        sequence: envelope.sequence,
        timestamp_utc,
        message_type: "elevated_start",
        payload: parsed,
    })
}

pub fn run_registered_task() -> io::Result<()> {
    #[cfg(windows)]
    {
        let status =
            hidden_command("schtasks.exe").args(["/Run", "/TN", ELEVATED_TASK_NAME]).status()?;
        if status.success() {
            return Ok(());
        }
        Err(io::Error::new(io::ErrorKind::PermissionDenied, "elevated task start failed"))
    }

    #[cfg(not(windows))]
    {
        Err(io::Error::new(io::ErrorKind::Unsupported, "scheduled tasks are Windows-only"))
    }
}

/// Starts the registered launcher and keeps the authenticated control pipe
/// alive for the lifetime of the elevated sidecar.
/// Two named pipes, not one duplex pipe: a single synchronous Windows handle (or a
/// [`std::fs::File::try_clone`] of it) must never have a read pending on one thread while another
/// thread issues a write on it — that combination serializes, and in practice the write blocks
/// until the unrelated read resolves. Each pipe here therefore has exactly one reader and one
/// writer for its whole life: `to_launcher` carries the caller's commands and is read by
/// [`proxy_child_over_pipe`]'s own thread alone; `from_launcher` carries the sidecar's stdout
/// (and marker-prefixed stderr) and is written by that same function's dedicated writer thread
/// alone (`proxies_a_real_child_both_ways_over_two_pipes_and_stops_it_when_the_caller_disconnects`
/// exercises the whole thing against a real pipe pair and a real child process).
#[cfg(windows)]
fn create_listening_pipe(name: &str) -> io::Result<*mut std::ffi::c_void> {
    let (pipe_handle, descriptor) = create_secure_pipe(name)?;
    // SAFETY: `descriptor` was allocated by the Windows security-descriptor API and is released
    // exactly once after the pipe has copied the pointer into its attributes.
    unsafe { free_security_descriptor(descriptor) };
    Ok(pipe_handle)
}

/// Blocks until a client connects to `pipe_handle` (created moments ago by
/// [`create_listening_pipe`], so a fast client may already have — `ERROR_PIPE_CONNECTED` is that
/// case, not a failure) and hands back the owned handle.
#[cfg(windows)]
fn finish_connect(pipe_handle: *mut std::ffi::c_void) -> io::Result<std::fs::File> {
    use std::os::windows::io::FromRawHandle;

    // SAFETY: `pipe_handle` is the valid synchronous server handle `create_listening_pipe` just
    // returned and a null overlapped pointer requests the documented blocking connection mode.
    let connected = unsafe { connect_named_pipe(pipe_handle, std::ptr::null_mut()) } != 0;
    // SAFETY: GetLastError is read immediately after the failed ConnectNamedPipe call.
    if !connected && unsafe { last_error() } != ERROR_PIPE_CONNECTED {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `pipe_handle` is a valid, owned handle returned by CreateNamedPipeW and the File
    // takes ownership so it closes the handle exactly once.
    Ok(unsafe { std::fs::File::from_raw_handle(pipe_handle as _) })
}

/// How long each pipe waits for the elevated launcher to connect to it, before giving up rather
/// than blocking the whole collector runtime thread forever (see the call site's comment for the
/// real failure this guards against).
///
/// 20 s, not 8 (2026-09-28, three real restarts logged with matching timestamps on both sides —
/// `throttlewatch.log` and `throttlewatch-launcher.log`): a restart mid-session (the collector's
/// own, or "Reparar acceso avanzado") abandons the previous elevated launcher's pipes, and that
/// process takes some 14-15 s on this machine to notice, kill its sidecar, and exit — apparently
/// serialized with starting the next instance of the same interactive scheduled task regardless of
/// `MultipleInstancesPolicy`. At 8 s the new launcher was still starting when this side gave up, so
/// the previous restart it *did* manage to start ended up talking to already-abandoned pipes and
/// exited immediately with "invalid launcher hello" — a second, self-inflicted failure on top of
/// the first. 20 s comfortably covers the observed ~15 s with margin, at the cost of a slower
/// fallback to unelevated on a launcher that is genuinely never going to answer.
#[cfg(windows)]
const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);

/// A Windows `HANDLE` has no thread affinity; moving the raw value into another thread to finish
/// the (possibly forever-blocking) connect there is safe even though `*mut c_void` is not `Send`.
#[cfg(windows)]
struct SendableHandle(*mut std::ffi::c_void);
#[cfg(windows)]
// SAFETY: see the type's doc comment.
unsafe impl Send for SendableHandle {}

/// Runs [`finish_connect`] on its own thread and gives up after `timeout` instead of blocking
/// forever. A timed-out connect is abandoned, not cancelled: the thread (and the handle) may
/// still complete later, or never; either way nothing here waits on it again.
#[cfg(windows)]
fn connect_with_timeout(
    pipe_handle: *mut std::ffi::c_void,
    timeout: Duration,
) -> io::Result<std::fs::File> {
    let handle = SendableHandle(pipe_handle);
    let (sender, receiver) = std::sync::mpsc::channel();
    thread::Builder::new().name("elevated-connect".to_owned()).spawn(move || {
        let handle = handle;
        let _ = sender.send(finish_connect(handle.0));
    })?;
    match receiver.recv_timeout(timeout) {
        Ok(result) => result,
        Err(_) => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "the elevated launcher never connected to the pipe",
        )),
    }
}

fn elevated_start_envelope(
    nonce: &str,
    sequence: u64,
    sidecar_path: &Path,
    sidecar_sha256: &str,
) -> serde_json::Value {
    serde_json::json!({
        "protocol_version": crate::ipc::protocol::PROTOCOL_VERSION,
        "session_nonce": nonce,
        "sequence": sequence,
        "timestamp_utc": format!("{:?}", std::time::SystemTime::now()),
        "type": "elevated_start",
        "payload": {
            "sidecar_path": sidecar_path,
            "sidecar_sha256": sidecar_sha256
        }
    })
}

fn elevated_stop_session_envelope(nonce: &str, sequence: u64) -> serde_json::Value {
    serde_json::json!({
        "protocol_version": crate::ipc::protocol::PROTOCOL_VERSION,
        "session_nonce": nonce,
        "sequence": sequence,
        "timestamp_utc": format!("{:?}", std::time::SystemTime::now()),
        "type": "elevated_stop_session",
        "payload": {}
    })
}

fn write_line(pipe: &mut std::fs::File, value: &serde_json::Value) -> io::Result<()> {
    serde_json::to_writer(&mut *pipe, value).map_err(io::Error::other)?;
    pipe.write_all(b"\n")?;
    pipe.flush()
}

/// Opens the registered task's pipes, starts it, and sends the very first `elevated_start` —
/// exactly what a one-shot elevated launch has always done. Returns the nonce alongside the pipes
/// (2026-09-28) so a caller that wants to keep the connection open ([`PersistentElevatedConnection`])
/// can authenticate further messages on it without generating a second, disagreeing nonce.
pub fn start_registered_sidecar(
    sidecar_path: &Path,
    sidecar_sha256: &str,
) -> io::Result<(std::fs::File, std::fs::File, String)> {
    #[cfg(windows)]
    {
        let base = random_pipe_name()?;
        // Both pipes start listening before the launcher exists to connect to them; only then is
        // it started, and only then do we block waiting for it to open each one.
        let in_handle = create_listening_pipe(&format!("{base}.in"))?;
        let out_handle = create_listening_pipe(&format!("{base}.out"))?;
        run_registered_task()?;
        // `ConnectNamedPipe` with no `OVERLAPPED` blocks forever: if the launcher never reaches
        // this pipe at all — a stale/deregistered scheduled task, a policy blocking Task
        // Scheduler, the launcher rejecting the request and exiting first (as it did once here:
        // it used to require the sidecar to sit beside it rather than in its `collector\`
        // subdirectory) — this must not hang the whole collector runtime thread forever with no
        // fallback. `CONNECT_TIMEOUT` bounds each side of the handshake instead.
        let mut to_launcher = connect_with_timeout(in_handle, CONNECT_TIMEOUT)?;
        let from_launcher = connect_with_timeout(out_handle, CONNECT_TIMEOUT)?;
        let nonce = session_nonce()?;
        let hello = serde_json::json!({
            "protocol_version": crate::ipc::protocol::PROTOCOL_VERSION,
            "session_nonce": nonce,
            "type": "hello"
        });
        write_line(&mut to_launcher, &hello)?;
        write_line(
            &mut to_launcher,
            &elevated_start_envelope(&nonce, 1, sidecar_path, sidecar_sha256),
        )?;
        Ok((to_launcher, from_launcher, nonce))
    }

    #[cfg(not(windows))]
    {
        let _ = (sidecar_path, sidecar_sha256);
        Err(io::Error::new(io::ErrorKind::Unsupported, "named pipes are Windows-only"))
    }
}

/// Keeps one elevated launcher connection alive across collector restarts within the application's
/// own session (ADR-0004 amendment, 2026-09-28) instead of tearing it down and asking Windows for a
/// fresh Task Scheduler activation — and a fresh UAC elevation — every time. Only the two pipe
/// handles and the authentication state (nonce, sequence) persist here; the sidecar child itself is
/// still killed and re-verified from its signed manifest on every single session, cold start or not.
pub struct PersistentElevatedConnection {
    to_launcher: std::fs::File,
    from_launcher: std::fs::File,
    nonce: String,
    sequence: u64,
}

impl PersistentElevatedConnection {
    /// The one-shot path: registers the task, opens fresh pipes, sends the first `elevated_start`.
    /// Same as calling [`start_registered_sidecar`] directly, wrapped so the connection can be kept
    /// — returns clones of the same two handles for the caller's immediate [`super::runtime::PipeLink`],
    /// exactly as [`Self::start_new_session`] does for every later session on this connection.
    pub fn cold_start(
        sidecar_path: &Path,
        sidecar_sha256: &str,
    ) -> io::Result<(Self, std::fs::File, std::fs::File)> {
        let (to_launcher, from_launcher, nonce) =
            start_registered_sidecar(sidecar_path, sidecar_sha256)?;
        let clones = (to_launcher.try_clone()?, from_launcher.try_clone()?);
        Ok((Self { to_launcher, from_launcher, nonce, sequence: 1 }, clones.0, clones.1))
    }

    /// Ends whatever session is running on this connection (`elevated_stop_session`) and starts a
    /// new one (`elevated_start`) on the same already-elevated, already-open pipes — no scheduled
    /// task, no UAC. Returns clones of the pipe handles for the caller's [`super::runtime::PipeLink`];
    /// this connection's own master handles are never touched, so a `PipeLink` built from the clones
    /// ending a session later never closes the connection out from under the next one.
    pub fn start_new_session(
        &mut self,
        sidecar_path: &Path,
        sidecar_sha256: &str,
    ) -> io::Result<(std::fs::File, std::fs::File)> {
        self.sequence += 1;
        write_line(
            &mut self.to_launcher,
            &elevated_stop_session_envelope(&self.nonce, self.sequence),
        )?;
        self.sequence += 1;
        write_line(
            &mut self.to_launcher,
            &elevated_start_envelope(&self.nonce, self.sequence, sidecar_path, sidecar_sha256),
        )?;
        Ok((self.to_launcher.try_clone()?, self.from_launcher.try_clone()?))
    }
}

/// Tauri's `app_data_dir` for this identifier; the launcher runs before (and without) Tauri, so it
/// derives the same folder itself. A test keeps it equal to `tauri.conf.json`.
const APP_IDENTIFIER: &str = "com.throttlewatch.desktop";

/// Where the launcher writes `throttlewatch-launcher.log`: the application's own `logs\`, so one
/// «Abrir carpeta de registros» shows both. `APPDATA` only locates the OS folder Tauri itself uses
/// (the scheduled task runs as the same user); it configures nothing.
pub fn launcher_log_directory() -> Option<PathBuf> {
    crate::dev_faults::data_dir_override()
        .or_else(|| {
            std::env::var_os("APPDATA").map(|folder| PathBuf::from(folder).join(APP_IDENTIFIER))
        })
        .map(|data_dir| data_dir.join("logs"))
}

/// Entry point used by the scheduled task. It accepts one authenticated
/// request, verifies that the sidecar stays inside the installed directory,
/// verifies its digest, and kills it when the control pipe closes.
pub fn run_elevated_launcher() -> io::Result<()> {
    #[cfg(windows)]
    {
        launcher_main(open_session_pipes, ensure_pawnio_service_running)
    }

    #[cfg(not(windows))]
    {
        Err(io::Error::new(io::ErrorKind::Unsupported, "elevated launcher is Windows-only"))
    }
}

/// Every step is logged (T-LOG-005): this process has no other way to say why the application
/// ended up with an unelevated collector.
#[cfg(windows)]
fn launcher_main(
    open_pipes: impl FnOnce() -> io::Result<(std::fs::File, std::fs::File)>,
    pawnio_check: impl Fn() -> io::Result<()>,
) -> io::Result<()> {
    crate::log_info!("LAUNCHER_STARTED", "elevated launcher started");
    // Before anything is spawned, so the sidecar inherits the normal class (T187).
    if let Err(error) = super::supervisor::raise_current_process_to_normal_priority() {
        crate::log_warn!(
            "LAUNCHER_PRIORITY_NOT_RAISED",
            format!("the launcher stays at Task Scheduler's below-normal priority: {error}")
        );
    }
    let result = launcher_steps(open_pipes, pawnio_check);
    match &result {
        Ok(()) => crate::log_info!("LAUNCHER_EXITED", "elevated launcher exited normally"),
        Err(error) => {
            crate::log_error!("LAUNCHER_EXITED", format!("elevated launcher stopped: {error}"))
        }
    }
    result
}

/// What ended a session's [`proxy_child_over_pipe`] call.
///
/// Enmienda 2026-09-28 a ADR-0004: a session ending is no longer automatically the whole
/// launcher's shutdown signal. `StopSessionRequested` (an authenticated `elevated_stop_session`
/// on the control channel) keeps the pipes and the launcher process alive for a following
/// `elevated_start` — the collector can restart without a fresh Task Scheduler activation and a
/// fresh UAC elevation every time. `CallerDisconnected` (EOF, or any framing/validation failure on
/// the control channel) is still the real end: [`launcher_steps`] returns and the launcher exits,
/// exactly as it always has.
#[cfg(windows)]
enum ChildEnd {
    StopSessionRequested,
    CallerDisconnected,
}

#[cfg(windows)]
fn launcher_steps(
    open_pipes: impl FnOnce() -> io::Result<(std::fs::File, std::fs::File)>,
    pawnio_check: impl Fn() -> io::Result<()>,
) -> io::Result<()> {
    let (to_launcher, from_launcher) = match open_pipes() {
        Ok(pipes) => {
            crate::log_info!("LAUNCHER_PIPES_OPENED", "connected to the application's pipes");
            pipes
        }
        Err(error) => {
            crate::log_error!(
                "LAUNCHER_PIPE_CONNECT_FAILED",
                format!("could not connect to the application's pipes: {error}")
            );
            return Err(error);
        }
    };
    let mut reader = BufReader::new(to_launcher);
    let mut hello_line = String::new();
    reader.read_line(&mut hello_line)?;
    let hello: serde_json::Value = serde_json::from_str(&hello_line)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid launcher hello"))?;
    let nonce = hello
        .get("session_nonce")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| io::Error::new(io::ErrorKind::PermissionDenied, "missing launcher nonce"))?
        .to_owned();
    // One monotonic sequence for every elevated-control message on this connection — the first
    // `elevated_start`, every `elevated_stop_session`, and every `elevated_start` that follows it —
    // not just the single handshake ADR-0004 originally described (see its 2026-09-28 amendment).
    let mut previous_sequence: Option<u64> = None;
    // Checked (and, if needed, started) only once per launcher process, not once per session
    // (found 2026-09-28): the retry loop of `ensure_pawnio_service_running_with` can legitimately
    // take many seconds when the driver is genuinely restarting, and on this connection's second
    // and later sessions the driver is already running — the previous child, whose handle to it
    // just closed cleanly a moment ago, proves that. Repeating the full check on every reused
    // session blocked this function's only thread from reading the pipe at all for the whole
    // retry window, which is indistinguishable from the launcher hanging to the application side:
    // its own handshake timeout (10 s) fired three times before this function ever got back to
    // `read_line`, exhausting the collector's restart budget and cancelling the guided test.
    let mut pawnio_checked = false;

    loop {
        let mut request_line = String::new();
        // EOF here — not mid-session, but while *waiting for the next session* — is the
        // application really closing the pipe (app shutdown), the one case that still ends the
        // launcher itself.
        if reader.read_line(&mut request_line)? == 0 {
            return Ok(());
        }
        let value: serde_json::Value = serde_json::from_str(&request_line)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid launcher request"))?;
        let payload = value.get("payload").ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "missing launcher payload")
        })?;
        let payload: ElevatedLaunchPayload = serde_json::from_value(payload.clone())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid launcher payload"))?;
        let install_dir =
            std::env::current_exe()?.parent().map(Path::to_path_buf).ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, "launcher directory is unavailable")
            })?;
        // The sidecar sits beside the launcher, or in its `collector\` subdirectory — the same
        // two locations `telemetry::launch::candidate_directories` tries for the unelevated path
        // (T112 bundles the self-contained .NET publish under `collector\`, not loose beside the
        // executable). The manifest that proves the sidecar's hash lives wherever the sidecar does.
        let Some(sidecar_dir) = payload.sidecar_path.parent() else {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "sidecar path has no parent directory",
            ));
        };
        if sidecar_dir != install_dir.as_path() && sidecar_dir != install_dir.join("collector") {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "sidecar is outside the installed directory",
            ));
        }
        // Re-verified on every session, including a reused connection's second, third, ... session
        // (ADR-0004 amendment, mitigation (b)): a persistent launcher must never let an update
        // replace the sidecar underneath an already-open connection without checking it again.
        let signed_sha256 = match verify_signed_sidecar(sidecar_dir, &payload.sidecar_path) {
            Ok(digest) => {
                crate::log_info!(
                    "LAUNCHER_SIDECAR_VERIFIED",
                    "the sidecar matches the signed release manifest"
                );
                digest
            }
            Err(error) => {
                crate::log_error!(
                    "LAUNCHER_SIDECAR_VERIFY_FAILED",
                    format!("the sidecar could not be verified: {error}")
                );
                return Err(error);
            }
        };
        let validated = validate_launch_request(
            request_line.as_bytes(),
            &nonce,
            previous_sequence,
            &payload.sidecar_path,
            &signed_sha256,
        )
        .map_err(|error| io::Error::new(io::ErrorKind::PermissionDenied, error))?;
        previous_sequence = Some(validated.sequence);
        if !pawnio_checked {
            pawnio_check()?;
            pawnio_checked = true;
        }
        let child = match super::supervisor::spawn_verified(&payload.sidecar_path, &signed_sha256) {
            Ok(child) => {
                crate::log_info!("LAUNCHER_CHILD_SPAWNED", "the elevated sidecar is running");
                child
            }
            Err(error) => {
                crate::log_error!(
                    "LAUNCHER_CHILD_SPAWN_FAILED",
                    format!("the sidecar could not be started: {error}")
                );
                return Err(error);
            }
        };
        match proxy_child_over_pipe(
            child,
            &mut reader,
            &from_launcher,
            &nonce,
            &mut previous_sequence,
        )? {
            ChildEnd::CallerDisconnected => return Ok(()),
            ChildEnd::StopSessionRequested => continue,
        }
    }
}

/// Pumps the sidecar's own NDJSON protocol both ways over the already-authenticated control pipe
/// (the caller's [`super::runtime::PipeLink`] is the other end) until the caller ends this session
/// or disconnects for good, then kills `child`. Split out of [`run_elevated_launcher`] so the
/// proxying itself — the part that actually matters for FR-088/SC-019 (advanced access must not
/// need a daily elevation) — is testable against a plain stand-in child instead of a real
/// scheduled task and signed sidecar.
///
/// `in_reader`/`out_pipe` are borrowed, not owned (ADR-0004 amendment, 2026-09-28): the caller
/// ([`launcher_steps`]) keeps them across sessions instead of a fresh pair per collector restart,
/// so this function must never close them itself — that is exactly what used to make the launcher
/// exit on every restart. `out_pipe` is written from a clone (`try_clone`); the original handle the
/// caller holds is never touched, so a session ending here never closes the caller's side of it.
#[cfg(windows)]
fn proxy_child_over_pipe(
    mut child: Child,
    in_reader: &mut BufReader<std::fs::File>,
    out_pipe: &std::fs::File,
    nonce: &str,
    previous_sequence: &mut Option<u64>,
) -> io::Result<ChildEnd> {
    let mut child_stdin = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("the elevated sidecar's stdin is unavailable"))?;
    let child_stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("the elevated sidecar's stdout is unavailable"))?;
    let child_stderr = child.stderr.take();

    // `out_pipe` has exactly one writer for its whole life (this thread), fed by a channel:
    // `child_stdout`/`child_stderr` are two independent OS objects, so two threads may block-read
    // them concurrently with no conflict, but funneling both into ONE thread to actually write
    // `out_pipe` is what keeps that pipe's handle single-threaded (see the type's doc comment).
    let out_pipe_for_writer = out_pipe.try_clone()?;
    let (lines_out, lines_in) = std::sync::mpsc::channel::<String>();
    let stdout_thread = {
        let lines_out = lines_out.clone();
        thread::Builder::new()
            .name("elevated-collector-stdout".to_owned())
            .spawn(move || read_lines_into(child_stdout, None, &lines_out))?
    };
    let stderr_thread = child_stderr
        .map(|child_stderr| {
            let lines_out = lines_out.clone();
            thread::Builder::new()
                .name("elevated-collector-stderr".to_owned())
                .spawn(move || read_lines_into(child_stderr, Some(STDERR_MARKER), &lines_out))
        })
        .transpose()?;
    drop(lines_out);
    let writer_thread =
        thread::Builder::new().name("elevated-collector-writer".to_owned()).spawn(move || {
            let mut out_pipe = out_pipe_for_writer;
            for line in lines_in {
                if out_pipe.write_all(line.as_bytes()).is_err() {
                    break;
                }
            }
        })?;

    // Every incoming line is checked for the elevated control channel before being forwarded: an
    // authenticated `elevated_stop_session` ends this child without ending the launcher (the
    // caller closing every handle on its side — real app shutdown — is still the only thing that
    // makes this a plain EOF). Anything else, `elevated_stop_session` itself included, is never
    // forwarded to the child; every other `type` is the collector's own protocol and passes through
    // untouched, exactly as before.
    let outcome = loop {
        let mut line = String::new();
        match in_reader.by_ref().take(MAX_LINE_BYTES + 1).read_line(&mut line) {
            Ok(0) | Err(_) => break ChildEnd::CallerDisconnected,
            Ok(_) if line.len() as u64 > MAX_LINE_BYTES => break ChildEnd::CallerDisconnected,
            Ok(_) => {
                let is_stop_session = serde_json::from_str::<serde_json::Value>(&line)
                    .ok()
                    .and_then(|value| value.get("type").and_then(|t| t.as_str().map(str::to_owned)))
                    .is_some_and(|message_type| message_type == "elevated_stop_session");
                if is_stop_session {
                    match crate::ipc::protocol::validate_message(
                        line.as_bytes(),
                        nonce,
                        *previous_sequence,
                    ) {
                        Ok(envelope) => {
                            *previous_sequence = Some(envelope.sequence);
                            crate::log_info!(
                                "LAUNCHER_SESSION_STOPPED",
                                "elevated session stopped, launcher stays up for the next one"
                            );
                            break ChildEnd::StopSessionRequested;
                        }
                        // An unauthenticated or out-of-sequence stop request is not trusted enough
                        // to keep this connection open on: treat it the same as a real disconnect.
                        Err(_) => break ChildEnd::CallerDisconnected,
                    }
                }
                if child_stdin.write_all(line.as_bytes()).is_err() {
                    break ChildEnd::CallerDisconnected;
                }
            }
        }
    };
    drop(child_stdin);
    let _ = child.kill();
    let _ = child.wait();
    // Killing the child closes its stdout/stderr, which ends `read_lines_into`'s loops, which
    // drops every remaining `lines_out` sender, which ends the writer thread's `for` loop.
    let _ = stdout_thread.join();
    if let Some(stderr_thread) = stderr_thread {
        let _ = stderr_thread.join();
    }
    let _ = writer_thread.join();
    Ok(outcome)
}

/// Reads newline-delimited lines from `source` and sends each, optionally prefixed with `marker`,
/// until `source` closes or a line desynchronizes the protocol (see [`MAX_LINE_BYTES`]). Sending
/// never blocks on the pipe itself — only the dedicated writer thread touches that handle.
#[cfg(windows)]
fn read_lines_into<R: Read>(
    source: R,
    marker: Option<&str>,
    sink: &std::sync::mpsc::Sender<String>,
) {
    let mut reader = BufReader::new(source);
    loop {
        let mut line = String::new();
        match reader.by_ref().take(MAX_LINE_BYTES + 1).read_line(&mut line) {
            Ok(0) | Err(_) => return,
            Ok(_) if line.len() as u64 > MAX_LINE_BYTES => return,
            Ok(_) => {
                let trimmed = line.trim_end_matches(['\r', '\n']);
                let framed = match marker {
                    Some(marker) => format!("{marker}{trimmed}\n"),
                    None => format!("{trimmed}\n"),
                };
                if sink.send(framed).is_err() {
                    return;
                }
            }
        }
    }
}

#[cfg(windows)]
fn verify_signed_sidecar(sidecar_dir: &Path, sidecar_path: &Path) -> io::Result<String> {
    let relative = sidecar_path.strip_prefix(sidecar_dir).map_err(|_| {
        io::Error::new(io::ErrorKind::PermissionDenied, "sidecar is outside its own directory")
    })?;
    if relative.components().any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "sidecar manifest path is not a plain relative path",
        ));
    }
    let public_key = crate::release_manifest::trusted_public_key().ok_or_else(|| {
        io::Error::new(io::ErrorKind::PermissionDenied, "no release manifest key is trusted")
    })?;
    // The manifest sits beside the sidecar it describes, not beside the launcher: T112 bundles the
    // sidecar (and its manifest) under `collector\`, one level below the launcher executable.
    let manifest_bytes = std::fs::read(sidecar_dir.join(RELEASE_MANIFEST_NAME))?;
    let signature = std::fs::read_to_string(sidecar_dir.join(RELEASE_SIGNATURE_NAME))?;
    let manifest =
        crate::release_manifest::ReleaseManifest::verify(&manifest_bytes, &signature, public_key)
            .map_err(|error| io::Error::new(io::ErrorKind::PermissionDenied, error.to_string()))?;
    manifest
        .check_file(sidecar_dir, relative)
        .map_err(|error| io::Error::new(io::ErrorKind::PermissionDenied, error.to_string()))?;
    manifest.sha256_for(relative).map(str::to_owned).ok_or_else(|| {
        io::Error::new(io::ErrorKind::PermissionDenied, "sidecar is not listed in release manifest")
    })
}

#[cfg(windows)]
trait PawnIoServiceController {
    fn query_running(&self) -> io::Result<Option<bool>>;
    fn start(&self) -> io::Result<()>;
}

#[cfg(windows)]
struct WindowsPawnIoServiceController;

#[cfg(windows)]
impl PawnIoServiceController for WindowsPawnIoServiceController {
    fn query_running(&self) -> io::Result<Option<bool>> {
        let query =
            hidden_command(r"C:\Windows\System32\sc.exe").args(["query", "PawnIO"]).output()?;
        if !query.status.success() {
            return Ok(None);
        }
        Ok(Some(String::from_utf8_lossy(&query.stdout).contains("RUNNING")))
    }

    fn start(&self) -> io::Result<()> {
        let status =
            hidden_command(r"C:\Windows\System32\sc.exe").args(["start", "PawnIO"]).status()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "PawnIO service could not start"))
        }
    }
}

#[cfg(windows)]
fn ensure_pawnio_service_running() -> io::Result<()> {
    ensure_pawnio_service_running_with(&WindowsPawnIoServiceController)
}

#[cfg(windows)]
fn ensure_pawnio_service_running_with<C: PawnIoServiceController>(
    controller: &C,
) -> io::Result<()> {
    match controller.query_running()? {
        None | Some(true) => Ok(()),
        Some(false) => {
            controller.start()?;
            for _ in 0..20 {
                if controller.query_running()? == Some(true) {
                    return Ok(());
                }
                std::thread::sleep(Duration::from_millis(250));
            }
            Err(io::Error::new(io::ErrorKind::TimedOut, "PawnIO service did not become running"))
        }
    }
}

#[cfg(windows)]
fn session_nonce() -> io::Result<String> {
    let mut bytes = [0_u8; 16];
    // SAFETY: the buffer is valid for `bytes.len()` writable bytes and the API only fills it.
    if unsafe { system_function_036(bytes.as_mut_ptr(), bytes.len() as u32) } == 0 {
        return Err(io::Error::other("could not generate session nonce"));
    }
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

#[cfg(windows)]
const ERROR_PIPE_CONNECTED: u32 = 535;

#[cfg(windows)]
const INVALID_HANDLE_VALUE: *mut std::ffi::c_void = -1_isize as *mut std::ffi::c_void;

#[cfg(windows)]
fn create_secure_pipe(
    pipe_name: &str,
) -> io::Result<(*mut std::ffi::c_void, *mut std::ffi::c_void)> {
    use std::os::windows::ffi::OsStrExt;
    let sddl: Vec<u16> =
        std::ffi::OsStr::new("D:P(A;;GA;;;OW)(A;;GA;;;SY)").encode_wide().chain([0]).collect();
    let mut descriptor = std::ptr::null_mut();
    // SAFETY: `sddl` is a valid nul-terminated UTF-16 string and the output pointer is valid for
    // the API to initialize; the returned descriptor is freed on every path below.
    let converted = unsafe {
        convert_string_security_descriptor(sddl.as_ptr(), 1, &mut descriptor, std::ptr::null_mut())
    };
    if converted == 0 {
        return Err(io::Error::last_os_error());
    }
    let name: Vec<u16> = std::ffi::OsStr::new(pipe_name).encode_wide().chain([0]).collect();
    let mut attributes = SecurityAttributes {
        length: std::mem::size_of::<SecurityAttributes>() as u32,
        descriptor,
        inherit: 0,
    };
    // SAFETY: all pointers reference live, nul-terminated data or initialized security attributes
    // for the duration of the synchronous CreateNamedPipeW call.
    let handle = unsafe {
        create_named_pipe(
            name.as_ptr(),
            0x00000003 | FILE_FLAG_FIRST_PIPE_INSTANCE,
            0x00000006,
            1,
            1024 * 1024,
            1024 * 1024,
            0,
            &mut attributes,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        // SAFETY: the descriptor was allocated by the matching conversion API and the pipe was
        // not created, so this path owns the only allocation.
        unsafe { free_security_descriptor(descriptor) };
        return Err(io::Error::last_os_error());
    }
    Ok((handle, descriptor))
}

#[cfg(windows)]
fn random_pipe_name() -> io::Result<String> {
    let mut bytes = [0_u8; 16];
    // SAFETY: the buffer is valid for `bytes.len()` writable bytes and the API only fills it.
    if unsafe { system_function_036(bytes.as_mut_ptr(), bytes.len() as u32) } == 0 {
        return Err(io::Error::other("could not generate pipe name"));
    }
    Ok(format!(
        "{ELEVATED_PIPE_PREFIX}{}",
        bytes.iter().map(|byte| format!("{byte:02x}")).collect::<String>()
    ))
}

#[cfg(windows)]
/// Finds the `.in` pipe [`start_registered_sidecar`] just created, plus its paired `.out` pipe
/// (same random base, the other direction — see the note on [`start_registered_sidecar`] for why
/// there are two).
fn open_session_pipes() -> io::Result<(std::fs::File, std::fs::File)> {
    for _ in 0..50 {
        if let Ok(entries) = std::fs::read_dir(r"\\.\pipe") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                let prefix = ELEVATED_PIPE_PREFIX.trim_start_matches(r"\\.\pipe\");
                let Some(in_name) = name.strip_prefix(prefix).filter(|rest| rest.ends_with(".in"))
                else {
                    continue;
                };
                let base = format!("{prefix}{}", in_name.trim_end_matches(".in"));
                let in_path = format!(r"\\.\pipe\{base}.in");
                let out_path = format!(r"\\.\pipe\{base}.out");
                let opened =
                    std::fs::OpenOptions::new().read(true).write(true).open(&in_path).and_then(
                        |to_launcher| {
                            std::fs::OpenOptions::new()
                                .read(true)
                                .write(true)
                                .open(&out_path)
                                .map(|from_launcher| (to_launcher, from_launcher))
                        },
                    );
                if let Ok(pipes) = opened {
                    return Ok(pipes);
                }
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err(io::Error::new(io::ErrorKind::TimedOut, "elevated launcher pipes did not open"))
}

#[cfg(windows)]
#[repr(C)]
struct SecurityAttributes {
    length: u32,
    descriptor: *mut std::ffi::c_void,
    inherit: i32,
}

#[cfg(windows)]
#[link(name = "kernel32")]
// SAFETY: these declarations mirror the stable Windows ABI; each call site documents pointer
// validity and ownership for the individual operation.
unsafe extern "system" {
    #[link_name = "CreateNamedPipeW"]
    fn create_named_pipe(
        name: *const u16,
        open_mode: u32,
        pipe_mode: u32,
        max_instances: u32,
        out_buffer_size: u32,
        in_buffer_size: u32,
        default_timeout: u32,
        attributes: *mut SecurityAttributes,
    ) -> *mut std::ffi::c_void;
    #[link_name = "ConnectNamedPipe"]
    fn connect_named_pipe(handle: *mut std::ffi::c_void, overlapped: *mut std::ffi::c_void) -> i32;
    #[link_name = "GetLastError"]
    fn last_error() -> u32;
    #[link_name = "LocalFree"]
    fn free_security_descriptor(descriptor: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
}

#[cfg(windows)]
#[link(name = "advapi32")]
// SAFETY: these declarations mirror the stable Windows ABI; each call site documents pointer
// validity and ownership for the individual operation.
unsafe extern "system" {
    #[link_name = "ConvertStringSecurityDescriptorToSecurityDescriptorW"]
    fn convert_string_security_descriptor(
        string: *const u16,
        revision: u32,
        descriptor: *mut *mut std::ffi::c_void,
        size: *mut u32,
    ) -> i32;
    #[link_name = "SystemFunction036"]
    fn system_function_036(buffer: *mut u8, length: u32) -> i32;
}

#[cfg(test)]
mod tests {
    use super::{APP_IDENTIFIER, ELEVATED_PIPE_PREFIX, validate_launch_request};
    use std::path::Path;

    #[test]
    fn the_launcher_logs_into_the_same_folder_as_the_application() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap_or_default();
        assert_eq!(config["identifier"].as_str(), Some(APP_IDENTIFIER));
    }

    /// T-LOG-005 diagnosis scenario: the failure behind «the elevated launcher never connected to
    /// the pipe» must be readable in the launcher's own file, not just its symptom in the app's.
    #[cfg(windows)]
    #[test]
    fn a_launcher_that_cannot_reach_the_pipes_says_so_in_its_own_file() -> std::io::Result<()> {
        let _session = crate::logging::session_test_lock();
        let directory =
            std::env::temp_dir().join(format!("throttlewatch-launcher-log-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        let (subscriber, guard) = crate::logging::scoped_file_subscriber(
            &directory,
            "throttlewatch-launcher.log",
            crate::logging::LogLevel::Debug,
            "launcher",
        )?;
        let result = tracing::subscriber::with_default(subscriber, || {
            super::launcher_main(
                || {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "elevated launcher pipes did not open",
                    ))
                },
                || Ok(()),
            )
        });
        assert!(result.is_err());
        drop(guard);
        let written = std::fs::read_to_string(directory.join("throttlewatch-launcher.log"))?;
        std::fs::remove_dir_all(&directory)?;
        let events: Vec<serde_json::Value> =
            written.lines().filter_map(|line| serde_json::from_str(line).ok()).collect();
        let codes: Vec<&str> = events.iter().filter_map(|event| event["code"].as_str()).collect();
        assert_eq!(codes, ["LAUNCHER_STARTED", "LAUNCHER_PIPE_CONNECT_FAILED", "LAUNCHER_EXITED"]);
        assert!(events.iter().all(|event| event["component"] == "launcher"));
        assert_eq!(
            events[1]["msg"],
            "could not connect to the application's pipes: elevated launcher pipes did not open"
        );
        assert_eq!(events[2]["level"], "error");
        Ok(())
    }

    #[test]
    fn rejects_wrong_nonce_and_sidecar_identity() {
        let raw = br#"{"protocol_version":1,"session_nonce":"0123456789abcdef","sequence":1,"timestamp_utc":"2026-09-19T10:00:00Z","type":"elevated_start","payload":{"sidecar_path":"C:\\ThrottleWatch\\agent.exe","sidecar_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}}"#;
        assert!(
            validate_launch_request(
                raw,
                "other_nonce_123456",
                None,
                Path::new("C:\\ThrottleWatch\\agent.exe"),
                &"a".repeat(64)
            )
            .is_err()
        );
        assert!(ELEVATED_PIPE_PREFIX.starts_with(r"\\.\pipe\"));
    }

    #[test]
    fn accepts_matching_nonce_path_and_hash() {
        let raw = br#"{"protocol_version":1,"session_nonce":"0123456789abcdef","sequence":1,"timestamp_utc":"2026-09-19T10:00:00Z","type":"elevated_start","payload":{"sidecar_path":"C:\\ThrottleWatch\\agent.exe","sidecar_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}}"#;
        let result = validate_launch_request(
            raw,
            "0123456789abcdef",
            None,
            Path::new("C:\\ThrottleWatch\\agent.exe"),
            &"a".repeat(64),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn rejects_unknown_commands_and_generic_msr_writes() {
        let prefix = br#"{"protocol_version":1,"session_nonce":"0123456789abcdef","sequence":1,"timestamp_utc":"2026-09-19T10:00:00Z","type":""#;
        for command in ["unknown", "elevated_write_msr"] {
            let raw = [prefix, command.as_bytes(), br#"","payload":{}}"#].concat();
            assert!(
                validate_launch_request(
                    &raw,
                    "0123456789abcdef",
                    None,
                    Path::new("C:\\ThrottleWatch\\agent.exe"),
                    &"a".repeat(64),
                )
                .is_err()
            );
        }
    }

    #[test]
    fn rejects_reused_sequence_nonce() {
        let raw = br#"{"protocol_version":1,"session_nonce":"0123456789abcdef","sequence":1,"timestamp_utc":"2026-09-19T10:00:00Z","type":"elevated_start","payload":{"sidecar_path":"C:\\ThrottleWatch\\agent.exe","sidecar_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}}"#;
        assert!(
            validate_launch_request(
                raw,
                "0123456789abcdef",
                Some(1),
                Path::new("C:\\ThrottleWatch\\agent.exe"),
                &"a".repeat(64),
            )
            .is_err()
        );
    }

    #[test]
    fn pipe_names_are_session_scoped_and_first_instance_is_required() {
        assert!(ELEVATED_PIPE_PREFIX.ends_with('.'));
        assert_eq!(super::FILE_FLAG_FIRST_PIPE_INSTANCE, 0x0008_0000);
    }

    #[cfg(windows)]
    #[test]
    fn rejects_an_occupied_pipe_name() {
        let name = format!("{ELEVATED_PIPE_PREFIX}occupied-test");
        let (first, descriptor) = match super::create_secure_pipe(&name) {
            Ok(value) => value,
            Err(error) => panic!("first pipe: {error}"),
        };
        // SAFETY: the descriptor belongs to this test and was allocated by the matching API.
        unsafe { super::free_security_descriptor(descriptor) };
        assert!(super::create_secure_pipe(&name).is_err());
        use std::os::windows::io::FromRawHandle;
        // SAFETY: `first` is the valid handle returned by the test-created named pipe and is
        // transferred into File for exactly-once cleanup.
        drop(unsafe { std::fs::File::from_raw_handle(first as _) });
    }

    #[cfg(windows)]
    #[test]
    fn starts_pawnio_after_a_stopped_service() {
        use std::cell::{Cell, RefCell};
        use std::collections::VecDeque;

        struct FakeService {
            states: RefCell<VecDeque<Option<bool>>>,
            starts: Cell<usize>,
        }

        impl super::PawnIoServiceController for FakeService {
            fn query_running(&self) -> std::io::Result<Option<bool>> {
                Ok(self.states.borrow_mut().pop_front().unwrap_or(Some(true)))
            }

            fn start(&self) -> std::io::Result<()> {
                self.starts.set(self.starts.get() + 1);
                Ok(())
            }
        }

        let fake = FakeService {
            states: RefCell::new(VecDeque::from([Some(false), Some(true)])),
            starts: Cell::new(0),
        };
        assert!(super::ensure_pawnio_service_running_with(&fake).is_ok());
        assert_eq!(fake.starts.get(), 1);
    }

    /// A real, connected named-pipe pair (one direction), built the same way
    /// [`super::create_listening_pipe`]/[`super::finish_connect`] (server) and
    /// [`super::open_session_pipes`] (client) build theirs, without a scheduled task.
    #[cfg(windows)]
    fn connected_test_pipe_pair(case: &str) -> std::io::Result<(std::fs::File, std::fs::File)> {
        let name = format!("{ELEVATED_PIPE_PREFIX}test-{case}-{:?}", std::thread::current().id());
        let server_handle = super::create_listening_pipe(&name)?;
        let client = std::fs::OpenOptions::new().read(true).write(true).open(&name)?;
        let server = super::finish_connect(server_handle)?;
        Ok((server, client))
    }

    #[cfg(windows)]
    #[test]
    fn connect_with_timeout_gives_up_instead_of_blocking_forever_when_nobody_connects() {
        let name = format!("{ELEVATED_PIPE_PREFIX}test-timeout-{:?}", std::thread::current().id());
        let handle = super::create_listening_pipe(&name)
            .unwrap_or_else(|error| panic!("create_listening_pipe: {error}"));
        // Nobody ever opens `name` as a client: the real failure this guards (the elevated
        // launcher rejecting the request and exiting before it ever reaches this pipe) looks
        // exactly like this from the caller's side.
        let started = std::time::Instant::now();
        let result = super::connect_with_timeout(handle, std::time::Duration::from_millis(300));
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
        assert_eq!(result.err().map(|error| error.kind()), Some(std::io::ErrorKind::TimedOut));
    }

    #[test]
    fn read_lines_into_frames_each_line_and_marks_stderr_without_touching_stdout() {
        use std::io::Cursor;
        use std::sync::mpsc;

        let (sender, receiver) = mpsc::channel();
        super::read_lines_into(Cursor::new(b"one\ntwo\r\n".to_vec()), None, &sender);
        super::read_lines_into(
            Cursor::new(b"oops\n".to_vec()),
            Some(super::STDERR_MARKER),
            &sender,
        );
        drop(sender);
        let lines: Vec<String> = receiver.into_iter().collect();
        assert_eq!(
            lines,
            vec!["one\n".to_owned(), "two\n".to_owned(), format!("{}oops\n", super::STDERR_MARKER)]
        );
    }

    /// Builds the `test-echo` sibling bin the first time it is needed and returns its path.
    /// `CARGO_BIN_EXE_*` is only set for integration tests/examples, not for a unit test inside
    /// the library target itself, so this locates (and builds, the first time) the sibling bin
    /// the ordinary way instead.
    #[cfg(windows)]
    fn test_echo_path() -> std::path::PathBuf {
        let current_exe =
            std::env::current_exe().unwrap_or_else(|error| panic!("current_exe: {error}"));
        let deps_dir =
            current_exe.parent().unwrap_or_else(|| panic!("{current_exe:?} has no parent"));
        let target_dir =
            deps_dir.parent().unwrap_or_else(|| panic!("{deps_dir:?} has no parent")).to_owned();
        let exe = target_dir.join("test-echo.exe");
        if !exe.is_file() {
            let manifest_dir = env!("CARGO_MANIFEST_DIR");
            let status = std::process::Command::new(env!("CARGO"))
                .args(["build", "--bin", "test-echo", "--manifest-path"])
                .arg(std::path::Path::new(manifest_dir).join("Cargo.toml"))
                .status()
                .unwrap_or_else(|error| panic!("cargo build --bin test-echo: {error}"));
            assert!(status.success(), "building the test-echo helper bin failed");
        }
        exe
    }

    #[cfg(windows)]
    fn spawn_test_echo() -> std::process::Child {
        use std::process::Stdio;
        let echo_path = test_echo_path();
        std::process::Command::new(&echo_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|error| panic!("spawn {}: {error}", echo_path.display()))
    }

    #[cfg(windows)]
    fn control_envelope(nonce: &str, sequence: u64, message_type: &str) -> String {
        serde_json::json!({
            "protocol_version": crate::ipc::protocol::PROTOCOL_VERSION,
            "session_nonce": nonce,
            "sequence": sequence,
            "timestamp_utc": "2026-09-28T00:00:00Z",
            "type": message_type,
            "payload": {}
        })
        .to_string()
    }

    #[cfg(windows)]
    #[test]
    fn proxies_a_real_child_both_ways_over_two_pipes_and_stops_it_when_the_caller_disconnects() {
        use crate::telemetry::runtime::{CollectorLink, Received};
        use std::process::Command;
        use std::time::Duration;

        // Two pipes, not one duplex pipe — see `proxy_child_over_pipe`'s doc comment for why a
        // single synchronous handle shared between a reader thread and a writer thread deadlocks.
        // The caller is always the pipe *server* in production (`start_registered_sidecar`); the
        // launcher connects as a client (`open_session_pipes`) — mirrored here without a real
        // scheduled task.
        let (caller_out, launcher_in) =
            connected_test_pipe_pair("in").unwrap_or_else(|error| panic!("in pipe pair: {error}"));
        let (caller_in, launcher_out) = connected_test_pipe_pair("out")
            .unwrap_or_else(|error| panic!("out pipe pair: {error}"));

        // A stand-in for the real sidecar's own stdin/stdout: reads a line, writes it straight
        // back, flushing every time — without needing the signed manifest or PawnIO at all.
        let child = spawn_test_echo();
        let child_id = child.id();
        let proxy = std::thread::spawn(move || {
            let mut reader = std::io::BufReader::new(launcher_in);
            let mut previous_sequence = None;
            super::proxy_child_over_pipe(
                child,
                &mut reader,
                &launcher_out,
                "test-nonce",
                &mut previous_sequence,
            )
        });

        let mut link = crate::telemetry::runtime::PipeLink::connect(caller_out, caller_in)
            .unwrap_or_else(|error| panic!("PipeLink::connect: {error}"));
        if let Err(error) = link.send("hello from the caller") {
            panic!("send: {error}");
        }
        match link.receive(Duration::from_secs(5)) {
            Received::Line(line) => assert_eq!(line, "hello from the caller"),
            other => panic!("expected the line to round-trip through test-echo, got {other:?}"),
        }

        drop(link); // the shutdown signal: closes the caller's side of the "in" pipe
        match proxy.join() {
            Ok(Ok(super::ChildEnd::CallerDisconnected)) => {}
            Ok(Ok(super::ChildEnd::StopSessionRequested)) => {
                panic!("dropping the link should read as a disconnect, not a stop-session request")
            }
            Ok(Err(error)) => panic!("proxy should exit cleanly: {error}"),
            Err(_) => panic!("proxy thread should not panic"),
        }
        // The proxy only returns after killing and waiting for the child.
        assert!(
            Command::new("tasklist")
                .args(["/FI", &format!("PID eq {child_id}"), "/NH"])
                .output()
                .is_ok_and(|output| {
                    !String::from_utf8_lossy(&output.stdout).contains(&child_id.to_string())
                })
        );
    }

    #[cfg(windows)]
    #[test]
    fn start_new_session_writes_an_authenticated_stop_then_start_with_increasing_sequence() {
        use std::io::BufRead;

        let (caller_out, launcher_in) = connected_test_pipe_pair("reuse-in")
            .unwrap_or_else(|error| panic!("in pipe pair: {error}"));
        let (caller_in, _launcher_out) = connected_test_pipe_pair("reuse-out")
            .unwrap_or_else(|error| panic!("out pipe pair: {error}"));
        let nonce = "reuse-nonce".to_owned();
        // Sequence 1 stands in for a cold `start_registered_sidecar` call's own initial
        // `elevated_start` having already happened on this connection, as `cold_start` leaves it.
        let mut connection = super::PersistentElevatedConnection {
            to_launcher: caller_out,
            from_launcher: caller_in,
            nonce: nonce.clone(),
            sequence: 1,
        };

        let sidecar_path = Path::new("C:\\ThrottleWatch\\agent.exe");
        let sidecar_sha256 = "a".repeat(64);
        let (_to_clone, _from_clone) = connection
            .start_new_session(sidecar_path, &sidecar_sha256)
            .unwrap_or_else(|error| panic!("start_new_session: {error}"));

        let mut reader = std::io::BufReader::new(launcher_in);
        let mut stop_line = String::new();
        reader.read_line(&mut stop_line).unwrap_or_else(|error| panic!("read stop: {error}"));
        let stop = crate::ipc::protocol::validate_message(stop_line.as_bytes(), &nonce, Some(1))
            .unwrap_or_else(|error| panic!("stop envelope: {error}"));
        assert_eq!((stop.message_type.as_str(), stop.sequence), ("elevated_stop_session", 2));

        let mut start_line = String::new();
        reader.read_line(&mut start_line).unwrap_or_else(|error| panic!("read start: {error}"));
        let start = crate::ipc::protocol::validate_message(start_line.as_bytes(), &nonce, Some(2))
            .unwrap_or_else(|error| panic!("start envelope: {error}"));
        assert_eq!((start.message_type.as_str(), start.sequence), ("elevated_start", 3));
    }

    /// ADR-0004 amendment (2026-09-28): an authenticated `elevated_stop_session` must end the
    /// current child without ending the launcher's own pipes — the whole point of the persistence
    /// fix. Runs two sessions back to back over the very same pipe pair and `PipeLink`, exactly as
    /// `launcher_steps`'s loop and `ElevatedSidecarLauncher` (`telemetry::launch`) do in production.
    #[cfg(windows)]
    #[test]
    fn a_stop_session_ends_the_child_but_keeps_the_pipes_open_for_a_second_session() {
        use crate::telemetry::runtime::{CollectorLink, Received};
        use std::time::Duration;

        let (caller_out, launcher_in) = connected_test_pipe_pair("persist-in")
            .unwrap_or_else(|error| panic!("in pipe pair: {error}"));
        let (caller_in, launcher_out) = connected_test_pipe_pair("persist-out")
            .unwrap_or_else(|error| panic!("out pipe pair: {error}"));
        let nonce = "test-persistent-nonce";

        let proxy = std::thread::spawn(move || -> std::io::Result<super::ChildEnd> {
            let mut reader = std::io::BufReader::new(launcher_in);
            let mut previous_sequence = None;
            let first = super::proxy_child_over_pipe(
                spawn_test_echo(),
                &mut reader,
                &launcher_out,
                nonce,
                &mut previous_sequence,
            )?;
            assert!(
                matches!(first, super::ChildEnd::StopSessionRequested),
                "the first session should end on the authenticated stop request, not a disconnect"
            );
            super::proxy_child_over_pipe(
                spawn_test_echo(),
                &mut reader,
                &launcher_out,
                nonce,
                &mut previous_sequence,
            )
        });

        let mut link = crate::telemetry::runtime::PipeLink::connect(caller_out, caller_in)
            .unwrap_or_else(|error| panic!("PipeLink::connect: {error}"));
        if let Err(error) = link.send("first session") {
            panic!("send: {error}");
        }
        match link.receive(Duration::from_secs(5)) {
            Received::Line(line) => assert_eq!(line, "first session"),
            other => panic!("expected the first session to echo, got {other:?}"),
        }

        if let Err(error) = link.send(&control_envelope(nonce, 1, "elevated_stop_session")) {
            panic!("send stop_session: {error}");
        }

        // The same link, never dropped, still works against the second child: the pipes and the
        // launcher process never noticed a disconnect.
        if let Err(error) = link.send("second session, same pipes") {
            panic!("send: {error}");
        }
        match link.receive(Duration::from_secs(5)) {
            Received::Line(line) => assert_eq!(line, "second session, same pipes"),
            other => {
                panic!("expected the second session to echo over the same link, got {other:?}")
            }
        }

        drop(link);
        match proxy.join() {
            Ok(Ok(super::ChildEnd::CallerDisconnected)) => {}
            Ok(Ok(super::ChildEnd::StopSessionRequested)) => {
                panic!("the final drop should read as a real disconnect")
            }
            Ok(Err(error)) => panic!("proxy should exit cleanly: {error}"),
            Err(_) => panic!("proxy thread should not panic"),
        }
    }
}
