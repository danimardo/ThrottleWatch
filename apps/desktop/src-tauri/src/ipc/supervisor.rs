use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

use super::elevated;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupervisorState {
    Stopped,
    Starting,
    Running,
    Degraded,
    Exited,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupervisorMode {
    Child,
    RegisteredElevatedTask,
}

pub struct Supervisor {
    executable: std::path::PathBuf,
    expected_sha256: String,
    policy: RestartPolicy,
    attempts: Vec<Instant>,
    child: Option<Child>,
    control_pipe: Option<File>,
    state: SupervisorState,
    mode: SupervisorMode,
}

impl Supervisor {
    pub fn new(executable: impl Into<std::path::PathBuf>, expected_sha256: String) -> Self {
        Self {
            executable: executable.into(),
            expected_sha256,
            policy: RestartPolicy::default(),
            attempts: Vec::new(),
            child: None,
            control_pipe: None,
            state: SupervisorState::Stopped,
            mode: SupervisorMode::Child,
        }
    }

    pub fn with_policy(mut self, policy: RestartPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn with_elevated_task(mut self) -> Self {
        self.mode = SupervisorMode::RegisteredElevatedTask;
        self
    }

    pub fn state(&self) -> SupervisorState {
        self.state
    }

    pub fn take_stdin(&mut self) -> io::Result<ChildStdin> {
        self.child.as_mut().and_then(|child| child.stdin.take()).ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotConnected, "sidecar stdin is unavailable")
        })
    }

    pub fn take_stdout(&mut self) -> io::Result<ChildStdout> {
        self.child.as_mut().and_then(|child| child.stdout.take()).ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotConnected, "sidecar stdout is unavailable")
        })
    }

    pub fn take_stderr(&mut self) -> io::Result<ChildStderr> {
        self.child.as_mut().and_then(|child| child.stderr.take()).ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotConnected, "sidecar stderr is unavailable")
        })
    }

    pub fn read_validated_stderr(&mut self) -> io::Result<Option<crate::logging::LogEvent>> {
        let mut stderr = BufReader::new(self.take_stderr()?);
        let Some(line) = read_line_or_eof(&mut stderr)? else {
            return Ok(None);
        };
        crate::logging::validate_collector_stderr(&line)
            .map(Some)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }

    pub fn start(&mut self) -> io::Result<()> {
        if self.child.is_some() {
            return Ok(());
        }
        self.state = SupervisorState::Starting;
        let result = match self.mode {
            SupervisorMode::Child => spawn_verified(&self.executable, &self.expected_sha256)
                .map(|child| (Some(child), None)),
            SupervisorMode::RegisteredElevatedTask => {
                elevated::start_registered_sidecar(&self.executable, &self.expected_sha256)
                    .map(|pipe| (None, Some(pipe)))
            }
        };
        match result {
            Ok((child, control_pipe)) => {
                self.child = child;
                self.control_pipe = control_pipe;
                self.state = SupervisorState::Running;
                Ok(())
            }
            Err(error) => {
                self.state = SupervisorState::Degraded;
                Err(error)
            }
        }
    }

    pub fn poll(&mut self) -> io::Result<SupervisorState> {
        let Some(child) = self.child.as_mut() else {
            return Ok(if self.control_pipe.is_some() {
                SupervisorState::Running
            } else {
                self.state
            });
        };
        if child.try_wait()?.is_some() {
            self.child = None;
            self.state = SupervisorState::Exited;
        } else {
            self.state = SupervisorState::Running;
        }
        Ok(self.state)
    }

    pub fn restart(&mut self, now: Instant) -> io::Result<bool> {
        if !self.policy.can_restart(&mut self.attempts, now) {
            self.state = SupervisorState::Degraded;
            return Ok(false);
        }
        if self.child.is_some() || self.control_pipe.is_some() {
            self.stop()?;
        }
        self.start().map(|()| true)
    }

    pub fn stop(&mut self) -> io::Result<()> {
        self.control_pipe.take();
        if let Some(mut child) = self.child.take() {
            child.kill()?;
            child.wait()?;
        }
        self.state = SupervisorState::Stopped;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RestartPolicy {
    pub max_restarts: usize,
    pub window: Duration,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
}

impl Default for RestartPolicy {
    fn default() -> Self {
        Self {
            max_restarts: 3,
            window: Duration::from_secs(10 * 60),
            initial_backoff: Duration::from_millis(250),
            max_backoff: Duration::from_secs(8),
        }
    }
}

impl RestartPolicy {
    pub fn can_restart(&self, attempts: &mut Vec<Instant>, now: Instant) -> bool {
        attempts.retain(|attempt| now.duration_since(*attempt) <= self.window);
        if attempts.len() >= self.max_restarts {
            return false;
        }
        attempts.push(now);
        true
    }

    pub fn backoff(&self, attempt: usize) -> Duration {
        let multiplier = 2u32.saturating_pow(attempt.min(31) as u32);
        self.initial_backoff
            .checked_mul(multiplier)
            .unwrap_or(self.max_backoff)
            .min(self.max_backoff)
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn spawn_verified(executable: &Path, expected_sha256: &str) -> io::Result<Child> {
    let bytes = std::fs::read(executable)?;
    if !sha256_hex(&bytes).eq_ignore_ascii_case(expected_sha256) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "sidecar hash does not match the expected digest",
        ));
    }

    Command::new(executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
}

/// Reads one newline-delimited IPC message. `None` means that the sidecar closed stdout.
pub fn read_line_or_eof<R: BufRead>(reader: &mut R) -> io::Result<Option<String>> {
    let mut line = String::new();
    let bytes = reader.read_line(&mut line)?;
    if bytes == 0 {
        return Ok(None);
    }
    Ok(Some(line.trim_end_matches(['\r', '\n']).to_owned()))
}

pub fn watchdog_expired(last_sample: Instant, now: Instant, expected_interval: Duration) -> bool {
    now.duration_since(last_sample) > expected_interval.saturating_mul(3)
}

#[cfg(windows)]
pub fn parent_process_alive(parent_pid: u32) -> bool {
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

    // SAFETY: OpenProcess only queries a handle for the supplied PID and the
    // returned handle is owned by the temporary result and closed on drop.
    unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, parent_pid).is_ok() }
}

