#![deny(clippy::unwrap_used, clippy::expect_used)]

use crate::ipc::supervisor::sha256_hex;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};

#[cfg(windows)]
use std::ffi::OsStr;
#[cfg(windows)]
use std::process::Command;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PawnIoInstallation {
    Missing,
    Current { version: String },
    Upgradable { version: String },
}

pub fn action_is_compatible(action: AccessAction, installation: &PawnIoInstallation) -> bool {
    matches!(
        (action, installation),
        (AccessAction::Install, PawnIoInstallation::Missing)
            | (AccessAction::Upgrade, PawnIoInstallation::Upgradable { .. })
            | (AccessAction::Repair, PawnIoInstallation::Current { .. })
    )
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

pub fn detect_installation(minimum_version: &str) -> io::Result<PawnIoInstallation> {
    #[cfg(windows)]
    {
        let library = Path::new(r"C:\Program Files\PawnIO\PawnIOLib.dll");
        if !library.is_file() {
            return Ok(PawnIoInstallation::Missing);
        }
        let Some(version) = registry_display_version()? else {
            return Ok(PawnIoInstallation::Missing);
        };
        Ok(if compare_versions(&version, minimum_version).is_lt() {
            PawnIoInstallation::Upgradable { version }
        } else {
            PawnIoInstallation::Current { version }
        })
    }

    #[cfg(not(windows))]
    {
        let _ = minimum_version;
        Err(io::Error::new(io::ErrorKind::Unsupported, "PawnIO is Windows-only"))
    }
}

fn compare_versions(left: &str, right: &str) -> std::cmp::Ordering {
    let parse = |value: &str| {
        value.split('.').map(|part| part.parse::<u64>().unwrap_or(0)).collect::<Vec<_>>()
    };
    let left = parse(left);
    let right = parse(right);
    let length = left.len().max(right.len());
    (0..length)
        .map(|index| {
            (left.get(index).copied().unwrap_or(0), right.get(index).copied().unwrap_or(0))
        })
        .find_map(|(left, right)| (left != right).then_some(left.cmp(&right)))
        .unwrap_or(std::cmp::Ordering::Equal)
}

#[cfg(windows)]
fn registry_display_version() -> io::Result<Option<String>> {
    let output = Command::new(r"C:\Windows\System32\reg.exe")
        .args([
            "query",
            r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO",
            "/v",
            "DisplayVersion",
        ])
        .output()?;
    if !output.status.success() {
        return Ok(None);
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text.lines().find_map(parse_display_version_line))
}

#[cfg(windows)]
fn parse_display_version_line(line: &str) -> Option<String> {
    let mut fields = line.split_whitespace();
    (fields.next() == Some("DisplayVersion"))
        .then(|| fields.next())
        .flatten()
        .and_then(|kind| (kind == "REG_SZ").then(|| fields.next()).flatten())
        .map(str::to_owned)
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
pub fn launch_installer_and_register_task(
    installer: &Path,
    launcher: &Path,
    action: AccessAction,
) -> io::Result<()> {
    #[cfg(windows)]
    {
        let installer_display = installer.display().to_string();
        let launcher_display = launcher.display().to_string();
        let installer = powershell_literal(&installer_display);
        let launcher = powershell_literal(&launcher_display);
        let task_name = powershell_literal(crate::ipc::elevated::ELEVATED_TASK_NAME);
        let upgrade = matches!(action, AccessAction::Upgrade);
        let uninstall = powershell_literal(r"C:\Program Files\PawnIO\uninstall.exe");
        let upgrade_step = if upgrade {
            format!(
                "$u='{uninstall}'; if (!(Test-Path -LiteralPath $u)) {{ exit 2 }}; $old=Start-Process -FilePath $u -ArgumentList '-uninstall','-silent' -Wait -PassThru; if ($old.ExitCode -ne 0) {{ exit $old.ExitCode }};"
            )
        } else {
            String::new()
        };
        let register_task = format!(
            "$user=[System.Security.Principal.WindowsIdentity]::GetCurrent().Name; $launcherXml=[System.Security.SecurityElement]::Escape('{launcher}'); $xml='<Task version=\"1.4\" xmlns=\"http://schemas.microsoft.com/windows/2004/02/mit/task\"><RegistrationInfo><Author>ThrottleWatch</Author><Description>Lanzador elevado bajo demanda del sidecar de ThrottleWatch</Description></RegistrationInfo><Triggers /><Principals><Principal id=\"Author\"><UserId>'+ $user +'</UserId><LogonType>InteractiveToken</LogonType><RunLevel>HighestAvailable</RunLevel></Principal></Principals><Settings><MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy><DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries><StopIfGoingOnBatteries>false</StopIfGoingOnBatteries><AllowHardTerminate>true</AllowHardTerminate><StartWhenAvailable>true</StartWhenAvailable><ExecutionTimeLimit>PT0S</ExecutionTimeLimit></Settings><Actions Context=\"Author\"><Exec><Command>'+ $launcherXml +'</Command><Arguments>--elevated-launcher</Arguments></Exec></Actions></Task>'; Register-ScheduledTask -TaskName '{task_name}' -Xml $xml -Force | Out-Null; if ($?) {{ exit 0 }}; exit 1"
        );
        let diagnostic = std::env::temp_dir().join("ThrottleWatch-pawnio-bootstrap.log");
        let diagnostic = powershell_literal(&diagnostic.display().to_string());
        let script = format!(
            "$ErrorActionPreference='Stop'; try {{ {upgrade_step} $p=Start-Process -FilePath '{installer}' -ArgumentList '-install','-silent' -Wait -PassThru; if (($p.ExitCode -ne 0) -and ($p.ExitCode -ne 183)) {{ exit $p.ExitCode }}; {register_task} }} catch {{ ($_ | Out-String) | Set-Content -LiteralPath '{diagnostic}'; exit 1 }}"
        );
        let encoded_script = encode_utf16_base64(&script);
        let powershell = r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe";
        let elevated_arguments = format!(
            "$args=@('-NoProfile','-NonInteractive','-EncodedCommand','{encoded_script}'); $p=Start-Process -FilePath '{powershell}' -Verb RunAs -ArgumentList $args -Wait -PassThru; exit $p.ExitCode"
        );
        let status = Command::new(powershell)
            .args(["-NoProfile", "-NonInteractive", "-Command", &elevated_arguments])
            .status()?;
        if !status.success() {
            let details_path = std::env::temp_dir().join("ThrottleWatch-pawnio-bootstrap.log");
            let details = std::fs::read_to_string(&details_path).unwrap_or_default();
            let _ = std::fs::remove_file(details_path);
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!(
                    "PawnIO bootstrap UAC launch was rejected (exit code {}): {}",
                    status.code().unwrap_or(-1),
                    details.trim()
                ),
            ));
        }
        Ok(())
    }

    #[cfg(not(windows))]
    {
        let _ = (installer, launcher, action);
        Err(io::Error::new(io::ErrorKind::Unsupported, "advanced access is Windows-only"))
    }
}

