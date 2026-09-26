#![deny(clippy::unwrap_used, clippy::expect_used)]

use crate::access::{self, AccessRequest, AccessRequestResult};
use crate::diagnostics::analysis::{EventBoundary, PointQuality, RawPoint, aggregate_track};
use crate::diagnostics::guided::{
    GuidedConfig, GuidedMachine, GuidedPhase, GuidedPreflight, GuidedProfile, GuidedSafetyState,
    GuidedStopReason, ThreadedGenerator, is_over_limit, is_severely_throttled, safety_limits,
};
use crate::diagnostics::{AdvancedAccess, ConfidenceCeiling, CoverageTier};
use crate::export::{
    ExportFormat, ExportScope, WriteOutcome, bundle_from_snapshot,
    preview_export as build_export_preview, write_csv_checked,
};
use crate::storage::{
    AppState, ExportSnapshot, GuidedCheckpoint, OnboardingState, OnboardingStatus, SessionSummary,
    StoredReport, StoredSampleValue, WindowState,
};
use crate::telemetry::live::CollectorState;
use crate::telemetry::power_context::{PowerContextTracker, PowerSource, PowerTransition};
use crate::telemetry::snapshot::Freshness;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

mod live;
pub use live::{LiveHandle, RecorderHandle, StorageResilience, TauriObserver, epoch_ms};

/// Owns the flag [`cancel_export`] sets and `export` checks (T172's export cancellation): reset
/// at the start of every export, so a stale cancellation can never affect the next one.
#[derive(Default)]
pub struct ExportController {
    cancelled: Arc<AtomicBool>,
}

#[derive(Default)]
pub struct GuidedController {
    pub machine: Arc<Mutex<Option<GuidedMachine>>>,
    pub stop_requested: Arc<AtomicBool>,
}

#[derive(Default)]
pub struct TrayController {
    pub paused: AtomicBool,
}

#[derive(Default)]
pub struct FrontendLogLimiter {
    timestamps: Mutex<VecDeque<Instant>>,
}

impl FrontendLogLimiter {
    fn take(&self, requested: usize, now: Instant) -> usize {
        let Ok(mut timestamps) = self.timestamps.lock() else { return 0 };
        while timestamps
            .front()
            .is_some_and(|at| now.saturating_duration_since(*at) >= Duration::from_secs(60))
        {
            timestamps.pop_front();
        }
        let remaining = 60_usize.saturating_sub(timestamps.len());
        let accepted = requested.min(remaining);
        timestamps.extend(std::iter::repeat_n(now, accepted));
        accepted
    }
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
    let values = live.read(|live| {
        live.analysis_values()
            .into_iter()
            .map(|value| StoredSampleValue {
                sensor_id: value.sensor_id,
                value: value.value,
                boolean: value.boolean,
                quality: value.quality,
            })
            .collect::<Vec<_>>()
    });
    // Session-relative, like every stored frame: the analysis view asks for `0..24 h`.
    let monotonic_ms = i64::try_from(machine.elapsed_ms).unwrap_or(i64::MAX);
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
    let mut timeline = crate::telemetry::timeline::Timeline::default();
    guided_loop_body(&app, &machine, &stop_requested, &live, &session_id, &mut timeline);
    finish_guided_session(&app, &machine, &live, &session_id, &mut timeline);
}

/// Whether a guided test is running: the passive recorder stands aside meanwhile, so the two never
/// write overlapping sessions for the same period.
pub fn guided_in_progress(app: &AppHandle) -> bool {
    app.try_state::<GuidedController>().is_some_and(|controller| {
        controller.machine.lock().is_ok_and(|guard| {
            guard.as_ref().is_some_and(|current| {
                !matches!(
                    current.phase,
                    GuidedPhase::Cancelled
                        | GuidedPhase::Result
                        | GuidedPhase::Error
                        | GuidedPhase::SafetyStop
                        | GuidedPhase::SensorLost
                )
            })
        })
    })
}

/// Closes the stored session however the loop ended: without this a finished or aborted test
/// stayed `running` for ever, so nothing could delete data or reset the application afterwards.
fn finish_guided_session(
    app: &AppHandle,
    machine: &Mutex<Option<GuidedMachine>>,
    live: &LiveHandle,
    session_id: &str,
    timeline: &mut crate::telemetry::timeline::Timeline,
) {
    let (phase, reason, elapsed_ms) = match machine.lock() {
        Ok(guard) => match guard.as_ref() {
            Some(current) => (current.phase, current.reason, current.elapsed_ms),
            None => (GuidedPhase::Error, None, 0),
        },
        Err(_) => (GuidedPhase::Error, None, 0),
    };
    let (status, incomplete_reason) = crate::diagnostics::guided::session_outcome(phase, reason);
    let tier = live.read(|live| match live.coverage_signals().tier() {
        CoverageTier::A => "A",
        CoverageTier::B => "B",
        CoverageTier::C => "C",
    });
    let Some(state) = app.try_state::<AppState>() else { return };
    let Ok(storage) = state.storage.lock() else { return };
    let _ = storage.set_session_coverage_tier(session_id, tier);
    let closed = storage
        .finish_session(
            session_id,
            status,
            &jiff::Timestamp::now().to_string(),
            i64::try_from(elapsed_ms).unwrap_or(i64::MAX),
            incomplete_reason,
        )
        .unwrap_or(false);
    if !closed {
        return;
    }
    // The limitations and the report come from what the engine concluded during the test, even
    // if it was cut short.
    let Ok(rules) = crate::diagnostics::Ruleset::v1() else { return };
    let min_ms =
        rules.parameter("session.class_min_s").map_or(60_000, |seconds| (seconds * 1_000.0) as u64);
    for span in timeline.drain_all(min_ms) {
        let _ = storage.insert_limit_event(
            session_id,
            span.kind.storage_key(),
            span.start_seq,
            span.end_seq,
        );
    }
    let _ = storage.freeze_report(session_id, timeline.classify(&rules).as_ref());
}

