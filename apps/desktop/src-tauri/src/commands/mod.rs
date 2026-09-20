#![deny(clippy::unwrap_used, clippy::expect_used)]

use crate::access::{self, AccessRequest, AccessRequestResult};
use crate::diagnostics::analysis::{EventBoundary, PointQuality, RawPoint, aggregate_track};
use crate::diagnostics::guided::{
    GuidedConfig, GuidedMachine, GuidedPhase, GuidedPreflight, GuidedProfile, GuidedSafetyState,
    GuidedStopReason, ThreadedGenerator, is_over_limit, is_severely_throttled, safety_limits,
};
use crate::diagnostics::{AdvancedAccess, ConfidenceCeiling, CoverageTier};
use crate::storage::{
    AppState, GuidedCheckpoint, OnboardingState, OnboardingStatus, StoredSampleValue, WindowState,
};
use crate::telemetry::live::CollectorState;
use crate::telemetry::power_context::{PowerContextTracker, PowerSource, PowerTransition};
use crate::telemetry::snapshot::Freshness;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};

mod live;
pub use live::{LiveHandle, TauriObserver};

#[derive(Default)]
pub struct GuidedController {
    pub machine: Arc<Mutex<Option<GuidedMachine>>>,
    pub stop_requested: Arc<AtomicBool>,
}

