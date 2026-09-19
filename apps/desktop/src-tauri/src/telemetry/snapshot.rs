use crate::diagnostics::classifier::{Classification, DiagnosticResult, Severity};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    Fresh,
    Stale,
    Disconnected,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapshotInput {
    pub captured_at_ms: u64,
    pub temperature_c: Option<f64>,
    pub thermal_limit_c: Option<f64>,
    pub load_percent: Option<f64>,
    pub active_clock_mhz: Option<f64>,
    pub base_clock_mhz: Option<f64>,
    pub package_power_w: Option<f64>,
    pub power_limit_w: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TelemetrySnapshot {
    pub captured_at_ms: u64,
    pub freshness: Freshness,
    pub age_ms: u64,
    pub temperature_c: Option<f64>,
    pub thermal_limit_c: Option<f64>,
    pub thermal_margin_c: Option<f64>,
    pub load_percent: Option<f64>,
    pub active_clock_mhz: Option<f64>,
    pub base_clock_mhz: Option<f64>,
    pub package_power_w: Option<f64>,
    pub power_limit_w: Option<f64>,
    pub classification: Option<Classification>,
    pub severity: Option<Severity>,
}

pub fn aggregate_snapshot(
    latest: Option<SnapshotInput>,
    now_ms: u64,
    stale_after_ms: u64,
    disconnected_after_ms: u64,
    diagnostic: Option<&DiagnosticResult>,
) -> TelemetrySnapshot {
    let Some(input) = latest else {
        return TelemetrySnapshot {
            captured_at_ms: now_ms,
            freshness: Freshness::Disconnected,
            age_ms: 0,
            temperature_c: None,
            thermal_limit_c: None,
            thermal_margin_c: None,
            load_percent: None,
            active_clock_mhz: None,
            base_clock_mhz: None,
            package_power_w: None,
            power_limit_w: None,
            classification: diagnostic.map(|result| result.classification),
            severity: diagnostic.and_then(|result| result.severity),
        };
    };
    let age_ms = now_ms.saturating_sub(input.captured_at_ms);
    let freshness = if age_ms >= disconnected_after_ms {
        Freshness::Disconnected
    } else if age_ms >= stale_after_ms {
        Freshness::Stale
    } else {
        Freshness::Fresh
    };
    TelemetrySnapshot {
        captured_at_ms: input.captured_at_ms,
        freshness,
        age_ms,
        temperature_c: input.temperature_c,
        thermal_limit_c: input.thermal_limit_c,
        thermal_margin_c: input
            .thermal_limit_c
            .zip(input.temperature_c)
            .map(|(limit, temp)| limit - temp),
        load_percent: input.load_percent,
        active_clock_mhz: input.active_clock_mhz,
        base_clock_mhz: input.base_clock_mhz,
        package_power_w: input.package_power_w,
        power_limit_w: input.power_limit_w,
        classification: diagnostic.map(|result| result.classification),
        severity: diagnostic.and_then(|result| result.severity),
    }
}

#[cfg(test)]
mod tests {
    use super::{Freshness, SnapshotInput, aggregate_snapshot};

    #[test]
    fn computes_margin_and_freshness_without_fabricating_missing_values() {
        let snapshot = aggregate_snapshot(
            Some(SnapshotInput {
                captured_at_ms: 1_000,
                temperature_c: Some(80.0),
                thermal_limit_c: Some(95.0),
                load_percent: None,
                active_clock_mhz: Some(3_500.0),
                base_clock_mhz: Some(3_500.0),
                package_power_w: None,
                power_limit_w: None,
            }),
            2_000,
            5_000,
            10_000,
            None,
        );
        assert_eq!(snapshot.freshness, Freshness::Fresh);
        assert_eq!(snapshot.thermal_margin_c, Some(15.0));
        assert_eq!(snapshot.load_percent, None);
    }

    #[test]
    fn distinguishes_stale_and_disconnected_data() {
        let input = SnapshotInput {
            captured_at_ms: 0,
            temperature_c: None,
            thermal_limit_c: None,
            load_percent: None,
            active_clock_mhz: None,
            base_clock_mhz: None,
            package_power_w: None,
            power_limit_w: None,
        };
        assert_eq!(
            aggregate_snapshot(Some(input), 5_000, 5_000, 10_000, None).freshness,
            Freshness::Stale
        );
        assert_eq!(
            aggregate_snapshot(Some(input), 10_000, 5_000, 10_000, None).freshness,
            Freshness::Disconnected
        );
    }
}
