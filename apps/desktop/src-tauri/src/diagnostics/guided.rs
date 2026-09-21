#![deny(clippy::unwrap_used, clippy::expect_used)]
// FR-018: the load generator must never touch a hardware write surface (MSR/SMU/PawnIO). Nothing
// in this module needs `unsafe`, so the compiler proves it instead of a runtime assertion.
#![forbid(unsafe_code)]

use super::Ruleset;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::{self, JoinHandle};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuidedPhase {
    Preflight,
    Ready,
    Rest,
    Warming,
    SteadyLoad,
    Recovery,
    Cancelling,
    Cancelled,
    SafetyStop,
    SensorLost,
    Error,
    Result,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GuidedProfile {
    Short,
    Standard,
    Long,
}

impl GuidedProfile {
    pub fn load_seconds(self, rules: &Ruleset) -> Option<u64> {
        let id = match self {
            Self::Short => "guided.load_short_s",
            Self::Standard => "guided.load_standard_s",
            Self::Long => "guided.load_long_s",
        };
        rules.parameter(id).map(|value| value as u64)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuidedPreflight {
    pub sensors: bool,
    pub ac_power: bool,
    pub profile: bool,
    pub disk_space: bool,
    pub generator: bool,
}

impl GuidedPreflight {
    pub fn passed(&self, require_ac: bool) -> bool {
        self.sensors
            && (!require_ac || self.ac_power)
            && self.profile
            && self.disk_space
            && self.generator
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuidedStopReason {
    UserRequested,
    ThermalSafety,
    SevereLowClock,
    CriticalSensorLost,
    GeneratorUnresponsive,
    Suspended,
    WindowHidden,
    ParentMissing,
    Battery,
}

impl GuidedStopReason {
    pub const fn key(self) -> &'static str {
        match self {
            Self::UserRequested => "guided.user_requested",
            Self::ThermalSafety => "guided.thermal_safety",
            Self::SevereLowClock => "guided.severe_low_clock",
            Self::CriticalSensorLost => "guided.sensor_lost",
            Self::GeneratorUnresponsive => "guided.generator_unresponsive",
            Self::Suspended => "guided.system_suspend",
            Self::WindowHidden => "guided.window_hidden",
            Self::ParentMissing => "guided.parent_missing",
            Self::Battery => "guided.battery",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuidedSafetyState {
    pub consecutive_over_limit: u8,
    pub consecutive_low_clock_ms: u64,
    pub consecutive_missing_sensor: u8,
    pub generator_silent_ms: u64,
}

impl GuidedSafetyState {
    pub const ZERO: Self = Self {
        consecutive_over_limit: 0,
        consecutive_low_clock_ms: 0,
        consecutive_missing_sensor: 0,
        generator_silent_ms: 0,
    };

    /// Folds one tick of raw observations into the next state. Every counter accumulates only
    /// while its condition holds this tick and resets the instant it does not (FR-085: it is
    /// *staying* past a threshold that is unsafe, not touching it once).
    pub fn observe(
        self,
        elapsed_ms: u64,
        over_limit: bool,
        severely_throttled: bool,
        sensor_missing: bool,
        generator_progressed: bool,
    ) -> Self {
        Self {
            consecutive_over_limit: if over_limit {
                self.consecutive_over_limit.saturating_add(1)
            } else {
                0
            },
            consecutive_low_clock_ms: if severely_throttled {
                self.consecutive_low_clock_ms.saturating_add(elapsed_ms)
            } else {
                0
            },
            consecutive_missing_sensor: if sensor_missing {
                self.consecutive_missing_sensor.saturating_add(1)
            } else {
                0
            },
            generator_silent_ms: if generator_progressed {
                0
            } else {
                self.generator_silent_ms.saturating_add(elapsed_ms)
            },
        }
    }
}

/// Whether the reading sits more than `over_limit_c` past the effective limit (FR-085): the
/// guided test does not stop for *reaching* the limit, which is the phenomenon it measures.
pub fn is_over_limit(
    temperature_c: Option<f64>,
    thermal_limit_c: Option<f64>,
    over_limit_c: f64,
) -> bool {
    match (temperature_c, thermal_limit_c) {
        (Some(temperature), Some(limit)) => temperature > limit + over_limit_c,
        _ => false,
    }
}

/// Whether the CPU sits at (or past) its limit while its active clock is below `low_freq_ratio`
/// of its base clock (FR-085): cooling is severely insufficient, not simply throttling as
/// expected once the limit is reached.
pub fn is_severely_throttled(
    temperature_c: Option<f64>,
    thermal_limit_c: Option<f64>,
    active_clock_mhz: Option<f64>,
    base_clock_mhz: Option<f64>,
    low_freq_ratio: f64,
) -> bool {
    let at_limit = matches!(
        (temperature_c, thermal_limit_c),
        (Some(temperature), Some(limit)) if temperature >= limit
    );
    at_limit
        && matches!(
            (active_clock_mhz, base_clock_mhz),
            (Some(active), Some(base)) if base > 0.0 && active < base * low_freq_ratio
        )
}

const GENERATOR_WORK_CHUNK: u64 = 20_000;

/// The real load generator: one OS thread per logical processor, each counting its own completed
/// operations in an atomic counter so the throughput reported to the interface is measured, not
/// estimated. The work itself is deterministic floating-point arithmetic with no side effects —
/// `#![forbid(unsafe_code)]` above proves it never reaches a hardware write surface (FR-018).
pub struct ThreadedGenerator {
    running: Arc<AtomicBool>,
    counters: Vec<Arc<AtomicU64>>,
    handles: Vec<JoinHandle<()>>,
}

impl ThreadedGenerator {
    pub fn start(threads: u16) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let mut counters = Vec::new();
        let mut handles = Vec::new();
        for index in 0..threads.max(1) {
            let counter = Arc::new(AtomicU64::new(0));
            let counter_ref = Arc::clone(&counter);
            let running_ref = Arc::clone(&running);
            if let Ok(handle) = thread::Builder::new()
                .name(format!("guided-generator-{index}"))
                .spawn(move || generator_worker(&running_ref, &counter_ref))
            {
                handles.push(handle);
            }
            counters.push(counter);
        }
        Self { running, counters, handles }
    }

    /// Sum of every worker's completed operations since `start`.
    pub fn total_ops(&self) -> u64 {
        self.counters.iter().map(|counter| counter.load(Ordering::Relaxed)).sum()
    }
}

impl Drop for ThreadedGenerator {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Release);
        for handle in self.handles.drain(..) {
            let _ = handle.join();
        }
    }
}

fn generator_worker(running: &AtomicBool, counter: &AtomicU64) {
    let mut value: f64 = 1.0;
    while running.load(Ordering::Acquire) {
        for _ in 0..GENERATOR_WORK_CHUNK {
            value = (value * 1.000_000_1 + 0.5).fract() + 1.0;
        }
        std::hint::black_box(value);
        counter.fetch_add(GENERATOR_WORK_CHUNK, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuidedSafetyLimits {
    pub over_limit_samples: u8,
    pub low_clock_ms: u64,
    pub missing_sensor_samples: u8,
    pub generator_timeout_ms: u64,
}

pub fn safety_limits(rules: &Ruleset) -> Option<GuidedSafetyLimits> {
    Some(GuidedSafetyLimits {
        over_limit_samples: rules.parameter("guided.stop_over_limit_samples")? as u8,
        low_clock_ms: (rules.parameter("guided.stop_low_freq_s")? * 1000.0) as u64,
        missing_sensor_samples: rules.parameter("guided.stop_missing_sensor_samples")? as u8,
        generator_timeout_ms: (rules.parameter("guided.stop_generator_timeout_s")? * 1000.0) as u64,
    })
}

pub fn stop_reason(
    state: GuidedSafetyState,
    over_limit: bool,
    below_half_base_at_limit: bool,
    critical_sensor_missing: bool,
    limits: GuidedSafetyLimits,
) -> Option<GuidedStopReason> {
    if over_limit && state.consecutive_over_limit >= limits.over_limit_samples {
        return Some(GuidedStopReason::ThermalSafety);
    }
    if below_half_base_at_limit && state.consecutive_low_clock_ms >= limits.low_clock_ms {
        return Some(GuidedStopReason::SevereLowClock);
    }
    if critical_sensor_missing && state.consecutive_missing_sensor >= limits.missing_sensor_samples
    {
        return Some(GuidedStopReason::CriticalSensorLost);
    }
    (state.generator_silent_ms >= limits.generator_timeout_ms)
        .then_some(GuidedStopReason::GeneratorUnresponsive)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuidedWatchdog {
    timeout_ms: u64,
    last_response_ms: u64,
}

impl GuidedWatchdog {
    pub const fn new(timeout_ms: u64) -> Self {
        Self { timeout_ms, last_response_ms: 0 }
    }

    pub const fn response(&mut self, now_ms: u64) {
        self.last_response_ms = now_ms;
    }

    pub const fn expired(&self, now_ms: u64) -> bool {
        now_ms.saturating_sub(self.last_response_ms) >= self.timeout_ms
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuidedConfig {
    pub require_ac: bool,
    pub rest_ms: u64,
    pub warmup_ms: u64,
    pub load_ms: u64,
    pub recovery_ms: u64,
}

impl GuidedConfig {
    pub fn from_ruleset(rules: &Ruleset, profile: GuidedProfile, require_ac: bool) -> Option<Self> {
        Some(Self {
            require_ac,
            rest_ms: (rules.parameter("guided.rest_s")? * 1000.0) as u64,
            warmup_ms: (rules.parameter("guided.warmup_s")? * 1000.0) as u64,
            load_ms: profile.load_seconds(rules)? * 1000,
            recovery_ms: (rules.parameter("guided.recovery_s")? * 1000.0) as u64,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuidedMachine {
    pub phase: GuidedPhase,
    pub profile: GuidedProfile,
    pub elapsed_ms: u64,
    pub reason: Option<GuidedStopReason>,
    config: GuidedConfig,
    rest_skipped: bool,
}

impl GuidedMachine {
    pub fn new(profile: GuidedProfile, config: GuidedConfig) -> Self {
        Self {
            phase: GuidedPhase::Preflight,
            profile,
            elapsed_ms: 0,
            reason: None,
            config,
            rest_skipped: false,
        }
    }
    pub const fn requires_ac(&self) -> bool {
        self.config.require_ac
    }
    pub fn complete_preflight(&mut self, checks: &GuidedPreflight) -> bool {
        if !checks.passed(self.config.require_ac) {
            self.phase = GuidedPhase::Error;
            return false;
        }
        self.phase = GuidedPhase::Ready;
        self.elapsed_ms = 0;
        true
    }
    pub fn begin(&mut self) -> bool {
        if self.phase != GuidedPhase::Ready {
            return false;
        }
        self.phase = GuidedPhase::Rest;
        self.elapsed_ms = 0;
        true
    }
    pub fn skip_rest(&mut self) {
        if self.phase == GuidedPhase::Rest {
            self.rest_skipped = true;
            self.elapsed_ms = 0;
            self.phase = GuidedPhase::Warming;
        }
    }
    pub fn request_cancel(&mut self, reason: GuidedStopReason) -> bool {
        if matches!(self.phase, GuidedPhase::Cancelled | GuidedPhase::Result | GuidedPhase::Error) {
            return false;
        }
        self.reason = Some(reason);
        self.phase = GuidedPhase::Cancelling;
        true
    }
    pub fn tick(&mut self, elapsed_ms: u64) {
        if self.phase == GuidedPhase::Cancelling {
            self.phase = GuidedPhase::Cancelled;
            self.elapsed_ms = self.elapsed_ms.saturating_add(elapsed_ms);
            return;
        }
        let mut remaining = elapsed_ms;
        while remaining > 0 {
            let phase_duration = match self.phase {
                GuidedPhase::Rest if !self.rest_skipped => self.config.rest_ms,
                GuidedPhase::Warming => self.config.warmup_ms,
                GuidedPhase::SteadyLoad => self.config.load_ms,
                GuidedPhase::Recovery => self.config.recovery_ms,
                _ => 0,
            };
            if phase_duration == 0 || self.elapsed_ms.saturating_add(remaining) < phase_duration {
                self.elapsed_ms = self.elapsed_ms.saturating_add(remaining);
                break;
            }
            remaining = remaining.saturating_sub(phase_duration.saturating_sub(self.elapsed_ms));
            self.elapsed_ms = 0;
            self.phase = match self.phase {
                GuidedPhase::Rest => GuidedPhase::Warming,
                GuidedPhase::Warming => GuidedPhase::SteadyLoad,
                GuidedPhase::SteadyLoad => GuidedPhase::Recovery,
                GuidedPhase::Recovery => GuidedPhase::Result,
                phase => phase,
            };
            if self.phase == GuidedPhase::Result {
                break;
            }
        }
    }
    pub fn skip_or_cancel_hidden(&mut self) {
        let _ = self.request_cancel(GuidedStopReason::WindowHidden);
    }
    pub fn suspend(&mut self) {
        let _ = self.request_cancel(GuidedStopReason::Suspended);
    }

    pub fn remaining_ms(&self) -> Option<u64> {
        let duration = match self.phase {
            GuidedPhase::Rest if !self.rest_skipped => self.config.rest_ms,
            GuidedPhase::Warming => self.config.warmup_ms,
            GuidedPhase::SteadyLoad => self.config.load_ms,
            GuidedPhase::Recovery => self.config.recovery_ms,
            _ => return None,
        };
        Some(duration.saturating_sub(self.elapsed_ms))
    }

    pub fn progress_percent(&self) -> Option<f64> {
        let duration = match self.phase {
            GuidedPhase::Rest if !self.rest_skipped => self.config.rest_ms,
            GuidedPhase::Warming => self.config.warmup_ms,
            GuidedPhase::SteadyLoad => self.config.load_ms,
            GuidedPhase::Recovery => self.config.recovery_ms,
            _ => return None,
        };
        if duration == 0 {
            return None;
        }
        Some((self.elapsed_ms as f64 / duration as f64 * 100.0).clamp(0.0, 100.0))
    }

    pub fn observe_safety(
        &mut self,
        state: GuidedSafetyState,
        over_limit: bool,
        below_half_base_at_limit: bool,
        critical_sensor_missing: bool,
        limits: GuidedSafetyLimits,
    ) -> Option<GuidedStopReason> {
        let reason = stop_reason(
            state,
            over_limit,
            below_half_base_at_limit,
            critical_sensor_missing,
            limits,
        )?;
        self.reason = Some(reason);
        self.phase = if reason == GuidedStopReason::CriticalSensorLost {
            GuidedPhase::SensorLost
        } else {
            GuidedPhase::SafetyStop
        };
        Some(reason)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GuidedResult {
    pub initial_ops_per_second: f64,
    pub sustained_ops_per_second: f64,
    pub relative_percent: Option<f64>,
    pub generator_version: String,
    pub profile: String,
    pub power_context: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparability {
    Comparable,
    NotComparable,
}

pub fn compare_results(before: &GuidedResult, after: &GuidedResult) -> Comparability {
    if before.generator_version == after.generator_version
        && before.profile == after.profile
        && before.power_context == after.power_context
        && before.relative_percent.is_some()
        && after.relative_percent.is_some()
    {
        Comparability::Comparable
    } else {
        Comparability::NotComparable
    }
}

pub trait LoadGenerator {
    fn version(&self) -> &'static str;
    fn profile(&self) -> &'static str;
    fn sample(&mut self, elapsed_ms: u64, threads: u16) -> f64;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct FixedLoopGenerator {
    operations: u64,
}

impl FixedLoopGenerator {
    pub const VERSION: &'static str = "fixed-loop-v1";
    pub const PROFILE: &'static str = "scalar-avx2-documented";
}

impl LoadGenerator for FixedLoopGenerator {
    fn version(&self) -> &'static str {
        Self::VERSION
    }
    fn profile(&self) -> &'static str {
        Self::PROFILE
    }
    fn sample(&mut self, elapsed_ms: u64, threads: u16) -> f64 {
        let ops = elapsed_ms.saturating_mul(u64::from(threads)).saturating_mul(1_000);
        self.operations = self.operations.saturating_add(ops);
        if elapsed_ms == 0 { 0.0 } else { ops as f64 / (elapsed_ms as f64 / 1000.0) }
    }
}

#[cfg(feature = "e2e")]
#[derive(Debug, Default, Clone, Copy)]
pub struct FakeGenerator;

#[cfg(feature = "e2e")]
impl LoadGenerator for FakeGenerator {
    fn version(&self) -> &'static str {
        "fake-loop-e2e-v1"
    }
    fn profile(&self) -> &'static str {
        "fake-no-hardware"
    }
    fn sample(&mut self, elapsed_ms: u64, threads: u16) -> f64 {
        FixedLoopGenerator::default().sample(elapsed_ms, threads)
    }
}

/// How a guided session ends in storage: `completed` only for a finished test; every other end
/// is `aborted` with the reason key the interface shows (never a silent, reason-less abort).
pub fn session_outcome(
    phase: GuidedPhase,
    reason: Option<GuidedStopReason>,
) -> (&'static str, Option<&'static str>) {
    if phase == GuidedPhase::Result {
        return ("completed", None);
    }
    let fallback = match phase {
        GuidedPhase::Cancelled | GuidedPhase::Cancelling => "guided.user_requested",
        GuidedPhase::SafetyStop => "guided.thermal_safety",
        GuidedPhase::SensorLost => "guided.sensor_lost",
        GuidedPhase::Error => "guided.error",
        _ => "guided.interrupted",
    };
    ("aborted", Some(reason.map_or(fallback, GuidedStopReason::key)))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config() -> GuidedConfig {
        GuidedConfig {
            require_ac: true,
            rest_ms: 60_000,
            warmup_ms: 90_000,
            load_ms: 180_000,
            recovery_ms: 120_000,
        }
    }
    fn checks() -> GuidedPreflight {
        GuidedPreflight {
            sensors: true,
            ac_power: true,
            profile: true,
            disk_space: true,
            generator: true,
        }
    }
    #[test]
    fn only_a_finished_test_is_completed_and_every_other_end_says_why() {
        assert_eq!(session_outcome(GuidedPhase::Result, None), ("completed", None));
        assert_eq!(
            session_outcome(GuidedPhase::Cancelled, Some(GuidedStopReason::Battery)),
            ("aborted", Some("guided.battery"))
        );
        assert_eq!(
            session_outcome(GuidedPhase::SafetyStop, None),
            ("aborted", Some("guided.thermal_safety"))
        );
        assert_eq!(
            session_outcome(GuidedPhase::SensorLost, None),
            ("aborted", Some("guided.sensor_lost"))
        );
        assert_eq!(
            session_outcome(GuidedPhase::SteadyLoad, None),
            ("aborted", Some("guided.interrupted")),
            "a loop that ends while the test is still running was interrupted"
        );
    }

    #[test]
    fn state_machine_runs_all_phases_and_allows_skipping_rest() {
        let mut m = GuidedMachine::new(GuidedProfile::Short, config());
        assert!(m.complete_preflight(&checks()));
        assert!(m.begin());
        m.skip_rest();
        assert_eq!(m.phase, GuidedPhase::Warming);
        m.tick(90_000);
        assert_eq!(m.phase, GuidedPhase::SteadyLoad);
        m.tick(180_000);
        assert_eq!(m.phase, GuidedPhase::Recovery);
        m.tick(120_000);
        assert_eq!(m.phase, GuidedPhase::Result);
    }
    #[test]
    fn preflight_blocks_battery_when_ac_is_required() {
        let mut m = GuidedMachine::new(GuidedProfile::Standard, config());
        let c = GuidedPreflight { ac_power: false, ..checks() };
        assert!(!m.complete_preflight(&c));
        assert_eq!(m.phase, GuidedPhase::Error);
    }
    #[test]
    fn cancellation_is_outside_the_worker_and_is_incomplete() {
        let mut m = GuidedMachine::new(GuidedProfile::Standard, config());
        assert!(m.complete_preflight(&checks()));
        assert!(m.begin());
        assert!(m.request_cancel(GuidedStopReason::UserRequested));
        m.tick(1);
        assert_eq!(m.phase, GuidedPhase::Cancelled);
        assert_eq!(m.reason, Some(GuidedStopReason::UserRequested));
    }
    #[test]
    fn hidden_and_suspend_are_explicit_cancellation_reasons() {
        let mut m = GuidedMachine::new(GuidedProfile::Standard, config());
        assert!(m.request_cancel(GuidedStopReason::WindowHidden));
        assert_eq!(m.reason.map(GuidedStopReason::key), Some("guided.window_hidden"));
        let mut s = GuidedMachine::new(GuidedProfile::Standard, config());
        s.suspend();
        assert_eq!(s.reason.map(GuidedStopReason::key), Some("guided.system_suspend"));
    }
    #[test]
    fn safety_only_stops_after_evidence_and_not_at_the_limit_alone() {
        let l = GuidedSafetyLimits {
            over_limit_samples: 3,
            low_clock_ms: 10_000,
            missing_sensor_samples: 3,
            generator_timeout_ms: 5_000,
        };
        let s = GuidedSafetyState {
            consecutive_over_limit: 0,
            consecutive_low_clock_ms: 0,
            consecutive_missing_sensor: 0,
            generator_silent_ms: 0,
        };
        assert_eq!(stop_reason(s, false, false, false, l), None);
        assert_eq!(
            stop_reason(
                GuidedSafetyState { consecutive_over_limit: 3, ..s },
                true,
                false,
                false,
                l
            ),
            Some(GuidedStopReason::ThermalSafety)
        );
        assert_eq!(
            stop_reason(
                GuidedSafetyState { consecutive_low_clock_ms: 10_000, ..s },
                false,
                true,
                false,
                l
            ),
            Some(GuidedStopReason::SevereLowClock)
        );
    }
    #[test]
    fn fixed_loop_reports_throughput_without_hardware_write_surface() {
        let mut g = FixedLoopGenerator::default();
        assert_eq!(g.version(), "fixed-loop-v1");
        assert!(g.sample(1000, 4) > 0.0);
    }

    #[test]
    fn watchdog_is_independent_from_the_load_worker() {
        let mut watchdog = GuidedWatchdog::new(5_000);
        watchdog.response(1_000);
        assert!(!watchdog.expired(5_999));
        assert!(watchdog.expired(6_000));
    }

    #[test]
    fn safety_observation_covers_sensor_loss_and_generator_timeout() {
        let limits = GuidedSafetyLimits {
            over_limit_samples: 3,
            low_clock_ms: 10_000,
            missing_sensor_samples: 3,
            generator_timeout_ms: 5_000,
        };
        let mut machine = GuidedMachine::new(GuidedProfile::Standard, config());
        let state = GuidedSafetyState {
            consecutive_over_limit: 0,
            consecutive_low_clock_ms: 0,
            consecutive_missing_sensor: 3,
            generator_silent_ms: 0,
        };
        assert_eq!(
            machine.observe_safety(state, false, false, true, limits),
            Some(GuidedStopReason::CriticalSensorLost)
        );
        assert_eq!(machine.phase, GuidedPhase::SensorLost);
        let mut generator = GuidedMachine::new(GuidedProfile::Standard, config());
        let state = GuidedSafetyState { generator_silent_ms: 5_000, ..state };
        assert_eq!(
            generator.observe_safety(state, false, false, false, limits),
            Some(GuidedStopReason::GeneratorUnresponsive)
        );
    }

    #[test]
    fn parent_loss_and_battery_are_explicit_cancellation_reasons() {
        let mut parent = GuidedMachine::new(GuidedProfile::Standard, config());
        parent.request_cancel(GuidedStopReason::ParentMissing);
        assert_eq!(parent.reason, Some(GuidedStopReason::ParentMissing));
        let mut battery = GuidedMachine::new(GuidedProfile::Standard, config());
        battery.request_cancel(GuidedStopReason::Battery);
        assert_eq!(battery.reason, Some(GuidedStopReason::Battery));
    }
    #[test]
    fn over_limit_needs_strictly_more_than_the_ruleset_margin() {
        // FR-085 boundary: exactly +2.0 °C does not count, +2.1 °C does.
        assert!(!is_over_limit(Some(82.0), Some(80.0), 2.0));
        assert!(is_over_limit(Some(82.1), Some(80.0), 2.0));
        assert!(!is_over_limit(None, Some(80.0), 2.0));
        assert!(!is_over_limit(Some(82.1), None, 2.0));
    }

    #[test]
    fn severe_throttling_needs_both_the_limit_and_the_ratio_boundary() {
        // FR-085 boundary: exactly 50 % of the base clock does not count, 49 % does — and only
        // while the temperature is actually at (or past) the limit.
        assert!(!is_severely_throttled(Some(80.0), Some(80.0), Some(2000.0), Some(4000.0), 0.5));
        assert!(is_severely_throttled(Some(80.0), Some(80.0), Some(1960.0), Some(4000.0), 0.5));
        assert!(!is_severely_throttled(Some(79.0), Some(80.0), Some(1000.0), Some(4000.0), 0.5));
        assert!(!is_severely_throttled(Some(80.0), Some(80.0), None, Some(4000.0), 0.5));
    }

    #[test]
    fn reaching_the_limit_alone_never_looks_unsafe() {
        // FR-085: the guided test measures reaching the limit; that alone must never register as
        // either predicate, whatever the exact clock happens to be at that instant.
        assert!(!is_over_limit(Some(80.0), Some(80.0), 2.0));
        assert!(!is_severely_throttled(Some(80.0), Some(80.0), Some(3_800.0), Some(4_000.0), 0.5));
    }

    #[test]
    fn the_safety_state_resets_the_instant_a_condition_stops_holding() {
        let state = GuidedSafetyState::ZERO.observe(1_000, true, false, false, true);
        let state = state.observe(1_000, true, false, false, true);
        assert_eq!(state.consecutive_over_limit, 2);
        let state = state.observe(1_000, false, false, false, true);
        assert_eq!(state.consecutive_over_limit, 0);

        let state = GuidedSafetyState::ZERO.observe(4_000, false, true, false, true);
        assert_eq!(state.consecutive_low_clock_ms, 4_000);
        let state = state.observe(1_000, false, false, false, true);
        assert_eq!(state.consecutive_low_clock_ms, 0);

        let silent = GuidedSafetyState::ZERO.observe(2_000, false, false, false, false);
        assert_eq!(silent.generator_silent_ms, 2_000);
        let responded = silent.observe(1_000, false, false, false, true);
        assert_eq!(responded.generator_silent_ms, 0);
    }

    #[test]
    fn the_threaded_generator_counts_real_work_per_thread_and_stops_promptly() {
        let generator = ThreadedGenerator::start(2);
        std::thread::sleep(std::time::Duration::from_millis(50));
        let ops = generator.total_ops();
        assert!(ops > 0, "the workers must have counted real operations by now");

        let stopping = std::time::Instant::now();
        drop(generator);
        assert!(stopping.elapsed() < std::time::Duration::from_secs(2), "drop must join promptly");
    }

    #[test]
    fn requires_ac_reflects_the_config_it_was_built_with() {
        let with_ac = GuidedMachine::new(GuidedProfile::Standard, config());
        assert!(with_ac.requires_ac());
        let without_ac = GuidedMachine::new(
            GuidedProfile::Standard,
            GuidedConfig { require_ac: false, ..config() },
        );
        assert!(!without_ac.requires_ac());
    }

    #[test]
    fn only_compares_results_with_the_same_generation_context() {
        let r = GuidedResult {
            initial_ops_per_second: 100.0,
            sustained_ops_per_second: 90.0,
            relative_percent: Some(-10.0),
            generator_version: "1".to_owned(),
            profile: "standard".to_owned(),
            power_context: "ac:balanced".to_owned(),
        };
        let mut d = r.clone();
        d.profile = "long".to_owned();
        assert_eq!(compare_results(&r, &r), Comparability::Comparable);
        assert_eq!(compare_results(&r, &d), Comparability::NotComparable);
    }
}
