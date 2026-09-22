//! Live state of the collector as the backend sees it: the catalog from `capabilities`, the latest
//! value of every published sensor and the health of the collector process. Pure and synchronous:
//! the runtime feeds it protocol payloads and the commands read snapshots out of it.
//!
//! Nothing here invents data. A sensor that is absent, `invalid` or unknown stays `None`, and the
//! coverage signals are computed from what the collector really delivers.
#![deny(clippy::unwrap_used, clippy::expect_used)]

use super::evaluator::LiveEvaluator;
use super::host_clock::HostClockReading;
use super::normalization::{SensorMetadata, SensorQuality, normalize_number};
use super::snapshot::SnapshotInput;
use crate::diagnostics::{CoverageSignals, DiagnosticResult, DiagnosticSample, Ruleset};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveError {
    InvalidPayload(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectorState {
    Starting,
    Running,
    Degraded,
    Restarting,
    Stopped,
    Failed,
}

impl CollectorState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Degraded => "degraded",
            Self::Restarting => "restarting",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct CpuInfo {
    pub vendor: String,
    pub display_name: String,
    pub logical_processors: u32,
    pub hybrid: bool,
    #[serde(default)]
    pub virtualized: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct CpuGroup {
    pub id: String,
    pub kind: String,
    pub logical_count: u32,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct SensorDescriptor {
    pub id: String,
    pub metric: String,
    pub scope: String,
    #[serde(default)]
    pub scope_ref: Option<String>,
    pub unit: String,
    pub quality: String,
    /// What the collector knows about the sensor (`thermal_limit_c`, `tjmax_c`, …), if anything.
    #[serde(default)]
    pub metadata: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct CapabilitiesPayload {
    cpu: CpuInfo,
    groups: Vec<CpuGroup>,
    sensors: Vec<SensorDescriptor>,
}

/// T156: the sidecar's own, live detection of PawnIO (service, `ROOT\PAWNIO` device,
/// `PawnIo.Version` — `LowLevelAccessProbe.Run()` in the .NET sidecar), sent once per
/// handshake (`contracts/ipc-protocol.md` § Handshake). `state` is one of `available`,
/// `reduced`, `missing`, `denied`, `error`, `unknown` — Rust never invents a seventh.
#[derive(Debug, Deserialize)]
struct HelloAckPayload {
    low_level_access: LowLevelAccessPayload,
}

#[derive(Debug, Deserialize)]
struct LowLevelAccessPayload {
    state: String,
}

#[derive(Debug, Deserialize)]
struct SamplePayload {
    values: Vec<SampleValue>,
}

#[derive(Debug, Deserialize)]
struct SampleValue {
    sensor_id: String,
    status: String,
    #[serde(default)]
    number: Option<f64>,
    #[serde(default)]
    boolean: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Reading {
    number: Option<f64>,
    boolean: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CoreView {
    pub id: String,
    pub index: u16,
    pub group: &'static str,
    pub temperature_c: Option<f64>,
    pub clock_mhz: Option<f64>,
    pub load_percent: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnalysisValue {
    pub sensor_id: String,
    pub value: Option<f64>,
    pub boolean: Option<bool>,
    pub quality: String,
}

#[derive(Debug, Clone)]
pub struct LiveState {
    cpu: Option<CpuInfo>,
    groups: Vec<CpuGroup>,
    sensors: Vec<SensorDescriptor>,
    readings: HashMap<String, Reading>,
    captured_at_ms: Option<u64>,
    collector: CollectorState,
    attempt: u32,
    message_key: Option<String>,
    evaluator: Option<LiveEvaluator>,
    host_clock: Option<HostClockReading>,
    sampling_interval_ms: Option<u64>,
    /// T156: the sidecar's own PawnIO detection from its last handshake — `None` before the
    /// first `hello_ack` of this process, or if it sent a payload Rust could not parse.
    sidecar_low_level_access: Option<String>,
}

impl Default for LiveState {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveState {
    pub fn new() -> Self {
        Self {
            cpu: None,
            groups: Vec::new(),
            sensors: Vec::new(),
            readings: HashMap::new(),
            captured_at_ms: None,
            collector: CollectorState::Stopped,
            attempt: 0,
            message_key: None,
            evaluator: Ruleset::v1().ok().map(LiveEvaluator::new),
            host_clock: None,
            sampling_interval_ms: None,
            sidecar_low_level_access: None,
        }
    }

    /// T156: records the sidecar's own PawnIO detection from `hello_ack.low_level_access.state`
    /// so `advanced_access` can reflect it (`commands::live::resolve_advanced_access`) instead of
    /// only Rust's own installer/registry check, which cannot tell a driver that is installed but
    /// blocked by policy or antivirus (`denied`) from one that is merely misbehaving (`error`).
    /// Never fatal: an old sidecar or a payload Rust cannot parse just leaves this `None`, exactly
    /// as if it had not handshaked yet — a raw `hello_ack` that fails another check already
    /// invalidates the message elsewhere.
    pub fn apply_hello_ack(&mut self, payload: &Value) {
        self.sidecar_low_level_access = serde_json::from_value::<HelloAckPayload>(payload.clone())
            .ok()
            .map(|parsed| parsed.low_level_access.state);
    }

    pub fn sidecar_low_level_access(&self) -> Option<&str> {
        self.sidecar_low_level_access.as_deref()
    }

    /// Replaces the catalog (a restart or a change of detail generates a new one) and forgets the
    /// readings of sensors that no longer exist.
    pub fn apply_capabilities(&mut self, payload: &Value) -> Result<(), LiveError> {
        let parsed: CapabilitiesPayload = serde_json::from_value(payload.clone())
            .map_err(|_| LiveError::InvalidPayload("capabilities"))?;
        if parsed.cpu.logical_processors == 0 {
            return Err(LiveError::InvalidPayload("capabilities.cpu.logical_processors"));
        }
        self.readings.retain(|id, _| parsed.sensors.iter().any(|sensor| &sensor.id == id));
        self.cpu = Some(parsed.cpu);
        self.groups = parsed.groups;
        self.sensors = parsed.sensors;
        if let Some(evaluator) = self.evaluator.as_mut() {
            evaluator.reset();
        }
        Ok(())
    }

    /// Stores the values of one sample. Values of unknown sensors are ignored; a value that is not
    /// `ok`, or is physically impossible, is stored as absent.
    pub fn apply_sample(&mut self, payload: &Value, now_ms: u64) -> Result<(), LiveError> {
        let parsed: SamplePayload = serde_json::from_value(payload.clone())
            .map_err(|_| LiveError::InvalidPayload("sample"))?;
        for value in parsed.values {
            let Some(descriptor) = self.sensors.iter().find(|sensor| sensor.id == value.sensor_id)
            else {
                continue;
            };
            let ok = value.status == "ok";
            let number = if ok {
                normalize_number(
                    descriptor.id.clone(),
                    descriptor.metric.clone(),
                    value.number,
                    metadata(descriptor),
                )
                .number
            } else {
                None
            };
            let boolean = if ok { value.boolean } else { None };
            self.readings.insert(value.sensor_id, Reading { number, boolean });
        }
        self.captured_at_ms = Some(now_ms);
        let sample = self.diagnostic_sample(now_ms);
        let coverage = self.coverage_signals();
        if let Some(evaluator) = self.evaluator.as_mut() {
            evaluator.observe(sample, coverage);
        }
        Ok(())
    }

    /// The engine's verdict on the recent window; `None` until a usable sample has arrived.
    pub fn diagnostic(&self) -> Option<&DiagnosticResult> {
        self.evaluator.as_ref().and_then(LiveEvaluator::latest)
    }

    /// One engine input from the latest readings; a sample without package load is not usable.
    fn diagnostic_sample(&self, monotonic_ms: u64) -> Option<DiagnosticSample> {
        let input = self.snapshot_input()?;
        Some(DiagnosticSample {
            monotonic_ms,
            load_percent: input.load_percent?,
            active_clock_mhz: input.active_clock_mhz,
            base_clock_mhz: input.base_clock_mhz,
            temperature_c: input.temperature_c,
            thermal_limit_c: input.thermal_limit_c,
            package_power_w: input.package_power_w,
            power_limit_w: input.power_limit_w,
            thermal_flag: self.flag("thermal_flag").unwrap_or(false),
            prochot_flag: self.flag("prochot_flag").unwrap_or(false),
            power_flag: self.flag("power_flag").unwrap_or(false),
            current_flag: self.flag("current_flag").unwrap_or(false),
            in_turbo_window: false,
        })
    }

    /// Stores the clock the host derived from the Windows counters (`None` keeps the last one
    /// out: an unreadable counter must not leave a stale clock behind).
    pub fn apply_host_clock(&mut self, reading: Option<HostClockReading>) {
        self.host_clock = reading;
    }

    /// The interval the collector was last asked to sample at; how long a reading stays fresh
    /// depends on it, so the interface never calls a slow profile «stale».
    pub fn set_sampling_interval_ms(&mut self, interval_ms: u64) {
        self.sampling_interval_ms = Some(interval_ms);
    }

    pub const fn sampling_interval_ms(&self) -> Option<u64> {
        self.sampling_interval_ms
    }

    pub fn set_collector(
        &mut self,
        state: CollectorState,
        attempt: u32,
        message_key: Option<String>,
    ) {
        self.collector = state;
        self.attempt = attempt;
        self.message_key = message_key;
    }

    pub const fn collector(&self) -> CollectorState {
        self.collector
    }

    pub const fn attempt(&self) -> u32 {
        self.attempt
    }

    pub fn message_key(&self) -> Option<&str> {
        self.message_key.as_deref()
    }

    pub fn cpu(&self) -> Option<&CpuInfo> {
        self.cpu.as_ref()
    }

    pub fn groups(&self) -> &[CpuGroup] {
        &self.groups
    }

    pub const fn has_data(&self) -> bool {
        self.captured_at_ms.is_some()
    }

    fn descriptor(&self, metric: &str, scope: &str) -> Option<&SensorDescriptor> {
        self.sensors.iter().find(|sensor| sensor.metric == metric && sensor.scope == scope)
    }

    fn number(&self, metric: &str, scope: &str) -> Option<f64> {
        let descriptor = self.descriptor(metric, scope)?;
        self.readings.get(&descriptor.id)?.number
    }

    fn flag(&self, metric: &str) -> Option<bool> {
        let descriptor = self.descriptor(metric, "package")?;
        self.readings.get(&descriptor.id)?.boolean
    }

    /// The effective thermal limit the collector published on the package temperature descriptor
    /// (`TjMax − TCC offset` on Intel, the family limit on AMD, FR-076). Absent until the collector
    /// knows it; a value outside a physically plausible range is discarded, never trusted.
    pub fn thermal_limit_c(&self) -> Option<f64> {
        let limit = self
            .descriptor("temperature", "package")?
            .metadata
            .as_ref()?
            .get("thermal_limit_c")?
            .as_f64()?;
        (40.0..=125.0).contains(&limit).then_some(limit)
    }

    /// Latest package-level values, or `None` when no sample has arrived yet.
    pub fn snapshot_input(&self) -> Option<SnapshotInput> {
        Some(SnapshotInput {
            captured_at_ms: self.captured_at_ms?,
            temperature_c: self.number("temperature", "package"),
            thermal_limit_c: self.thermal_limit_c(),
            load_percent: self.number("load", "package"),
            active_clock_mhz: self
                .number("active_clock", "package")
                .or_else(|| self.number("clock", "package"))
                .or(self.host_clock.map(|clock| clock.active_mhz)),
            base_clock_mhz: self
                .number("base_clock", "package")
                .or(self.host_clock.map(|clock| clock.base_mhz)),
            package_power_w: self.number("power", "package"),
            power_limit_w: self.number("power_limit", "package"),
        })
    }

    /// Returns the catalog IDs and their current values without renaming them. Analysis owns the
    /// mapping from these stable IDs to tracks; substring matching would make a new sensor silently
    /// appear in the wrong chart.
    pub fn analysis_values(&self) -> Vec<AnalysisValue> {
        let host_clock = self.host_clock.filter(|_| {
            self.number("active_clock", "package").is_none()
                && self.number("clock", "package").is_none()
        });
        let host_values = host_clock.into_iter().flat_map(|clock| {
            [("host.active_clock", clock.active_mhz), ("host.base_clock", clock.base_mhz)].map(
                |(id, value)| AnalysisValue {
                    sensor_id: id.to_owned(),
                    value: Some(value),
                    boolean: None,
                    quality: "derived".to_owned(),
                },
            )
        });
        self.sensors
            .iter()
            .map(|sensor| {
                let reading = self.readings.get(&sensor.id);
                AnalysisValue {
                    sensor_id: sensor.id.clone(),
                    value: reading.and_then(|reading| reading.number),
                    boolean: reading.and_then(|reading| reading.boolean),
                    quality: if reading.is_some_and(|reading| {
                        reading.number.is_some() || reading.boolean.is_some()
                    }) {
                        sensor.quality.clone()
                    } else {
                        "missing".to_owned()
                    },
                }
            })
            .chain(host_values)
            .collect()
    }

    /// What the collector really delivers right now; drives the A/B/C coverage tier.
    pub fn coverage_signals(&self) -> CoverageSignals {
        let per_core_load = self
            .sensors
            .iter()
            .filter(|sensor| sensor.metric == "load" && sensor.scope == "core")
            .any(|sensor| {
                self.readings.get(&sensor.id).and_then(|reading| reading.number).is_some()
            });
        CoverageSignals {
            temperature: self.number("temperature", "package").is_some(),
            active_clock: self.number("active_clock", "package").is_some()
                || self.number("clock", "package").is_some()
                || self.host_clock.is_some(),
            per_core_load,
            package_power: self.number("power", "package").is_some(),
            power_limit: self.number("power_limit", "package").is_some(),
            limit_reasons: ["thermal_flag", "prochot_flag", "power_flag", "current_flag"]
                .iter()
                .any(|metric| self.flag(metric).is_some()),
        }
    }

    /// Drops only readings that require the advanced provider. The ordinary
    /// temperature/clock/load/power path remains usable at coverage B/C after
    /// a recoverable provider failure.
    pub fn clear_advanced_readings(&mut self) {
        let advanced_ids: Vec<String> = self
            .sensors
            .iter()
            .filter(|sensor| {
                matches!(
                    sensor.metric.as_str(),
                    "power_limit" | "thermal_flag" | "prochot_flag" | "power_flag" | "current_flag"
                )
            })
            .map(|sensor| sensor.id.clone())
            .collect();
        for id in advanced_ids {
            self.readings.remove(&id);
        }
    }

    /// Which advertised magnitudes exist in the catalog at all (independent of the latest values).
    pub fn advertises(&self, metric: &str) -> bool {
        self.sensors.iter().any(|sensor| sensor.metric == metric)
    }

    pub fn is_virtualized(&self) -> bool {
        self.cpu.as_ref().is_some_and(|cpu| cpu.virtualized)
    }

    /// One entry per core the collector reports per-core sensors for.
    pub fn cores(&self) -> Vec<CoreView> {
        let mut keys: Vec<&str> = self
            .sensors
            .iter()
            .filter(|sensor| sensor.scope == "core")
            .filter_map(|sensor| sensor.scope_ref.as_deref())
            .collect();
        keys.sort_by_key(|key| (group_of(key), core_number(key)));
        keys.dedup();
        keys.into_iter()
            .map(|key| CoreView {
                id: format!("core-{key}"),
                index: core_number(key).saturating_sub(1),
                group: group_of(key),
                temperature_c: self.core_number("temperature", key),
                clock_mhz: self.core_number("clock", key),
                load_percent: self.core_number("load", key),
            })
            .collect()
    }

    fn core_number(&self, metric: &str, key: &str) -> Option<f64> {
        let descriptor = self.sensors.iter().find(|sensor| {
            sensor.scope == "core"
                && sensor.metric == metric
                && sensor.scope_ref.as_deref() == Some(key)
        })?;
        self.readings.get(&descriptor.id)?.number
    }
}

fn metadata(descriptor: &SensorDescriptor) -> SensorMetadata {
    SensorMetadata {
        source_id: descriptor.id.clone(),
        source_name: descriptor.id.clone(),
        scope: descriptor.scope.clone(),
        quality: match descriptor.quality.as_str() {
            "direct" => SensorQuality::Direct,
            "derived" => SensorQuality::Derived,
            "substitute" => SensorQuality::Substitute,
            _ => SensorQuality::Unknown,
        },
        thermal_limit_c: None,
        base_clock_mhz: None,
    }
}

/// `p1`, `e9`, `lpe1` → hybrid group; a bare number → not grouped.
fn group_of(key: &str) -> &'static str {
    if key.starts_with("lpe") {
        "lp"
    } else if key.starts_with('p') {
        "p"
    } else if key.starts_with('e') {
        "e"
    } else {
        "ungrouped"
    }
}

fn core_number(key: &str) -> u16 {
    let digits: String = key.chars().filter(char::is_ascii_digit).collect();
    digits.parse().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sensor(id: &str, metric: &str, scope: &str, scope_ref: Option<&str>) -> Value {
        json!({"id": id, "source_id": id, "source_name": id, "metric": metric, "scope": scope,
               "scope_ref": scope_ref, "unit": "unknown", "quality": "direct"})
    }

    fn capabilities(sensors: Vec<Value>) -> Value {
        json!({
            "cpu": {"vendor": "amd", "display_name": "Test CPU", "logical_processors": 12, "hybrid": false},
            "groups": [{"id": "all", "kind": "homogeneous", "logical_count": 12}],
            "sensors": sensors
        })
    }

    fn sample(values: Vec<Value>) -> Value {
        json!({"monotonic_ms": 1000, "duration_ms": 10, "values": values})
    }

    fn ok(id: &str, number: f64) -> Value {
        json!({"sensor_id": id, "number": number, "status": "ok"})
    }

    fn state_with_full_b_coverage() -> LiveState {
        let mut state = LiveState::new();
        state
            .apply_capabilities(&capabilities(vec![
                sensor("cpu.package.temp", "temperature", "package", None),
                sensor("cpu.package.load", "load", "package", None),
                sensor("cpu.package.power", "power", "package", None),
                sensor("cpu.package.clock", "clock", "package", None),
                sensor("cpu.core.1.load", "load", "core", Some("1")),
                sensor("cpu.core.2.load", "load", "core", Some("2")),
                sensor("cpu.core.1.clock", "clock", "core", Some("1")),
            ]))
            .unwrap_or_else(|error| panic!("capabilities: {error:?}"));
        state
    }

    #[test]
    fn a_hello_ack_records_the_sidecars_low_level_access_state() {
        let mut state = LiveState::new();
        assert_eq!(state.sidecar_low_level_access(), None);
        state.apply_hello_ack(&json!({
            "agent_version": "0.1.0",
            "selected_protocol": 1,
            "runtime": ".NET",
            "low_level_access": {"state": "denied", "provider": "pawnio", "details_code": "X"}
        }));
        assert_eq!(state.sidecar_low_level_access(), Some("denied"));
    }

    #[test]
    fn a_later_hello_ack_replaces_the_earlier_low_level_access_state() {
        let mut state = LiveState::new();
        state.apply_hello_ack(&json!({"low_level_access": {"state": "denied"}}));
        state.apply_hello_ack(&json!({"low_level_access": {"state": "available"}}));
        assert_eq!(state.sidecar_low_level_access(), Some("available"));
    }

    #[test]
    fn a_hello_ack_rust_cannot_parse_leaves_the_state_as_if_none_had_arrived() {
        let mut state = LiveState::new();
        state.apply_hello_ack(&json!({"low_level_access": {"state": "denied"}}));
        state.apply_hello_ack(&json!({"nothing_useful": true}));
        assert_eq!(
            state.sidecar_low_level_access(),
            None,
            "a malformed hello_ack must not keep stale access state around either"
        );
    }

    #[test]
    fn nothing_arrived_yet_means_no_snapshot_and_no_coverage() {
        let state = state_with_full_b_coverage();
        assert!(state.snapshot_input().is_none());
        assert!(!state.has_data());
        let signals = state.coverage_signals();
        assert!(!signals.temperature && !signals.package_power && !signals.per_core_load);
    }

    #[test]
    fn a_full_sample_fills_the_snapshot_and_reaches_coverage_b() {
        let mut state = state_with_full_b_coverage();
        state
            .apply_sample(
                &sample(vec![
                    ok("cpu.package.temp", 61.5),
                    ok("cpu.package.load", 12.0),
                    ok("cpu.package.power", 34.0),
                    ok("cpu.package.clock", 3925.0),
                    ok("cpu.core.1.load", 10.0),
                    ok("cpu.core.2.load", 20.0),
                    ok("cpu.core.1.clock", 3900.0),
                ]),
                5_000,
            )
            .unwrap_or_else(|error| panic!("sample: {error:?}"));

        let input = state.snapshot_input().unwrap_or_else(|| panic!("snapshot expected"));
        assert_eq!(input.captured_at_ms, 5_000);
        assert_eq!(input.temperature_c, Some(61.5));
        assert_eq!(input.load_percent, Some(12.0));
        assert_eq!(input.package_power_w, Some(34.0));
        assert_eq!(input.active_clock_mhz, Some(3925.0));
        assert_eq!(input.thermal_limit_c, None);
        assert_eq!(input.base_clock_mhz, None);
        assert_eq!(state.coverage_signals().tier(), crate::diagnostics::CoverageTier::B);
    }

    fn temperature_descriptor_with(metadata: Value) -> Value {
        json!({"id": "cpu.package.temp", "source_id": "t", "source_name": "CPU Package",
               "metric": "temperature", "scope": "package", "scope_ref": null,
               "unit": "celsius", "quality": "direct", "metadata": metadata})
    }

    #[test]
    fn the_collectors_thermal_limit_reaches_the_snapshot_and_implausible_ones_do_not() {
        let mut state = LiveState::new();
        state
            .apply_capabilities(&capabilities(vec![temperature_descriptor_with(
                json!({"tjmax_c": 100.0, "tcc_offset_c": 5.0, "thermal_limit_c": 95.0}),
            )]))
            .unwrap_or_else(|error| panic!("capabilities: {error:?}"));
        state
            .apply_sample(&sample(vec![ok("cpu.package.temp", 70.0)]), 1_000)
            .unwrap_or_else(|error| panic!("sample: {error:?}"));
        assert_eq!(state.thermal_limit_c(), Some(95.0));
        let input = state.snapshot_input().unwrap_or_else(|| panic!("snapshot expected"));
        assert_eq!(input.thermal_limit_c, Some(95.0));

        for bogus in [
            json!({"thermal_limit_c": 5.0}),
            json!({"thermal_limit_c": 900.0}),
            json!({"thermal_limit_c": null}),
            json!({}),
        ] {
            let mut other = LiveState::new();
            other
                .apply_capabilities(&capabilities(vec![temperature_descriptor_with(bogus)]))
                .unwrap_or_else(|error| panic!("capabilities: {error:?}"));
            assert_eq!(other.thermal_limit_c(), None);
        }
    }

    #[test]
    fn a_collector_that_publishes_no_limit_leaves_it_unknown() {
        let state = state_with_full_b_coverage();
        assert_eq!(state.thermal_limit_c(), None);
    }

    #[test]
    fn the_host_clock_fills_the_clocks_the_collector_does_not_publish() {
        use crate::telemetry::host_clock::HostClockReading;
        let mut state = state_with_full_b_coverage();
        state.apply_host_clock(Some(HostClockReading { active_mhz: 3780.0, base_mhz: 3600.0 }));
        state
            .apply_sample(&sample(vec![ok("cpu.package.load", 40.0)]), 1_000)
            .unwrap_or_else(|error| panic!("sample: {error:?}"));

        let input = state.snapshot_input().unwrap_or_else(|| panic!("snapshot expected"));
        assert_eq!(input.active_clock_mhz, Some(3780.0));
        assert_eq!(input.base_clock_mhz, Some(3600.0));
        assert!(state.coverage_signals().active_clock);

        state.apply_host_clock(None);
        let input = state.snapshot_input().unwrap_or_else(|| panic!("snapshot expected"));
        assert_eq!(input.active_clock_mhz, None, "an unreadable counter leaves no stale clock");
        assert_eq!(input.base_clock_mhz, None);
    }

    #[test]
    fn the_collector_clock_wins_over_the_host_clock() {
        use crate::telemetry::host_clock::HostClockReading;
        let mut state = state_with_full_b_coverage();
        state.apply_host_clock(Some(HostClockReading { active_mhz: 1000.0, base_mhz: 3600.0 }));
        state
            .apply_sample(&sample(vec![ok("cpu.package.clock", 4200.0)]), 1_000)
            .unwrap_or_else(|error| panic!("sample: {error:?}"));

        let input = state.snapshot_input().unwrap_or_else(|| panic!("snapshot expected"));
        assert_eq!(input.active_clock_mhz, Some(4200.0));
        assert_eq!(input.base_clock_mhz, Some(3600.0));
    }

    #[test]
    fn a_loaded_trace_with_limit_reasons_is_classified_from_the_live_samples() {
        let mut state = LiveState::new();
        state
            .apply_capabilities(&capabilities(vec![
                sensor("cpu.package.temp", "temperature", "package", None),
                sensor("cpu.package.load", "load", "package", None),
                sensor("cpu.package.clock", "active_clock", "package", None),
                sensor("cpu.package.base", "base_clock", "package", None),
                sensor("cpu.package.power", "power", "package", None),
                sensor("cpu.package.limit", "power_limit", "package", None),
                sensor("cpu.core.1.load", "load", "core", Some("1")),
                sensor("cpu.package.thermal", "thermal_flag", "package", None),
            ]))
            .unwrap_or_else(|error| panic!("capabilities: {error:?}"));
        assert!(state.diagnostic().is_none(), "no verdict before the first sample");
        for second in 0..=200_u64 {
            let mut values = vec![
                ok("cpu.package.temp", 94.0),
                ok("cpu.package.load", 98.0),
                ok("cpu.package.clock", 3400.0),
                ok("cpu.package.base", 3600.0),
                ok("cpu.package.power", 60.0),
                ok("cpu.package.limit", 90.0),
                ok("cpu.core.1.load", 98.0),
            ];
            values
                .push(json!({"sensor_id": "cpu.package.thermal", "status": "ok", "boolean": true}));
            state
                .apply_sample(&sample(values), second * 1_000)
                .unwrap_or_else(|error| panic!("sample: {error:?}"));
        }
        let result = state.diagnostic().unwrap_or_else(|| panic!("a verdict is expected"));
        assert_eq!(result.classification.key(), "thermal_confirmed");
    }

    #[test]
    fn invalid_missing_and_impossible_values_stay_absent_never_zero() {
        let mut state = state_with_full_b_coverage();
        state
            .apply_sample(
                &sample(vec![
                    json!({"sensor_id": "cpu.package.temp", "status": "invalid"}),
                    ok("cpu.package.load", 250.0), // impossible load
                    json!({"sensor_id": "cpu.package.power", "status": "missing"}),
                    ok("cpu.package.clock", 3000.0),
                ]),
                1,
            )
            .unwrap_or_else(|error| panic!("sample: {error:?}"));

        let input = state.snapshot_input().unwrap_or_else(|| panic!("snapshot expected"));
        assert_eq!(input.temperature_c, None);
        assert_eq!(input.load_percent, None);
        assert_eq!(input.package_power_w, None);
        assert_eq!(input.active_clock_mhz, Some(3000.0));
        assert_eq!(state.coverage_signals().tier(), crate::diagnostics::CoverageTier::C);
    }

    #[test]
    fn coverage_stays_c_without_per_core_load_even_with_everything_else() {
        let mut state = LiveState::new();
        state
            .apply_capabilities(&capabilities(vec![
                sensor("cpu.package.temp", "temperature", "package", None),
                sensor("cpu.package.power", "power", "package", None),
                sensor("cpu.package.clock", "clock", "package", None),
            ]))
            .unwrap_or_else(|error| panic!("capabilities: {error:?}"));
        state
            .apply_sample(
                &sample(vec![
                    ok("cpu.package.temp", 50.0),
                    ok("cpu.package.power", 20.0),
                    ok("cpu.package.clock", 3000.0),
                ]),
                1,
            )
            .unwrap_or_else(|error| panic!("sample: {error:?}"));

        assert!(!state.coverage_signals().per_core_load);
        assert_eq!(state.coverage_signals().tier(), crate::diagnostics::CoverageTier::C);
    }

    #[test]
    fn provider_failure_clears_only_advanced_signals_and_keeps_level_b_usable() {
        let mut state = LiveState::new();
        state
            .apply_capabilities(&capabilities(vec![
                sensor("cpu.package.temp", "temperature", "package", None),
                sensor("cpu.package.clock", "clock", "package", None),
                sensor("cpu.package.power", "power", "package", None),
                sensor("cpu.package.limit", "power_limit", "package", None),
                sensor("cpu.package.thermal", "thermal_flag", "package", None),
                sensor("cpu.core.1.load", "load", "core", Some("1")),
            ]))
            .unwrap_or_else(|error| panic!("capabilities: {error:?}"));
        state
            .apply_sample(
                &sample(vec![
                    ok("cpu.package.temp", 70.0),
                    ok("cpu.package.clock", 3900.0),
                    ok("cpu.package.power", 60.0),
                    ok("cpu.package.limit", 95.0),
                    json!({"sensor_id": "cpu.package.thermal", "boolean": true, "status": "ok"}),
                    ok("cpu.core.1.load", 80.0),
                ]),
                1,
            )
            .unwrap_or_else(|error| panic!("sample: {error:?}"));
        assert_eq!(state.coverage_signals().tier(), crate::diagnostics::CoverageTier::A);

        state.clear_advanced_readings();

        assert_eq!(state.coverage_signals().tier(), crate::diagnostics::CoverageTier::B);
        assert!(state.snapshot_input().is_some_and(|input| input.package_power_w == Some(60.0)));
    }

    #[test]
    fn unknown_sensors_are_ignored_and_malformed_payloads_are_rejected() {
        let mut state = state_with_full_b_coverage();
        assert!(state.apply_sample(&sample(vec![ok("cpu.unknown", 1.0)]), 1).is_ok());
        assert!(state.snapshot_input().is_some_and(|input| input.temperature_c.is_none()));
        assert_eq!(
            state.apply_sample(&json!({"values": "nope"}), 1),
            Err(LiveError::InvalidPayload("sample"))
        );
        assert_eq!(
            state.apply_capabilities(&json!({"cpu": {}})),
            Err(LiveError::InvalidPayload("capabilities"))
        );
    }

    #[test]
    fn a_new_catalog_forgets_the_readings_of_sensors_that_disappear() {
        let mut state = state_with_full_b_coverage();
        state
            .apply_sample(
                &sample(vec![ok("cpu.package.temp", 61.5), ok("cpu.package.load", 5.0)]),
                1,
            )
            .unwrap_or_else(|error| panic!("sample: {error:?}"));

        state
            .apply_capabilities(&capabilities(vec![sensor(
                "cpu.package.load",
                "load",
                "package",
                None,
            )]))
            .unwrap_or_else(|error| panic!("capabilities: {error:?}"));

        let input = state.snapshot_input().unwrap_or_else(|| panic!("snapshot expected"));
        assert_eq!(input.temperature_c, None);
        assert_eq!(input.load_percent, Some(5.0));
    }

    #[test]
    fn cores_are_grouped_by_kind_and_carry_their_own_readings() {
        let mut state = LiveState::new();
        state
            .apply_capabilities(&capabilities(vec![
                sensor("cpu.core.p1.load", "load", "core", Some("p1")),
                sensor("cpu.core.p1.temp", "temperature", "core", Some("p1")),
                sensor("cpu.core.e9.load", "load", "core", Some("e9")),
                sensor("cpu.core.lpe1.clock", "clock", "core", Some("lpe1")),
            ]))
            .unwrap_or_else(|error| panic!("capabilities: {error:?}"));
        state
            .apply_sample(
                &sample(vec![
                    ok("cpu.core.p1.load", 40.0),
                    ok("cpu.core.p1.temp", 70.0),
                    ok("cpu.core.e9.load", 5.0),
                ]),
                1,
            )
            .unwrap_or_else(|error| panic!("sample: {error:?}"));

        let cores = state.cores();
        assert_eq!(cores.iter().map(|core| core.group).collect::<Vec<_>>(), vec!["e", "lp", "p"]);
        let p = cores.iter().find(|core| core.id == "core-p1").unwrap_or_else(|| panic!("p1"));
        assert_eq!((p.temperature_c, p.load_percent, p.clock_mhz), (Some(70.0), Some(40.0), None));
        assert_eq!(p.index, 0);
    }

    #[test]
    fn collector_health_is_tracked_separately_from_data() {
        let mut state = LiveState::new();
        assert_eq!(state.collector(), CollectorState::Stopped);
        state.set_collector(CollectorState::Restarting, 2, Some("collector.restarting".to_owned()));
        assert_eq!(
            (state.collector(), state.attempt(), state.message_key()),
            (CollectorState::Restarting, 2, Some("collector.restarting"))
        );
        assert_eq!(CollectorState::Failed.as_str(), "failed");
    }
}
