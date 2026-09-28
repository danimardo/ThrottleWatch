//! Finds the sidecar and decides how it may be started.
//!
//! The shipped sidecar runs only if its SHA-256 equals the one in the release manifest, and the
//! manifest itself only counts with a valid minisign signature (ADR-0004 R2/C3; see
//! `docs/dev/release-signing.md`). A release build trusts no key yet, so it cannot start a collector
//! until the updater key exists; debug builds trust the development key.
#![deny(clippy::unwrap_used, clippy::expect_used)]

use super::runtime::{
    CollectorLauncher, CollectorLink, CommandLauncher, PipeLink, SidecarLauncher,
};
use crate::ipc::supervisor::spawn_verified;
use crate::release_manifest::{ManifestError, ReleaseManifest, trusted_public_key};
use std::io;
use std::path::{Path, PathBuf};

pub const SIDECAR_FILE: &str = "SensorAgent.exe";
const MANIFEST_FILE: &str = "release-manifest.json";
const SIGNATURE_FILE: &str = "release-manifest.json.minisig";

#[derive(Debug)]
pub enum LaunchError {
    SidecarNotFound,
    NoTrustedKey,
    Manifest(ManifestError),
    NotListed,
    Io(io::Error),
}

impl LaunchError {
    /// Stable code for the log; never carries paths.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::SidecarNotFound => "COLLECTOR_SIDECAR_NOT_FOUND",
            Self::NoTrustedKey => "COLLECTOR_NO_TRUSTED_KEY",
            Self::Manifest(_) => "COLLECTOR_MANIFEST_REJECTED",
            Self::NotListed => "COLLECTOR_NOT_IN_MANIFEST",
            Self::Io(_) => "COLLECTOR_MANIFEST_UNREADABLE",
        }
    }
}

/// The directories that may hold the sidecar, in order of preference.
pub fn candidate_directories() -> Vec<PathBuf> {
    let mut directories = Vec::new();
    if let Some(directory) =
        std::env::current_exe().ok().and_then(|exe| exe.parent().map(Path::to_path_buf))
    {
        // T112: the installer bundles the self-contained sidecar under `collector\` rather than
        // beside the executable, because a self-contained .NET publish is a couple of hundred
        // files. Security is unchanged: whichever directory wins, `locate_in` still demands a
        // manifest signed by a trusted key and a matching SHA-256 before anything is started.
        directories.push(directory.join("collector"));
        directories.push(directory);
    }
    #[cfg(debug_assertions)]
    directories.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../sensor-agent/bin/Debug/net10.0-windows"),
    );
    directories
}

/// Verifies the manifest in `directory` and returns the launcher for the sidecar next to it.
pub fn locate_in(
    directory: &Path,
    public_key: Option<&str>,
) -> Result<SidecarLauncher, LaunchError> {
    let executable = directory.join(SIDECAR_FILE);
    if !executable.is_file() {
        return Err(LaunchError::SidecarNotFound);
    }
    let key = public_key.ok_or(LaunchError::NoTrustedKey)?;
    let manifest_bytes = std::fs::read(directory.join(MANIFEST_FILE)).map_err(LaunchError::Io)?;
    let signature =
        std::fs::read_to_string(directory.join(SIGNATURE_FILE)).map_err(LaunchError::Io)?;
    let manifest =
        ReleaseManifest::verify(&manifest_bytes, &signature, key).map_err(LaunchError::Manifest)?;
    let expected_sha256 =
        manifest.sha256_for(Path::new(SIDECAR_FILE)).ok_or(LaunchError::NotListed)?.to_owned();
    Ok(SidecarLauncher { executable, expected_sha256 })
}

/// A launcher that reports why the sidecar cannot be started; the runtime turns it into `failed`.
struct Unavailable {
    code: &'static str,
}

impl CollectorLauncher for Unavailable {
    fn launch(&mut self) -> io::Result<Box<dyn CollectorLink>> {
        crate::log_warn!(self.code, "the collector cannot be started");
        Err(io::Error::from(io::ErrorKind::PermissionDenied))
    }
}

/// A sidecar started elevated on demand (FR-088), so a person on an account without an elevated
/// session still reaches coverage tier A once advanced access is installed — installing it is the
/// only UAC prompt SC-019 allows. Falls back to a plain, unelevated [`SidecarLauncher`] for this one
/// attempt when the elevated path itself fails (the scheduled task missing, Task Scheduler blocked
/// by policy, …): the collector still runs, only at a lower tier, and Ajustes already shows a
/// repair action for that (`advanced_access` reads `denied`/`error`, same as an unelevated sidecar
/// PawnIO itself refuses). The next restart tries elevated again, so a transient failure heals
/// itself instead of leaving the person degraded forever.
pub struct ElevatedSidecarLauncher {
    pub executable: PathBuf,
    pub expected_sha256: String,
}

impl CollectorLauncher for ElevatedSidecarLauncher {
    fn launch(&mut self) -> io::Result<Box<dyn CollectorLink>> {
        match crate::ipc::elevated::start_registered_sidecar(
            &self.executable,
            &self.expected_sha256,
        ) {
            Ok((to_launcher, from_launcher)) => {
                Ok(Box::new(PipeLink::connect(to_launcher, from_launcher)?))
            }
            Err(error) => {
                crate::log_warn!(
                    "COLLECTOR_ELEVATED_LAUNCH_FAILED",
                    format!(
                        "the elevated sidecar could not be started, falling back to an unelevated one this time: {error}"
                    )
                );
                let child = spawn_verified(&self.executable, &self.expected_sha256)?;
                Ok(Box::new(super::runtime::ProcessLink::from_child(child)?))
            }
        }
    }
}

