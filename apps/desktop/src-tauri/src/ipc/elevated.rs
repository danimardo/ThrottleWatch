use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

pub const ELEVATED_TASK_NAME: &str = "ThrottleWatch\\SidecarElevated";
pub const ELEVATED_PIPE_PREFIX: &str = r"\\.\pipe\ThrottleWatch.ElevatedSidecar.";
const FILE_FLAG_FIRST_PIPE_INSTANCE: u32 = 0x0008_0000;
const RELEASE_MANIFEST_NAME: &str = "release-manifest.json";
const RELEASE_SIGNATURE_NAME: &str = "release-manifest.json.minisig";

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
            Command::new("schtasks.exe").args(["/Run", "/TN", ELEVATED_TASK_NAME]).status()?;
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
pub fn start_registered_sidecar(
    sidecar_path: &Path,
    sidecar_sha256: &str,
) -> io::Result<std::fs::File> {
    #[cfg(windows)]
    {
        use std::os::windows::io::FromRawHandle;

        let pipe_name = random_pipe_name()?;
        let (pipe_handle, descriptor) = create_secure_pipe(&pipe_name)?;
        // SAFETY: `descriptor` was allocated by the Windows security-descriptor API and is
        // released exactly once after the pipe has copied the pointer into its attributes.
        unsafe { free_security_descriptor(descriptor) };
        // SAFETY: `pipe_handle` is a valid, owned handle returned by CreateNamedPipeW and the
        // File takes ownership so it closes the handle exactly once.
        let mut pipe = unsafe { std::fs::File::from_raw_handle(pipe_handle as _) };
        run_registered_task()?;
        let nonce = session_nonce()?;
        let request = serde_json::json!({
            "protocol_version": crate::ipc::protocol::PROTOCOL_VERSION,
            "session_nonce": nonce,
            "sequence": 1,
            "timestamp_utc": format!("{:?}", std::time::SystemTime::now()),
            "type": "elevated_start",
            "payload": {
                "sidecar_path": sidecar_path,
                "sidecar_sha256": sidecar_sha256
            }
        });
        let hello = serde_json::json!({
            "protocol_version": crate::ipc::protocol::PROTOCOL_VERSION,
            "session_nonce": nonce,
            "type": "hello"
        });
        // SAFETY: `pipe_handle` is the valid synchronous server handle created above and a null
        // overlapped pointer requests the documented blocking connection mode.
        let connected = unsafe { connect_named_pipe(pipe_handle, std::ptr::null_mut()) } != 0;
        // SAFETY: GetLastError is read immediately after the failed ConnectNamedPipe call.
        if !connected && unsafe { last_error() } != ERROR_PIPE_CONNECTED {
            return Err(io::Error::last_os_error());
        }
        serde_json::to_writer(&mut pipe, &hello).map_err(io::Error::other)?;
        pipe.write_all(b"\n")
            .and_then(|_| serde_json::to_writer(&mut pipe, &request).map_err(io::Error::other))?;
        pipe.write_all(b"\n")?;
        pipe.flush()?;
        Ok(pipe)
    }

    #[cfg(not(windows))]
    {
        let _ = (sidecar_path, sidecar_sha256);
        Err(io::Error::new(io::ErrorKind::Unsupported, "named pipes are Windows-only"))
    }
}

/// Entry point used by the scheduled task. It accepts one authenticated
/// request, verifies that the sidecar stays inside the installed directory,
/// verifies its digest, and kills it when the control pipe closes.
pub fn run_elevated_launcher() -> io::Result<()> {
    #[cfg(windows)]
    {
        let pipe = open_session_pipe()?;
        let mut reader = BufReader::new(pipe);
        let mut hello_line = String::new();
        reader.read_line(&mut hello_line)?;
        let hello: serde_json::Value = serde_json::from_str(&hello_line)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid launcher hello"))?;
        let nonce = hello
            .get("session_nonce")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::PermissionDenied, "missing launcher nonce")
            })?;
        let mut request_line = String::new();
        reader.read_line(&mut request_line)?;
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
        if payload.sidecar_path.parent() != Some(install_dir.as_path()) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "sidecar is outside the installed directory",
            ));
        }
        let signed_sha256 = verify_signed_sidecar(&install_dir, &payload.sidecar_path)?;
        let _validated = validate_launch_request(
            request_line.as_bytes(),
            nonce,
            None,
            &payload.sidecar_path,
            &signed_sha256,
        )
        .map_err(|error| io::Error::new(io::ErrorKind::PermissionDenied, error))?;
        ensure_pawnio_service_running()?;
        let mut child = super::supervisor::spawn_verified(&payload.sidecar_path, &signed_sha256)?;
        let mut control_tail = Vec::new();
        let _ = reader.read_to_end(&mut control_tail);
        let _ = child.kill();
        let _ = child.wait();
        Ok(())
    }

    #[cfg(not(windows))]
    {
        Err(io::Error::new(io::ErrorKind::Unsupported, "elevated launcher is Windows-only"))
    }
}

#[cfg(windows)]
fn verify_signed_sidecar(install_dir: &Path, sidecar_path: &Path) -> io::Result<String> {
    let relative = sidecar_path.strip_prefix(install_dir).map_err(|_| {
        io::Error::new(io::ErrorKind::PermissionDenied, "sidecar is outside the install directory")
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
    let manifest_bytes = std::fs::read(install_dir.join(RELEASE_MANIFEST_NAME))?;
    let signature = std::fs::read_to_string(install_dir.join(RELEASE_SIGNATURE_NAME))?;
    let manifest =
        crate::release_manifest::ReleaseManifest::verify(&manifest_bytes, &signature, public_key)
            .map_err(|error| io::Error::new(io::ErrorKind::PermissionDenied, error.to_string()))?;
    manifest
        .check_file(install_dir, relative)
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
            Command::new(r"C:\Windows\System32\sc.exe").args(["query", "PawnIO"]).output()?;
        if !query.status.success() {
            return Ok(None);
        }
        Ok(Some(String::from_utf8_lossy(&query.stdout).contains("RUNNING")))
    }

    fn start(&self) -> io::Result<()> {
        let status =
            Command::new(r"C:\Windows\System32\sc.exe").args(["start", "PawnIO"]).status()?;
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
fn open_session_pipe() -> io::Result<std::fs::File> {
    for _ in 0..50 {
        if let Ok(entries) = std::fs::read_dir(r"\\.\pipe") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with(ELEVATED_PIPE_PREFIX.trim_start_matches(r"\\.\pipe\")) {
                    let path = format!(r"\\.\pipe\{name}");
                    if let Ok(pipe) = std::fs::OpenOptions::new().read(true).write(true).open(path)
                    {
                        return Ok(pipe);
                    }
                }
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err(io::Error::new(io::ErrorKind::TimedOut, "elevated launcher pipe did not open"))
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
    use super::{ELEVATED_PIPE_PREFIX, validate_launch_request};
    use std::path::Path;

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
}
