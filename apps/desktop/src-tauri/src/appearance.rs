//! `appearance.glass` resolution (T131). The effective level combines three inputs, each capable
//! only of lowering it, never raising it above what the one above allows:
//!
//! 1. The stored preference (`full`/`reduced`/`off`, or `system`).
//! 2. When the preference is `system`, Windows' own "Efectos de transparencia" setting
//!    (`UISettings.AdvancedEffectsEnabled`), polled rather than pushed: the classic `UISettings`
//!    API has no dedicated change event for this specific property, and claiming a push
//!    subscription that may never fire would be worse than an honest poll (docs/spikes,
//!    ADR discipline: never promise a signal that was not verified to arrive).
//! 3. An automatic performance ceiling with hysteresis (`glass.*` in ruleset-v1), driven only by
//!    the frame rate the frontend reports in short windows — a real, visible stutter. It is no
//!    longer driven by this process's own idle WebView2 CPU cost (removed 2026-09-28, product
//!    decision, FR-043b amended): at `glass.degrade_idle_cpu_pct` = 1 %, routine live-dashboard
//!    repaints alone crossed it on an idle machine, degrading and partially recovering on a cycle
//!    close to `restore_window_s` with nothing actually wrong — and, separately, any CPU load
//!    elsewhere on the machine (the guided test's own generator included) held it degraded for as
//!    long as that load ran, which the person explicitly does not want: the glass effect should
//!    look the same whether the CPU is idle or pinned at 100 % by something else entirely.
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
/// or anything else, treated like `"system"`). With Windows' transparency off, `system` gives
/// `Reduced` (no blur, nearly opaque cards, the ambient gradient kept) rather than `Off`: the
/// setting is respected (constitution, accessibility) and the look stays close to the design mockup;
/// the person can still ask for `off` explicitly. `system_advanced_effects` is `None` when the OS
/// read failed — resolved to `Full`, the same "unknown treated as the unrestricted case" choice
/// `sampling_control::on_ac_power` makes for an unknown power source: a failed read must never by
/// itself force a degraded look.
pub fn resolve_base_level(preference: &str, system_advanced_effects: Option<bool>) -> GlassLevel {
    match GlassLevel::from_preference(preference) {
        Some(level) => level,
        None => match system_advanced_effects {
            Some(false) => GlassLevel::Reduced,
            Some(true) | None => GlassLevel::Full,
        },
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GlassThresholds {
    pub degrade_fps: f64,
    pub degrade_window: Duration,
    pub restore_fps: f64,
    pub restore_window: Duration,
}

impl GlassThresholds {
    pub fn from_ruleset(rules: &Ruleset) -> Option<Self> {
        Some(Self {
            degrade_fps: rules.parameter("glass.degrade_fps")?,
            degrade_window: Duration::from_secs_f64(rules.parameter("glass.degrade_window_s")?),
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

/// Pure hysteresis state machine: feed it fps samples, ask it to `tick`, get back the performance
/// ceiling. Never depends on real time passing or real hardware — every method takes `now`
/// explicitly, so tests drive it with synthetic instants (constitution XIII).
pub struct GlassDegradation {
    thresholds: GlassThresholds,
    fps: RollingWindow,
    ceiling: GlassLevel,
    degraded_at: Option<Instant>,
}

impl GlassDegradation {
    pub fn new(thresholds: GlassThresholds) -> Self {
        Self {
            thresholds,
            fps: RollingWindow::default(),
            ceiling: GlassLevel::Full,
            degraded_at: None,
        }
    }

    pub fn record_fps(&mut self, now: Instant, fps: f64) {
        self.fps.push(now, fps);
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
    /// never more than one tick late. Driven only by frame rate (2026-09-28): a real, visible
    /// stutter, never this process's own background CPU cost — see the module doc comment.
    pub fn tick(&mut self, now: Instant) -> GlassLevel {
        let fps_bad = self
            .fps
            .mean_over(now, self.thresholds.degrade_window)
            .is_some_and(|mean| mean < self.thresholds.degrade_fps);

        if fps_bad && self.ceiling != GlassLevel::Off {
            self.ceiling = self.ceiling.step_down();
            self.degraded_at = Some(now);
            // A confirmed problem must not immediately count as resolved by its own evidence: the
            // next step down (or the eventual restore) needs fresh samples, not this same window.
            self.fps = RollingWindow::default();
            return self.ceiling;
        }

        if self.ceiling == GlassLevel::Full {
            return self.ceiling;
        }

        let fps_ok = self
            .fps
            .mean_over(now, self.thresholds.restore_window)
            .is_none_or(|mean| mean >= self.thresholds.restore_fps);
        let held_long_enough = self
            .degraded_at
            .is_none_or(|at| now.duration_since(at) >= self.thresholds.restore_window);

        if fps_ok && held_long_enough {
            self.ceiling = self.ceiling.step_up();
            self.degraded_at = None;
            self.fps = RollingWindow::default();
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

/// How often the monitor thread rechecks the OS transparency setting. Short enough to notice a
/// preference/system change quickly, long enough not to matter as its own CPU cost.
const GLASS_POLL: Duration = Duration::from_secs(3);
const GLASS_EFFECTIVE_EVENT: &str = "appearance:glass-effective";

/// Owned by Tauri (`app.manage`) for the life of the process: the hysteresis state plus the last
/// effective level actually emitted, so the monitor thread and `report_glass_fps` never emit a
/// duplicate event when nothing changed.
pub struct GlassMonitorState {
    degradation: std::sync::Mutex<GlassDegradation>,
    last_emitted: std::sync::Mutex<Option<GlassLevel>>,
}

impl GlassMonitorState {
    pub fn new(thresholds: GlassThresholds) -> Self {
        Self {
            degradation: std::sync::Mutex::new(GlassDegradation::new(thresholds)),
            last_emitted: std::sync::Mutex::new(None),
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
        crate::log_info!(
            "GLASS_LEVEL_CHANGED",
            "effective glass level changed",
            from = last_emitted.map_or("none", GlassLevel::as_str),
            to = effective.as_str(),
            reason = if ceiling < base { "performance_ceiling" } else { "preference_or_system" }
        );
        *last_emitted = Some(effective);
        let _ = app.emit(GLASS_EFFECTIVE_EVENT, serde_json::json!({ "level": effective.as_str() }));
    }
}

/// Runs for the life of the process: every [`GLASS_POLL`], re-resolves the effective level — this
/// is what lets `appearance.glass = 'system'` react to the OS transparency setting changing while
/// the app is running, and what lets a degraded ceiling restore itself even when the frontend
/// never reports another fps sample (a hidden/minimized window stops sampling fps at all).
pub fn spawn_glass_monitor(app: tauri::AppHandle, state: std::sync::Arc<GlassMonitorState>) {
    // Resolves the preference/system level once immediately — otherwise the frontend would sit on
    // its own local placeholder for a few seconds after every launch.
    recompute_and_emit(&app, &state, Instant::now());
    let spawned = std::thread::Builder::new().name("glass-monitor".to_owned()).spawn(move || {
        loop {
            std::thread::sleep(GLASS_POLL);
            recompute_and_emit(&app, &state, Instant::now());
        }
    });
    if let Err(error) = spawned {
        crate::log_error!(
            "GLASS_MONITOR_SPAWN_FAILED",
            format!(
                "the glass monitor could not start; a degraded ceiling will not restore itself and the system transparency setting will not be picked up while the app runs: {error}"
            )
        );
    }
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
        assert_eq!(resolve_base_level("system", Some(false)), GlassLevel::Reduced);
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

    /// Found 2026-09-28: this process's own idle CPU cost, at any level, must never degrade the
    /// glass effect — including a full logical processor pinned at 100 % by something else
    /// entirely (the guided test's own load generator, in the real report that led here).
    /// `GlassDegradation` no longer has an idle-CPU input to record at all; a sustained low fps is
    /// the only thing that can still degrade it, exercised by the sibling test below.
    #[test]
    fn nothing_but_fps_can_degrade_the_ceiling() {
        let mut degradation = GlassDegradation::new(thresholds());
        let now = Instant::now();
        assert_eq!(degradation.tick(now), GlassLevel::Full);
        assert_eq!(degradation.tick(now + Duration::from_secs(120)), GlassLevel::Full);
    }

    #[test]
    fn recovering_before_the_restore_window_elapses_does_not_restore_yet() {
        let mut degradation = GlassDegradation::new(thresholds());
        let t0 = Instant::now();
        degradation.record_fps(t0, 30.0);
        assert_eq!(degradation.tick(t0), GlassLevel::Reduced);

        let t1 = t0 + Duration::from_secs(5);
        degradation.record_fps(t1, 60.0);
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
}
