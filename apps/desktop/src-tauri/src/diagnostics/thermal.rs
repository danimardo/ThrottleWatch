use super::{DiagnosticSample, rules::Ruleset, windows::StableWindow};

pub fn thermal_plateau(window: &StableWindow, rules: &Ruleset) -> bool {
    window.thermal_margin_c.is_some_and(|margin| margin <= rules.thermal_critical_margin_c)
        && window.temperature_stddev_c <= rules.thermal_plateau_stddev_c
}

pub fn temperature_is_high(window: &StableWindow, rules: &Ruleset) -> bool {
    window.thermal_margin_c.is_some_and(|margin| margin <= rules.thermal_high_margin_c)
}

pub fn has_thermal_evidence(window: &StableWindow, rules: &Ruleset) -> bool {
    window.thermal_occupancy >= rules.reason_occupancy || thermal_plateau(window, rules)
}

pub fn missing_or_invalid_temperature(samples: &[DiagnosticSample]) -> bool {
    samples.iter().all(|sample| sample.temperature_c.is_none())
}

#[cfg(test)]
mod tests {
    use super::{temperature_is_high, thermal_plateau};
    use crate::diagnostics::{rules::Ruleset, windows::StableWindow};

    fn window(margin: f64, stddev: f64) -> StableWindow {
        StableWindow {
            start_ms: 0,
            end_ms: 60_000,
            sample_count: 60,
            active_clock_mhz: 3000.0,
            base_clock_mhz: 3500.0,
            temperature_mean_c: 92.0,
            temperature_stddev_c: stddev,
            thermal_margin_c: Some(margin),
            package_power_mean_w: Some(50.0),
            package_power_cv: Some(0.01),
            thermal_occupancy: 0.0,
            prochot_occupancy: 0.0,
            power_occupancy: 0.0,
            current_occupancy: 0.0,
            sustained_load: true,
            in_turbo_window: false,
        }
    }

    #[test]
    fn recognizes_high_temperature_without_confusing_it_with_a_plateau() -> serde_json::Result<()> {
        let rules = Ruleset::v1()?;
        assert!(temperature_is_high(&window(8.0, 3.0), &rules));
        assert!(!thermal_plateau(&window(3.0, 2.0), &rules));
        assert!(thermal_plateau(&window(3.0, 1.0), &rules));
        Ok(())
    }
}