#[cfg(not(windows))]
pub fn parent_process_alive(parent_pid: u32) -> bool {
    parent_pid != 0
}

#[cfg(test)]
mod tests {
    use super::{
        RestartPolicy, parent_process_alive, read_line_or_eof, sha256_hex, watchdog_expired,
    };
    use std::io::Cursor;
    use std::time::{Duration, Instant};

    #[test]
    fn computes_a_stable_sha256_digest() {
        assert_eq!(
            sha256_hex(b"ThrottleWatch"),
            "1957305d4217cda038941abf2cd6918281cba7e0913a3f8280f9dab85419ff3e"
        );
    }

    #[test]
    fn limits_restarts_to_the_configured_window() {
        let policy = RestartPolicy {
            max_restarts: 2,
            window: Duration::from_secs(10),
            ..RestartPolicy::default()
        };
        let now = Instant::now();
        let mut attempts = Vec::new();

        assert!(policy.can_restart(&mut attempts, now));
        assert!(policy.can_restart(&mut attempts, now + Duration::from_secs(1)));
        assert!(!policy.can_restart(&mut attempts, now + Duration::from_secs(2)));
        assert!(policy.can_restart(&mut attempts, now + Duration::from_secs(11)));
    }

    #[test]
    fn backoff_is_capped() {
        let policy = RestartPolicy::default();
        assert_eq!(policy.backoff(0), Duration::from_millis(250));
        assert_eq!(policy.backoff(10), Duration::from_secs(8));
    }

    #[test]
    fn watchdog_expires_after_three_intervals() {
        let start = Instant::now();
        assert!(!watchdog_expired(start, start + Duration::from_secs(3), Duration::from_secs(1)));
        assert!(watchdog_expired(start, start + Duration::from_secs(4), Duration::from_secs(1)));
    }

    #[test]
    fn detects_messages_and_stdout_eof() {
        let mut reader = Cursor::new(b"sample\r\n".to_vec());
        assert_eq!(read_line_or_eof(&mut reader).ok(), Some(Some("sample".to_owned())));
        assert_eq!(read_line_or_eof(&mut reader).ok(), Some(None));
    }

    #[test]
    fn treats_a_missing_parent_pid_as_dead() {
        assert!(!parent_process_alive(0));
    }
}
