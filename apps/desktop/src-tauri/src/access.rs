#![deny(clippy::unwrap_used, clippy::expect_used)]

use crate::ipc::supervisor::sha256_hex;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};

#[cfg(windows)]
use std::ffi::OsStr;

const MANIFEST_JSON: &str = include_str!("../resources/pawnio-manifest.json");

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PawnIoManifest {
    pub minimum_version: String,
    pub packaged_version: String,
    pub installer: String,
    pub sha256: String,
    pub publisher_subject: String,
    pub source_url: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccessAction {
    Install,
    Upgrade,
    Repair,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AccessRequest {
    pub action: AccessAction,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AccessRequestResult {
    pub state: &'static str,
    pub action: AccessAction,
    pub reboot_may_be_required: bool,
}

pub fn manifest() -> io::Result<PawnIoManifest> {
    serde_json::from_str(MANIFEST_JSON).map_err(|error| {
        io::Error::new(io::ErrorKind::InvalidData, format!("invalid PawnIO manifest: {error}"))
    })
}

pub fn installer_path(resource_dir: &Path, value: &PawnIoManifest) -> PathBuf {
    resource_dir.join(Path::new(&value.installer))
}

pub fn verify_installer(path: &Path, value: &PawnIoManifest) -> io::Result<()> {
    let bytes = std::fs::read(path)?;
    let actual = sha256_hex(&bytes);
    if !actual.eq_ignore_ascii_case(&value.sha256) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "PawnIO installer hash does not match the pinned manifest",
        ));
    }
    verify_authenticode(path, &value.publisher_subject)
}

pub fn launch_installer(path: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let verb: Vec<u16> = OsStr::new("runas").encode_wide().chain([0]).collect();
        let file: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
        let result = unsafe {
            shell_execute_w(
                std::ptr::null_mut(),
                verb.as_ptr(),
                file.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1,
            )
        };
        if result as usize <= 32 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "PawnIO UAC launch was rejected",
            ));
        }
        Ok(())
    }

    #[cfg(not(windows))]
    {
        let _ = path;
        Err(io::Error::new(io::ErrorKind::Unsupported, "advanced access is Windows-only"))
    }
}

/// Runs the fixed bootstrap sequence under one UAC consent: PawnIO first and
/// then the already-installed elevated launcher task. Paths are supplied only
/// after the installer has passed the pinned hash/signature checks.
pub fn launch_installer_and_register_task(installer: &Path, launcher: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let installer_display = installer.display().to_string();
        let launcher_display = launcher.display().to_string();
        let installer = powershell_literal(&installer_display);
        let launcher = powershell_literal(&launcher_display);
        let task_name = powershell_literal(crate::ipc::elevated::ELEVATED_TASK_NAME);
        let script = format!(
            "$ErrorActionPreference='Stop'; $p=Start-Process -FilePath '{installer}' -Wait -PassThru; if ($p.ExitCode -ne 0) {{ exit $p.ExitCode }}; & schtasks.exe /Create /TN '{task_name}' /TR ('\"' + '{launcher}' + '\" --elevated-launcher') /SC ONDEMAND /RL HIGHEST /F; exit $LASTEXITCODE"
        );
        let parameters =
            format!("-NoProfile -NonInteractive -WindowStyle Hidden -Command \"{script}\"");
        let verb: Vec<u16> = OsStr::new("runas").encode_wide().chain([0]).collect();
        let file: Vec<u16> = OsStr::new("powershell.exe").encode_wide().chain([0]).collect();
        let parameters: Vec<u16> = OsStr::new(&parameters).encode_wide().chain([0]).collect();
        let result = unsafe {
            shell_execute_w(
                std::ptr::null_mut(),
                verb.as_ptr(),
                file.as_ptr(),
                parameters.as_ptr(),
                std::ptr::null(),
                0,
            )
        };
        if result as usize <= 32 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "PawnIO bootstrap UAC launch was rejected",
            ));
        }
        Ok(())
    }

    #[cfg(not(windows))]
    {
        let _ = (installer, launcher);
        Err(io::Error::new(io::ErrorKind::Unsupported, "advanced access is Windows-only"))
    }
}

#[cfg(windows)]
fn powershell_literal(value: &str) -> String {
    value.replace('\'', "''")
}

