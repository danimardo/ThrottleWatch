//! Builds the DTOs of the live screens from the collector's [`LiveState`] and emits the events.
//! Nothing here fabricates a value: a magnitude the collector does not deliver is `None` and its
//! coverage row says why.
#![deny(clippy::unwrap_used, clippy::expect_used)]

use super::{
    AdvancedAccessDto, CoverageDto, CoverageMatrixDto, CoverageRowDto, CpuCoreDto, CpuTopologyDto,
    LiveSnapshotDto, access, confidence, freshness, tier,
};
use crate::access::PawnIoInstallation;
use crate::diagnostics::{AdvancedAccess, CoverageSignals, Ruleset};
use crate::storage::AppState;
use crate::telemetry::clock::SampleQuality;
use crate::telemetry::live::{CollectorState, LiveState};
use crate::telemetry::power_context::{PowerSource, read_windows_power_context};
use crate::telemetry::recorder::{RecorderAction, RecorderInput, SessionRecorder};
use crate::telemetry::runtime::{Change, LiveObserver};
use crate::telemetry::session::SplitReason;
use crate::telemetry::sink;
use crate::telemetry::snapshot::{Freshness, aggregate_snapshot};
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};

/// The shared live state the runtime writes and the commands read.
#[derive(Clone, Default)]
pub struct LiveHandle(pub Arc<Mutex<LiveState>>);

