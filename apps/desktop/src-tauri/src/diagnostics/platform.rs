use super::{rules::Ruleset, windows::StableWindow};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformKind {
    ChassisThermal,
    ExternalProchot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitTrend {
    Progressive,
    SingleStep,
    Unchanged,
}

pub fn limit_trend(values: &[f64], rules: &Ruleset) -> LimitTrend {
    if values.len() < 2 {
        return LimitTrend::Unchanged;
    }
    let first = values[0];
    let last = *values.last().unwrap_or(&first);
    if first <= 0.0 || (first - last) / first < rules.platform_limit_drop_ratio {
        return LimitTrend::Unchanged;
    }
    let steps = values
        .windows(2)
        .filter(|pair| {
            pair[0] > 0.0 && (pair[0] - pair[1]) / pair[0] >= rules.platform_limit_drop_ratio / 2.0
        })
        .count();
    if steps >= 2
        || values.len() as u64 >= rules.platform_progressive_window_s / rules.stable_step_s.max(1)
    {
        LimitTrend::Progressive
    } else {
        LimitTrend::SingleStep
    }
}

pub fn external_prochot(window: &StableWindow, rules: &Ruleset) -> bool {
    window.prochot_occupancy >= rules.reason_occupancy
        && window.thermal_occupancy < rules.reason_occupancy
}

pub fn chassis_thermal(
    limit_first: Option<f64>,
    limit_last: Option<f64>,
    window: &StableWindow,
    rules: &Ruleset,
) -> bool {
    let Some((first, last)) = limit_first.zip(limit_last) else {
        return false;
    };
    first > 0.0
        && (first - last) / first >= rules.platform_limit_drop_ratio
        && window.thermal_margin_c.is_some_and(|margin| margin > rules.thermal_high_margin_c)
}

#[cfg(test)]
mod tests {
    use super::{LimitTrend, PlatformKind, chassis_thermal, external_prochot, limit_trend};
    use crate::diagnostics::{rules::Ruleset, windows::StableWindow};

    fn window(prochot: f64) -> StableWindow {
        StableWindow {
            start_ms: 0,
            end_ms: 60_000,
            sample_count: 60,
            active_clock_mhz: 2500.0,
            base_clock_mhz: 3500.0,
            temperature_mean_c: 70.0,
            temperature_stddev_c: 0.5,
            thermal_margin_c: Some(20.0),
            package_power_mean_w: Some(45.0),
            package_power_cv: Some(0.01),
            thermal_occupancy: 0.0,
            prochot_occupancy: prochot,
            power_occupancy: 0.0,
            current_occupancy: 0.0,
            sustained_load: true,
            in_turbo_window: false,
        }
    }

    #[test]
    fn distinguishes_external_signal_from_chassis_limit() {
        let rules = Ruleset::v1().expect("valid rules");
        assert_eq!(PlatformKind::ExternalProchot, PlatformKind::ExternalProchot);
        assert!(external_prochot(&window(0.25), &rules));
        assert!(chassis_thermal(Some(100.0), Some(85.0), &window(0.0), &rules));
        assert_eq!(limit_trend(&[100.0, 95.0, 85.0], &rules), LimitTrend::Progressive);
        assert_eq!(limit_trend(&[100.0, 85.0], &rules), LimitTrend::SingleStep);
    }
}
