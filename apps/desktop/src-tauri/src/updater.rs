//! Máquina de estados del actualizador opt-in (T096).
//!
//! Este módulo no hace red ni toca el sistema de archivos. La capa de transporte
//! y el instalador se conectan después; aquí queda cerrada la secuencia segura de
//! estados y la cadencia de comprobación.
#![deny(clippy::unwrap_used, clippy::expect_used)]

use serde::{Deserialize, Serialize};

const CHECK_INTERVAL_SECONDS: i64 = 24 * 60 * 60;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum UpdateState {
    Idle,
    Checking,
    UpToDate,
    Available { version: String, notes: String },
    Downloading { version: String, downloaded: u64, total: Option<u64> },
    Verified { version: String },
    Installing { version: String },
    Error { code: String, recoverable: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateError {
    Disabled,
    CheckAlreadyRunning,
    InvalidTransition,
    NoAvailableUpdate,
    NotVerified,
    DownloadIncomplete,
    InstallAlreadyRunning,
}

impl UpdateError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Disabled => "update.disabled",
            Self::CheckAlreadyRunning => "update.check_already_running",
            Self::InvalidTransition => "update.invalid_transition",
            Self::NoAvailableUpdate => "update.no_available_update",
            Self::NotVerified => "update.not_verified",
            Self::DownloadIncomplete => "update.download_incomplete",
            Self::InstallAlreadyRunning => "update.install_already_running",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateMachine {
    enabled: bool,
    last_check: Option<String>,
    state: UpdateState,
}

impl Default for UpdateMachine {
    fn default() -> Self {
        Self::new(false)
    }
}

impl UpdateMachine {
    pub const fn new(enabled: bool) -> Self {
        Self { enabled, last_check: None, state: UpdateState::Idle }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.state = UpdateState::Idle;
        }
    }

    /// After a restart nothing of a check, a download or an install in flight survives (the
    /// artifact lived in memory): the state returns to `idle`, keeping the switch and the time
    /// of the last check that decides the cadence.
    pub fn recover(&mut self) {
        self.state = UpdateState::Idle;
    }

    pub fn state(&self) -> &UpdateState {
        &self.state
    }

    pub fn last_check(&self) -> Option<&str> {
        self.last_check.as_deref()
    }

    pub fn automatic_check_allowed(&self, now: &str) -> bool {
        if !self.enabled || matches!(self.state, UpdateState::Checking) {
            return false;
        }
        let Some(last) = self.last_check.as_deref() else { return true };
        let Ok(last) = last.parse::<jiff::Timestamp>() else { return true };
        let Ok(now) = now.parse::<jiff::Timestamp>() else { return false };
        now.since(last).ok().is_some_and(|elapsed| elapsed.get_seconds() >= CHECK_INTERVAL_SECONDS)
    }

    pub fn begin_check(&mut self, now: &str, manual: bool) -> Result<(), UpdateError> {
        if !self.enabled {
            return Err(UpdateError::Disabled);
        }
        if matches!(self.state, UpdateState::Checking) {
            return Err(UpdateError::CheckAlreadyRunning);
        }
        if !manual && !self.automatic_check_allowed(now) {
            return Err(UpdateError::InvalidTransition);
        }
        if now.parse::<jiff::Timestamp>().is_err() {
            return Err(UpdateError::InvalidTransition);
        }
        self.last_check = Some(now.to_owned());
        self.state = UpdateState::Checking;
        Ok(())
    }

    pub fn finish_check(&mut self, available: Option<(String, String)>) -> Result<(), UpdateError> {
        if !matches!(self.state, UpdateState::Checking) {
            return Err(UpdateError::InvalidTransition);
        }
        self.state = match available {
            Some((version, notes)) => UpdateState::Available { version, notes },
            None => UpdateState::UpToDate,
        };
        Ok(())
    }

    pub fn begin_download(&mut self) -> Result<(), UpdateError> {
        let UpdateState::Available { version, .. } = &self.state else {
            return Err(UpdateError::NoAvailableUpdate);
        };
        self.state =
            UpdateState::Downloading { version: version.clone(), downloaded: 0, total: None };
        Ok(())
    }

    pub fn report_download(
        &mut self,
        downloaded: u64,
        total: Option<u64>,
    ) -> Result<(), UpdateError> {
        let UpdateState::Downloading { version, downloaded: current, total: current_total } =
            &mut self.state
        else {
            return Err(UpdateError::InvalidTransition);
        };
        if let Some(total) = total {
            if total == 0 || downloaded > total {
                return Err(UpdateError::DownloadIncomplete);
            }
            *current_total = Some(total);
        }
        *current = downloaded.max(*current);
        let _ = version;
        Ok(())
    }

    pub fn mark_verified(&mut self) -> Result<(), UpdateError> {
        let UpdateState::Downloading { version, downloaded, total } = &self.state else {
            return Err(UpdateError::InvalidTransition);
        };
        if total.is_none() || total.is_some_and(|total| *downloaded < total) {
            return Err(UpdateError::DownloadIncomplete);
        }
        self.state = UpdateState::Verified { version: version.clone() };
        Ok(())
    }

    pub fn begin_install(&mut self) -> Result<(), UpdateError> {
        let UpdateState::Verified { version } = &self.state else {
            return Err(UpdateError::NotVerified);
        };
        self.state = UpdateState::Installing { version: version.clone() };
        Ok(())
    }

    pub fn finish_install(&mut self) -> Result<(), UpdateError> {
        if !matches!(self.state, UpdateState::Installing { .. }) {
            return Err(UpdateError::InstallAlreadyRunning);
        }
        self.state = UpdateState::Idle;
        Ok(())
    }

    pub fn fail(&mut self, code: String, recoverable: bool) {
        self.state = UpdateState::Error { code, recoverable };
    }
}

#[cfg(test)]
mod tests {
    use super::{UpdateError, UpdateMachine, UpdateState};

    const BEFORE: &str = "2026-09-20T08:00:00Z";
    const AFTER: &str = "2026-09-21T08:00:00Z";

    #[test]
    fn automatic_checks_are_opt_in_and_cadenced() {
        let mut machine = UpdateMachine::new(false);
        assert_eq!(machine.begin_check(BEFORE, false), Err(UpdateError::Disabled));
        machine.set_enabled(true);
        assert!(machine.begin_check(BEFORE, false).is_ok());
        assert_eq!(machine.finish_check(None), Ok(()));
        assert!(!machine.automatic_check_allowed(BEFORE));
        assert!(machine.automatic_check_allowed(AFTER));
    }

    #[test]
    fn manual_check_is_independent_of_the_automatic_cadence() {
        let mut machine = UpdateMachine::new(true);
        assert!(machine.begin_check(BEFORE, false).is_ok());
        assert!(machine.finish_check(None).is_ok());
        assert!(machine.begin_check(BEFORE, true).is_ok());
    }

    #[test]
    fn only_a_complete_download_can_be_verified_and_installed() {
        let mut machine = UpdateMachine::new(true);
        machine.begin_check(BEFORE, true).ok();
        machine.finish_check(Some(("1.2.0".to_owned(), "notes".to_owned()))).ok();
        machine.begin_download().ok();
        assert_eq!(machine.mark_verified(), Err(UpdateError::DownloadIncomplete));
        machine.report_download(5, Some(10)).ok();
        assert_eq!(machine.mark_verified(), Err(UpdateError::DownloadIncomplete));
        machine.report_download(10, Some(10)).ok();
        assert!(machine.mark_verified().is_ok());
        assert!(machine.begin_install().is_ok());
        assert!(machine.finish_install().is_ok());
        assert_eq!(machine.state(), &UpdateState::Idle);
    }

    #[test]
    fn a_restart_forgets_the_transient_state_but_keeps_the_cadence() {
        let mut machine = UpdateMachine::new(true);
        machine.begin_check(BEFORE, true).ok();
        machine.finish_check(Some(("1.2.0".to_owned(), String::new()))).ok();
        machine.begin_download().ok();
        machine.recover();
        assert_eq!(machine.state(), &UpdateState::Idle);
        assert!(machine.enabled());
        assert_eq!(machine.last_check(), Some(BEFORE));
        assert!(!machine.automatic_check_allowed(BEFORE));
    }

    #[test]
    fn invalid_timestamps_do_not_start_a_check() {
        let mut machine = UpdateMachine::new(true);
        assert_eq!(machine.begin_check("not-a-date", true), Err(UpdateError::InvalidTransition));
    }
}