fn guided_loop_body(
    app: &AppHandle,
    machine: &Arc<Mutex<Option<GuidedMachine>>>,
    stop_requested: &Arc<AtomicBool>,
    live: &LiveHandle,
    session_id: &str,
    timeline: &mut crate::telemetry::timeline::Timeline,
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

        let readings = guided_readings(live);
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
        persist_guided_tick(app, live, session_id, sequence, current);
        if let Ok(seq) = i64::try_from(sequence) {
            let verdict = live.read(|live| live.diagnostic().cloned());
            timeline.push(current.elapsed_ms, seq, verdict.as_ref());
        }
        sequence = sequence.saturating_add(1);
        drop(guard);
        if app.emit("guided:phase", &dto).is_err() {
            break;
        }
        if terminal {
            // Close the stored session first: the interface reads it as soon as it hears this.
            finish_guided_session(app, machine, live, session_id, timeline);
            crate::alerting::guided_finished(app, session_id);
            let _ = app
                .emit("guided:finished", &GuidedFinishedDto { session_id: session_id.to_owned() });
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
    Upgradable,
    Denied,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommandError {
    pub code: &'static str,
    pub message_key: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionSummaryDto {
    pub session_id: String,
    pub kind: String,
    pub status: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub coverage_tier: String,
    pub is_reference: bool,
    pub frame_count: i64,
    pub report_classification: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionPageDto {
    pub sessions: Vec<SessionSummaryDto>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListSessionsRequest {
    pub cursor: Option<String>,
    pub limit: u32,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionIdRequest {
    pub session_id: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeleteSessionRequest {
    pub session_id: String,
    pub confirmation_token: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetSessionReferenceRequest {
    pub session_id: String,
    pub is_reference: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionDetailDto {
    pub summary: SessionSummaryDto,
    pub report: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportRequest {
    pub scope: ExportScope,
    pub format: ExportFormat,
    pub anonymize: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExportPreviewDto {
    pub format: ExportFormat,
    pub included_fields: Vec<String>,
    pub excluded_fields: Vec<String>,
    pub estimated_bytes: usize,
    pub proposed_file_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExportResultDto {
    pub bytes: usize,
    pub anonymized: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportResultDto {
    pub session_id: String,
    pub schema_version: u32,
    pub migrated: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticReportDto {
    pub session_id: String,
    pub schema_version: i64,
    pub report: serde_json::Value,
    pub frozen_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OnboardingStateDto {
    pub flow_version: i64,
    pub last_slide: i64,
    pub status: &'static str,
    pub completed_at: Option<String>,
    pub last_seen_notice_version: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PreferencesSnapshotDto {
    pub schema_version: u32,
    pub values: BTreeMap<String, Value>,
    pub adjusted: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StorageUsageDto {
    pub database_bytes: u64,
    pub logs_bytes: u64,
    pub total_bytes: u64,
    pub session_count: u64,
    /// FR-075: the file name of `<original>.corrupt-<fecha>` moved aside at startup, when this
    /// run recovered from a corrupt database — `None` once it has been exported or on any run
    /// where storage opened cleanly. Only the name is exposed; the full path never leaves Rust.
    pub corrupt_backup: Option<String>,
}

/// Remembers, for this run only, the corrupt database [`open_recovering_corruption`] moved
/// aside at startup (FR-075) — so Ajustes can offer to export it once, and `export_corrupt_backup`
/// knows where to read it from without the frontend ever seeing the real path.
#[derive(Default)]
pub struct CorruptBackupNotice(pub Mutex<Option<std::path::PathBuf>>);

#[derive(Debug, Clone, Serialize)]
pub struct DataOperationDto {
    pub cleared: Vec<String>,
    pub failed: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TechnicalSummaryDto {
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LicenseEntryDto {
    pub id: String,
    pub name: String,
    pub version: Option<String>,
    pub license: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ThirdPartyNoticesDto {
    pub entries: Vec<LicenseEntryDto>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfirmationRequest {
    pub confirmation_token: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenExternalRequest {
    pub target: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrontendLogEventRequest {
    pub level: String,
    pub code: String,
    pub target: String,
    pub msg: String,
    #[serde(default)]
    pub fields: Option<serde_json::Map<String, Value>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetPreferenceRequest {
    pub key: String,
    pub value: Value,
    pub expected_schema_version: u32,
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

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetTrayPausedRequest {
    pub paused: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrayStateDto {
    pub icon: &'static str,
    pub paused: bool,
}

#[tauri::command]
pub fn set_tray_paused(
    app: AppHandle,
    state: State<'_, TrayController>,
    runtime: State<'_, crate::CollectorHandle>,
    request: SetTrayPausedRequest,
) -> Result<TrayStateDto, CommandError> {
    apply_tray_pause(&app, &state, &runtime, request.paused)
}

pub(crate) fn apply_tray_pause(
    app: &AppHandle,
    state: &TrayController,
    runtime: &crate::CollectorHandle,
    paused: bool,
) -> Result<TrayStateDto, CommandError> {
    state.paused.store(paused, Ordering::SeqCst);
    runtime.0.lock().map_err(|_| CommandError::operation_failed())?.set_paused(paused);
    crate::tray::refresh(app, None, false);
    let dto = TrayStateDto { icon: crate::tray::current_state(app, None).key(), paused };
    app.emit("tray:state", &dto).map_err(|_| CommandError::operation_failed())?;
    Ok(dto)
}

impl CommandError {
    fn operation_failed() -> Self {
        Self { code: "LOW_LEVEL_ACCESS_OPERATION_FAILED", message_key: "access.operation_failed" }
    }

    fn access_not_installable() -> Self {
        Self { code: "ACCESS_NOT_INSTALLABLE", message_key: "coverage.access_not_installable" }
    }

    fn preference_schema_mismatch() -> Self {
        Self {
            code: "preferences.schema_version_mismatch",
            message_key: "preferences.schema_version_mismatch",
        }
    }

    fn preference_error(error: crate::preferences::PreferenceError) -> Self {
        match error {
            crate::preferences::PreferenceError::UnknownKey => {
                Self { code: "preferences.unknown_key", message_key: "preferences.unknown_key" }
            }
            crate::preferences::PreferenceError::InvalidValue => {
                Self { code: "preferences.invalid_value", message_key: "preferences.invalid_value" }
            }
            crate::preferences::PreferenceError::Dependency => {
                Self { code: "preferences.dependency", message_key: "preferences.dependency" }
            }
        }
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
    live.read(|live| live::coverage_dto(live, advanced_access_enabled(&state), live::detect_pawnio))
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
    let Ok(data_dir) = crate::dev_faults::resolve_data_dir(app) else { return false };
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
    // The live state is read before the storage lock is taken: the observer takes them in that
    // order (live, then storage), so taking them the other way round could deadlock.
    let cpu = crate::telemetry::sink::cpu_identity(live.read(|live| live.cpu().cloned()).as_ref());
    let storage = storage_state.storage.lock().map_err(|_| CommandError::operation_failed())?;
    let started_at = jiff::Timestamp::now().to_string();
    storage
        .upsert_cpu(
            &cpu.id,
            &cpu.vendor,
            &cpu.display_name,
            cpu.logical_processors,
            cpu.hybrid,
            &started_at,
        )
        .map_err(|_| CommandError::operation_failed())?;
    storage
        .create_guided_session(&session_id, &cpu.id, &started_at)
        .map_err(|_| CommandError::operation_failed())?;
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
        "cpu.package.clock" | "host.active_clock" => Some("clock"),
        "cpu.package.load" => Some("load"),
        "cpu.package.power" => Some("power"),
        _ => None,
    }
}

fn export_scope_session(scope: &ExportScope) -> Option<(&str, Option<(i64, i64)>)> {
    match scope {
        ExportScope::Session { session_id } | ExportScope::Report { session_id } => {
            Some((session_id.as_str(), None))
        }
        ExportScope::Range { session_id, start_ms, end_ms } if *end_ms > *start_ms => {
            Some((session_id.as_str(), Some((*start_ms, *end_ms))))
        }
        ExportScope::Range { .. } => None,
    }
}

fn export_snapshot(state: &AppState, scope: &ExportScope) -> Result<ExportSnapshot, CommandError> {
    let (session_id, range) = export_scope_session(scope).ok_or(CommandError {
        code: "validation.invalid_parameter",
        message_key: "validation.invalid_parameter",
    })?;
    let guard = state.storage.lock().map_err(|_| CommandError::operation_failed())?;
    guard
        .export_snapshot(session_id, range)
        .map_err(|_| CommandError::operation_failed())?
        .ok_or(CommandError { code: "session.not_found", message_key: "session.not_found" })
}

fn export_bytes(
    snapshot: &ExportSnapshot,
    format: ExportFormat,
    anonymize: bool,
) -> Result<Vec<u8>, CommandError> {
    match format {
        ExportFormat::Csv => {
            let mut bytes = Vec::new();
            write_csv_checked(&mut bytes, snapshot.samples.clone(), None)
                .map_err(|_| CommandError::operation_failed())?;
            Ok(bytes)
        }
        ExportFormat::Json => bundle_from_snapshot(snapshot)
            .to_json_bytes(anonymize)
            .map_err(|_| CommandError::operation_failed()),
    }
}

/// Writes the export straight to `path`: CSV streams row by row and stops within
/// [`crate::export::write_csv_checked`]'s check interval of a cancellation; JSON builds one
/// in-memory document (`bundle_from_snapshot`/`to_json_bytes` are not streamable), so it is only
/// checked before and after that step. Either way a cancelled export never leaves a partial file.
fn write_export_file(
    path: &std::path::Path,
    snapshot: &ExportSnapshot,
    format: ExportFormat,
    anonymize: bool,
    cancelled: &AtomicBool,
) -> Result<usize, CommandError> {
    match format {
        ExportFormat::Csv => {
            let file = fs::File::create(path).map_err(|_| CommandError::operation_failed())?;
            let mut writer = std::io::BufWriter::new(file);
            let outcome = write_csv_checked(&mut writer, snapshot.samples.clone(), Some(cancelled))
                .map_err(|_| CommandError::operation_failed())?;
            use std::io::Write as _;
            writer.flush().map_err(|_| CommandError::operation_failed())?;
            drop(writer);
            match outcome {
                WriteOutcome::Cancelled => {
                    let _ = fs::remove_file(path);
                    Err(CommandError { code: "export.cancelled", message_key: "export.cancelled" })
                }
                WriteOutcome::Written => {
                    Ok(fs::metadata(path).map(|meta| meta.len() as usize).unwrap_or(0))
                }
            }
        }
        ExportFormat::Json => {
            if cancelled.load(Ordering::SeqCst) {
                return Err(CommandError {
                    code: "export.cancelled",
                    message_key: "export.cancelled",
                });
            }
            let bytes = bundle_from_snapshot(snapshot)
                .to_json_bytes(anonymize)
                .map_err(|_| CommandError::operation_failed())?;
            if cancelled.load(Ordering::SeqCst) {
                return Err(CommandError {
                    code: "export.cancelled",
                    message_key: "export.cancelled",
                });
            }
            fs::write(path, &bytes).map_err(|_| CommandError::operation_failed())?;
            Ok(bytes.len())
        }
    }
}

/// Shows the native save dialog without blocking the IPC thread the whole time it is open (a
/// person can leave it open for a while, and everything else — live telemetry, the tray — waits
/// on that thread). `None` means the person cancelled it.
async fn pick_save_path(app: &AppHandle, file_name: &str) -> Option<std::path::PathBuf> {
    let (tx, mut rx) = tauri::async_runtime::channel(1);
    app.dialog().file().set_file_name(file_name).save_file(move |path| {
        let _ = tx.try_send(path);
    });
    rx.recv().await.flatten().and_then(|value| value.into_path().ok())
}

/// The open-file counterpart of [`pick_save_path`], for `import_session`.
async fn pick_open_path(
    app: &AppHandle,
    filter_name: &str,
    extensions: &[&str],
) -> Option<std::path::PathBuf> {
    let (tx, mut rx) = tauri::async_runtime::channel(1);
    app.dialog().file().add_filter(filter_name, extensions).pick_file(move |path| {
        let _ = tx.try_send(path);
    });
    rx.recv().await.flatten().and_then(|value| value.into_path().ok())
}

#[tauri::command]
pub async fn preview_export(
    app: AppHandle,
    request: ExportRequest,
) -> Result<ExportPreviewDto, CommandError> {
    // The size estimate needs the full snapshot and serialization; off the IPC thread so
    // toggling format/anonymize in the dialog never stalls the rest of the interface.
    tauri::async_runtime::spawn_blocking(move || -> Result<ExportPreviewDto, CommandError> {
        let state = app.state::<AppState>();
        let snapshot = export_snapshot(&state, &request.scope)?;
        let bytes = export_bytes(&snapshot, request.format, request.anonymize)?;
        let preview =
            build_export_preview(&request.scope, request.format, request.anonymize, bytes.len());
        Ok(ExportPreviewDto {
            format: preview.format,
            included_fields: preview.included_fields,
            excluded_fields: preview.excluded_fields,
            estimated_bytes: preview.estimated_bytes,
            proposed_file_name: preview.proposed_file_name,
        })
    })
    .await
    .map_err(|_| CommandError::operation_failed())?
}

#[tauri::command]
pub async fn export(
    app: AppHandle,
    export_state: State<'_, ExportController>,
    request: ExportRequest,
) -> Result<ExportResultDto, CommandError> {
    export_state.cancelled.store(false, Ordering::SeqCst);
    let cancelled = Arc::clone(&export_state.cancelled);
    // Kept for the whole async export, including both awaits below: an install must wait for it
    // exactly as it did while this command was synchronous.
    let _busy = app
        .try_state::<crate::updates_app::BusyOperations>()
        .map(|busy| busy.begin(crate::updates::Blocker::Export));

    // No snapshot needed yet: the proposed name only depends on the scope and format.
    let preview = build_export_preview(&request.scope, request.format, request.anonymize, 0);
    let Some(path) = pick_save_path(&app, &preview.proposed_file_name).await else {
        return Err(CommandError { code: "export.cancelled", message_key: "export.cancelled" });
    };
    if cancelled.load(Ordering::SeqCst) {
        return Err(CommandError { code: "export.cancelled", message_key: "export.cancelled" });
    }

    let scope = request.scope.clone();
    let format = request.format;
    let anonymize = request.anonymize;
    let write_app = app.clone();
    let bytes = tauri::async_runtime::spawn_blocking(move || -> Result<usize, CommandError> {
        let state = write_app.state::<AppState>();
        let snapshot = export_snapshot(&state, &scope)?;
        write_export_file(&path, &snapshot, format, anonymize, &cancelled)
    })
    .await
    .map_err(|_| CommandError::operation_failed())??;

    Ok(ExportResultDto { bytes, anonymized: request.anonymize, warnings: Vec::new() })
}

/// Stops the export in progress (T172): the CSV write checks this within
/// [`crate::export::write_csv_checked`]'s row interval and discards the partial file; a JSON
/// export is checked before and after building its one document, not mid-build.
#[tauri::command]
pub fn cancel_export(export_state: State<'_, ExportController>) {
    export_state.cancelled.store(true, Ordering::SeqCst);
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfirmCloseRequest {
    pub stop_operation: bool,
}

/// Answers a `lifecycle:close-blocked` the window just emitted (T090, `contracts/
/// application-commands.md`'s "Cierre con operaciones en curso"). The reason is recomputed here,
/// never taken from the request: what is actually running may have finished, or changed, between
/// the event firing and this call.
#[tauri::command]
pub fn confirm_close(
    app: AppHandle,
    export_state: State<'_, ExportController>,
    request: ConfirmCloseRequest,
) -> Result<(), CommandError> {
    let reason = crate::lifecycle::close_block_reason(&app);
    match crate::lifecycle::confirm_close_outcome(reason, request.stop_operation) {
        crate::lifecycle::ConfirmCloseOutcome::Denied => Err(CommandError {
            code: "lifecycle.close_denied",
            message_key: "lifecycle.close_denied",
        }),
        crate::lifecycle::ConfirmCloseOutcome::Proceed { stop_guided, stop_export } => {
            if stop_guided {
                cancel_guided_for_lifecycle(&app, GuidedStopReason::UserRequested);
            }
            if stop_export {
                export_state.cancelled.store(true, Ordering::SeqCst);
            }
            // Bypasses `CloseRequested` (and this same gate) instead of asking the window to
            // close again: the person already confirmed, so nothing should ask a second time.
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.destroy();
            }
            Ok(())
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolveFirstCloseRequest {
    pub action: String,
}

/// Answers a `lifecycle:close-decision-required` (T182, FR-046): persists the choice through
/// `set_preference` — so the same dependency correction applies (choosing `tray` also turns
/// `tray.monitoring_enabled` on) — then does what was chosen. `dismiss` persists nothing and
/// leaves the window open, matching `Esc`/closing the dialog itself.
#[tauri::command]
pub fn resolve_first_close(
    app: AppHandle,
    state: State<'_, AppState>,
    logging: State<'_, crate::logging::LogControl>,
    request: ResolveFirstCloseRequest,
) -> Result<(), CommandError> {
    match request.action.as_str() {
        "dismiss" => Ok(()),
        "exit" | "tray" => {
            set_preference(
                app.clone(),
                state,
                logging,
                SetPreferenceRequest {
                    key: "lifecycle.close_action".to_owned(),
                    value: serde_json::json!(request.action),
                    expected_schema_version: crate::preferences::SCHEMA_VERSION,
                },
            )?;
            if let Some(window) = app.get_webview_window("main") {
                if request.action == "tray" {
                    let _ = window.hide();
                } else {
                    let _ = window.destroy();
                }
            }
            Ok(())
        }
        _ => Err(CommandError {
            code: "validation.invalid_parameter",
            message_key: "validation.invalid_parameter",
        }),
    }
}

#[tauri::command]
pub async fn import_session(app: AppHandle) -> Result<ImportResultDto, CommandError> {
    let _busy = app
        .try_state::<crate::updates_app::BusyOperations>()
        .map(|busy| busy.begin(crate::updates::Blocker::Import));
    let _ = app.emit("import:progress", serde_json::json!({ "phase": "reading" }));

    let Some(path) = pick_open_path(&app, "ThrottleWatch JSON", &["json"]).await else {
        return Err(CommandError { code: "import.cancelled", message_key: "import.cancelled" });
    };

    let worker_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<ImportResultDto, CommandError> {
        let bytes = fs::read(path).map_err(|_| CommandError::operation_failed())?;
        let _ = worker_app.emit("import:progress", serde_json::json!({ "phase": "validating" }));
        let bundle = crate::export::import_bundle(&bytes).map_err(|error| match error {
            crate::export::ImportError::SchemaTooNew { .. } => {
                CommandError { code: "import.schema_too_new", message_key: "import.schema_too_new" }
            }
            crate::export::ImportError::InvalidDocument => CommandError {
                code: "import.invalid_document",
                message_key: "import.invalid_document",
            },
        })?;
        let migrated = bundle.schema_version < crate::export::EXPORT_SCHEMA_VERSION;
        if migrated {
            let _ = worker_app.emit("import:progress", serde_json::json!({ "phase": "migrating" }));
        }
        let _ = worker_app.emit("import:progress", serde_json::json!({ "phase": "storing" }));
        let session_id = worker_app
            .state::<AppState>()
            .storage
            .lock()
            .map_err(|_| CommandError::operation_failed())?
            .import_export_bundle(&bundle)
            .map_err(|_| CommandError::operation_failed())?;
        let _ = worker_app.emit(
            "session:changed",
            serde_json::json!({ "session_id": session_id, "status": "imported" }),
        );
        Ok(ImportResultDto {
            session_id,
            schema_version: bundle.schema_version,
            migrated,
            warnings: Vec::new(),
        })
    })
    .await
    .map_err(|_| CommandError::operation_failed())?
}

#[tauri::command]
pub fn get_onboarding_state(
    state: State<'_, AppState>,
) -> Result<OnboardingStateDto, CommandError> {
    let guard = state.storage.lock().map_err(|_| CommandError::operation_failed())?;
    let value = guard.onboarding_state().map_err(|_| CommandError::operation_failed())?;
    Ok(onboarding_state_dto(value))
}

fn preferences_snapshot(
    values: BTreeMap<String, Value>,
    adjusted: Vec<String>,
) -> PreferencesSnapshotDto {
    PreferencesSnapshotDto { schema_version: crate::preferences::SCHEMA_VERSION, values, adjusted }
}

#[tauri::command]
pub fn get_preferences(state: State<'_, AppState>) -> Result<PreferencesSnapshotDto, CommandError> {
    let guard = state.storage.lock().map_err(|_| CommandError::operation_failed())?;
    let values = guard.user_preferences().map_err(|_| CommandError::operation_failed())?;
    Ok(preferences_snapshot(values, Vec::new()))
}

#[tauri::command]
pub fn get_storage_usage(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<StorageUsageDto, CommandError> {
    let usage = state
        .storage
        .lock()
        .map_err(|_| CommandError::operation_failed())?
        .usage()
        .map_err(|_| CommandError::operation_failed())?;
    let logs = crate::dev_faults::resolve_data_dir(&app)
        .map_err(|_| CommandError::operation_failed())?
        .join("logs");
    let logs_bytes =
        crate::logging::directory_bytes(logs).map_err(|_| CommandError::operation_failed())?;
    let corrupt_backup = app.try_state::<CorruptBackupNotice>().and_then(|notice| {
        notice
            .0
            .lock()
            .ok()
            .and_then(|guard| guard.clone())
            .and_then(|path| path.file_name().map(|name| name.to_string_lossy().into_owned()))
    });
    Ok(StorageUsageDto {
        database_bytes: usage.database_bytes,
        logs_bytes,
        total_bytes: usage.database_bytes.saturating_add(logs_bytes),
        session_count: usage.session_count,
        corrupt_backup,
    })
}

/// FR-075 ("se ofrece exportar la dañada"): copies the corrupt backup this run recovered from
/// to a location the person chooses, leaving the original in place — exporting is read-only,
/// never the "destructive intervention" the FR forbids. Cancelling the dialog is not an error.
#[tauri::command]
pub async fn export_corrupt_backup(
    app: AppHandle,
    notice: State<'_, CorruptBackupNotice>,
) -> Result<(), CommandError> {
    let source = notice.0.lock().map_err(|_| CommandError::operation_failed())?.clone();
    let Some(source) = source else {
        return Err(CommandError {
            code: "storage.no_corrupt_backup",
            message_key: "storage.no_corrupt_backup",
        });
    };
    let file_name = source
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "throttlewatch.db.corrupt".to_owned());
    let Some(target) = pick_save_path(&app, &file_name).await else {
        return Err(CommandError { code: "export.cancelled", message_key: "export.cancelled" });
    };
    tauri::async_runtime::spawn_blocking(move || std::fs::copy(&source, &target))
        .await
        .map_err(|_| CommandError::operation_failed())?
        .map_err(|_| CommandError::operation_failed())?;
    Ok(())
}

#[tauri::command]
pub fn log_frontend(
    limiter: State<'_, FrontendLogLimiter>,
    logging: State<'_, crate::logging::LogControl>,
    events: Vec<FrontendLogEventRequest>,
) -> Result<(), CommandError> {
    for event in &events {
        if !matches!(event.level.as_str(), "trace" | "debug" | "info" | "warn" | "error")
            || event.code.is_empty()
            || event.code.len() > 128
            || !event.code.chars().all(|character| {
                character.is_ascii_uppercase() || character.is_ascii_digit() || character == '_'
            })
            || event.target.is_empty()
            || event.target.len() > 128
            || event.msg.is_empty()
            || event.msg.len() > 256
        {
            return Err(CommandError {
                code: "validation.invalid_parameter",
                message_key: "validation.invalid_parameter",
            });
        }
    }
    let detailed = logging.is_detailed();
    let requested = events.len();
    let accepted = limiter.take(requested, Instant::now());
    for event in events.into_iter().take(accepted) {
        if matches!(event.level.as_str(), "trace" | "debug" | "info") && !detailed {
            continue;
        }
        match event.level.as_str() {
            "trace" => {
                tracing::event!(target: "ui", tracing::Level::TRACE, component = "ui", target = %event.target, code = %event.code, msg = %event.msg, fields = ?event.fields)
            }
            "debug" => {
                tracing::event!(target: "ui", tracing::Level::DEBUG, component = "ui", target = %event.target, code = %event.code, msg = %event.msg, fields = ?event.fields)
            }
            "info" => {
                tracing::event!(target: "ui", tracing::Level::INFO, component = "ui", target = %event.target, code = %event.code, msg = %event.msg, fields = ?event.fields)
            }
            "warn" => {
                tracing::event!(target: "ui", tracing::Level::WARN, component = "ui", target = %event.target, code = %event.code, msg = %event.msg, fields = ?event.fields)
            }
            "error" => {
                tracing::event!(target: "ui", tracing::Level::ERROR, component = "ui", target = %event.target, code = %event.code, msg = %event.msg, fields = ?event.fields)
            }
            _ => {}
        }
    }
    if accepted < requested {
        tracing::warn!(
            component = "ui",
            code = "FRONTEND_LOG_RATE_LIMIT",
            msg = "frontend log events were rate limited",
            discarded = requested.saturating_sub(accepted)
        );
    }
    Ok(())
}

#[tauri::command]
pub fn get_technical_summary(
    live: State<'_, LiveHandle>,
    state: State<'_, AppState>,
) -> Result<TechnicalSummaryDto, CommandError> {
    let (collector_state, coverage_tier) = live.read(|value| {
        let tier = match value.coverage_signals().summary().tier {
            crate::diagnostics::CoverageTier::A => "A",
            crate::diagnostics::CoverageTier::B => "B",
            crate::diagnostics::CoverageTier::C => "C",
        };
        (value.collector().as_str().to_owned(), tier)
    });
    let usage = state
        .storage
        .lock()
        .map_err(|_| CommandError::operation_failed())?
        .usage()
        .map_err(|_| CommandError::operation_failed())?;
    Ok(TechnicalSummaryDto {
        text: technical_summary_text(
            &collector_state,
            coverage_tier,
            usage.session_count,
            usage.database_bytes,
        ),
    })
}

fn technical_summary_text(
    collector_state: &str,
    coverage_tier: &str,
    session_count: u64,
    database_bytes: u64,
) -> String {
    format!(
        "ThrottleWatch {version}\nIPC protocol: 1\nCollector: {collector_state}\nCoverage tier: {coverage_tier}\nStored sessions: {session_count}\nDatabase bytes: {database_bytes}\nLast error codes: not exposed\n\nThis summary contains no CPU identifiers, paths, URLs, or raw log lines.",
        version = env!("CARGO_PKG_VERSION"),
    )
}

#[tauri::command]
pub fn get_third_party_notices() -> ThirdPartyNoticesDto {
    ThirdPartyNoticesDto {
        entries: vec![
            LicenseEntryDto {
                id: "throttlewatch".to_owned(),
                name: "ThrottleWatch".to_owned(),
                version: Some(env!("CARGO_PKG_VERSION").to_owned()),
                license: "GPL-3.0-only".to_owned(),
                text: include_str!("../../../../../LICENSE").to_owned(),
            },
            LicenseEntryDto {
                id: "librehardwaremonitor".to_owned(),
                name: "LibreHardwareMonitorLib".to_owned(),
                version: None,
                license: "MPL-2.0".to_owned(),
                text: include_str!("../../../../../docs/licenses/inventory.md").to_owned(),
            },
            LicenseEntryDto {
                id: "pawnio".to_owned(),
                name: "PawnIO".to_owned(),
                version: Some("2.2.0".to_owned()),
                license: "GPL-2.0-or-later".to_owned(),
                text: include_str!("../../../../../THIRD-PARTY-NOTICES").to_owned(),
            },
        ],
    }
}

fn validate_confirmation(request: &ConfirmationRequest) -> Result<(), CommandError> {
    if request.confirmation_token.trim().is_empty() {
        Err(CommandError {
            code: "validation.invalid_parameter",
            message_key: "validation.invalid_parameter",
        })
    } else {
        Ok(())
    }
}

/// Deleting or resetting is refused only while a guided test runs. Passive monitoring never
/// blocks it: its session is closed (and its report frozen) first, and dropped after the wipe.
fn prepare_for_wipe(app: &AppHandle) -> Result<(), CommandError> {
    if guided_in_progress(app) {
        return Err(CommandError {
            code: "operation.active_session",
            message_key: "operation.active_session",
        });
    }
    if let Some(recorder) = app.try_state::<RecorderHandle>() {
        recorder.flush(app);
    }
    Ok(())
}

fn forget_recorded_session(app: &AppHandle) {
    if let Some(recorder) = app.try_state::<RecorderHandle>() {
        recorder.discard();
    }
}

#[tauri::command]
pub fn delete_monitoring_data(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ConfirmationRequest,
) -> Result<DataOperationDto, CommandError> {
    validate_confirmation(&request)?;
    prepare_for_wipe(&app)?;
    let _busy = app
        .try_state::<crate::updates_app::BusyOperations>()
        .map(|busy| busy.begin(crate::updates::Blocker::DataOperation));
    let mut cleared = Vec::new();
    let mut failed = Vec::new();
    match state
        .storage
        .lock()
        .map_err(|_| CommandError::operation_failed())?
        .clear_data_preserving_preferences()
    {
        Ok(()) => cleared.push("data".to_owned()),
        Err(_) => failed.push("data".to_owned()),
    }
    forget_recorded_session(&app);
    let logs = crate::dev_faults::resolve_data_dir(&app)
        .map_err(|_| CommandError::operation_failed())?
        .join("logs");
    match crate::logging::clear_directory(logs) {
        Ok(()) => cleared.push("logs".to_owned()),
        Err(_) => failed.push("logs".to_owned()),
    }
    Ok(DataOperationDto { cleared, failed })
}

#[tauri::command]
pub fn reset_application(
    app: AppHandle,
    state: State<'_, AppState>,
    logging: State<'_, crate::logging::LogControl>,
    request: ConfirmationRequest,
) -> Result<DataOperationDto, CommandError> {
    validate_confirmation(&request)?;
    prepare_for_wipe(&app)?;
    let _busy = app
        .try_state::<crate::updates_app::BusyOperations>()
        .map(|busy| busy.begin(crate::updates::Blocker::DataOperation));
    let mut cleared = Vec::new();
    let mut failed = Vec::new();
    match state.storage.lock().map_err(|_| CommandError::operation_failed())?.reset_all() {
        Ok(()) => cleared.push("data_and_preferences".to_owned()),
        Err(_) => failed.push("data_and_preferences".to_owned()),
    }
    forget_recorded_session(&app);
    let logs = crate::dev_faults::resolve_data_dir(&app)
        .map_err(|_| CommandError::operation_failed())?
        .join("logs");
    match crate::logging::clear_directory(logs) {
        Ok(()) => cleared.push("logs".to_owned()),
        Err(_) => failed.push("logs".to_owned()),
    }
    match crate::startup::set_enabled(&app, false) {
        Ok(()) => cleared.push("startup".to_owned()),
        Err(_) => failed.push("startup".to_owned()),
    }
    logging.set_detailed(false);
    Ok(DataOperationDto { cleared, failed })
}

#[tauri::command]
pub fn open_logs_folder(app: AppHandle) -> Result<(), CommandError> {
    let logs = crate::dev_faults::resolve_data_dir(&app)
        .map_err(|_| CommandError::operation_failed())?
        .join("logs");
    fs::create_dir_all(&logs).map_err(|_| CommandError::operation_failed())?;
    std::process::Command::new("explorer.exe")
        .arg(logs)
        .spawn()
        .map_err(|_| CommandError::operation_failed())?;
    Ok(())
}

#[tauri::command]
pub fn open_external_url(app: AppHandle, request: OpenExternalRequest) -> Result<(), CommandError> {
    match request.target.as_str() {
        // Packaged with the installer (`tauri.conf.json`'s `bundle.resources`), never fetched:
        // help must work with the zero-network guarantee, not just the updater's opt-in traffic.
        "help" => {
            let file = match crate::tray::current_locale(&app) {
                crate::i18n::Locale::Es => "help/es.md",
                crate::i18n::Locale::En => "help/en.md",
            };
            let path =
                app.path().resource_dir().map_err(|_| CommandError::operation_failed())?.join(file);
            tauri_plugin_opener::OpenerExt::opener(&app)
                .open_path(path.to_string_lossy(), None::<&str>)
                .map_err(|_| CommandError::operation_failed())?;
        }
        "source" => {
            tauri_plugin_opener::OpenerExt::opener(&app)
                .open_url("https://github.com/ThrottleWatch/ThrottleWatch", None::<&str>)
                .map_err(|_| CommandError::operation_failed())?;
        }
        _ => {
            return Err(CommandError {
                code: "validation.invalid_parameter",
                message_key: "validation.invalid_parameter",
            });
        }
    }
    Ok(())
}

#[tauri::command]
pub fn set_preference(
    app: AppHandle,
    state: State<'_, AppState>,
    logging: State<'_, crate::logging::LogControl>,
    request: SetPreferenceRequest,
) -> Result<PreferencesSnapshotDto, CommandError> {
    if request.expected_schema_version != crate::preferences::SCHEMA_VERSION {
        return Err(CommandError::preference_schema_mismatch());
    }
    let guard = state.storage.lock().map_err(|_| CommandError::operation_failed())?;
    let current = guard.user_preferences().map_err(|_| CommandError::operation_failed())?;
    let now = jiff::Timestamp::now().to_string();
    let (next, adjusted) = crate::preferences::update(&current, &request.key, request.value, &now)
        .map_err(CommandError::preference_error)?;
    if request.key == "startup.enabled" {
        let enabled = next.get("startup.enabled").and_then(Value::as_bool).unwrap_or(false);
        crate::startup::set_enabled(&app, enabled).map_err(|_| CommandError::operation_failed())?;
    }
    guard.set_user_preferences(&next).map_err(|_| CommandError::operation_failed())?;
    if request.key == "logging.detailed_until" {
        logging.set_detailed_until(
            next.get("logging.detailed_until").and_then(Value::as_str).map(str::to_owned),
        );
    }
    drop(guard);
    if (request.key.starts_with("notifications.") || request.key == "locale.mode")
        && let Some(alerts) = app.try_state::<crate::alerting::AlertHandle>()
    {
        alerts.forget_settings();
    }
    if request.key == "updates.enabled"
        && let Some(updates) = app.try_state::<crate::updates_app::UpdateHandle>()
    {
        updates.set_enabled(
            &app,
            next.get("updates.enabled").and_then(Value::as_bool).unwrap_or(false),
        );
    }
    if request.key == "locale.mode" {
        crate::tray::refresh(&app, None, true);
    }
    if request.key.starts_with("sampling.") {
        // Storage's lock is released first: the collector's is never taken while holding it.
        crate::sampling_control::apply(&app);
    }
    Ok(preferences_snapshot(next, adjusted))
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
    app: AppHandle,
    state: State<'_, AppState>,
    live: State<'_, LiveHandle>,
) -> LiveSnapshotDto {
    let locale = crate::tray::current_locale(&app);
    live.read(|live| {
        live::snapshot_dto(live, advanced_access_enabled(&state), live::epoch_ms(), locale)
    })
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
    {
        // Released before the live state is read: the observer locks live first, then storage.
        let guard = state.storage.lock().map_err(|_| CommandError::operation_failed())?;
        guard.set_advanced_access_enabled(false).map_err(|_| CommandError::operation_failed())?;
    }
    Ok(live.read(|live| live::coverage_dto(live, false, || None)))
}

fn session_summary_dto(value: SessionSummary) -> SessionSummaryDto {
    let status = match value.status.as_str() {
        "running" => "active".to_owned(),
        "finished" => "completed".to_owned(),
        "aborted" => "cancelled".to_owned(),
        other => other.to_owned(),
    };
    SessionSummaryDto {
        session_id: value.session_id,
        kind: value.kind,
        status,
        started_at: value.started_at,
        ended_at: value.ended_at,
        duration_ms: value.duration_ms,
        coverage_tier: value.coverage_tier,
        is_reference: value.is_reference,
        frame_count: value.frame_count,
        report_classification: value.report_classification,
    }
}

fn report_dto(value: StoredReport) -> Result<DiagnosticReportDto, CommandError> {
    let report = serde_json::from_str(&value.payload_json).map_err(|_| CommandError {
        code: "storage.invalid_report",
        message_key: "storage.invalid_report",
    })?;
    Ok(DiagnosticReportDto {
        session_id: value.session_id,
        schema_version: value.schema_version,
        report,
        frozen_at: value.frozen_at,
    })
}

#[tauri::command]
pub fn list_sessions(
    state: State<'_, AppState>,
    request: ListSessionsRequest,
) -> Result<SessionPageDto, CommandError> {
    if request.cursor.is_some() {
        return Err(CommandError {
            code: "validation.invalid_parameter",
            message_key: "validation.invalid_parameter",
        });
    }
    let guard = state.storage.lock().map_err(|_| CommandError::operation_failed())?;
    let sessions = guard
        .list_sessions(request.limit)
        .map_err(|_| CommandError::operation_failed())?
        .into_iter()
        .map(session_summary_dto)
        .collect();
    Ok(SessionPageDto { sessions, next_cursor: None })
}

#[tauri::command]
pub fn get_session(
    state: State<'_, AppState>,
    request: SessionIdRequest,
) -> Result<SessionDetailDto, CommandError> {
    if request.session_id.is_empty() {
        return Err(CommandError {
            code: "validation.invalid_parameter",
            message_key: "validation.invalid_parameter",
        });
    }
    let guard = state.storage.lock().map_err(|_| CommandError::operation_failed())?;
    let summary = guard
        .list_sessions(100)
        .map_err(|_| CommandError::operation_failed())?
        .into_iter()
        .find(|item| item.session_id == request.session_id)
        .ok_or(CommandError { code: "session.not_found", message_key: "session.not_found" })?;
    let report = guard
        .stored_report(&request.session_id)
        .map_err(|_| CommandError::operation_failed())?
        .map(report_dto)
        .transpose()?
        .map(|value| value.report);
    Ok(SessionDetailDto { summary: session_summary_dto(summary), report })
}

#[tauri::command]
pub fn get_report(
    state: State<'_, AppState>,
    request: SessionIdRequest,
) -> Result<DiagnosticReportDto, CommandError> {
    let guard = state.storage.lock().map_err(|_| CommandError::operation_failed())?;
    let report = guard
        .stored_report(&request.session_id)
        .map_err(|_| CommandError::operation_failed())?
        .ok_or(CommandError { code: "report.not_found", message_key: "report.not_found" })?;
    report_dto(report)
}

/// `reevaluate_report` needs the samples of a potentially large session; off the IPC thread for
/// the same reason `export`/`preview_export` are (see T172/T173's export fix). The command itself
/// is the thin adapter the constitution asks for: [`build_reevaluated_report`] does the work and
/// is tested directly, against a plain [`Storage`], with no Tauri app needed.
#[tauri::command]
pub async fn reevaluate_report(
    app: AppHandle,
    request: SessionIdRequest,
) -> Result<DiagnosticReportDto, CommandError> {
    tauri::async_runtime::spawn_blocking(move || -> Result<DiagnosticReportDto, CommandError> {
        let state = app.state::<AppState>();
        let guard = state.storage.lock().map_err(|_| CommandError::operation_failed())?;
        build_reevaluated_report(&guard, &request.session_id)
    })
    .await
    .map_err(|_| CommandError::operation_failed())?
}

/// Only for an `imported` session (FR-073): everything `reevaluate_report` needs from storage,
/// with no Tauri types, so it is directly unit-testable.
fn build_reevaluated_report(
    storage: &crate::storage::Storage,
    session_id: &str,
) -> Result<DiagnosticReportDto, CommandError> {
    let (kind, coverage_tier) = storage
        .session_kind_and_tier(session_id)
        .map_err(|_| CommandError::operation_failed())?
        .ok_or(CommandError { code: "session.not_found", message_key: "session.not_found" })?;
    if kind != "imported" {
        return Err(CommandError {
            code: "report.reevaluation_not_imported",
            message_key: "report.reevaluation_not_imported",
        });
    }
    let frames =
        storage.session_frames(session_id).map_err(|_| CommandError::operation_failed())?;
    let coverage_history =
        storage.coverage_changes(session_id).map_err(|_| CommandError::operation_failed())?;

    let rules = crate::diagnostics::Ruleset::v1().map_err(|_| CommandError::operation_failed())?;
    let replay = crate::telemetry::reevaluate::reevaluate_frames(&frames, &rules);
    let (start_ms, end_ms) = (
        frames.first().map_or(0, |frame| frame.monotonic_ms),
        frames.last().map_or(0, |frame| frame.monotonic_ms),
    );
    let (classification, severity, duration_by_class_ms, analyzed_start_ms, analyzed_end_ms) =
        match &replay.verdict {
            Some(verdict) => (
                verdict.classification.key().to_owned(),
                verdict.severity.map(|value| value.key().to_owned()),
                verdict
                    .class_durations_ms
                    .iter()
                    .map(|(class, duration)| {
                        (class.clone(), i64::try_from(*duration).unwrap_or(i64::MAX))
                    })
                    .collect(),
                verdict
                    .analyzed_from_ms
                    .and_then(|value| i64::try_from(value).ok())
                    .unwrap_or(start_ms),
                verdict
                    .analyzed_to_ms
                    .and_then(|value| i64::try_from(value).ok())
                    .unwrap_or(end_ms),
            ),
            None => (
                "indeterminate".to_owned(),
                None,
                std::collections::BTreeMap::new(),
                start_ms,
                end_ms,
            ),
        };
    let confidence = match (classification.as_str(), coverage_tier.as_str()) {
        ("indeterminate", _) => "low",
        (_, "A") => "high",
        (_, "B") => "medium",
        _ => "low",
    };
    let events = replay
        .events
        .into_iter()
        .map(|span| crate::export::ExportEvent {
            kind: span.kind.storage_key().to_owned(),
            start_ms: i64::try_from(span.start_ms).unwrap_or(i64::MAX),
            end_ms: i64::try_from(span.end_ms).unwrap_or(i64::MAX),
            severity: None,
        })
        .collect();
    let report = crate::export::DiagnosticReport {
        classification,
        severity,
        coverage_tier,
        confidence: confidence.to_owned(),
        analyzed_start_ms,
        analyzed_end_ms,
        duration_by_class_ms,
        cooling_potential: None,
        guided_result: None,
        ruleset_version: "ruleset-v1".to_owned(),
        rules: [("ruleset_version".to_owned(), "ruleset-v1".to_owned())].into_iter().collect(),
        coverage_history,
        events,
    };
    Ok(DiagnosticReportDto {
        session_id: session_id.to_owned(),
        schema_version: 1,
        report: serde_json::to_value(report).map_err(|_| CommandError::operation_failed())?,
        // Never persisted, so never frozen: this is a fresh, ephemeral second opinion, shown
        // next to the original (FR-073), not a replacement for it.
        frozen_at: None,
    })
}

#[tauri::command]
pub fn delete_session(
    app: AppHandle,
    state: State<'_, AppState>,
    request: DeleteSessionRequest,
) -> Result<(), CommandError> {
    if request.session_id.is_empty() || request.confirmation_token.is_empty() {
        return Err(CommandError {
            code: "validation.invalid_parameter",
            message_key: "validation.invalid_parameter",
        });
    }
    let guard = state.storage.lock().map_err(|_| CommandError::operation_failed())?;
    if guard
        .session_status(&request.session_id)
        .map_err(|_| CommandError::operation_failed())?
        .as_deref()
        == Some("running")
    {
        return Err(CommandError {
            code: "session.active_not_deletable",
            message_key: "session.active_not_deletable",
        });
    }
    if !guard.delete_session(&request.session_id).map_err(|_| CommandError::operation_failed())? {
        return Err(CommandError { code: "session.not_found", message_key: "session.not_found" });
    }
    app.emit(
        "session:changed",
        serde_json::json!({
            "session_id": request.session_id,
            "status": "deleted"
        }),
    )
    .map_err(|_| CommandError::operation_failed())?;
    Ok(())
}

#[tauri::command]
pub fn set_session_reference(
    app: AppHandle,
    state: State<'_, AppState>,
    request: SetSessionReferenceRequest,
) -> Result<(), CommandError> {
    if request.session_id.is_empty() {
        return Err(CommandError {
            code: "validation.invalid_parameter",
            message_key: "validation.invalid_parameter",
        });
    }
    let guard = state.storage.lock().map_err(|_| CommandError::operation_failed())?;
    if !guard
        .set_session_reference(&request.session_id, request.is_reference)
        .map_err(|_| CommandError::operation_failed())?
    {
        return Err(CommandError {
            code: "guided.reference_not_guided",
            message_key: "guided.reference_not_guided",
        });
    }
    app.emit(
        "session:changed",
        serde_json::json!({
            "session_id": request.session_id,
            "status": if request.is_reference { "reference" } else { "completed" }
        }),
    )
    .map_err(|_| CommandError::operation_failed())?;
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::{
        ConfirmationRequest, FrontendLogLimiter, build_reevaluated_report, technical_summary_text,
        validate_confirmation,
    };
    use crate::storage::{Storage, StoredSampleValue};
    use std::time::{Duration, Instant};

    #[test]
    fn technical_summary_contains_metrics_but_never_raw_log_content() {
        let summary = technical_summary_text("running", "B", 3, 4096);
        assert!(summary.contains("IPC protocol: 1"));
        assert!(summary.contains("Stored sessions: 3"));
        assert!(summary.contains("Coverage tier: B"));
        assert!(summary.contains("Database bytes: 4096"));
        assert!(!summary.contains("throttlewatch.log"));
        assert!(!summary.contains("SENSOR_ENUMERATION_FAILED"));
    }

    #[test]
    fn frontend_log_limiter_accepts_sixty_events_per_minute() {
        let limiter = FrontendLogLimiter::default();
        let now = Instant::now();

        assert_eq!(limiter.take(60, now), 60);
        assert_eq!(limiter.take(1, now), 0);
        assert_eq!(limiter.take(1, now + Duration::from_secs(60)), 1);
    }

    #[test]
    fn deleting_data_or_resetting_requires_a_non_empty_confirmation_token() {
        assert!(
            validate_confirmation(&ConfirmationRequest { confirmation_token: String::new() })
                .is_err()
        );
        assert!(
            validate_confirmation(&ConfirmationRequest { confirmation_token: "   ".to_owned() })
                .is_err()
        );
        assert!(
            validate_confirmation(&ConfirmationRequest {
                confirmation_token: "confirm".to_owned()
            })
            .is_ok()
        );
    }

    fn imported_thermal_session(storage: &Storage) -> String {
        storage
            .upsert_cpu("cpu-1", "amd", "Test CPU", 12, false, "2026-09-21T08:00:00Z")
            .unwrap_or_else(|error| panic!("upsert_cpu: {error}"));
        storage
            .create_session("session-1", "cpu-1", "start", "2026-09-21T08:00:00Z")
            .unwrap_or_else(|error| panic!("create_session: {error}"));
        for second in 0..200_i64 {
            storage
                .insert_sample(
                    "session-1",
                    second,
                    second * 1_000,
                    1_000,
                    &[
                        StoredSampleValue {
                            sensor_id: "cpu.package.load".to_owned(),
                            value: Some(98.0),
                            boolean: None,
                            quality: "direct".to_owned(),
                        },
                        StoredSampleValue {
                            sensor_id: "cpu.package.temp".to_owned(),
                            value: Some(94.0),
                            boolean: None,
                            quality: "direct".to_owned(),
                        },
                        StoredSampleValue {
                            sensor_id: "cpu.package.clock".to_owned(),
                            value: Some(3_400.0),
                            boolean: None,
                            quality: "direct".to_owned(),
                        },
                        StoredSampleValue {
                            sensor_id: "host.base_clock".to_owned(),
                            value: Some(3_600.0),
                            boolean: None,
                            quality: "direct".to_owned(),
                        },
                        StoredSampleValue {
                            sensor_id: "cpu.package.power".to_owned(),
                            value: Some(60.0),
                            boolean: None,
                            quality: "direct".to_owned(),
                        },
                        StoredSampleValue {
                            sensor_id: "cpu.package.power_limit".to_owned(),
                            value: Some(90.0),
                            boolean: None,
                            quality: "direct".to_owned(),
                        },
                        StoredSampleValue {
                            sensor_id: "cpu.core.1.load".to_owned(),
                            value: Some(98.0),
                            boolean: None,
                            quality: "direct".to_owned(),
                        },
                        StoredSampleValue {
                            sensor_id: "msr/thermal_flag".to_owned(),
                            value: None,
                            boolean: Some(true),
                            quality: "direct".to_owned(),
                        },
                    ],
                )
                .unwrap_or_else(|error| panic!("insert_sample: {error}"));
        }
        let snapshot = storage
            .export_snapshot("session-1", None)
            .unwrap_or_else(|error| panic!("export_snapshot: {error}"))
            .unwrap_or_else(|| panic!("snapshot expected"));
        let bundle = crate::export::bundle_from_snapshot(&snapshot);
        storage
            .import_export_bundle(&bundle)
            .unwrap_or_else(|error| panic!("import_export_bundle: {error}"))
    }

    #[test]
    fn reevaluating_an_imported_session_finds_the_same_thermal_verdict_a_live_one_would() {
        let storage = Storage::in_memory().unwrap_or_else(|error| panic!("storage: {error}"));
        let imported_id = imported_thermal_session(&storage);

        let result = build_reevaluated_report(&storage, &imported_id)
            .unwrap_or_else(|error| panic!("build_reevaluated_report: {error:?}"));

        assert_eq!(result.session_id, imported_id);
        assert!(result.frozen_at.is_none(), "a reevaluation is never persisted as frozen");
        assert_eq!(
            result.report.get("classification").and_then(serde_json::Value::as_str),
            Some("thermal_confirmed")
        );
        let events = result.report.get("events").and_then(serde_json::Value::as_array);
        assert_eq!(events.map(Vec::len), Some(1));
    }

    #[test]
    fn only_an_imported_session_can_be_reevaluated() {
        let storage = Storage::in_memory().unwrap_or_else(|error| panic!("storage: {error}"));
        storage
            .upsert_cpu("cpu-1", "amd", "Test CPU", 12, false, "2026-09-21T08:00:00Z")
            .unwrap_or_else(|error| panic!("upsert_cpu: {error}"));
        storage
            .create_session("session-1", "cpu-1", "start", "2026-09-21T08:00:00Z")
            .unwrap_or_else(|error| panic!("create_session: {error}"));

        let error = build_reevaluated_report(&storage, "session-1")
            .err()
            .unwrap_or_else(|| panic!("a non-imported session must be refused"));

        assert_eq!(error.code, "report.reevaluation_not_imported");
    }
}
