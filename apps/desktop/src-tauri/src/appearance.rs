//! `appearance.glass` resolution (T131). The effective level combines three inputs, each capable
//! only of lowering it, never raising it above what the one above allows:
//!
//! 1. The stored preference (`full`/`reduced`/`off`, or `system`).
//! 2. When the preference is `system`, Windows' own "Efectos de transparencia" setting
//!    (`UISettings.AdvancedEffectsEnabled`), polled rather than pushed: the classic `UISettings`
//!    API has no dedicated change event for this specific property, and claiming a push
//!    subscription that may never fire would be worse than an honest poll (docs/spikes,
//!    ADR discipline: never promise a signal that was not verified to arrive).
//! 3. An automatic performance ceiling with hysteresis (`glass.*` in ruleset-v1), driven by the
//!    frame rate the frontend reports in short windows and this process's own WebView2 CPU cost,
//!    sampled the same way T019c's spike measured it (`docs/spikes/glass-cost.md`): summed
//!    `GetProcessTimes` of every `msedgewebview2.exe` descending from this process, as a percent
//!    of the whole machine.
#![deny(clippy::unwrap_used, clippy::expect_used)]

use crate::diagnostics::Ruleset;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GlassLevel {
    Off,
    Reduced,
    Full,
}

impl GlassLevel {
    pub const fn as_str(self) -> &'static str {
        match self {
            GlassLevel::Full => "full",
            GlassLevel::Reduced => "reduced",
            GlassLevel::Off => "off",
        }
    }

    pub fn from_preference(value: &str) -> Option<Self> {
        match value {
            "full" => Some(GlassLevel::Full),
            "reduced" => Some(GlassLevel::Reduced),
            "off" => Some(GlassLevel::Off),
            _ => None,
        }
    }

    const fn step_down(self) -> GlassLevel {
        match self {
            GlassLevel::Full => GlassLevel::Reduced,
            GlassLevel::Reduced | GlassLevel::Off => GlassLevel::Off,
        }
    }

    const fn step_up(self) -> GlassLevel {
        match self {
            GlassLevel::Off => GlassLevel::Reduced,
            GlassLevel::Reduced | GlassLevel::Full => GlassLevel::Full,
        }
    }
}

