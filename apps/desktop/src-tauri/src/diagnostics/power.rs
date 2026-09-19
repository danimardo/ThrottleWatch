use super::{rules::Ruleset, windows::StableWindow};

pub fn power_plateau(window: &StableWindow, rules: &Ruleset) -> bool {
    window.package_power_cv.is_some_and(|coefficient| coefficient <= rules.power_plateau_cv)
        && window.thermal_margin_c.is_some_and(|margin| margin > rules.thermal_high_margin_c)
}

pub fn has_power_reason(window: &StableWindow, rules: &Ruleset) -> bool {
    window.power_occupancy.max(window.current_occupancy) >= rules.reason_occupancy
}

pub fn progressive_limit_drop(first_w: Option<f64>, last_w: Option<f64>, rules: &Ruleset) -> bool {
    let Some((first, last)) = first_w.zip(last_w) else {
        return false;
    };
    first > 0.0 && (first - last) / first >= rules.platform_limit_drop_ratio
}

#[cfg(test)]
mod tests {
    use super::{has_power_reason, power_plateau, progressive_limit_drop};
    use crate::diagnostics::{rules::Ruleset, windows::StableWindow};

    fn window() -> StableWindow {
        StableWindow {
            start_ms: 0,
            end_ms: 60_000,
            sample_count: 60,
            active_clock_mhz: 2500.0,
            base_clock_mhz: 3500.0,
            temperature_mean_c: 70.0,
            temperature_stddev_c: 0.5,
            thermal_margin_c: Some(25.0),
            package_power_mean_w: Some(45.0),
            package_power_cv: Some(0.01),
            thermal_occupancy: 0.0,
            prochot_occupancy: 0.0,
            power_occupancy: 0.25,
            current_occupancy: 0.0,
            sustained_load: true,
            in_turbo_window: false,
        }
    }

    #[test]
    fn recognizes_power_plateaus_and_direct_reasons() -> serde_json::Result<()> {
        let rules = Ruleset::v1()?;
        assert!(power_plateau(&window(), &rules));
        assert!(has_power_reason(&window(), &rules));
        assert!(progressive_limit_drop(Some(100.0), Some(85.0), &rules));
        Ok(())
    }
}
