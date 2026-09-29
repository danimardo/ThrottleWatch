use sha2::{Digest, Sha256};
use std::io::{self, BufRead};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

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

/// A command for a console program that never gets a window. The application is a GUI process with
/// no console to lend, so Windows gave every console child (`SensorAgent.exe`, `reg.exe`,
/// `powershell.exe`, `schtasks.exe`, `sc.exe`) a new visible one: the black windows seen at start-up
/// and flashing whenever advanced access was checked (2026-09-28/29).
pub fn hidden_command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut command = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

pub fn spawn_verified(executable: &Path, expected_sha256: &str) -> io::Result<Child> {
    let bytes = std::fs::read(executable)?;
    if !sha256_hex(&bytes).eq_ignore_ascii_case(expected_sha256) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "sidecar hash does not match the expected digest",
        ));
    }

    hidden_command(executable)
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

/// Returns the process that launched the current process, when Windows exposes it in a snapshot.
/// The guided watchdog uses this as a best-effort liveness signal; a missing parent is never
/// treated as a reason to keep a diagnostic running indefinitely.
#[cfg(windows)]
pub fn current_parent_process_id() -> Option<u32> {
    use std::mem::size_of;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32, Process32First, Process32Next, TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::Threading::GetCurrentProcessId;

    // SAFETY: the snapshot is read-only, initialized with the documented entry size, and its
    // handle is closed on every successful path before returning.
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).ok()? };
    let current_pid = unsafe { GetCurrentProcessId() };
    let mut entry =
        PROCESSENTRY32 { dwSize: size_of::<PROCESSENTRY32>() as u32, ..Default::default() };
    // SAFETY: `entry` is a valid writable PROCESSENTRY32 with its required size set, and the
    // snapshot handle came from CreateToolhelp32Snapshot.
    let mut found = unsafe { Process32First(snapshot, &mut entry).is_ok() };
    let mut parent = None;
    while found {
        if entry.th32ProcessID == current_pid {
            parent = Some(entry.th32ParentProcessID);
            break;
        }
        // SAFETY: the same valid snapshot and entry are reused for the documented enumeration.
        found = unsafe { Process32Next(snapshot, &mut entry).is_ok() };
    }
    // SAFETY: `snapshot` is the handle returned by CreateToolhelp32Snapshot and is closed once.
    let _ = unsafe { CloseHandle(snapshot) };
    parent.filter(|pid| *pid != 0)
}

#[cfg(not(windows))]
pub fn parent_process_alive(parent_pid: u32) -> bool {
    parent_pid != 0
}

#[cfg(not(windows))]
pub fn current_parent_process_id() -> Option<u32> {
    None
}

/// Runs the calling thread at a lower OS scheduling priority than the rest of the application, so
/// it always yields the processor to everything else under contention. The guided diagnostic's
/// load generator (`diagnostics::guided::ThreadedGenerator`) calls this from each of its worker
/// threads: they deliberately saturate every logical core, and without this they can starve the
/// collector's own reader thread long enough to blow its stall window and trigger a false restart
/// (T187, found 2026-09-28).
#[cfg(windows)]
pub fn lower_current_thread_priority() {
    use windows::Win32::System::Threading::{
        GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_BELOW_NORMAL,
    };

    // SAFETY: GetCurrentThread returns a pseudo-handle that needs no closing, and
    // SetThreadPriority only ever affects the scheduling priority of the calling thread.
    unsafe {
        let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL);
    }
}

#[cfg(not(windows))]
pub fn lower_current_thread_priority() {}

/// Puts the calling process in `NORMAL_PRIORITY_CLASS` (T187, 2026-09-28). The elevated launcher
/// is started by Task Scheduler, whose default task priority is below normal; its sidecar child
/// inherits that class and, under any normal-priority CPU load, got no CPU at all — not one sample,
/// not even a line on stderr. Raising the launcher before it spawns the sidecar fixes both.
#[cfg(windows)]
pub fn raise_current_process_to_normal_priority() -> std::io::Result<()> {
    use windows::Win32::System::Threading::{
        GetCurrentProcess, NORMAL_PRIORITY_CLASS, SetPriorityClass,
    };

    // SAFETY: GetCurrentProcess returns a pseudo-handle that needs no closing, and
    // SetPriorityClass only changes the scheduling class of this very process.
    unsafe { SetPriorityClass(GetCurrentProcess(), NORMAL_PRIORITY_CLASS) }
        .map_err(|error| std::io::Error::other(error.to_string()))
}

#[cfg(not(windows))]
pub fn raise_current_process_to_normal_priority() -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        RestartPolicy, lower_current_thread_priority, parent_process_alive, read_line_or_eof,
        sha256_hex, watchdog_expired,
    };
    use std::io::Cursor;
    use std::time::{Duration, Instant};

    #[test]
    fn lowering_the_current_thread_priority_never_panics() {
        lower_current_thread_priority();
    }

    #[test]
    fn a_process_can_put_itself_in_the_normal_priority_class() {
        assert!(super::raise_current_process_to_normal_priority().is_ok());
    }

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