/// Whether the collector should try the elevated launch path this run: advanced access is turned
/// on (the person has not disabled it in Ajustes) and PawnIO is installed at a version this build
/// trusts. `Upgradable` stays unelevated — an old client library reading the wrong register
/// offsets is worse than the coverage gap it would close.
pub fn should_launch_elevated(advanced_access_enabled: bool) -> bool {
    advanced_access_enabled
        && crate::access::manifest()
            .and_then(|manifest| crate::access::detect_installation(&manifest.minimum_version))
            .is_ok_and(|installation| {
                matches!(installation, crate::access::PawnIoInstallation::Current { .. })
            })
}

pub fn collector_launcher(advanced_access_enabled: bool) -> Box<dyn CollectorLauncher> {
    // T181: a debug/`e2e` build may be told to run a stand-in collector (`TW_DEV_COLLECTOR_CMD`), so
    // an E2E run has live telemetry without a signed manifest. It replaces the launcher and skips
    // the check below rather than weakening it; `dev_faults` answers `None` in a release build, so
    // this branch cannot be taken there. The warning keeps such a run from passing for a real one.
    // It is never elevated: the harness runs unattended and must not depend on a scheduled task.
    if let Some((program, args)) = crate::dev_faults::fake_collector_command() {
        crate::log_warn!(
            "COLLECTOR_TEST_DOUBLE_IN_USE",
            "TW_DEV_COLLECTOR_CMD replaces the collector with a test double"
        );
        return Box::new(CommandLauncher { program, args });
    }
    let elevate = should_launch_elevated(advanced_access_enabled);
    let mut last = LaunchError::SidecarNotFound;
    for directory in candidate_directories() {
        match locate_in(&directory, trusted_public_key()) {
            Ok(launcher) if elevate => {
                return Box::new(ElevatedSidecarLauncher {
                    executable: launcher.executable,
                    expected_sha256: launcher.expected_sha256,
                });
            }
            Ok(launcher) => return Box::new(launcher),
            // Keep the most informative reason: "not found" is the least.
            Err(error) if !matches!(error, LaunchError::SidecarNotFound) => last = error,
            Err(_) => {}
        }
    }
    Box::new(Unavailable { code: last.code() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::release_manifest::DEV_PUBLIC_KEY;

    const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/release-manifest");

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tw-launch-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap_or_else(|error| panic!("scratch: {error}"));
        dir
    }

    /// The signed fixture manifest lists `SensorAgent.exe` with the SHA-256 of the text "sidecar-fixture".
    fn install_fixture(dir: &Path, sidecar_bytes: &[u8]) {
        std::fs::write(dir.join(SIDECAR_FILE), sidecar_bytes)
            .unwrap_or_else(|error| panic!("write: {error}"));
        std::fs::copy(Path::new(FIXTURES).join(MANIFEST_FILE), dir.join(MANIFEST_FILE))
            .unwrap_or_else(|error| panic!("copy: {error}"));
        std::fs::copy(Path::new(FIXTURES).join(SIGNATURE_FILE), dir.join(SIGNATURE_FILE))
            .unwrap_or_else(|error| panic!("copy: {error}"));
    }

    #[test]
    fn a_signed_manifest_yields_the_launcher_with_the_hash_it_lists() {
        let dir = scratch("ok");
        install_fixture(&dir, b"sidecar-fixture");

        let launcher = locate_in(&dir, Some(DEV_PUBLIC_KEY))
            .unwrap_or_else(|error| panic!("locate: {error:?}"));

        assert_eq!(launcher.executable, dir.join(SIDECAR_FILE));
        assert_eq!(launcher.expected_sha256.len(), 64);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_release_build_without_a_trusted_key_cannot_start_the_collector() {
        let dir = scratch("nokey");
        install_fixture(&dir, b"sidecar-fixture");

        let error = locate_in(&dir, None).err().unwrap_or_else(|| panic!("must fail"));

        assert_eq!(error.code(), "COLLECTOR_NO_TRUSTED_KEY");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_manifest_signed_by_someone_else_is_rejected() {
        let dir = scratch("otherkey");
        install_fixture(&dir, b"sidecar-fixture");
        std::fs::copy(
            Path::new(FIXTURES).join("release-manifest.json.other.minisig"),
            dir.join(SIGNATURE_FILE),
        )
        .unwrap_or_else(|error| panic!("copy: {error}"));

        let error =
            locate_in(&dir, Some(DEV_PUBLIC_KEY)).err().unwrap_or_else(|| panic!("must fail"));

        assert_eq!(error.code(), "COLLECTOR_MANIFEST_REJECTED");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_sidecar_or_manifest_is_reported_without_starting_anything() {
        let dir = scratch("missing");
        assert_eq!(
            locate_in(&dir, Some(DEV_PUBLIC_KEY)).err().map(|error| error.code()),
            Some("COLLECTOR_SIDECAR_NOT_FOUND")
        );

        std::fs::write(dir.join(SIDECAR_FILE), b"x")
            .unwrap_or_else(|error| panic!("write: {error}"));
        assert_eq!(
            locate_in(&dir, Some(DEV_PUBLIC_KEY)).err().map(|error| error.code()),
            Some("COLLECTOR_MANIFEST_UNREADABLE")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_altered_sidecar_is_refused_by_the_verified_spawn() {
        let dir = scratch("altered");
        install_fixture(&dir, b"a different program");
        let mut launcher = locate_in(&dir, Some(DEV_PUBLIC_KEY))
            .unwrap_or_else(|error| panic!("locate: {error:?}"));

        let error = launcher.launch().err().unwrap_or_else(|| panic!("must refuse"));

        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