impl LiveHandle {
    pub fn read<T>(&self, reader: impl FnOnce(&LiveState) -> T) -> T {
        reader(&self.0.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

pub fn epoch_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |elapsed| elapsed.as_millis() as u64)
}

pub fn snapshot_dto(
    live: &LiveState,
    advanced_access_enabled: bool,
    now_ms: u64,
    locale: crate::i18n::Locale,
) -> LiveSnapshotDto {
    let rules = Ruleset::v1().ok();
    let interval_ms = live.sampling_interval_ms().map(|value| value as f64).or_else(|| {
        rules.as_ref().and_then(|rules| rules.parameter("sampling.interval_normal_ms"))
    });
    let stall_intervals =
        rules.as_ref().and_then(|rules| rules.parameter("collector.stall_intervals"));
    // Stale after `stall_intervals` intervals without a sample (ruleset); whether the collector is
    // gone is the supervisor's word, not the age's, so "disconnected" never comes from the age here.
    let stale_after_ms = interval_ms
        .zip(stall_intervals)
        .map_or(u64::MAX, |(interval, stall)| (interval * stall) as u64);
    let aggregated =
        aggregate_snapshot(live.snapshot_input(), now_ms, stale_after_ms, u64::MAX, None);
    let freshness_value = match live.collector() {
        CollectorState::Failed | CollectorState::Stopped => Freshness::Disconnected,
        _ if !live.has_data() => Freshness::Disconnected,
        _ => aggregated.freshness,
    };
    let signals = live.coverage_signals();
    let summary = signals.summary();
    let power =
        read_windows_power_context().map(|context| context.source).unwrap_or(PowerSource::Unknown);
    LiveSnapshotDto {
        captured_at_ms: aggregated.captured_at_ms,
        freshness: freshness(freshness_value),
        age_ms: aggregated.age_ms,
        temperature_c: aggregated.temperature_c,
        thermal_limit_c: aggregated.thermal_limit_c,
        thermal_margin_c: aggregated.thermal_margin_c,
        load_percent: aggregated.load_percent,
        active_clock_mhz: aggregated.active_clock_mhz,
        base_clock_mhz: aggregated.base_clock_mhz,
        package_power_w: aggregated.package_power_w,
        power_limit_w: aggregated.power_limit_w,
        classification: Some(
            live.diagnostic().map_or("indeterminate", |result| result.classification.key()),
        ),
        severity: live.diagnostic().and_then(|result| result.severity).map(|value| value.key()),
        cpu_label: live.cpu().map_or_else(
            || crate::i18n::text(locale, "native.live.cpu_unknown"),
            |cpu| cpu.display_name.clone(),
        ),
        topology_label: topology_label(live, locale),
        power_label: crate::i18n::text(
            locale,
            match power {
                PowerSource::Ac => "native.live.power.ac",
                PowerSource::Battery => "native.live.power.battery",
                PowerSource::Unknown => "native.live.power.unknown",
            },
        ),
        collector_state: live.collector().as_str(),
        coverage: CoverageDto {
            tier: tier(summary.tier),
            confidence_ceiling: confidence(summary.confidence_ceiling),
            advanced_access: resolve_advanced_access(
                advanced_access_enabled,
                summary.advanced_access,
                detect_pawnio,
            ),
        },
        confidence_label: crate::i18n::text(
            locale,
            match summary.confidence_ceiling {
                crate::diagnostics::ConfidenceCeiling::Low => "native.live.confidence.low",
                crate::diagnostics::ConfidenceCeiling::Medium => "native.live.confidence.medium",
                crate::diagnostics::ConfidenceCeiling::High => "native.live.confidence.high",
            },
        ),
        active_cores: None,
        platform_kind: None,
        in_turbo_window: false,
    }
}

/// How long a PawnIO probe stays valid: the registry lookup spawns a process and the coverage
/// DTO is rebuilt on every snapshot.
const PROBE_TTL: Duration = Duration::from_secs(30);

/// Reads the installed PawnIO (FR-089) against the pinned minimum version, cached for
/// [`PROBE_TTL`]. `None` when the manifest or the probe fails.
pub fn detect_pawnio() -> Option<PawnIoInstallation> {
    type Probe = Option<(Instant, Option<PawnIoInstallation>)>;
    static CACHE: OnceLock<Mutex<Probe>> = OnceLock::new();
    let mut cache =
        CACHE.get_or_init(|| Mutex::new(None)).lock().unwrap_or_else(PoisonError::into_inner);
    if let Some((at, value)) = cache.as_ref()
        && at.elapsed() < PROBE_TTL
    {
        return value.clone();
    }
    let value = access::manifest()
        .and_then(|manifest| access::detect_installation(&manifest.minimum_version))
        .ok();
    *cache = Some((Instant::now(), value.clone()));
    value
}

/// Derives the advanced-access state from the tier and, only when the tier asks for it, from
/// what is installed: a missing PawnIO is `installable`, an older one `upgradable`, and a current
/// one that still does not deliver tier A needs `error` (repair) rather than a new install.
pub(super) fn resolve_advanced_access(
    enabled: bool,
    computed: AdvancedAccess,
    detect: impl FnOnce() -> Option<PawnIoInstallation>,
) -> AdvancedAccessDto {
    if !enabled {
        return AdvancedAccessDto::Denied;
    }
    if !matches!(computed, AdvancedAccess::Installable) {
        return access(computed);
    }
    match detect() {
        None | Some(PawnIoInstallation::Missing) => AdvancedAccessDto::Installable,
        Some(PawnIoInstallation::Upgradable { .. }) => AdvancedAccessDto::Upgradable,
        Some(PawnIoInstallation::Current { .. }) => AdvancedAccessDto::Error,
    }
}

fn topology_label(live: &LiveState, locale: crate::i18n::Locale) -> String {
    let Some(cpu) = live.cpu() else {
        return crate::i18n::text(locale, "native.live.topology_waiting");
    };
    let groups = live
        .groups()
        .iter()
        .filter(|group| matches!(group.kind.as_str(), "p" | "e" | "lp_e"))
        .map(|group| {
            format!("{} {}", group.logical_count, group.kind.to_uppercase().replace('_', "-"))
        })
        .collect::<Vec<_>>();
    let virtualized = if cpu.virtualized {
        crate::i18n::text(locale, "native.live.topology_virtualized_suffix")
    } else {
        String::new()
    };
    if groups.is_empty() {
        let template = crate::i18n::text(locale, "native.live.topology_logical_processors");
        format!("{}{virtualized}", template.replace("{count}", &cpu.logical_processors.to_string()))
    } else {
        format!("{}{virtualized}", groups.join(" + "))
    }
}

pub fn coverage_dto(
    live: &LiveState,
    advanced_access_enabled: bool,
    detect: impl FnOnce() -> Option<PawnIoInstallation>,
) -> CoverageMatrixDto {
    let signals = live.coverage_signals();
    let summary = signals.summary();
    CoverageMatrixDto {
        tier: tier(summary.tier),
        confidence_ceiling: confidence(summary.confidence_ceiling),
        advanced_access: resolve_advanced_access(
            advanced_access_enabled,
            summary.advanced_access,
            detect,
        ),
        rows: coverage_rows(live, signals),
        conclusion_key: match summary.tier {
            crate::diagnostics::CoverageTier::A => "coverage.conclusion.a",
            crate::diagnostics::CoverageTier::B => "coverage.conclusion.b",
            crate::diagnostics::CoverageTier::C => "coverage.conclusion.c",
        },
    }
}

fn coverage_rows(live: &LiveState, signals: CoverageSignals) -> Vec<CoverageRowDto> {
    // Why a row is missing: the host has no such sensor, or it has one that returns nothing usable.
    let reason = |advertised: bool, missing: &'static str| {
        if advertised { "coverage.reading_unavailable" } else { missing }
    };
    let row = |id: &'static str,
               quality: &'static str,
               quality_label: &'static str,
               source: &'static str,
               available: bool,
               reason_key: &'static str| CoverageRowDto {
        id,
        label: id,
        available,
        quality,
        quality_label,
        source_label: source,
        reason_key: (!available).then_some(reason_key),
    };
    vec![
        row(
            "temperature",
            "direct",
            "Directa",
            "CPU package",
            signals.temperature,
            reason(live.advertises("temperature"), "coverage.temperature_missing"),
        ),
        row(
            "active_clock",
            "substitute",
            "Sustituta",
            "LibreHardwareMonitor",
            signals.active_clock,
            reason(live.advertises("clock"), "coverage.active_clock_missing"),
        ),
        row(
            "power",
            "direct",
            "Directa",
            "CPU package",
            signals.package_power,
            reason(live.advertises("power"), "coverage.power_missing"),
        ),
        row(
            "thermal_flag",
            "direct",
            "Directa",
            "MSR/SMU",
            signals.limit_reasons,
            "coverage.advanced_access_required",
        ),
    ]
}