/// `preference` is the raw `appearance.glass` value (`"system"`, `"full"`, `"reduced"`, `"off"`,
/// or anything else, treated like `"system"`). `system_advanced_effects` is `None` when the OS
/// read failed — resolved to `Full`, the same "unknown treated as the unrestricted case" choice
/// `sampling_control::on_ac_power` makes for an unknown power source: a failed read must never by
/// itself force a degraded look.
pub fn resolve_base_level(preference: &str, system_advanced_effects: Option<bool>) -> GlassLevel {
    match GlassLevel::from_preference(preference) {
        Some(level) => level,
        None => match system_advanced_effects {
            Some(false) => GlassLevel::Off,
            Some(true) | None => GlassLevel::Full,
        },
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GlassThresholds {
    pub degrade_fps: f64,
    pub degrade_window: Duration,
    pub degrade_idle_cpu_pct: f64,
    pub degrade_idle_window: Duration,
    pub restore_fps: f64,
    pub restore_window: Duration,
}

impl GlassThresholds {
    pub fn from_ruleset(rules: &Ruleset) -> Option<Self> {
        Some(Self {
            degrade_fps: rules.parameter("glass.degrade_fps")?,
            degrade_window: Duration::from_secs_f64(rules.parameter("glass.degrade_window_s")?),
            degrade_idle_cpu_pct: rules.parameter("glass.degrade_idle_cpu_pct")?,
            degrade_idle_window: Duration::from_secs_f64(
                rules.parameter("glass.degrade_idle_window_s")?,
            ),
            restore_fps: rules.parameter("glass.restore_fps")?,
            restore_window: Duration::from_secs_f64(rules.parameter("glass.restore_window_s")?),
        })
    }
}

/// Timestamped samples of one signal, trimmed to whatever window a caller last asked about.
#[derive(Debug, Default)]
struct RollingWindow {
    samples: Vec<(Instant, f64)>,
}

impl RollingWindow {
    fn push(&mut self, now: Instant, value: f64) {
        self.samples.push((now, value));
        // Nothing needs a sample older than the longest window this module ever asks for; nothing
        // here asks for more than a couple of minutes, so an unbounded-looking Vec never grows far.
        self.samples.retain(|(t, _)| now.duration_since(*t) <= Duration::from_secs(600));
    }

    /// `None` when there is no sample at all inside the window — "no evidence yet", never
    /// mistaken for a good or a bad reading.
    fn mean_over(&self, now: Instant, window: Duration) -> Option<f64> {
        let mut sum = 0.0;
        let mut count = 0u32;
        for (t, value) in &self.samples {
            if now.duration_since(*t) <= window {
                sum += value;
                count += 1;
            }
        }
        (count > 0).then_some(sum / f64::from(count))
    }
}

/// Pure hysteresis state machine: feed it fps and idle-CPU-percent samples, ask it to `tick`, get
/// back the performance ceiling. Never depends on real time passing or real hardware — every
/// method takes `now` explicitly, so tests drive it with synthetic instants (constitution XIII).
pub struct GlassDegradation {
    thresholds: GlassThresholds,
    fps: RollingWindow,
    idle_cpu: RollingWindow,
    ceiling: GlassLevel,
    degraded_at: Option<Instant>,
}

impl GlassDegradation {
    pub fn new(thresholds: GlassThresholds) -> Self {
        Self {
            thresholds,
            fps: RollingWindow::default(),
            idle_cpu: RollingWindow::default(),
            ceiling: GlassLevel::Full,
            degraded_at: None,
        }
    }

    pub fn record_fps(&mut self, now: Instant, fps: f64) {
        self.fps.push(now, fps);
    }

    pub fn record_idle_cpu_pct(&mut self, now: Instant, pct: f64) {
        self.idle_cpu.push(now, pct);
    }

    pub fn ceiling(&self) -> GlassLevel {
        self.ceiling
    }

    /// The level to actually apply: never more permissive than `base` (the preference/system
    /// result), regardless of how good performance looks.
    pub fn effective_level(&self, base: GlassLevel) -> GlassLevel {
        base.min(self.ceiling)
    }

    /// Re-evaluates the ceiling from whatever samples are on record and returns it. Call this
    /// periodically (the monitor loop) and after every new sample, so a degrade or a restore is
    /// never more than one tick late.
    pub fn tick(&mut self, now: Instant) -> GlassLevel {
        let fps_bad = self
            .fps
            .mean_over(now, self.thresholds.degrade_window)
            .is_some_and(|mean| mean < self.thresholds.degrade_fps);
        let cpu_bad = self
            .idle_cpu
            .mean_over(now, self.thresholds.degrade_idle_window)
            .is_some_and(|mean| mean > self.thresholds.degrade_idle_cpu_pct);

        if (fps_bad || cpu_bad) && self.ceiling != GlassLevel::Off {
            self.ceiling = self.ceiling.step_down();
            self.degraded_at = Some(now);
            // A confirmed problem must not immediately count as resolved by its own evidence: the
            // next step down (or the eventual restore) needs fresh samples, not this same window.
            self.fps = RollingWindow::default();
            self.idle_cpu = RollingWindow::default();
            return self.ceiling;
        }

        if self.ceiling == GlassLevel::Full {
            return self.ceiling;
        }

        // No separate "restore" idle-CPU threshold exists in ruleset-v1: restoring reuses the
        // same degrade_idle_cpu_pct, held clear for the longer restore_window instead of the
        // shorter degrade_idle_window — the asymmetry is in the timing, not the level, matching
        // how `restore_fps` (55) is itself already a looser bound than `degrade_fps` (50). A
        // signal with no samples yet is "not disproven", not "bad" — it does not block a restore
        // driven by the other signal.
        let fps_ok = self
            .fps
            .mean_over(now, self.thresholds.restore_window)
            .is_none_or(|mean| mean >= self.thresholds.restore_fps);
        let cpu_ok = self
            .idle_cpu
            .mean_over(now, self.thresholds.restore_window)
            .is_none_or(|mean| mean <= self.thresholds.degrade_idle_cpu_pct);
        let held_long_enough = self
            .degraded_at
            .is_none_or(|at| now.duration_since(at) >= self.thresholds.restore_window);

        if fps_ok && cpu_ok && held_long_enough {
            self.ceiling = self.ceiling.step_up();
            self.degraded_at = None;
            self.fps = RollingWindow::default();
            self.idle_cpu = RollingWindow::default();
        }

        self.ceiling
    }
}

/// `UISettings.AdvancedEffectsEnabled` — Windows' system-wide "Efectos de transparencia" switch.
/// `None` when the read fails; `resolve_base_level` treats that the same as "enabled" (fail open
/// to `Full`, never force a degraded look from a read error alone). Polled, not subscribed: the
/// classic `UISettings` surface has no dedicated change event for this one property, and this
/// module does not claim a push notification it did not verify fires.
#[cfg(windows)]
pub fn advanced_effects_enabled() -> Option<bool> {
    use windows::UI::ViewManagement::UISettings;

    // windows-rs activates WinRT classes through `RoGetActivationFactory`, which initializes the
    // apartment it needs on first use; a manual `CoInitializeEx` here previously crashed the
    // process (STATUS_ACCESS_VIOLATION), most likely fighting that internal initialization on the
    // same thread. Verified without it against this machine's real transparency setting.
    let settings = UISettings::new().ok()?;
    settings.AdvancedEffectsEnabled().ok()
}

#[cfg(not(windows))]
pub fn advanced_effects_enabled() -> Option<bool> {
    None
}

/// Every `msedgewebview2.exe` descending from `current_pid`, walking parent links from the same
/// snapshot `ipc::supervisor::current_parent_process_id` already uses for the guided watchdog.
#[cfg(windows)]
fn webview2_process_ids(current_pid: u32) -> Vec<u32> {
    use std::collections::HashMap;
    use std::mem::size_of;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32, Process32First, Process32Next, TH32CS_SNAPPROCESS,
    };

    let Ok(snapshot) = (unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }) else {
        return Vec::new();
    };
    let mut entry =
        PROCESSENTRY32 { dwSize: size_of::<PROCESSENTRY32>() as u32, ..Default::default() };
    let mut rows: Vec<(u32, u32, String)> = Vec::new();
    // SAFETY: `entry` is valid and sized per the documented struct; `snapshot` came from
    // CreateToolhelp32Snapshot above and is closed once, on every path out of this function.
    let mut found = unsafe { Process32First(snapshot, &mut entry).is_ok() };
    while found {
        let name: String = entry
            .szExeFile
            .iter()
            .take_while(|&&b| b != 0)
            .map(|&b| b as u8 as char)
            .collect::<String>()
            .to_lowercase();
        rows.push((entry.th32ProcessID, entry.th32ParentProcessID, name));
        // SAFETY: same valid snapshot/entry, the documented enumeration pattern.
        found = unsafe { Process32Next(snapshot, &mut entry).is_ok() };
    }
    // SAFETY: closes the handle `CreateToolhelp32Snapshot` returned above, exactly once.
    let _ = unsafe { CloseHandle(snapshot) };

    let parent_of: HashMap<u32, u32> =
        rows.iter().map(|(pid, parent, _)| (*pid, *parent)).collect();
    let descends_from_current = |pid: u32| -> bool {
        let mut cursor = pid;
        // Bounded walk: browser -> GPU/renderer/utility is a handful of hops at most, never
        // unbounded even if a cycle existed in a corrupted snapshot.
        for _ in 0..8 {
            if cursor == current_pid {
                return true;
            }
            match parent_of.get(&cursor) {
                Some(&parent) if parent != 0 && parent != cursor => cursor = parent,
                _ => return false,
            }
        }
        false
    };
    rows.into_iter()
        .filter(|(pid, _, name)| name == "msedgewebview2.exe" && descends_from_current(*pid))
        .map(|(pid, _, _)| pid)
        .collect()
}