/// Requests a guided run to stop because the native window lifecycle changed. This is deliberately
/// idempotent: a close event can arrive after a hide/minimize event, and the watchdog still owns the
/// phase transition and the final generator join.
pub fn cancel_guided_for_lifecycle(app: &AppHandle, reason: GuidedStopReason) {
    let Some(state) = app.try_state::<GuidedController>() else { return };
    state.stop_requested.store(true, Ordering::Release);
    let Ok(mut guard) = state.machine.lock() else { return };
    let Some(machine) = guard.as_mut() else { return };
    if machine.request_cancel(reason) {
        machine.tick(1);
        let _ = app.emit("guided:phase", guided_phase_dto(machine));
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct GuidedPreflightDto {
    pub sensors: bool,
    pub ac_power: bool,
    pub profile: bool,
    pub disk_space: bool,
    pub generator: bool,
    pub require_ac: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartGuidedRequest {
    pub profile: String,
    pub skip_rest: bool,
    pub require_ac: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct GuidedPhaseDto {
    pub phase: String,
    pub elapsed_ms: u64,
    pub remaining_ms: Option<u64>,
    pub reason_key: Option<String>,
    pub temperature_c: Option<f64>,
    pub thermal_limit_c: Option<f64>,
    pub active_clock_mhz: Option<f64>,
    pub base_clock_mhz: Option<f64>,
    pub throughput_ops_s: Option<f64>,
    pub progress_percent: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GuidedFinishedDto {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnalysisWindowDto {
    pub session_id: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub is_aggregated: bool,
    pub tracks: Vec<AnalysisTrackDto>,
    pub events: Vec<AnalysisEventDto>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnalysisTrackDto {
    pub kind: String,
    pub points: Vec<AnalysisPointDto>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnalysisPointDto {
    pub start_ms: u64,
    pub end_ms: u64,
    pub first: Option<f64>,
    pub last: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub average: Option<f64>,
    pub quality: PointQuality,
    pub gap: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnalysisEventDto {
    pub id: String,
    pub kind: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub label: String,
    pub informational: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CpuTopologyDto {
    pub cores: Vec<CpuCoreDto>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CpuCoreDto {
    pub id: String,
    pub index: u16,
    pub group: &'static str,
    pub temperature_c: Option<f64>,
    pub clock_mhz: Option<f64>,
    pub load_percent: Option<f64>,
    pub throttling: Option<bool>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalysisWindowRequest {
    pub session_id: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub target_points_per_track: u16,
}

#[tauri::command]
pub fn get_cpu_topology(live: State<'_, LiveHandle>) -> CpuTopologyDto {
    live.read(live::topology_dto)
}

fn guided_phase_name(phase: GuidedPhase) -> &'static str {
    match phase {
        GuidedPhase::Preflight => "preflight",
        GuidedPhase::Ready => "ready",
        GuidedPhase::Rest => "rest",
        GuidedPhase::Warming => "warming",
        GuidedPhase::SteadyLoad => "steady_load",
        GuidedPhase::Recovery => "recovery",
        GuidedPhase::Cancelling => "cancelling",
        GuidedPhase::Cancelled => "cancelled",
        GuidedPhase::SafetyStop => "safety_stop",
        GuidedPhase::SensorLost => "sensor_lost",
        GuidedPhase::Error => "error",
        GuidedPhase::Result => "result",
    }
}

fn guided_profile_name(profile: GuidedProfile) -> &'static str {
    match profile {
        GuidedProfile::Short => "short",
        GuidedProfile::Standard => "standard",
        GuidedProfile::Long => "long",
    }
}

fn guided_phase_dto(machine: &GuidedMachine) -> GuidedPhaseDto {
    GuidedPhaseDto {
        phase: guided_phase_name(machine.phase).to_owned(),
        elapsed_ms: machine.elapsed_ms,
        remaining_ms: machine.remaining_ms(),
        reason_key: machine.reason.map(|reason| reason.key().to_owned()),
        temperature_c: None,
        thermal_limit_c: None,
        active_clock_mhz: None,
        base_clock_mhz: None,
        throughput_ops_s: None,
        progress_percent: machine.progress_percent(),
    }
}

fn guided_session_id() -> String {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    format!("guided-{nanos}")
}

/// Readings taken directly from the live collector state, as they are during the guided test:
/// never fabricated, `None` when the sensor is unreadable.
struct GuidedReadings {
    temperature_c: Option<f64>,
    thermal_limit_c: Option<f64>,
    active_clock_mhz: Option<f64>,
    base_clock_mhz: Option<f64>,
}

fn guided_readings(live: &LiveHandle) -> GuidedReadings {
    live.read(|live| {
        let input = live.snapshot_input();
        GuidedReadings {
            temperature_c: input.as_ref().and_then(|input| input.temperature_c),
            thermal_limit_c: input.as_ref().and_then(|input| input.thermal_limit_c),
            active_clock_mhz: input.as_ref().and_then(|input| input.active_clock_mhz),
            base_clock_mhz: input.as_ref().and_then(|input| input.base_clock_mhz),
        }
    })
}

fn persist_guided_tick(
    app: &AppHandle,
    live: &LiveHandle,
    session_id: &str,
    sequence: u64,
    machine: &GuidedMachine,
) {
    let Some(sequence) = i64::try_from(sequence).ok() else { return };
    let (monotonic_ms, values) = live.read(|live| {
        let captured_at_ms = live.snapshot_input().map(|input| input.captured_at_ms);
        let values = live
            .analysis_values()
            .into_iter()
            .map(|value| StoredSampleValue {
                sensor_id: value.sensor_id,
                value: value.value,
                boolean: value.boolean,
                quality: value.quality,
            })
            .collect::<Vec<_>>();
        (captured_at_ms, values)
    });
    let monotonic_ms = monotonic_ms
        .and_then(|value| i64::try_from(value).ok())
        .unwrap_or_else(|| i64::try_from(machine.elapsed_ms).unwrap_or(i64::MAX));
    let Some(state) = app.try_state::<AppState>() else { return };
    let Ok(storage) = state.storage.lock() else { return };
    if storage.insert_sample(session_id, sequence, monotonic_ms, 1_000, &values).is_err() {
        return;
    }
    let checkpoint = GuidedCheckpoint {
        session_id: session_id.to_owned(),
        phase: guided_phase_name(machine.phase).to_owned(),
        profile: guided_profile_name(machine.profile).to_owned(),
        elapsed_ms: i64::try_from(machine.elapsed_ms).unwrap_or(i64::MAX),
        reason: machine.reason.map(|reason| reason.key().to_owned()),
        updated_at: jiff::Timestamp::now().to_string(),
    };
    let _ = storage.save_guided_checkpoint(&checkpoint);
}

/// Runs on its own thread, independent from anything the interface does (T057/T166): it is the
/// one place that may call [`GuidedMachine::observe_safety`], reading the real collector state on
/// every tick instead of trusting whatever the interface last asked for.
fn run_guided_loop(
    app: AppHandle,
    machine: Arc<Mutex<Option<GuidedMachine>>>,
    stop_requested: Arc<AtomicBool>,
    live: LiveHandle,
    session_id: String,
) {
    let Ok(rules) = crate::diagnostics::Ruleset::v1() else {
        tracing::error!(component = "core", msg = "guided loop: ruleset failed to parse");
        return;
    };
    let Some(limits) = safety_limits(&rules) else { return };
    let Some(over_limit_c) = rules.parameter("guided.stop_over_limit_c") else { return };
    let Some(low_freq_ratio) = rules.parameter("guided.stop_low_freq_ratio") else { return };
    let threads = std::thread::available_parallelism()
        .map(|value| value.get().min(u16::MAX as usize) as u16)
        .unwrap_or(1);

    let mut generator: Option<ThreadedGenerator> = None;
    let mut generator_ops_at_last_tick: u64 = 0;
    let mut safety = GuidedSafetyState::ZERO;
    let mut sequence = 0_u64;
    let mut power_tracker = PowerContextTracker::default();
    let parent_pid = crate::ipc::supervisor::current_parent_process_id();

    loop {
        thread::sleep(Duration::from_millis(1_000));
        if stop_requested.load(Ordering::Acquire) {
            break;
        }
        let Ok(mut guard) = machine.lock() else { break };
        let Some(current) = guard.as_mut() else { break };
        if matches!(
            current.phase,
            GuidedPhase::Cancelled
                | GuidedPhase::Result
                | GuidedPhase::Error
                | GuidedPhase::SafetyStop
                | GuidedPhase::SensorLost
        ) {
            break;
        }

        if parent_pid.is_some_and(|pid| !crate::ipc::supervisor::parent_process_alive(pid)) {
            current.request_cancel(GuidedStopReason::ParentMissing);
        }

        if let Some(power) = crate::telemetry::power_context::read_windows_power_context()
            && power_tracker.observe(crate::commands::live::epoch_ms(), power.source, power.resumed)
                == PowerTransition::Resumed
        {
            current.suspend();
        }

        let collector_failed = live.read(|live| {
            matches!(live.collector(), CollectorState::Stopped | CollectorState::Failed)
        });
        if collector_failed {
            current.request_cancel(GuidedStopReason::CriticalSensorLost);
        }

        // Losing AC mid-run stops the test outright when it was required to start it (independent
        // of the phase timer): this is a cancellation, not a safety-limit stop.
        if current.requires_ac() && !ac_power_available() {
            current.request_cancel(GuidedStopReason::Battery);
        }

        current.tick(1_000);

        let loads = matches!(current.phase, GuidedPhase::Warming | GuidedPhase::SteadyLoad);
        match (loads, generator.is_some()) {
            (true, false) => {
                generator = Some(ThreadedGenerator::start(threads));
                generator_ops_at_last_tick = 0;
            }
            (false, true) => generator = None, // dropping joins the workers before this returns
            _ => {}
        }

        let readings = guided_readings(&live);
        let mut throughput_ops_s = None;
        if loads {
            let total_ops = generator.as_ref().map_or(0, ThreadedGenerator::total_ops);
            let delta = total_ops.saturating_sub(generator_ops_at_last_tick);
            generator_ops_at_last_tick = total_ops;
            throughput_ops_s = Some(delta as f64); // one tick = one second

            let over_limit =
                is_over_limit(readings.temperature_c, readings.thermal_limit_c, over_limit_c);
            let severely_throttled = is_severely_throttled(
                readings.temperature_c,
                readings.thermal_limit_c,
                readings.active_clock_mhz,
                readings.base_clock_mhz,
                low_freq_ratio,
            );
            let sensor_missing = readings.temperature_c.is_none();
            let generator_progressed = delta > 0;
            safety = safety.observe(
                1_000,
                over_limit,
                severely_throttled,
                sensor_missing,
                generator_progressed,
            );
            current.observe_safety(safety, over_limit, severely_throttled, sensor_missing, limits);
        } else {
            safety = GuidedSafetyState::ZERO;
        }

        let dto = GuidedPhaseDto {
            phase: guided_phase_name(current.phase).to_owned(),
            elapsed_ms: current.elapsed_ms,
            remaining_ms: current.remaining_ms(),
            reason_key: current.reason.map(|reason| reason.key().to_owned()),
            temperature_c: readings.temperature_c,
            thermal_limit_c: readings.thermal_limit_c,
            active_clock_mhz: readings.active_clock_mhz,
            base_clock_mhz: readings.base_clock_mhz,
            throughput_ops_s,
            progress_percent: current.progress_percent(),
        };
        let terminal = current.phase == GuidedPhase::Result;
        persist_guided_tick(&app, &live, &session_id, sequence, current);
        sequence = sequence.saturating_add(1);
        drop(guard);
        if app.emit("guided:phase", &dto).is_err() {
            break;
        }
        if terminal {
            let _ = app.emit("guided:finished", &GuidedFinishedDto { session_id });
            break;
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CoverageDto {
    pub tier: CoverageTierDto,
    pub confidence_ceiling: ConfidenceDto,
    pub advanced_access: AdvancedAccessDto,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum CoverageTierDto {
    A,
    B,
    C,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfidenceDto {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdvancedAccessDto {
    NotNeeded,
    Available,
    Installable,
    Denied,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommandError {
    pub code: &'static str,
    pub message_key: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct OnboardingStateDto {
    pub flow_version: i64,
    pub last_slide: i64,
    pub status: &'static str,
    pub completed_at: Option<String>,
    pub last_seen_notice_version: i64,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetOnboardingStateRequest {
    pub flow_version: i64,
    pub last_slide: i64,
    pub status: String,
    pub completed_at: Option<String>,
    pub last_seen_notice_version: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct WindowStateDto {
    pub restored_x: i64,
    pub restored_y: i64,
    pub restored_width: i64,
    pub restored_height: i64,
    pub maximized: bool,
    pub display_fingerprint: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetWindowStateRequest {
    pub restored_x: i64,
    pub restored_y: i64,
    pub restored_width: i64,
    pub restored_height: i64,
    pub maximized: bool,
    pub display_fingerprint: Option<String>,
    pub updated_at: String,
}

impl CommandError {
    fn operation_failed() -> Self {
        Self { code: "LOW_LEVEL_ACCESS_OPERATION_FAILED", message_key: "access.operation_failed" }
    }

    fn access_not_installable() -> Self {
        Self { code: "ACCESS_NOT_INSTALLABLE", message_key: "coverage.access_not_installable" }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CoverageMatrixDto {
    pub tier: CoverageTierDto,
    pub confidence_ceiling: ConfidenceDto,
    pub advanced_access: AdvancedAccessDto,
    pub rows: Vec<CoverageRowDto>,
    pub conclusion_key: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct CoverageRowDto {
    pub id: &'static str,
    pub label: &'static str,
    pub available: bool,
    pub quality: &'static str,
    pub quality_label: &'static str,
    pub source_label: &'static str,
    pub reason_key: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveSnapshotDto {
    pub captured_at_ms: u64,
    pub freshness: &'static str,
    pub age_ms: u64,
    pub temperature_c: Option<f64>,
    pub thermal_limit_c: Option<f64>,
    pub thermal_margin_c: Option<f64>,
    pub load_percent: Option<f64>,
    pub active_clock_mhz: Option<f64>,
    pub base_clock_mhz: Option<f64>,
    pub package_power_w: Option<f64>,
    pub power_limit_w: Option<f64>,
    pub classification: Option<&'static str>,
    pub severity: Option<&'static str>,
    pub cpu_label: String,
    pub topology_label: String,
    pub power_label: String,
    pub collector_state: &'static str,
    pub coverage: CoverageDto,
    pub confidence_label: String,
    pub active_cores: Option<u16>,
    pub platform_kind: Option<&'static str>,
    pub in_turbo_window: bool,
}

#[tauri::command]
pub fn get_coverage(state: State<'_, AppState>, live: State<'_, LiveHandle>) -> CoverageMatrixDto {
    live.read(|live| live::coverage_dto(live, advanced_access_enabled(&state)))
}

fn advanced_access_enabled(state: &AppState) -> bool {
    state
        .storage
        .lock()
        .map(|storage| storage.advanced_access_enabled().unwrap_or(true))
        .unwrap_or(true)
}

fn ac_power_available() -> bool {
    #[cfg(windows)]
    {
        crate::telemetry::power_context::read_windows_power_context()
            .is_some_and(|context| context.source == PowerSource::Ac)
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Whether the volume that holds `app`'s data directory has at least `guided.min_free_disk_mb`
/// free (T167): the test writes real samples for minutes and must not start if it plausibly
/// cannot hold them. `None`/unreadable counts as failing the check, never as passing it.
fn guided_disk_space_ok(app: &AppHandle, rules: &crate::diagnostics::Ruleset) -> bool {
    let Some(min_free_mb) = rules.parameter("guided.min_free_disk_mb") else { return false };
    let Ok(data_dir) = app.path().app_data_dir() else { return false };
    crate::diagnostics::disk_space::free_disk_mb(&data_dir)
        .is_some_and(|free_mb| free_mb as f64 >= min_free_mb)
}

fn guided_preflight_for(
    app: &AppHandle,
    live: &LiveHandle,
    require_ac: bool,
) -> GuidedPreflightDto {
    // Sensors: what the collector delivers right now, not what the catalog advertises.
    let signals = live.read(|live| live.coverage_signals());
    let rules = crate::diagnostics::Ruleset::v1();
    GuidedPreflightDto {
        sensors: signals.temperature && signals.active_clock,
        ac_power: ac_power_available(),
        profile: rules.is_ok(),
        disk_space: rules.as_ref().is_ok_and(|rules| guided_disk_space_ok(app, rules)),
        // The threads the real generator will spawn: whether the OS can even report this.
        generator: std::thread::available_parallelism().is_ok(),
        require_ac,
    }
}

#[tauri::command]
pub fn get_guided_preflight(app: AppHandle, live: State<'_, LiveHandle>) -> GuidedPreflightDto {
    guided_preflight_for(&app, &live, true)
}

#[tauri::command]
pub fn start_guided(
    app: AppHandle,
    state: State<'_, GuidedController>,
    storage_state: State<'_, AppState>,
    live: State<'_, LiveHandle>,
    request: StartGuidedRequest,
) -> Result<GuidedPhaseDto, CommandError> {
    let profile = match request.profile.as_str() {
        "short" => GuidedProfile::Short,
        "standard" => GuidedProfile::Standard,
        "long" => GuidedProfile::Long,
        _ => return Err(CommandError::operation_failed()),
    };
    let rules = crate::diagnostics::Ruleset::v1().map_err(|_| CommandError::operation_failed())?;
    let config = GuidedConfig::from_ruleset(&rules, profile, request.require_ac)
        .ok_or_else(CommandError::operation_failed)?;
    let preflight = guided_preflight_for(&app, &live, request.require_ac);
    let checks = GuidedPreflight {
        sensors: preflight.sensors,
        ac_power: preflight.ac_power,
        profile: preflight.profile,
        disk_space: preflight.disk_space,
        generator: preflight.generator,
    };
    let mut machine = GuidedMachine::new(profile, config);
    if !machine.complete_preflight(&checks) || !machine.begin() {
        return Err(CommandError::operation_failed());
    }
    if request.skip_rest {
        machine.skip_rest();
    }
    let dto = guided_phase_dto(&machine);
    let session_id = guided_session_id();
    let storage = storage_state.storage.lock().map_err(|_| CommandError::operation_failed())?;
    storage.create_guided_session(&session_id).map_err(|_| CommandError::operation_failed())?;
    state.stop_requested.store(false, Ordering::Release);
    state.machine.lock().map_err(|_| CommandError::operation_failed())?.replace(machine);
    let machine_ref = Arc::clone(&state.machine);
    let stop_requested = Arc::clone(&state.stop_requested);
    let loop_app = app.clone();
    let loop_live = live.inner().clone();
    let _ = thread::Builder::new().name("guided-watchdog".to_owned()).spawn(move || {
        run_guided_loop(loop_app, machine_ref, stop_requested, loop_live, session_id)
    });
    app.emit("guided:phase", &dto).map_err(|_| CommandError::operation_failed())?;
    Ok(dto)
}

#[tauri::command]
pub fn stop_guided(
    app: AppHandle,
    state: State<'_, GuidedController>,
) -> Result<GuidedPhaseDto, CommandError> {
    state.stop_requested.store(true, Ordering::Release);
    let mut guard = state.machine.lock().map_err(|_| CommandError::operation_failed())?;
    let machine = guard.as_mut().ok_or_else(CommandError::operation_failed)?;
    if !machine.request_cancel(GuidedStopReason::UserRequested) {
        return Err(CommandError::operation_failed());
    }
    machine.tick(1);
    let dto = guided_phase_dto(machine);
    app.emit("guided:phase", &dto).map_err(|_| CommandError::operation_failed())?;
    Ok(dto)
}

#[tauri::command]
pub fn get_analysis_window(
    state: State<'_, AppState>,
    request: AnalysisWindowRequest,
) -> Result<AnalysisWindowDto, CommandError> {
    if request.session_id.is_empty()
        || request.end_ms <= request.start_ms
        || !(1..=3_000).contains(&request.target_points_per_track)
    {
        return Err(CommandError::operation_failed());
    }
    let guard = state.storage.lock().map_err(|_| CommandError::operation_failed())?;
    let session_id = if request.session_id == "latest" {
        guard
            .latest_session_id()
            .map_err(|_| CommandError::operation_failed())?
            .ok_or_else(CommandError::operation_failed)?
    } else {
        request.session_id.clone()
    };
    let start_ms = i64::try_from(request.start_ms).map_err(|_| CommandError::operation_failed())?;
    let end_ms = i64::try_from(request.end_ms).map_err(|_| CommandError::operation_failed())?;
    let source_points = guard
        .analysis_points(&session_id, start_ms, end_ms)
        .map_err(|_| CommandError::operation_failed())?;
    let source_events = guard
        .analysis_events(&session_id, start_ms, end_ms)
        .map_err(|_| CommandError::operation_failed())?;
    let boundaries: Vec<_> = source_events
        .iter()
        .flat_map(|event| {
            [event.start_ms, event.end_ms]
                .into_iter()
                .filter_map(|at_ms| u64::try_from(at_ms).ok())
                .map(|at_ms| EventBoundary { at_ms })
        })
        .collect();
    let track_kinds = ["temperature", "clock", "load", "power"];
    let mut tracks = Vec::new();
    let mut source_count = 0_usize;
    for kind in track_kinds {
        let raw: Vec<_> = source_points
            .iter()
            .filter(|point| analysis_track_for_sensor_id(&point.sensor_id) == Some(kind))
            .map(|point| RawPoint {
                t_ms: u64::try_from(point.monotonic_ms).unwrap_or(0),
                value: point.value,
                quality: match point.quality.as_str() {
                    "complete" | "direct" => PointQuality::Complete,
                    "missing" | "invalid" => PointQuality::Missing,
                    _ => PointQuality::Reduced,
                },
                gap: point.value.is_none(),
            })
            .collect();
        source_count += raw.len();
        if raw.is_empty() {
            continue;
        }
        let points = aggregate_track(
            &raw,
            request.start_ms,
            request.end_ms,
            request.target_points_per_track as usize,
            &boundaries,
        )
        .into_iter()
        .map(|point| AnalysisPointDto {
            start_ms: point.start_ms,
            end_ms: point.end_ms,
            first: point.first,
            last: point.last,
            min: point.min,
            max: point.max,
            average: point.average,
            quality: point.quality,
            gap: point.gap,
        })
        .collect();
        tracks.push(AnalysisTrackDto { kind: kind.to_owned(), points });
    }
    let events = source_events
        .into_iter()
        .map(|event| {
            let informational = matches!(event.kind.as_str(), "turbo_end" | "oem_mode_change");
            let kind = match event.kind.as_str() {
                "thermal" => "thermal",
                "power" | "current" => "electrical",
                "platform" => "platform",
                "mixed" => "mixed",
                _ => "info",
            };
            AnalysisEventDto {
                id: event.id,
                kind: kind.to_owned(),
                start_ms: event.start_ms.max(0) as u64,
                end_ms: event.end_ms.max(0) as u64,
                label: kind.to_owned(),
                informational,
            }
        })
        .collect();
    Ok(AnalysisWindowDto {
        session_id,
        start_ms: request.start_ms,
        end_ms: request.end_ms,
        is_aggregated: source_count > request.target_points_per_track as usize,
        tracks,
        events,
    })
}

/// Stable catalog IDs are the only source of chart membership. Do not replace this with a
/// substring check: a future `cpu.package.power_limit` must not become a power track by accident.
fn analysis_track_for_sensor_id(sensor_id: &str) -> Option<&'static str> {
    match sensor_id {
        "cpu.package.temp" => Some("temperature"),
        "cpu.package.clock" => Some("clock"),
        "cpu.package.load" => Some("load"),
        "cpu.package.power" => Some("power"),
        _ => None,
    }
}

#[tauri::command]
pub fn get_onboarding_state(
    state: State<'_, AppState>,
) -> Result<OnboardingStateDto, CommandError> {
    let guard = state.storage.lock().map_err(|_| CommandError::operation_failed())?;
    let value = guard.onboarding_state().map_err(|_| CommandError::operation_failed())?;
    Ok(onboarding_state_dto(value))
}

#[tauri::command]
pub fn set_onboarding_state(
    state: State<'_, AppState>,
    request: SetOnboardingStateRequest,
) -> Result<OnboardingStateDto, CommandError> {
    let status = match request.status.as_str() {
        "pending" => OnboardingStatus::Pending,
        "completed" => OnboardingStatus::Completed,
        "skipped" => OnboardingStatus::Skipped,
        _ => return Err(CommandError::operation_failed()),
    };
    let value = OnboardingState {
        flow_version: request.flow_version,
        last_slide: request.last_slide,
        status,
        completed_at: request.completed_at,
        last_seen_notice_version: request.last_seen_notice_version,
    };
    let guard = state.storage.lock().map_err(|_| CommandError::operation_failed())?;
    guard.set_onboarding_state(value.clone()).map_err(|_| CommandError::operation_failed())?;
    Ok(onboarding_state_dto(value))
}

#[tauri::command]
pub fn get_window_state(state: State<'_, AppState>) -> Result<WindowStateDto, CommandError> {
    let guard = state.storage.lock().map_err(|_| CommandError::operation_failed())?;
    let value = guard.window_state().map_err(|_| CommandError::operation_failed())?;
    Ok(window_state_dto(value))
}

#[tauri::command]
pub fn set_window_state(
    state: State<'_, AppState>,
    request: SetWindowStateRequest,
) -> Result<WindowStateDto, CommandError> {
    let value = WindowState {
        restored_x: request.restored_x,
        restored_y: request.restored_y,
        restored_width: request.restored_width,
        restored_height: request.restored_height,
        maximized: request.maximized,
        display_fingerprint: request.display_fingerprint,
        updated_at: request.updated_at,
    };
    let guard = state.storage.lock().map_err(|_| CommandError::operation_failed())?;
    guard.set_window_state(value.clone()).map_err(|_| CommandError::operation_failed())?;
    Ok(window_state_dto(value))
}

#[tauri::command]
pub fn recheck_coverage(
    state: State<'_, AppState>,
    live: State<'_, LiveHandle>,
) -> CoverageMatrixDto {
    get_coverage(state, live)
}

#[tauri::command]
pub fn get_live_snapshot(
    state: State<'_, AppState>,
    live: State<'_, LiveHandle>,
) -> LiveSnapshotDto {
    live.read(|live| live::snapshot_dto(live, advanced_access_enabled(&state), live::epoch_ms()))
}

#[tauri::command]
pub fn request_low_level_access(
    app: AppHandle,
    state: State<'_, AppState>,
    request: AccessRequest,
) -> Result<AccessRequestResult, CommandError> {
    let manifest = access::manifest().map_err(|_| CommandError::operation_failed())?;
    let installation = access::detect_installation(&manifest.minimum_version)
        .map_err(|_| CommandError::operation_failed())?;
    if !access::action_is_compatible(request.action, &installation) {
        return Err(CommandError::access_not_installable());
    }
    let resource_dir = app.path().resource_dir().map_err(|_| CommandError::operation_failed())?;
    let installer = access::installer_path(&resource_dir, &manifest);
    access::verify_installer(&installer, &manifest)
        .map_err(|_| CommandError::operation_failed())?;
    let launcher = std::env::current_exe().map_err(|_| CommandError::operation_failed())?;
    access::launch_installer_and_register_task(&installer, &launcher, request.action)
        .map_err(|_| CommandError::operation_failed())?;
    let guard = state.storage.lock().map_err(|_| CommandError::operation_failed())?;
    guard.set_advanced_access_enabled(true).map_err(|_| CommandError::operation_failed())?;
    Ok(AccessRequestResult {
        state: "install_requested",
        action: request.action,
        reboot_may_be_required: true,
    })
}

#[tauri::command]
pub fn disable_advanced_access(
    state: State<'_, AppState>,
    live: State<'_, LiveHandle>,
) -> Result<CoverageMatrixDto, CommandError> {
    let guard = state.storage.lock().map_err(|_| CommandError::operation_failed())?;
    guard.set_advanced_access_enabled(false).map_err(|_| CommandError::operation_failed())?;
    Ok(live.read(|live| live::coverage_dto(live, false)))
}

fn tier(value: CoverageTier) -> CoverageTierDto {
    match value {
        CoverageTier::A => CoverageTierDto::A,
        CoverageTier::B => CoverageTierDto::B,
        CoverageTier::C => CoverageTierDto::C,
    }
}

fn confidence(value: ConfidenceCeiling) -> ConfidenceDto {
    match value {
        ConfidenceCeiling::Low => ConfidenceDto::Low,
        ConfidenceCeiling::Medium => ConfidenceDto::Medium,
        ConfidenceCeiling::High => ConfidenceDto::High,
    }
}

fn access(value: AdvancedAccess) -> AdvancedAccessDto {
    match value {
        AdvancedAccess::NotNeeded => AdvancedAccessDto::NotNeeded,
        AdvancedAccess::Available => AdvancedAccessDto::Available,
        AdvancedAccess::Installable => AdvancedAccessDto::Installable,
        AdvancedAccess::Denied => AdvancedAccessDto::Denied,
        AdvancedAccess::Error => AdvancedAccessDto::Error,
    }
}

fn onboarding_state_dto(value: OnboardingState) -> OnboardingStateDto {
    OnboardingStateDto {
        flow_version: value.flow_version,
        last_slide: value.last_slide,
        status: match value.status {
            OnboardingStatus::Pending => "pending",
            OnboardingStatus::Completed => "completed",
            OnboardingStatus::Skipped => "skipped",
        },
        completed_at: value.completed_at,
        last_seen_notice_version: value.last_seen_notice_version,
    }
}

fn window_state_dto(value: WindowState) -> WindowStateDto {
    WindowStateDto {
        restored_x: value.restored_x,
        restored_y: value.restored_y,
        restored_width: value.restored_width,
        restored_height: value.restored_height,
        maximized: value.maximized,
        display_fingerprint: value.display_fingerprint,
        updated_at: value.updated_at,
    }
}

fn freshness(value: Freshness) -> &'static str {
    match value {
        Freshness::Fresh => "fresh",
        Freshness::Stale => "stale",
        Freshness::Disconnected => "disconnected",
    }
}
