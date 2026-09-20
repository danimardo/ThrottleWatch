//! Live state of the collector as the backend sees it: the catalog from `capabilities`, the latest
//! value of every published sensor and the health of the collector process. Pure and synchronous:
//! the runtime feeds it protocol payloads and the commands read snapshots out of it.
//!
//! Nothing here invents data. A sensor that is absent, `invalid` or unknown stays `None`, and the
//! coverage signals are computed from what the collector really delivers.
#![deny(clippy::unwrap_used, clippy::expect_used)]

use super::normalization::{SensorMetadata, SensorQuality, normalize_number};
use super::snapshot::SnapshotInput;
use crate::diagnostics::CoverageSignals;
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
}

#[derive(Debug, Deserialize)]
struct CapabilitiesPayload {
    cpu: CpuInfo,
    groups: Vec<CpuGroup>,
    sensors: Vec<SensorDescriptor>,
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
        }
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
        Ok(())
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

    /// Latest package-level values, or `None` when no sample has arrived yet.
    pub fn snapshot_input(&self) -> Option<SnapshotInput> {
        Some(SnapshotInput {
            captured_at_ms: self.captured_at_ms?,
            temperature_c: self.number("temperature", "package"),
            // The sidecar does not publish the thermal limit yet (it needs the level A tables): unknown.
            thermal_limit_c: None,
            load_percent: self.number("load", "package"),
            active_clock_mhz: self
                .number("active_clock", "package")
                .or_else(|| self.number("clock", "package")),
            base_clock_mhz: self.number("base_clock", "package"),
            package_power_w: self.number("power", "package"),
            power_limit_w: self.number("power_limit", "package"),
        })
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
                || self.number("clock", "package").is_some(),
            per_core_load,
            package_power: self.number("power", "package").is_some(),
            power_limit: self.number("power_limit", "package").is_some(),
            limit_reasons: ["thermal_flag", "prochot_flag", "power_flag", "current_flag"]
                .iter()
                .any(|metric| self.flag(metric).is_some()),
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
