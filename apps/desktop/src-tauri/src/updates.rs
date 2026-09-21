//! Voluntary signed updates (FR-053, FR-056, FR-058, ADR-0005): the sequence check → download
//! (signature verified) → install, the 24 h cadence, the zero-traffic guarantee while the updater
//! is off, and the operations that must finish before an install.
//!
//! [`UpdateService`] is pure over an [`UpdateTransport`]: the real transport is the official
//! `tauri-plugin-updater` against one fixed endpoint; tests use a fake and a local server. The
//! interface never provides a URL, a path, a key or a channel.
#![deny(clippy::unwrap_used, clippy::expect_used)]

use crate::updater::{UpdateError, UpdateMachine, UpdateState};
use serde::Serialize;

/// The only endpoint the updater talks to (ADR-0005). It is not a parameter of any command.
pub const ENDPOINT: &str =
    "https://github.com/ThrottleWatch/ThrottleWatch/releases/latest/download/update-manifest.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseInfo {
    pub version: String,
    pub notes: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportError {
    /// No trusted public key in this build: nothing may be downloaded or installed.
    NoTrustedKey,
    Network,
    /// The artifact does not match its signature: it is discarded, never installed.
    SignatureInvalid,
    Install,
}

impl TransportError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::NoTrustedKey => "update.no_trusted_key",
            Self::Network => "update.network",
            Self::SignatureInvalid => "update.signature_invalid",
            Self::Install => "update.install_failed",
        }
    }

    /// Whether trying again can help: a network hiccup can, a bad signature or a missing key
    /// cannot.
    pub const fn recoverable(self) -> bool {
        matches!(self, Self::Network | Self::Install)
    }
}

pub trait UpdateTransport: Send {
    /// Asks the fixed endpoint for a newer release.
    fn check(&mut self) -> Result<Option<ReleaseInfo>, TransportError>;
    /// Downloads the release found by the last check and verifies its signature; the bytes stay
    /// inside the transport. Returns their length. Fails without keeping anything if the
    /// signature is wrong.
    fn download(
        &mut self,
        progress: &mut dyn FnMut(u64, Option<u64>),
    ) -> Result<u64, TransportError>;
    /// Runs the verified artifact.
    fn install(&mut self) -> Result<(), TransportError>;
    /// Drops what was downloaded (a failure, a new check).
    fn discard(&mut self);
}

/// Why an install must wait (`update.blocked_by_running_operation { reason }`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blocker {
    GuidedTest,
    Export,
    Import,
    DataOperation,
}

impl Blocker {
    pub const fn reason(self) -> &'static str {
        match self {
            Self::GuidedTest => "guided_test",
            Self::Export => "export",
            Self::Import => "import",
            Self::DataOperation => "data_operation",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateFailure {
    Machine(UpdateError),
    Transport(TransportError),
    Blocked(Blocker),
}

impl UpdateFailure {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Machine(error) => error.code(),
            Self::Transport(error) => error.code(),
            Self::Blocked(_) => "update.blocked_by_running_operation",
        }
    }
}

/// What the interface sees of the updater.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpdateStateDto {
    pub state: &'static str,
    pub enabled: bool,
    pub current_version: &'static str,
    pub last_check: Option<String>,
    pub version: Option<String>,
    pub notes: Option<String>,
    pub downloaded: Option<u64>,
    pub total: Option<u64>,
    pub error_code: Option<String>,
    pub recoverable: Option<bool>,
}

pub struct UpdateService {
    machine: UpdateMachine,
    transport: Box<dyn UpdateTransport>,
    current_version: &'static str,
}

impl UpdateService {
    pub fn new(
        machine: UpdateMachine,
        transport: Box<dyn UpdateTransport>,
        current_version: &'static str,
    ) -> Self {
        Self { machine, transport, current_version }
    }

    pub fn machine(&self) -> &UpdateMachine {
        &self.machine
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        if !enabled {
            self.transport.discard();
        }
        self.machine.set_enabled(enabled);
    }

    pub fn automatic_check_due(&self, now: &str) -> bool {
        self.machine.automatic_check_allowed(now)
    }