fn verify_authenticode(path: &Path, expected_subject: &str) -> io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let _ = expected_subject;
        let path_wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
        let mut file_info = WinTrustFileInfo {
            cb_struct: std::mem::size_of::<WinTrustFileInfo>() as u32,
            file_path: path_wide.as_ptr(),
            file_handle: std::ptr::null_mut(),
            known_subject: std::ptr::null_mut(),
        };
        let mut data = WinTrustData {
            cb_struct: std::mem::size_of::<WinTrustData>() as u32,
            policy_callback_data: std::ptr::null_mut(),
            sip_client_data: std::ptr::null_mut(),
            ui_choice: 2,
            revocation_checks: 0,
            union_choice: 1,
            file_info: &mut file_info,
            state_action: 1,
            state_data: std::ptr::null_mut(),
            url_reference: std::ptr::null_mut(),
            prov_flags: 0x00000010,
            ui_context: 0,
            signature_settings: std::ptr::null_mut(),
        };
        let status =
            unsafe { win_verify_trust(std::ptr::null_mut(), &GENERIC_VERIFY_V2, &mut data) };
        data.state_action = 2;
        let _ = unsafe { win_verify_trust(std::ptr::null_mut(), &GENERIC_VERIFY_V2, &mut data) };
        if status == 0 {
            return Ok(());
        }
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "PawnIO Authenticode signature is invalid",
        ))
    }

    #[cfg(not(windows))]
    {
        let _ = (path, expected_subject);
        Err(io::Error::new(io::ErrorKind::Unsupported, "Authenticode is Windows-only"))
    }
}

#[cfg(windows)]
#[repr(C)]
struct WinTrustFileInfo {
    cb_struct: u32,
    file_path: *const u16,
    file_handle: *mut std::ffi::c_void,
    known_subject: *mut std::ffi::c_void,
}

#[cfg(windows)]
#[repr(C)]
struct WinTrustData {
    cb_struct: u32,
    policy_callback_data: *mut std::ffi::c_void,
    sip_client_data: *mut std::ffi::c_void,
    ui_choice: u32,
    revocation_checks: u32,
    union_choice: u32,
    file_info: *mut WinTrustFileInfo,
    state_action: u32,
    state_data: *mut std::ffi::c_void,
    url_reference: *mut u16,
    prov_flags: u32,
    ui_context: u32,
    signature_settings: *mut std::ffi::c_void,
}

#[cfg(windows)]
#[repr(C)]
struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

#[cfg(windows)]
static GENERIC_VERIFY_V2: Guid = Guid {
    data1: 0x00AAC56B,
    data2: 0xCD44,
    data3: 0x11D0,
    data4: [0x8C, 0xC2, 0x00, 0xC0, 0x4F, 0xC2, 0x95, 0xEE],
};

#[cfg(windows)]
#[link(name = "wintrust")]
unsafe extern "system" {
    fn WinVerifyTrust(
        window: *mut std::ffi::c_void,
        action_id: *const Guid,
        data: *mut WinTrustData,
    ) -> i32;
}

#[cfg(windows)]
use WinVerifyTrust as win_verify_trust;

#[cfg(windows)]
#[link(name = "shell32")]
unsafe extern "system" {
    fn ShellExecuteW(
        window: *mut std::ffi::c_void,
        operation: *const u16,
        file: *const u16,
        parameters: *const u16,
        directory: *const u16,
        show_command: i32,
    ) -> *mut std::ffi::c_void;
}

#[cfg(windows)]
use ShellExecuteW as shell_execute_w;

#[cfg(test)]
mod tests {
    use super::{AccessAction, AccessRequest, manifest};

    #[test]
    fn manifest_pins_the_verified_release() {
        let value = match manifest() {
            Ok(value) => value,
            Err(error) => panic!("manifest fixture is part of the binary: {error}"),
        };
        assert_eq!(value.minimum_version, "2.2.0");
        assert_eq!(value.packaged_version, "2.2.0");
        assert_eq!(value.sha256.len(), 64);
    }

    #[test]
    fn request_actions_are_explicit_and_closed() {
        let request: AccessRequest = match serde_json::from_str(r#"{"action":"repair"}"#) {
            Ok(request) => request,
            Err(error) => panic!("valid action: {error}"),
        };
        assert_eq!(request.action, AccessAction::Repair);
        assert!(serde_json::from_str::<AccessRequest>(r#"{"action":"arbitrary"}"#).is_err());
    }
}