#[cfg(windows)]
fn process_cpu_time_100ns(pid: u32) -> Option<u64> {
    use windows::Win32::Foundation::{CloseHandle, FILETIME};
    use windows::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    // SAFETY: opens a handle with only the access right GetProcessTimes needs; closed below.
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    // SAFETY: `handle` is the process handle just opened above; the four out-parameters point at
    // local `FILETIME` values the API is documented to fill in place.
    let ok = unsafe { GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user) }
        .is_ok();
    // SAFETY: closes the handle opened by `OpenProcess` above, exactly once.
    let _ = unsafe { CloseHandle(handle) };
    if !ok {
        return None;
    }
    let as_100ns =
        |ft: FILETIME| (u64::from(ft.dwHighDateTime) << 32) | u64::from(ft.dwLowDateTime);
    Some(as_100ns(kernel) + as_100ns(user))
}

/// Cumulative (lifetime, not a delta) kernel+user CPU time of every WebView2 process this
/// application owns, in milliseconds. `None` when there is no WebView2 process yet (too early in
/// startup) or the platform is not Windows — the monitor loop treats that as "no sample", never
/// as zero CPU.
#[cfg(windows)]
pub fn own_webview_cpu_time_ms() -> Option<u64> {
    use windows::Win32::System::Threading::GetCurrentProcessId;

    // SAFETY: takes no arguments and only reads the calling process's own id.
    let current_pid = unsafe { GetCurrentProcessId() };
    let pids = webview2_process_ids(current_pid);
    if pids.is_empty() {
        return None;
    }
    let total_100ns: u64 = pids.iter().filter_map(|&pid| process_cpu_time_100ns(pid)).sum();
    Some(total_100ns / 10_000)
}