pub fn topology_dto(live: &LiveState) -> CpuTopologyDto {
    CpuTopologyDto {
        cores: live
            .cores()
            .into_iter()
            .map(|core| CpuCoreDto {
                id: core.id,
                index: core.index,
                group: core.group,
                temperature_c: core.temperature_c,
                clock_mhz: core.clock_mhz,
                load_percent: core.load_percent,
                throttling: None,
            })
            .collect(),
    }
}

/// The passive-session recorder, shared by the observer (which feeds it) and the exit handler
/// (which closes the session in progress).
pub struct RecorderHandle {
    recorder: Mutex<Option<SessionRecorder>>,
    per_core_history: AtomicBool,
}

impl RecorderHandle {
    pub fn new() -> Self {
        Self {
            recorder: Mutex::new(Ruleset::v1().ok().and_then(SessionRecorder::new)),
            per_core_history: AtomicBool::new(false),
        }
    }

    /// Closes the session in progress and freezes its report: the application is exiting, or the
    /// data is about to be wiped and the session must not be lost half-written.
    pub fn flush(&self, app: &AppHandle) {
        let cpu = match app.try_state::<LiveHandle>() {
            Some(live) => sink::cpu_identity(live.read(|live| live.cpu().cloned()).as_ref()),
            None => sink::cpu_identity(None),
        };
        let Ok(mut guard) = self.recorder.lock() else { return };
        let Some(recorder) = guard.as_mut() else { return };
        let actions = recorder.close(epoch_ms());
        write_actions(app, &cpu, &actions);
    }

    /// The id of the passive session in progress, for the alert that opens it.
    pub fn session_id(&self) -> Option<String> {
        self.recorder.lock().ok().and_then(|guard| {
            guard.as_ref().and_then(|recorder| recorder.open_session_id().map(str::to_owned))
        })
    }

    /// Drops the session in progress after its rows were deleted.
    pub fn discard(&self) {
        if let Ok(mut guard) = self.recorder.lock()
            && let Some(recorder) = guard.as_mut()
        {
            recorder.abandon();
        }
    }
}

impl Default for RecorderHandle {
    fn default() -> Self {
        Self::new()
    }
}

fn write_actions(app: &AppHandle, cpu: &sink::CpuIdentity, actions: &[RecorderAction]) {
    if actions.is_empty() {
        return;
    }
    let Some(state) = app.try_state::<AppState>() else { return };
    let Ok(storage) = state.storage.lock() else { return };
    if let Err(error) = sink::execute(&storage, cpu, actions) {
        tracing::warn!(component = "storage", msg = "session recording failed", error = %error);
    }
}

/// The values a passive session keeps: everything the catalog delivers, except the per-core
/// series unless the person asked for that history (`sampling.per_core_history`).
fn recorded_values(
    live: &LiveState,
    per_core_history: bool,
) -> Vec<crate::storage::StoredSampleValue> {
    live.analysis_values()
        .into_iter()
        .filter(|value| per_core_history || !value.sensor_id.starts_with("cpu.core."))
        .map(|value| crate::storage::StoredSampleValue {
            sensor_id: value.sensor_id,
            value: value.value,
            boolean: value.boolean,
            quality: value.quality,
        })
        .collect()
}

