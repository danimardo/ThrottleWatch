#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuidedStopReason {
    UserRequested,
    ThermalSafety,
    SevereLowClock,
    CriticalSensorLost,
    GeneratorUnresponsive,
    Suspended,
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GuidedSafetyState {
    pub consecutive_over_limit: u8,
    pub consecutive_low_clock_ms: u64,
    pub consecutive_missing_sensor: u8,
    pub generator_silent_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuidedSafetyLimits {
    pub over_limit_samples: u8,
    pub low_clock_ms: u64,
    pub missing_sensor_samples: u8,
    pub generator_timeout_ms: u64,
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

#[cfg(test)]
mod tests {
    use super::{
        Comparability, GuidedResult, GuidedSafetyLimits, GuidedSafetyState, GuidedStopReason,
        compare_results, stop_reason,
    };

    fn limits() -> GuidedSafetyLimits {
        GuidedSafetyLimits {
            over_limit_samples: 3,
            low_clock_ms: 10_000,
            missing_sensor_samples: 3,
            generator_timeout_ms: 5_000,
        }
    }

    #[test]
    fn reaching_the_thermal_limit_is_not_itself_a_stop() {
        assert_eq!(
            stop_reason(
                GuidedSafetyState {
                    consecutive_over_limit: 0,
                    consecutive_low_clock_ms: 0,
                    consecutive_missing_sensor: 0,
                    generator_silent_ms: 0,
                },
                false,
                false,
                false,
                limits()
            ),
            None
        );
    }

    #[test]
    fn stops_after_the_configured_safety_evidence() {
        let state = GuidedSafetyState {
            consecutive_over_limit: 3,
            consecutive_low_clock_ms: 10_000,
            consecutive_missing_sensor: 3,
            generator_silent_ms: 5_000,
        };
        assert_eq!(
            stop_reason(state, true, false, false, limits()),
            Some(GuidedStopReason::ThermalSafety)
        );
        assert_eq!(
            stop_reason(state, false, true, false, limits()),
            Some(GuidedStopReason::SevereLowClock)
        );
    }

    #[test]
    fn only_compares_results_with_the_same_generation_context() {
        let result = GuidedResult {
            initial_ops_per_second: 100.0,
            sustained_ops_per_second: 90.0,
            relative_percent: Some(-10.0),
            generator_version: "1".to_owned(),
            profile: "standard".to_owned(),
            power_context: "ac:balanced".to_owned(),
        };
        let mut different = result.clone();
        different.profile = "long".to_owned();
        assert_eq!(compare_results(&result, &result), Comparability::Comparable);
        assert_eq!(compare_results(&result, &different), Comparability::NotComparable);
    }
}