#[cfg(windows)]
fn powershell_literal(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(windows)]
fn encode_utf16_base64(value: &str) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = value.encode_utf16().flat_map(u16::to_le_bytes).collect::<Vec<_>>();
    let mut encoded = String::new();
    for chunk in bytes.chunks(3) {
        let first = chunk.first().copied().unwrap_or(0);
        let second = chunk.get(1).copied().unwrap_or(0);
        let third = chunk.get(2).copied().unwrap_or(0);
        let number = (u32::from(first) << 16) | (u32::from(second) << 8) | u32::from(third);
        encoded.push(ALPHABET[((number >> 18) & 0x3f) as usize] as char);
        encoded.push(ALPHABET[((number >> 12) & 0x3f) as usize] as char);
        encoded.push(if chunk.len() > 1 {
            ALPHABET[((number >> 6) & 0x3f) as usize] as char
        } else {
            '='
        });
        encoded.push(if chunk.len() > 2 {
            ALPHABET[(number & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    encoded
}

fn verify_authenticode(path: &Path, expected_subject: &str) -> io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
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
            let path_literal = powershell_literal(&path.display().to_string());
            let subject_literal = powershell_literal(expected_subject);
            let script = format!(
                "$s=Get-AuthenticodeSignature -LiteralPath '{path_literal}'; if ($s.Status -ne 'Valid') {{ exit 1 }}; if ($s.SignerCertificate.Subject -ne '{subject_literal}') {{ exit 2 }}"
            );
            let powershell =
                Path::new(r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe");
            let checked = Command::new(powershell)
                .args(["-NoProfile", "-NonInteractive", "-Command", &script])
                .status()?;
            if checked.success() {
                return Ok(());
            }
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
    use super::{
        AccessAction, AccessRequest, PawnIoInstallation, action_is_compatible, compare_versions,
        manifest,
    };

    /// T112: the bundled installer has to sit exactly where `installer_path` looks for it, which
    /// is `resource_dir()` — the directory holding the executable — joined with the manifest's
    /// `installer`. It did not: `tauri.conf.json` shipped it under `resources\pawnio\…` while
    /// this resolves to `<exe>\pawnio\…`, so the real application could never find it and
    /// "install advanced access" (FR-087) failed in dev and in the installed product alike. Only
    /// the fake bridge covered that flow, so no test saw it. This keeps the two ends nailed
    /// together: whoever changes the bundle destination has to change the manifest to match.
    #[test]
    fn the_bundled_installer_is_looked_for_directly_under_the_resource_directory() {
        let value = manifest().unwrap_or_else(|error| panic!("manifest: {error}"));
        let resolved = super::installer_path(std::path::Path::new(r"C:\app"), &value);
        assert_eq!(
            resolved,
            std::path::Path::new(r"C:\app").join("pawnio/PawnIO_setup_2.2.0.exe"),
            "the manifest's installer path is relative to the resource directory, not to a \
             `resources` subfolder; keep tauri.conf.json's bundle destination in step"
        );
        assert!(
            !value.installer.replace('\\', "/").starts_with("resources/"),
            "prefixing the manifest with `resources/` would put the file one level below where \
             `resource_dir()` points"
        );
    }

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

    #[test]
    fn version_comparison_treats_missing_components_as_zero() {
        assert!(compare_versions("2.2.0.0", "2.2.0").is_eq());
        assert!(compare_versions("2.1.9", "2.2.0").is_lt());
        assert!(compare_versions("2.3.0", "2.2.0").is_gt());
    }

    #[test]
    fn installation_states_are_explicit() {
        assert_eq!(
            PawnIoInstallation::Upgradable { version: "2.1.0".to_owned() },
            PawnIoInstallation::Upgradable { version: "2.1.0".to_owned() }
        );
        assert_ne!(
            PawnIoInstallation::Missing,
            PawnIoInstallation::Current { version: "2.2.0".to_owned() }
        );
    }

    #[test]
    fn actions_only_apply_to_the_matching_installation_state() {
        assert!(action_is_compatible(AccessAction::Install, &PawnIoInstallation::Missing));
        assert!(action_is_compatible(
            AccessAction::Upgrade,
            &PawnIoInstallation::Upgradable { version: "2.1.0".to_owned() }
        ));
        assert!(action_is_compatible(
            AccessAction::Repair,
            &PawnIoInstallation::Current { version: "2.2.0".to_owned() }
        ));
        assert!(!action_is_compatible(
            AccessAction::Install,
            &PawnIoInstallation::Current { version: "2.2.0".to_owned() }
        ));
    }

    #[cfg(windows)]
    #[test]
    fn registry_output_parser_requires_the_pawnio_display_version() {
        assert_eq!(
            super::parse_display_version_line("    DisplayVersion    REG_SZ    2.2.0.0"),
            Some("2.2.0.0".to_owned())
        );
        assert_eq!(
            super::parse_display_version_line(
                "    InstallLocation    REG_SZ    C:\\Program Files\\PawnIO"
            ),
            None
        );
    }
}