/// Emits the Tauri events the interface listens to whenever the live state changes, and records
/// the passive sessions.
pub struct TauriObserver {
    pub app: AppHandle,
}

impl TauriObserver {
    fn record(&self, change: Change, live: &LiveState) {
        let Some(handle) = self.app.try_state::<RecorderHandle>() else { return };
        let Ok(mut guard) = handle.recorder.lock() else { return };
        let Some(recorder) = guard.as_mut() else { return };
        let now = epoch_ms();
        let actions = match change {
            Change::Sample if crate::commands::guided_in_progress(&self.app) => recorder.close(now),
            Change::Sample => recorder.observe(RecorderInput {
                epoch_ms: live.snapshot_input().map_or(now, |input| input.captured_at_ms),
                values: recorded_values(live, handle.per_core_history.load(Ordering::Relaxed)),
                verdict: live.diagnostic(),
                tier: live.coverage_signals().tier(),
                quality: SampleQuality::Complete,
                resumed: false,
            }),
            Change::Collector
                if matches!(
                    live.collector(),
                    CollectorState::Restarting | CollectorState::Failed | CollectorState::Stopped
                ) =>
            {
                recorder.interrupt(SplitReason::CollectorRestart, now)
            }
            _ => return,
        };
        if actions.iter().any(|action| matches!(action, RecorderAction::Open { .. }))
            && let Some(state) = self.app.try_state::<AppState>()
            && let Ok(storage) = state.storage.lock()
            && let Ok(preferences) = storage.user_preferences()
        {
            handle.per_core_history.store(
                preferences
                    .get("sampling.per_core_history")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
                Ordering::Relaxed,
            );
        }
        write_actions(&self.app, &sink::cpu_identity(live.cpu()), &actions);
    }

    fn advanced_access_enabled(&self) -> bool {
        self.app.try_state::<AppState>().is_none_or(|state| {
            state
                .storage
                .lock()
                .map(|storage| storage.advanced_access_enabled().unwrap_or(true))
                .unwrap_or(true)
        })
    }
}

impl LiveObserver for TauriObserver {
    fn updated(&self, change: Change, live: &LiveState) {
        let enabled = self.advanced_access_enabled();
        let now_ms = epoch_ms();
        self.record(change, live);
        if matches!(change, Change::Sample | Change::Collector) {
            let session =
                self.app.try_state::<RecorderHandle>().and_then(|handle| handle.session_id());
            crate::alerting::observe(&self.app, live, session.as_deref());
        }
        if matches!(change, Change::Sample | Change::Collector) {
            crate::tray::refresh(&self.app, Some(live), false);
        }
        if change == Change::Collector {
            let mut payload =
                json!({"state": live.collector().as_str(), "attempt": live.attempt()});
            if let (Some(key), Some(object)) = (live.message_key(), payload.as_object_mut()) {
                object.insert("message_key".to_owned(), json!(key));
            }
            let _ = self.app.emit("collector:state", payload);
        }
        if change == Change::Catalog {
            let _ = self.app.emit("coverage:changed", coverage_dto(live, enabled, detect_pawnio));
        }
        if let Change::Coverage { from, to, reason } = change {
            let mut payload = match serde_json::to_value(coverage_dto(live, enabled, detect_pawnio))
            {
                Ok(value) => value,
                Err(_) => json!({}),
            };
            if let Some(object) = payload.as_object_mut() {
                object.insert("from_tier".to_owned(), json!(tier_name(from)));
                object.insert("to_tier".to_owned(), json!(tier_name(to)));
                object.insert("reason".to_owned(), json!(reason.as_str()));
            }
            let _ = self.app.emit("coverage:changed", payload);
            if let Some(state) = self.app.try_state::<AppState>()
                && let Ok(storage) = state.storage.lock()
                && let Some(session_id) = storage.active_session_id().ok().flatten()
            {
                // Relative to the session start, like every frame and event of the session.
                let started_ms = storage
                    .session_started_at(&session_id)
                    .ok()
                    .flatten()
                    .and_then(|started| started.parse::<jiff::Timestamp>().ok())
                    .map(|started| started.as_millisecond());
                let at = live
                    .snapshot_input()
                    .and_then(|input| i64::try_from(input.captured_at_ms).ok())
                    .zip(started_ms)
                    .map_or(0, |(captured, started)| (captured - started).max(0));
                let _ = storage.record_coverage_change(
                    &session_id,
                    at,
                    tier_name(from),
                    tier_name(to),
                    reason.as_str(),
                );
                let _ = storage.set_session_coverage_tier(&session_id, tier_name(to));
            }
        }
        let locale = crate::tray::current_locale(&self.app);
        let _ = self.app.emit("telemetry:snapshot", snapshot_dto(live, enabled, now_ms, locale));
    }
}