#[cfg(not(windows))]
pub fn own_webview_cpu_time_ms() -> Option<u64> {
    None
}

/// `cpu_ms` is a CPU-time delta over `elapsed` (never the cumulative total `own_webview_cpu_time_ms`
/// returns); matches T019c's own formula (`docs/spikes/glass-cost.md`): time spent divided by
/// wall-clock time available across every logical processor.
pub fn cpu_percent_of_machine(cpu_ms: u64, elapsed: Duration, logical_processors: usize) -> f64 {
    if elapsed.is_zero() || logical_processors == 0 {
        return 0.0;
    }
    let elapsed_ms = elapsed.as_secs_f64() * 1000.0;
    (cpu_ms as f64) / (elapsed_ms * logical_processors as f64) * 100.0
}

/// How often the monitor thread rechecks the OS transparency setting and this process's own
/// WebView2 CPU cost. Short enough to track `glass.degrade_window_s` (3 s by default) without a
/// large delay, long enough not to matter as its own CPU cost.
const GLASS_POLL: Duration = Duration::from_secs(3);
const GLASS_EFFECTIVE_EVENT: &str = "appearance:glass-effective";

/// Owned by Tauri (`app.manage`) for the life of the process: the hysteresis state plus the last
/// effective level actually emitted, so the monitor thread and `report_glass_fps` never emit a
/// duplicate event when nothing changed.
pub struct GlassMonitorState {
    degradation: std::sync::Mutex<GlassDegradation>,
    last_emitted: std::sync::Mutex<Option<GlassLevel>>,
    previous_cpu_ms: std::sync::Mutex<Option<u64>>,
}

impl GlassMonitorState {
    pub fn new(thresholds: GlassThresholds) -> Self {
        Self {
            degradation: std::sync::Mutex::new(GlassDegradation::new(thresholds)),
            last_emitted: std::sync::Mutex::new(None),
            previous_cpu_ms: std::sync::Mutex::new(None),
        }
    }
}

fn current_glass_preference(app: &tauri::AppHandle) -> String {
    use tauri::Manager;
    app.try_state::<crate::storage::AppState>()
        .and_then(|state| {
            state.storage.lock().ok().and_then(|storage| storage.user_preferences().ok())
        })
        .and_then(|preferences| {
            preferences.get("appearance.glass").and_then(|v| v.as_str().map(str::to_owned))
        })
        .unwrap_or_else(|| "system".to_owned())
}

/// Re-evaluates the effective level from whatever is on record right now and emits
/// [`GLASS_EFFECTIVE_EVENT`] only when it actually changed. Called after every new sample
/// (`report_glass_fps`, the periodic poll) so a degrade or restore reaches the frontend within one
/// tick, never waiting for an unrelated future event.
fn recompute_and_emit(app: &tauri::AppHandle, state: &GlassMonitorState, now: Instant) {
    use tauri::Emitter;

    let base = resolve_base_level(&current_glass_preference(app), advanced_effects_enabled());
    let ceiling = state
        .degradation
        .lock()
        .map(|mut degradation| degradation.tick(now))
        .unwrap_or(GlassLevel::Full);
    let effective = base.min(ceiling);

    let mut last_emitted =
        state.last_emitted.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if *last_emitted != Some(effective) {
        *last_emitted = Some(effective);
        let _ = app.emit(GLASS_EFFECTIVE_EVENT, serde_json::json!({ "level": effective.as_str() }));
    }
}