    pub fn dto(&self) -> UpdateStateDto {
        let mut dto = UpdateStateDto {
            state: "idle",
            enabled: self.machine.enabled(),
            current_version: self.current_version,
            last_check: self.machine.last_check().map(str::to_owned),
            version: None,
            notes: None,
            downloaded: None,
            total: None,
            error_code: None,
            recoverable: None,
        };
        match self.machine.state() {
            UpdateState::Idle => {}
            UpdateState::Checking => dto.state = "checking",
            UpdateState::UpToDate => dto.state = "up_to_date",
            UpdateState::Available { version, notes } => {
                dto.state = "available";
                dto.version = Some(version.clone());
                dto.notes = Some(notes.clone());
            }
            UpdateState::Downloading { version, downloaded, total } => {
                dto.state = "downloading";
                dto.version = Some(version.clone());
                dto.downloaded = Some(*downloaded);
                dto.total = *total;
            }
            UpdateState::Verified { version } => {
                dto.state = "verified";
                dto.version = Some(version.clone());
            }
            UpdateState::Installing { version } => {
                dto.state = "installing";
                dto.version = Some(version.clone());
            }
            UpdateState::Error { code, recoverable } => {
                dto.state = "error";
                dto.error_code = Some(code.clone());
                dto.recoverable = Some(*recoverable);
            }
        }
        dto
    }

    fn fail(&mut self, error: TransportError) -> UpdateFailure {
        self.transport.discard();
        self.machine.fail(error.code().to_owned(), error.recoverable());
        UpdateFailure::Transport(error)
    }

    /// Checks for a newer release. With the updater off this returns before any transport call,
    /// so nothing leaves the machine (constitution, quality gate 12).
    pub fn check(
        &mut self,
        now: &str,
        manual: bool,
        notify: &mut dyn FnMut(&UpdateStateDto),
    ) -> Result<Option<ReleaseInfo>, UpdateFailure> {
        self.machine.begin_check(now, manual).map_err(UpdateFailure::Machine)?;
        notify(&self.dto());
        self.transport.discard();
        let outcome = match self.transport.check() {
            Ok(release) => {
                self.machine
                    .finish_check(
                        release.as_ref().map(|item| (item.version.clone(), item.notes.clone())),
                    )
                    .map_err(UpdateFailure::Machine)?;
                Ok(release)
            }
            Err(error) => Err(self.fail(error)),
        };
        notify(&self.dto());
        outcome
    }

    /// Downloads the release the last check found. `verified` is reached only when the transport
    /// confirms the signature; any failure discards the partial artifact.
    pub fn download(
        &mut self,
        progress: &mut dyn FnMut(&UpdateStateDto),
    ) -> Result<(), UpdateFailure> {
        self.machine.begin_download().map_err(UpdateFailure::Machine)?;
        progress(&self.dto());
        let current_version = self.current_version;
        let enabled = self.machine.enabled();
        let machine = &mut self.machine;
        let mut report = |downloaded: u64, total: Option<u64>| {
            if machine.report_download(downloaded, total).is_ok() {
                let dto = UpdateStateDto {
                    state: "downloading",
                    enabled,
                    current_version,
                    last_check: machine.last_check().map(str::to_owned),
                    version: match machine.state() {
                        UpdateState::Downloading { version, .. } => Some(version.clone()),
                        _ => None,
                    },
                    notes: None,
                    downloaded: Some(downloaded),
                    total,
                    error_code: None,
                    recoverable: None,
                };
                progress(&dto);
            }
        };
        let outcome = match self.transport.download(&mut report) {
            Ok(length) => self
                .machine
                .report_download(length, Some(length))
                .and_then(|()| self.machine.mark_verified())
                .map_err(UpdateFailure::Machine),
            Err(error) => Err(self.fail(error)),
        };
        progress(&self.dto());
        outcome
    }

