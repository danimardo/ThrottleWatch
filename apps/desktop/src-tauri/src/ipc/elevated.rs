use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

pub const ELEVATED_TASK_NAME: &str = "ThrottleWatch\\SidecarElevated";
pub const ELEVATED_PIPE_NAME: &str = r"\\.\pipe\ThrottleWatch.ElevatedSidecar";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ElevatedLaunchPayload {
    pub sidecar_path: PathBuf,
    pub sidecar_sha256: String,
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
    let payload = value.get("payload").ok_or_else(|| "missing payload".to_owned())?;
    let parsed: ElevatedLaunchPayload =
        serde_json::from_value(payload.clone()).map_err(|_| "invalid launch payload".to_owned())?;
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

pub fn task_create_arguments(launcher_path: &Path) -> Vec<String> {
    vec![
        "/Create".to_owned(),
        "/TN".to_owned(),
        ELEVATED_TASK_NAME.to_owned(),
        "/TR".to_owned(),
        format!("\"{}\" --elevated-launcher", launcher_path.display()),
        "/SC".to_owned(),
        "ONDEMAND".to_owned(),
        "/RL".to_owned(),
        "HIGHEST".to_owned(),
        "/F".to_owned(),
    ]
}

pub fn register_task(launcher_path: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        let status =
            Command::new("schtasks.exe").args(task_create_arguments(launcher_path)).status()?;
        if status.success() {
            return Ok(());
        }
        Err(io::Error::new(io::ErrorKind::PermissionDenied, "elevated task registration failed"))
    }

    #[cfg(not(windows))]
    {
        let _ = launcher_path;
        Err(io::Error::new(io::ErrorKind::Unsupported, "scheduled tasks are Windows-only"))
    }
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
        let mut pipe = None;
        for _ in 0..50 {
            match std::fs::OpenOptions::new().write(true).open(ELEVATED_PIPE_NAME) {
                Ok(value) => {
                    pipe = Some(value);
                    break;
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(error) => return Err(error),
            }
        }
        let mut pipe = pipe.ok_or_else(|| {
            io::Error::new(io::ErrorKind::TimedOut, "elevated launcher pipe did not open")
        })?;
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
        use std::os::windows::io::FromRawHandle;
        let (handle, descriptor) = create_secure_pipe()?;
        let connected = unsafe { connect_named_pipe(handle, std::ptr::null_mut()) } != 0;
        if !connected && unsafe { last_error() } != ERROR_PIPE_CONNECTED {
            unsafe { close_handle(handle) };
            unsafe { free_security_descriptor(descriptor) };
            return Err(io::Error::last_os_error());
        }
        unsafe { free_security_descriptor(descriptor) };
        let pipe = unsafe { std::fs::File::from_raw_handle(handle as _) };
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
        let _validated = validate_launch_request(
            request_line.as_bytes(),
            nonce,
            None,
            &payload.sidecar_path,
            &payload.sidecar_sha256,
        )
        .map_err(|error| io::Error::new(io::ErrorKind::PermissionDenied, error))?;
        let mut child =
            super::supervisor::spawn_verified(&payload.sidecar_path, &payload.sidecar_sha256)?;
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
fn session_nonce() -> io::Result<String> {
    let mut bytes = [0_u8; 16];
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
fn create_secure_pipe() -> io::Result<(*mut std::ffi::c_void, *mut std::ffi::c_void)> {
    use std::os::windows::ffi::OsStrExt;
    let sddl: Vec<u16> =
        std::ffi::OsStr::new("D:P(A;;GA;;;OW)(A;;GA;;;SY)").encode_wide().chain([0]).collect();
    let mut descriptor = std::ptr::null_mut();
    let converted = unsafe {
        convert_string_security_descriptor(sddl.as_ptr(), 1, &mut descriptor, std::ptr::null_mut())
    };
    if converted == 0 {
        return Err(io::Error::last_os_error());
    }
    let name: Vec<u16> =
        std::ffi::OsStr::new(ELEVATED_PIPE_NAME).encode_wide().chain([0]).collect();
    let mut attributes = SecurityAttributes {
        length: std::mem::size_of::<SecurityAttributes>() as u32,
        descriptor,
        inherit: 0,
    };
    let handle = unsafe {
        create_named_pipe(
            name.as_ptr(),
            0x00000003,
            0x00000006,
            1,
            1024 * 1024,
            1024 * 1024,
            0,
            &mut attributes,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        unsafe { free_security_descriptor(descriptor) };
        return Err(io::Error::last_os_error());
    }
    Ok((handle, descriptor))
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
    #[link_name = "CloseHandle"]
    fn close_handle(handle: *mut std::ffi::c_void) -> i32;
    #[link_name = "LocalFree"]
    fn free_security_descriptor(descriptor: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
}

#[cfg(windows)]
#[link(name = "advapi32")]
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
    use super::{ELEVATED_PIPE_NAME, validate_launch_request};
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
        assert!(ELEVATED_PIPE_NAME.starts_with(r"\\.\pipe\"));
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
}
