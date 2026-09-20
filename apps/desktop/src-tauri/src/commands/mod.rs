#![deny(clippy::unwrap_used, clippy::expect_used)]

use crate::access::{self, AccessRequest, AccessRequestResult};
use crate::diagnostics::analysis::{EventBoundary, PointQuality, RawPoint, aggregate_track};
use crate::diagnostics::guided::{
    FixedLoopGenerator, GuidedConfig, GuidedMachine, GuidedPhase, GuidedPreflight, GuidedProfile,
    GuidedStopReason, LoadGenerator,
};
use crate::diagnostics::{AdvancedAccess, ConfidenceCeiling, CoverageTier};
use crate::storage::{AppState, OnboardingState, OnboardingStatus, WindowState};
use crate::telemetry::power_context::PowerSource;
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

fn guided_phase_dto(machine: &GuidedMachine) -> GuidedPhaseDto {
    guided_phase_dto_with_throughput(machine, None)
}

fn guided_phase_dto_with_throughput(
    machine: &GuidedMachine,
    throughput_ops_s: Option<f64>,
) -> GuidedPhaseDto {
    GuidedPhaseDto {
        phase: guided_phase_name(machine.phase).to_owned(),
        elapsed_ms: machine.elapsed_ms,
        remaining_ms: machine.remaining_ms(),
        reason_key: machine.reason.map(|reason| reason.key().to_owned()),
        temperature_c: None,
        thermal_limit_c: None,
        active_clock_mhz: None,
        base_clock_mhz: None,
        throughput_ops_s,
        progress_percent: machine.progress_percent(),
    }
}

fn guided_session_id() -> String {
    let millis = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
    format!("guided-{millis}")
}

fn run_guided_loop(
    app: AppHandle,
    machine: Arc<Mutex<Option<GuidedMachine>>>,
    stop_requested: Arc<AtomicBool>,
    session_id: String,
) {
    let mut generator = FixedLoopGenerator::default();
    let threads = std::thread::available_parallelism()
        .map(|value| value.get().min(u16::MAX as usize) as u16)
        .unwrap_or(1);
    loop {
        thread::sleep(Duration::from_millis(1_000));
        if stop_requested.load(Ordering::Acquire) {
            break;
        }
        let Ok(mut guard) = machine.lock() else { break };
        let Some(current) = guard.as_mut() else { break };
        if matches!(
            current.phase,
            GuidedPhase::Cancelled | GuidedPhase::Result | GuidedPhase::Error
        ) {
            break;
        }
        current.tick(1_000);
        let throughput =
            (current.phase == GuidedPhase::SteadyLoad).then(|| generator.sample(1_000, threads));
        let dto = guided_phase_dto_with_throughput(current, throughput);
        let terminal = current.phase == GuidedPhase::Result;
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

fn guided_preflight_for(live: &LiveHandle, require_ac: bool) -> GuidedPreflightDto {
    // Sensors: what the collector delivers right now, not what the catalog advertises.
    let signals = live.read(|live| live.coverage_signals());
    GuidedPreflightDto {
        sensors: signals.temperature && signals.active_clock,
        ac_power: ac_power_available(),
        profile: crate::diagnostics::Ruleset::v1().is_ok(),
        disk_space: std::fs::metadata(".").is_ok(),
        generator: true,
        require_ac,
    }
}

#[tauri::command]
pub fn get_guided_preflight(live: State<'_, LiveHandle>) -> GuidedPreflightDto {
    guided_preflight_for(&live, true)
}

#[tauri::command]
pub fn start_guided(
    app: AppHandle,
    state: State<'_, GuidedController>,
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
    let preflight = guided_preflight_for(&live, request.require_ac);
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
    state.stop_requested.store(false, Ordering::Release);
    state.machine.lock().map_err(|_| CommandError::operation_failed())?.replace(machine);
    let machine_ref = Arc::clone(&state.machine);
    let stop_requested = Arc::clone(&state.stop_requested);
    let session_id = guided_session_id();
    let loop_app = app.clone();
    let _ = thread::Builder::new()
        .name("guided-watchdog".to_owned())
        .spawn(move || run_guided_loop(loop_app, machine_ref, stop_requested, session_id));
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
    let source_points = guard
        .analysis_points(&request.session_id, request.start_ms as i64, request.end_ms as i64)
        .map_err(|_| CommandError::operation_failed())?;
    let source_events = guard
        .analysis_events(&request.session_id, request.start_ms as i64, request.end_ms as i64)
        .map_err(|_| CommandError::operation_failed())?;
    let boundaries: Vec<_> = source_events
        .iter()
        .map(|event| EventBoundary { at_ms: event.start_ms.max(0) as u64 })
        .collect();
    let track_kinds = ["temperature", "clock", "load", "power"];
    let mut tracks = Vec::new();
    let mut source_count = 0_usize;
    for kind in track_kinds {
        let raw: Vec<_> = source_points
            .iter()
            .filter(|point| point.sensor_id.contains(kind))
            .map(|point| RawPoint {
                t_ms: point.monotonic_ms.max(0) as u64,
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
        session_id: request.session_id,
        start_ms: request.start_ms,
        end_ms: request.end_ms,
        is_aggregated: source_count > request.target_points_per_track as usize,
        tracks,
        events,
    })
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
