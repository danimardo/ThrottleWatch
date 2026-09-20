//! Builds the DTOs of the live screens from the collector's [`LiveState`] and emits the events.
//! Nothing here fabricates a value: a magnitude the collector does not deliver is `None` and its
//! coverage row says why.
#![deny(clippy::unwrap_used, clippy::expect_used)]

use super::{
    AdvancedAccessDto, CoverageDto, CoverageMatrixDto, CoverageRowDto, CpuCoreDto, CpuTopologyDto,
    LiveSnapshotDto, access, confidence, freshness, tier,
};
use crate::diagnostics::{CoverageSignals, Ruleset};
use crate::storage::AppState;
use crate::telemetry::live::{CollectorState, LiveState};
use crate::telemetry::power_context::{PowerSource, read_windows_power_context};
use crate::telemetry::runtime::{Change, LiveObserver};
use crate::telemetry::snapshot::{Freshness, aggregate_snapshot};
use serde_json::json;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};
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
) -> LiveSnapshotDto {
    let rules = Ruleset::v1().ok();
    let interval_ms =
        rules.as_ref().and_then(|rules| rules.parameter("sampling.interval_normal_ms"));
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
        // The engine that classifies windows is not fed from the live samples yet: say so.
        classification: Some("indeterminate"),
        severity: None,
        cpu_label: live
            .cpu()
            .map_or_else(|| "CPU no detectada todavía".to_owned(), |cpu| cpu.display_name.clone()),
        topology_label: topology_label(live),
        power_label: match power {
            PowerSource::Ac => "Corriente alterna",
            PowerSource::Battery => "Batería",
            PowerSource::Unknown => "Fuente desconocida",
        }
        .to_owned(),
        collector_state: live.collector().as_str(),
        coverage: CoverageDto {
            tier: tier(summary.tier),
            confidence_ceiling: confidence(summary.confidence_ceiling),
            advanced_access: advanced_access(advanced_access_enabled, summary.advanced_access),
        },
        confidence_label: match summary.confidence_ceiling {
            crate::diagnostics::ConfidenceCeiling::Low => "Confianza máxima alcanzable: baja",
            crate::diagnostics::ConfidenceCeiling::Medium => "Confianza máxima alcanzable: media",
            crate::diagnostics::ConfidenceCeiling::High => "Confianza máxima alcanzable: alta",
        }
        .to_owned(),
        active_cores: None,
        platform_kind: None,
        in_turbo_window: false,
    }
}

fn advanced_access(
    enabled: bool,
    computed: crate::diagnostics::AdvancedAccess,
) -> AdvancedAccessDto {
    if enabled { access(computed) } else { AdvancedAccessDto::Denied }
}

fn topology_label(live: &LiveState) -> String {
    let Some(cpu) = live.cpu() else {
        return "Esperando catálogo del colector".to_owned();
    };
    let groups = live
        .groups()
        .iter()
        .filter(|group| matches!(group.kind.as_str(), "p" | "e" | "lp_e"))
        .map(|group| {
            format!("{} {}", group.logical_count, group.kind.to_uppercase().replace('_', "-"))
        })
        .collect::<Vec<_>>();
    let virtualized = if cpu.virtualized { " · máquina virtual" } else { "" };
    if groups.is_empty() {
        format!("{} procesadores lógicos{virtualized}", cpu.logical_processors)
    } else {
        format!("{}{virtualized}", groups.join(" + "))
    }
}

pub fn coverage_dto(live: &LiveState, advanced_access_enabled: bool) -> CoverageMatrixDto {
    let signals = live.coverage_signals();
    let summary = signals.summary();
    CoverageMatrixDto {
        tier: tier(summary.tier),
        confidence_ceiling: confidence(summary.confidence_ceiling),
        advanced_access: advanced_access(advanced_access_enabled, summary.advanced_access),
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

/// Emits the Tauri events the interface listens to whenever the live state changes.
pub struct TauriObserver {
    pub app: AppHandle,
}

impl TauriObserver {
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
        if change == Change::Collector {
            let mut payload =
                json!({"state": live.collector().as_str(), "attempt": live.attempt()});
            if let (Some(key), Some(object)) = (live.message_key(), payload.as_object_mut()) {
                object.insert("message_key".to_owned(), json!(key));
            }
            let _ = self.app.emit("collector:state", payload);
        }
        if change == Change::Catalog {
            let _ = self.app.emit("coverage:changed", coverage_dto(live, enabled));
        }
        let _ = self.app.emit("telemetry:snapshot", snapshot_dto(live, enabled, now_ms));
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
        let dto = snapshot_dto(&LiveState::new(), true, 5_000);

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

        let fresh = snapshot_dto(&live, true, 10_500);
        assert_eq!(fresh.freshness, "fresh");
        assert_eq!((fresh.temperature_c, fresh.load_percent), (Some(64.0), Some(21.0)));
        assert_eq!(fresh.cpu_label, "Ryzen Test");
        assert_eq!(fresh.collector_state, "running");
        assert_eq!(fresh.coverage.tier as u8, crate::commands::CoverageTierDto::C as u8);

        let stale = snapshot_dto(&live, true, 10_000 + 3_000);
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

        assert_eq!(snapshot_dto(&live, true, 10_100).freshness, "disconnected");
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

        let matrix = coverage_dto(&live, true);

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
    fn disabling_advanced_access_reports_denied_whatever_the_coverage() {
        let live = live_with(vec![], vec![], json!({}));

        assert!(matches!(coverage_dto(&live, false).advanced_access, AdvancedAccessDto::Denied));
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

        assert_eq!(topology_label(&live), "12 P + 8 E · máquina virtual");
    }
}
