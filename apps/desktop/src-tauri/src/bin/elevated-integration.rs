use std::env;
use std::io::{self, Read};
use std::path::PathBuf;
use std::process::ExitCode;
use std::thread;
use std::time::Duration;

use throttlewatch_lib::access::{self, AccessAction, PawnIoInstallation};
use throttlewatch_lib::ipc::elevated;
use throttlewatch_lib::ipc::supervisor::sha256_hex;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join("elevated-integration-error.txt");
            let _ = std::fs::write(path, error.to_string());
            ExitCode::FAILURE
        }
    }
}

fn run() -> io::Result<()> {
    match env::args().nth(1).as_deref() {
        Some("--elevated-launcher") => elevated::run_elevated_launcher(),
        Some("--sidecar") => {
            let mut input = Vec::new();
            io::stdin().read_to_end(&mut input)?;
            Ok(())
        }
        Some("--bootstrap") => bootstrap(),
        _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "unknown integration mode")),
    }
}

fn bootstrap() -> io::Result<()> {
    let manifest = access::manifest()?;
    let resources = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources");
    let installer = access::installer_path(&resources, &manifest);
    access::verify_installer(&installer, &manifest)?;
    let action = match access::detect_installation(&manifest.minimum_version)? {
        PawnIoInstallation::Missing => AccessAction::Install,
        PawnIoInstallation::Upgradable { .. } => AccessAction::Upgrade,
        PawnIoInstallation::Current { .. } => AccessAction::Repair,
    };
    let launcher = env::current_exe()?;
    access::launch_installer_and_register_task(&installer, &launcher, action)?;

    let sidecar = launcher.clone();
    let sidecar_hash = sha256_hex(&std::fs::read(&sidecar)?);
    for _ in 0..60 {
        match elevated::start_registered_sidecar(&sidecar, &sidecar_hash) {
            Ok(pipe) => {
                drop(pipe);
                return Ok(());
            }
            Err(_) => thread::sleep(Duration::from_millis(500)),
        }
    }
    Err(io::Error::new(io::ErrorKind::TimedOut, "elevated task did not start"))
}