/// Runs for the life of the process: every [`GLASS_POLL`], samples this process's own WebView2 CPU
/// cost as a percent of the machine (T019c's own formula) and re-resolves the effective level —
/// this is what lets `appearance.glass = 'system'` react to the OS setting changing while the app
/// is running, and what drives the automatic performance ceiling even when the frontend never
/// reports an fps sample (a hidden/minimized window still costs real CPU).
pub fn spawn_glass_monitor(app: tauri::AppHandle, state: std::sync::Arc<GlassMonitorState>) {
    let logical_processors = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    // Resolves the preference/system level once immediately, before the first CPU sample is even
    // possible (it needs two points GLASS_POLL apart) — otherwise the frontend would sit on its
    // own local placeholder for a few seconds after every launch.
    recompute_and_emit(&app, &state, Instant::now());
    let _ = std::thread::Builder::new().name("glass-monitor".to_owned()).spawn(move || {
        let mut previous_tick = Instant::now();
        loop {
            std::thread::sleep(GLASS_POLL);
            let now = Instant::now();
            let elapsed = now.duration_since(previous_tick);
            previous_tick = now;

            if let Some(total_ms) = own_webview_cpu_time_ms() {
                let mut previous =
                    state.previous_cpu_ms.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                if let Some(before) = *previous {
                    let delta_ms = total_ms.saturating_sub(before);
                    let pct = cpu_percent_of_machine(delta_ms, elapsed, logical_processors);
                    if let Ok(mut degradation) = state.degradation.lock() {
                        degradation.record_idle_cpu_pct(now, pct);
                    }
                }
                *previous = Some(total_ms);
            }

            recompute_and_emit(&app, &state, now);
        }
    });
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct GlassEffectiveDto {
    pub level: String,
}

/// A plain query for the level right now, alongside the pushed [`GLASS_EFFECTIVE_EVENT`]: the
/// monitor thread emits its first push during app setup, which can land before the frontend has
/// registered its listener — a fresh mount asks once instead of trusting it never missed that
/// first push. Never ticks the shared state itself (a read, not a step): only the monitor thread
/// and `report_glass_fps` drive real transitions.
#[tauri::command]
pub fn get_effective_glass_level(
    app: tauri::AppHandle,
    state: tauri::State<'_, std::sync::Arc<GlassMonitorState>>,
) -> Result<GlassEffectiveDto, crate::commands::CommandError> {
    let base = resolve_base_level(&current_glass_preference(&app), advanced_effects_enabled());
    let ceiling = state
        .degradation
        .lock()
        .map(|degradation| degradation.ceiling())
        .unwrap_or(GlassLevel::Full);
    Ok(GlassEffectiveDto { level: base.min(ceiling).as_str().to_owned() })
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportGlassFpsRequest {
    pub fps: f64,
}

/// The frontend samples fps in short windows (per `docs/spikes/glass-cost.md` finding 5: a
/// continuous `requestAnimationFrame` counter itself costs measurable CPU) and reports the result
/// here instead of running its own always-on measurement loop.
#[tauri::command]
pub fn report_glass_fps(
    app: tauri::AppHandle,
    state: tauri::State<'_, std::sync::Arc<GlassMonitorState>>,
    request: ReportGlassFpsRequest,
) -> Result<(), crate::commands::CommandError> {
    if !request.fps.is_finite() || request.fps < 0.0 {
        return Ok(()); // a malformed sample is dropped, never recorded as evidence either way
    }
    let now = Instant::now();
    if let Ok(mut degradation) = state.degradation.lock() {
        degradation.record_fps(now, request.fps);
    }
    recompute_and_emit(&app, &state, now);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{GlassDegradation, GlassLevel, GlassThresholds, resolve_base_level};
    use std::time::{Duration, Instant};

    fn thresholds() -> GlassThresholds {
        GlassThresholds {
            degrade_fps: 50.0,
            degrade_window: Duration::from_secs(3),
            degrade_idle_cpu_pct: 1.0,
            degrade_idle_window: Duration::from_secs(30),
            restore_fps: 55.0,
            restore_window: Duration::from_secs(60),
        }
    }

    #[test]
    fn an_explicit_preference_wins_over_the_system_reading() {
        assert_eq!(resolve_base_level("off", Some(true)), GlassLevel::Off);
        assert_eq!(resolve_base_level("full", Some(false)), GlassLevel::Full);
    }

    #[test]
    fn system_maps_the_os_transparency_setting_and_fails_open() {
        assert_eq!(resolve_base_level("system", Some(true)), GlassLevel::Full);
        assert_eq!(resolve_base_level("system", Some(false)), GlassLevel::Off);
        assert_eq!(resolve_base_level("system", None), GlassLevel::Full);
    }

    #[test]
    fn no_samples_never_degrades() {
        let mut degradation = GlassDegradation::new(thresholds());
        assert_eq!(degradation.tick(Instant::now()), GlassLevel::Full);
    }

    #[test]
    fn sustained_low_fps_steps_the_ceiling_down_once_per_confirmed_window() {
        let mut degradation = GlassDegradation::new(thresholds());
        let t0 = Instant::now();
        degradation.record_fps(t0, 30.0);
        assert_eq!(degradation.tick(t0 + Duration::from_millis(10)), GlassLevel::Reduced);

        // The same evidence that just caused the drop must not immediately cause a second one.
        assert_eq!(degradation.tick(t0 + Duration::from_millis(20)), GlassLevel::Reduced);

        let t1 = t0 + Duration::from_secs(5);
        degradation.record_fps(t1, 30.0);
        assert_eq!(degradation.tick(t1 + Duration::from_millis(10)), GlassLevel::Off);
    }

    #[test]
    fn sustained_high_idle_cpu_also_degrades() {
        let mut degradation = GlassDegradation::new(thresholds());
        let t0 = Instant::now();
        degradation.record_idle_cpu_pct(t0, 5.0);
        assert_eq!(degradation.tick(t0), GlassLevel::Reduced);
    }

    #[test]
    fn recovering_before_the_restore_window_elapses_does_not_restore_yet() {
        let mut degradation = GlassDegradation::new(thresholds());
        let t0 = Instant::now();
        degradation.record_fps(t0, 30.0);
        assert_eq!(degradation.tick(t0), GlassLevel::Reduced);

        let t1 = t0 + Duration::from_secs(5);
        degradation.record_fps(t1, 60.0);
        degradation.record_idle_cpu_pct(t1, 0.1);
        assert_eq!(degradation.tick(t1), GlassLevel::Reduced);
    }

    #[test]
    fn recovering_for_the_full_restore_window_steps_back_up_one_level() {
        let mut degradation = GlassDegradation::new(thresholds());
        let t0 = Instant::now();
        degradation.record_fps(t0, 30.0);
        assert_eq!(degradation.tick(t0), GlassLevel::Reduced);

        let t1 = t0 + Duration::from_secs(61);
        degradation.record_fps(t1, 60.0);
        degradation.record_idle_cpu_pct(t1, 0.1);
        assert_eq!(degradation.tick(t1), GlassLevel::Full);
    }

    #[test]
    fn never_restores_past_full() {
        let mut degradation = GlassDegradation::new(thresholds());
        let now = Instant::now();
        degradation.record_fps(now, 60.0);
        assert_eq!(degradation.tick(now), GlassLevel::Full);
    }

    #[test]
    fn effective_level_is_never_more_permissive_than_the_base() {
        let mut degradation = GlassDegradation::new(thresholds());
        let now = Instant::now();
        degradation.record_fps(now, 30.0);
        degradation.tick(now);
        assert_eq!(degradation.ceiling(), GlassLevel::Reduced);
        // Base already `off` (explicit choice): the ceiling can never make it look better.
        assert_eq!(degradation.effective_level(GlassLevel::Off), GlassLevel::Off);
        assert_eq!(degradation.effective_level(GlassLevel::Full), GlassLevel::Reduced);
    }

    #[test]
    #[cfg(windows)]
    fn advanced_effects_enabled_reads_a_real_answer_on_this_machine() {
        // Whatever this machine's transparency setting is, the call must succeed and return a
        // real bool, not silently fail — a `None` here would mean the WinRT apartment/projection
        // is broken on this build, which the rest of the module treats as "unknown, fail open",
        // but a passing CI/dev machine should never actually hit that path.
        assert!(super::advanced_effects_enabled().is_some());
    }

    #[test]
    fn cpu_percent_of_machine_matches_t019c_formula() {
        // 1000 ms of CPU time over a 10 s window on a 22-logical-processor machine.
        let pct = super::cpu_percent_of_machine(1000, Duration::from_secs(10), 22);
        assert!((pct - (1000.0 / (10_000.0 * 22.0) * 100.0)).abs() < 1e-9);
    }
}