    /// Installs the verified artifact, unless an operation that must not be cut is running. The
    /// blocker is checked first: a refused install changes nothing.
    pub fn install(
        &mut self,
        blocker: Option<Blocker>,
        notify: &mut dyn FnMut(&UpdateStateDto),
    ) -> Result<(), UpdateFailure> {
        if !matches!(self.machine.state(), UpdateState::Verified { .. }) {
            return Err(UpdateFailure::Machine(UpdateError::NotVerified));
        }
        if let Some(blocker) = blocker {
            return Err(UpdateFailure::Blocked(blocker));
        }
        self.machine.begin_install().map_err(UpdateFailure::Machine)?;
        notify(&self.dto());
        match self.transport.install() {
            Ok(()) => Ok(()),
            Err(error) => {
                let failure = self.fail(error);
                notify(&self.dto());
                Err(failure)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Blocker, ReleaseInfo, TransportError, UpdateFailure, UpdateService, UpdateTransport,
    };
    use crate::updater::{UpdateError, UpdateMachine};
    use std::sync::{Arc, Mutex};

    const NOW: &str = "2026-09-21T08:00:00Z";
    const LATER: &str = "2026-09-22T09:00:00Z";

    #[derive(Default)]
    struct Calls {
        checks: usize,
        downloads: usize,
        installs: usize,
        discards: usize,
    }

    struct Fake {
        calls: Arc<Mutex<Calls>>,
        release: Option<ReleaseInfo>,
        check_error: Option<TransportError>,
        download_error: Option<TransportError>,
        install_error: Option<TransportError>,
    }

    impl Fake {
        fn boxed(calls: &Arc<Mutex<Calls>>) -> Box<Self> {
            Box::new(Self {
                calls: Arc::clone(calls),
                release: Some(ReleaseInfo {
                    version: "1.2.0".to_owned(),
                    notes: "Fixes".to_owned(),
                }),
                check_error: None,
                download_error: None,
                install_error: None,
            })
        }
    }

    impl UpdateTransport for Fake {
        fn check(&mut self) -> Result<Option<ReleaseInfo>, TransportError> {
            self.calls.lock().map(|mut calls| calls.checks += 1).ok();
            self.check_error.map_or_else(|| Ok(self.release.clone()), Err)
        }

        fn download(
            &mut self,
            progress: &mut dyn FnMut(u64, Option<u64>),
        ) -> Result<u64, TransportError> {
            self.calls.lock().map(|mut calls| calls.downloads += 1).ok();
            progress(40, Some(100));
            if let Some(error) = self.download_error {
                return Err(error);
            }
            progress(100, Some(100));
            Ok(100)
        }

        fn install(&mut self) -> Result<(), TransportError> {
            self.calls.lock().map(|mut calls| calls.installs += 1).ok();
            self.install_error.map_or(Ok(()), Err)
        }

        fn discard(&mut self) {
            self.calls.lock().map(|mut calls| calls.discards += 1).ok();
        }
    }

    fn service(enabled: bool, fake: Box<Fake>) -> UpdateService {
        UpdateService::new(UpdateMachine::new(enabled), fake, "1.0.0")
    }

    fn calls(shared: &Arc<Mutex<Calls>>) -> (usize, usize, usize) {
        shared.lock().map(|calls| (calls.checks, calls.downloads, calls.installs)).unwrap_or((
            usize::MAX,
            0,
            0,
        ))
    }

    #[test]
    fn with_the_updater_off_nothing_reaches_the_transport() {
        let shared = Arc::new(Mutex::new(Calls::default()));
        let mut service = service(false, Fake::boxed(&shared));
        for manual in [false, true] {
            assert_eq!(
                service.check(NOW, manual, &mut |_| {}),
                Err(UpdateFailure::Machine(UpdateError::Disabled))
            );
        }
        assert_eq!(
            service.download(&mut |_| {}),
            Err(UpdateFailure::Machine(UpdateError::NoAvailableUpdate))
        );
        assert_eq!(
            service.install(None, &mut |_| {}),
            Err(UpdateFailure::Machine(UpdateError::NotVerified))
        );
        assert_eq!(calls(&shared), (0, 0, 0), "zero traffic while off");
        assert!(!service.automatic_check_due(LATER));
    }

    #[test]
    fn the_whole_sequence_is_check_then_download_then_install_each_by_a_gesture() {
        let shared = Arc::new(Mutex::new(Calls::default()));
        let mut service = service(true, Fake::boxed(&shared));

        let release =
            service.check(NOW, false, &mut |_| {}).unwrap_or_else(|error| panic!("{error:?}"));
        assert_eq!(release.map(|item| item.version), Some("1.2.0".to_owned()));
        assert_eq!(service.dto().state, "available");
        assert_eq!(calls(&shared), (1, 0, 0), "a check never downloads");

        let mut progress = Vec::new();
        service
            .download(&mut |dto| progress.push((dto.downloaded, dto.total)))
            .unwrap_or_else(|error| panic!("{error:?}"));
        assert_eq!(
            progress,
            vec![(Some(0), None), (Some(40), Some(100)), (Some(100), Some(100)), (None, None)],
            "starts, reports progress, and ends in `verified` (no byte counts)"
        );
        assert_eq!(service.dto().state, "verified");
        assert_eq!(calls(&shared), (1, 1, 0), "a download never installs");

        service.install(None, &mut |_| {}).unwrap_or_else(|error| panic!("{error:?}"));
        assert_eq!(service.dto().state, "installing");
        assert_eq!(calls(&shared), (1, 1, 1));
    }

    #[test]
    fn a_bad_signature_discards_the_artifact_and_can_never_be_installed() {
        let shared = Arc::new(Mutex::new(Calls::default()));
        let mut fake = Fake::boxed(&shared);
        fake.download_error = Some(TransportError::SignatureInvalid);
        let mut service = service(true, fake);
        service.check(NOW, true, &mut |_| {}).ok();

        let result = service.download(&mut |_| {});

        assert_eq!(result, Err(UpdateFailure::Transport(TransportError::SignatureInvalid)));
        let dto = service.dto();
        assert_eq!(dto.state, "error");
        assert_eq!(dto.error_code.as_deref(), Some("update.signature_invalid"));
        assert_eq!(dto.recoverable, Some(false));
        assert_eq!(
            service.install(None, &mut |_| {}),
            Err(UpdateFailure::Machine(UpdateError::NotVerified))
        );
        assert_eq!(calls(&shared).2, 0);
        assert!(shared.lock().map(|calls| calls.discards >= 1).unwrap_or(false));
    }

    #[test]
    fn a_network_failure_is_isolated_and_recoverable() {
        let shared = Arc::new(Mutex::new(Calls::default()));
        let mut fake = Fake::boxed(&shared);
        fake.check_error = Some(TransportError::Network);
        let mut service = service(true, fake);

        assert_eq!(
            service.check(NOW, true, &mut |_| {}),
            Err(UpdateFailure::Transport(TransportError::Network))
        );
        let dto = service.dto();
        assert_eq!((dto.state, dto.recoverable), ("error", Some(true)));
        assert_eq!(dto.last_check.as_deref(), Some(NOW), "the attempt is remembered");
    }

    #[test]
    fn a_running_operation_blocks_the_install_without_changing_anything() {
        let shared = Arc::new(Mutex::new(Calls::default()));
        let mut service = service(true, Fake::boxed(&shared));
        service.check(NOW, true, &mut |_| {}).ok();
        service.download(&mut |_| {}).ok();

        for blocker in
            [Blocker::GuidedTest, Blocker::Export, Blocker::Import, Blocker::DataOperation]
        {
            let result = service.install(Some(blocker), &mut |_| {});
            assert_eq!(result, Err(UpdateFailure::Blocked(blocker)));
            assert_eq!(service.dto().state, "verified", "still ready once the operation ends");
        }
        assert_eq!(calls(&shared).2, 0);
        assert_eq!(Blocker::GuidedTest.reason(), "guided_test");
        assert!(service.install(None, &mut |_| {}).is_ok());
    }

    #[test]
    fn the_automatic_check_waits_for_the_cadence_and_the_manual_one_does_not() {
        let shared = Arc::new(Mutex::new(Calls::default()));
        let mut service = service(true, Fake::boxed(&shared));
        assert!(service.automatic_check_due(NOW));
        service.check(NOW, false, &mut |_| {}).ok();
        assert!(!service.automatic_check_due("2026-09-21T20:00:00Z"));
        assert!(service.automatic_check_due(LATER));
        assert!(
            service.check("2026-09-21T20:00:00Z", true, &mut |_| {}).is_ok(),
            "manual retries at once"
        );
    }

    #[test]
    fn turning_the_updater_off_forgets_what_was_downloaded() {
        let shared = Arc::new(Mutex::new(Calls::default()));
        let mut service = service(true, Fake::boxed(&shared));
        service.check(NOW, true, &mut |_| {}).ok();
        service.download(&mut |_| {}).ok();
        service.set_enabled(false);
        assert_eq!(service.dto().state, "idle");
        assert!(!service.dto().enabled);
        assert_eq!(
            service.install(None, &mut |_| {}),
            Err(UpdateFailure::Machine(UpdateError::NotVerified))
        );
    }

    #[test]
    fn an_install_that_fails_ends_in_a_recoverable_error() {
        let shared = Arc::new(Mutex::new(Calls::default()));
        let mut fake = Fake::boxed(&shared);
        fake.install_error = Some(TransportError::Install);
        let mut service = service(true, fake);
        service.check(NOW, true, &mut |_| {}).ok();
        service.download(&mut |_| {}).ok();
        assert_eq!(
            service.install(None, &mut |_| {}),
            Err(UpdateFailure::Transport(TransportError::Install))
        );
        assert_eq!(service.dto().state, "error");
    }

    #[test]
    fn the_endpoint_is_https_and_fixed() {
        assert!(super::ENDPOINT.starts_with("https://github.com/ThrottleWatch/"));
    }
}