fn tier_name(value: crate::diagnostics::CoverageTier) -> &'static str {
    match value {
        crate::diagnostics::CoverageTier::A => "A",
        crate::diagnostics::CoverageTier::B => "B",
        crate::diagnostics::CoverageTier::C => "C",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn live_with(sensors: Vec<Value>, values: Vec<Value>, cpu_extra: Value) -> LiveState {
        let mut live = LiveState::new();
        let mut cpu = json!({"vendor": "amd", "display_name": "Ryzen Test", "logical_processors": 12, "hybrid": false});
        if let (Some(base), Some(extra)) = (cpu.as_object_mut(), cpu_extra.as_object()) {
            base.extend(extra.clone());
        }
        live.apply_capabilities(&json!({
            "cpu": cpu,
            "groups": [{"id": "all", "kind": "homogeneous", "logical_count": 12}],
            "sensors": sensors
        }))
        .unwrap_or_else(|error| panic!("capabilities: {error:?}"));
        live.apply_sample(&json!({"monotonic_ms": 1, "duration_ms": 1, "values": values}), 10_000)
            .unwrap_or_else(|error| panic!("sample: {error:?}"));
        live.set_collector(CollectorState::Running, 0, None);
        live
    }

    fn sensor(id: &str, metric: &str, scope: &str, scope_ref: Option<&str>) -> Value {
        json!({"id": id, "source_id": id, "source_name": id, "metric": metric, "scope": scope, "scope_ref": scope_ref, "unit": "unknown", "quality": "direct"})
    }

    fn value(id: &str, number: f64) -> Value {
        json!({"sensor_id": id, "number": number, "status": "ok"})
    }

    #[test]
    fn a_disconnected_collector_without_data_is_reported_as_such() {
        let dto = snapshot_dto(&LiveState::new(), true, 5_000, crate::i18n::Locale::Es);

        assert_eq!(dto.freshness, "disconnected");
        assert_eq!(dto.collector_state, "stopped");
        assert_eq!(dto.cpu_label, "CPU no detectada todavía");
        assert!(dto.temperature_c.is_none() && dto.load_percent.is_none());
    }

    #[test]
    fn real_values_reach_the_snapshot_and_age_makes_them_stale() {
        let live = live_with(
            vec![
                sensor("cpu.package.temp", "temperature", "package", None),
                sensor("cpu.package.load", "load", "package", None),
            ],
            vec![value("cpu.package.temp", 64.0), value("cpu.package.load", 21.0)],
            json!({}),
        );

        let fresh = snapshot_dto(&live, true, 10_500, crate::i18n::Locale::Es);
        assert_eq!(fresh.freshness, "fresh");
        assert_eq!((fresh.temperature_c, fresh.load_percent), (Some(64.0), Some(21.0)));
        assert_eq!(fresh.cpu_label, "Ryzen Test");
        assert_eq!(fresh.collector_state, "running");
        assert_eq!(fresh.coverage.tier as u8, crate::commands::CoverageTierDto::C as u8);

        let stale = snapshot_dto(&live, true, 10_000 + 3_000, crate::i18n::Locale::Es);
        assert_eq!(stale.freshness, "stale");
        assert_eq!(stale.age_ms, 3_000);
    }

    #[test]
    fn a_failed_or_stopped_collector_is_disconnected_even_with_old_data() {
        let mut live = live_with(
            vec![sensor("cpu.package.load", "load", "package", None)],
            vec![value("cpu.package.load", 1.0)],
            json!({}),
        );
        live.set_collector(CollectorState::Failed, 3, Some("collector.restart_limit".to_owned()));

        assert_eq!(
            snapshot_dto(&live, true, 10_100, crate::i18n::Locale::Es).freshness,
            "disconnected"
        );
    }

    #[test]
    fn coverage_rows_say_whether_the_sensor_is_missing_or_unreadable() {
        // A Ryzen without low-level access: the sensors exist, but every reading is invalid.
        let live = live_with(
            vec![
                sensor("cpu.package.temp", "temperature", "package", None),
                sensor("cpu.package.load", "load", "package", None),
            ],
            vec![
                json!({"sensor_id": "cpu.package.temp", "status": "invalid"}),
                value("cpu.package.load", 5.0),
            ],
            json!({}),
        );

        let matrix = coverage_dto(&live, true, || None);

        let temperature =
            matrix.rows.iter().find(|row| row.id == "temperature").unwrap_or_else(|| panic!("row"));
        assert!(!temperature.available);
        assert_eq!(temperature.reason_key, Some("coverage.reading_unavailable"));
        let power =
            matrix.rows.iter().find(|row| row.id == "power").unwrap_or_else(|| panic!("row"));
        assert_eq!(power.reason_key, Some("coverage.power_missing")); // not advertised at all
        assert_eq!(matrix.conclusion_key, "coverage.conclusion.c");
    }

    #[test]
    fn advanced_access_follows_what_is_installed_only_when_the_tier_asks_for_it() {
        let version = |value: &str| value.to_owned();
        let cases = [
            (None, AdvancedAccess::Installable, AdvancedAccessDto::Installable),
            (
                Some(PawnIoInstallation::Missing),
                AdvancedAccess::Installable,
                AdvancedAccessDto::Installable,
            ),
            (
                Some(PawnIoInstallation::Upgradable { version: version("2.1.0") }),
                AdvancedAccess::Installable,
                AdvancedAccessDto::Upgradable,
            ),
            (
                Some(PawnIoInstallation::Current { version: version("2.2.0") }),
                AdvancedAccess::Installable,
                AdvancedAccessDto::Error,
            ),
            (
                Some(PawnIoInstallation::Upgradable { version: version("2.1.0") }),
                AdvancedAccess::NotNeeded,
                AdvancedAccessDto::NotNeeded,
            ),
        ];
        for (installation, computed, expected) in cases {
            let actual = resolve_advanced_access(true, computed, || installation.clone());
            assert_eq!(
                format!("{actual:?}"),
                format!("{expected:?}"),
                "{installation:?} / {computed:?}"
            );
        }
    }

    #[test]
    fn a_disabled_advanced_access_never_probes_the_machine() {
        let actual = resolve_advanced_access(false, AdvancedAccess::Installable, || {
            panic!("the probe must not run when advanced access is disabled")
        });
        assert!(matches!(actual, AdvancedAccessDto::Denied));
    }

    #[test]
    fn a_tier_a_machine_does_not_probe_the_installation() {
        let actual = resolve_advanced_access(true, AdvancedAccess::NotNeeded, || {
            panic!("tier A needs no probe")
        });
        assert!(matches!(actual, AdvancedAccessDto::NotNeeded));
    }

    #[test]
    fn disabling_advanced_access_reports_denied_whatever_the_coverage() {
        let live = live_with(vec![], vec![], json!({}));

        assert!(matches!(
            coverage_dto(&live, false, || None).advanced_access,
            AdvancedAccessDto::Denied
        ));
    }

    #[test]
    fn the_topology_lists_the_cores_the_collector_reports_and_nothing_else() {
        let live = live_with(
            vec![
                sensor("cpu.core.1.load", "load", "core", Some("1")),
                sensor("cpu.core.2.load", "load", "core", Some("2")),
            ],
            vec![value("cpu.core.1.load", 10.0)],
            json!({}),
        );

        let topology = topology_dto(&live);

        assert_eq!(topology.cores.len(), 2);
        assert_eq!(topology.cores[0].load_percent, Some(10.0));
        assert_eq!(topology.cores[1].load_percent, None);
        assert!(topology_dto(&LiveState::new()).cores.is_empty());
    }

    #[test]
    fn the_topology_label_names_hybrid_groups_and_virtual_machines() {
        let mut live = LiveState::new();
        live.apply_capabilities(&json!({
            "cpu": {"vendor": "intel", "display_name": "Core", "logical_processors": 20, "hybrid": true, "virtualized": true},
            "groups": [{"id": "p", "kind": "p", "logical_count": 12}, {"id": "e", "kind": "e", "logical_count": 8}],
            "sensors": []
        }))
        .unwrap_or_else(|error| panic!("capabilities: {error:?}"));

        assert_eq!(topology_label(&live, crate::i18n::Locale::Es), "12 P + 8 E · máquina virtual");
    }
}
