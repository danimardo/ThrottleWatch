use super::{DiagnosticSample, rules::Ruleset};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StableWindow {
    pub start_ms: u64,
    pub end_ms: u64,
    pub sample_count: usize,
    pub active_clock_mhz: f64,
    pub base_clock_mhz: f64,
    pub temperature_mean_c: f64,
    pub temperature_stddev_c: f64,
    pub thermal_margin_c: Option<f64>,
    pub package_power_mean_w: Option<f64>,
    pub package_power_cv: Option<f64>,
    pub thermal_occupancy: f64,
    pub prochot_occupancy: f64,
    pub power_occupancy: f64,
    pub current_occupancy: f64,
    pub sustained_load: bool,
    pub in_turbo_window: bool,
}

pub fn stable_windows(samples: &[DiagnosticSample], rules: &Ruleset) -> Vec<StableWindow> {
    let window_ms = rules.stable_window_s.saturating_mul(1000);
    let step_ms = rules.stable_step_s.saturating_mul(1000).max(1);
    if samples.is_empty() || window_ms == 0 {
        return Vec::new();
    }

    let mut output = Vec::new();
    let first = samples[0].monotonic_ms;
    let last = samples.last().map_or(first, |sample| sample.monotonic_ms);
    let mut start = first;
    while start.saturating_add(window_ms) <= last.saturating_add(1) {
        let end = start.saturating_add(window_ms);
        let window: Vec<_> = samples
            .iter()
            .filter(|sample| sample.monotonic_ms >= start && sample.monotonic_ms < end)
            .copied()
            .collect();
        if let Some(feature) = feature_window(&window, rules) {
            output.push(StableWindow { start_ms: start, end_ms: end, ..feature });
        }
        start = start.saturating_add(step_ms);
    }
    output
}

pub fn active_processor_count(loads_percent: &[f64], active_threshold_percent: f64) -> usize {
    loads_percent.iter().filter(|load| **load >= active_threshold_percent).count()
}

pub fn sustained_load_started(
    previous_load_percent: f64,
    current_load_percent: f64,
    rules: &Ruleset,
) -> bool {
    previous_load_percent < rules.sustained_load_start_percent
        && current_load_percent >= rules.active_load_percent
}

pub fn turbo_exclusion_ms(tau_s: Option<f64>, rules: &Ruleset) -> u64 {
    tau_s
        .filter(|tau| tau.is_finite() && *tau >= 0.0)
        .map(|tau| (tau.max(rules.turbo_min_window_s as f64) * 1000.0) as u64)
        .unwrap_or(rules.turbo_min_window_s.saturating_mul(1000))
}

pub fn turbo_end(
    previous_power_w: f64,
    current_power_w: f64,
    elapsed_ms: u64,
    rules: &Ruleset,
) -> bool {
    previous_power_w > 0.0
        && elapsed_ms <= rules.turbo_end_window_s.saturating_mul(1000)
        && (previous_power_w - current_power_w) / previous_power_w >= rules.turbo_end_drop_ratio
}

fn feature_window(samples: &[DiagnosticSample], rules: &Ruleset) -> Option<StableWindow> {
    let clocks: Vec<f64> = samples.iter().filter_map(|sample| sample.active_clock_mhz).collect();
    let bases: Vec<f64> = samples.iter().filter_map(|sample| sample.base_clock_mhz).collect();
    let temperatures: Vec<f64> = samples.iter().filter_map(|sample| sample.temperature_c).collect();
    if clocks.is_empty() || bases.is_empty() || temperatures.is_empty() {
        return None;
    }
    let powers: Vec<f64> = samples.iter().filter_map(|sample| sample.package_power_w).collect();
    let mean_temperature = mean(&temperatures);
    let temperature_stddev = stddev(&temperatures, mean_temperature);
    let thermal_margin = samples
        .iter()
        .filter_map(|sample| {
            sample.temperature_c.zip(sample.thermal_limit_c).map(|(temp, limit)| limit - temp)
        })
        .reduce(|left, right| left.min(right));
    let power_mean = (!powers.is_empty()).then(|| mean(&powers));
    let power_cv = power_mean.and_then(|mean_power| {
        (mean_power > f64::EPSILON).then(|| stddev(&powers, mean_power) / mean_power)
    });
    let count = samples.len() as f64;
    Some(StableWindow {
        start_ms: 0,
        end_ms: 0,
        sample_count: samples.len(),
        active_clock_mhz: mean(&clocks),
        base_clock_mhz: mean(&bases),
        temperature_mean_c: mean_temperature,
        temperature_stddev_c: temperature_stddev,
        thermal_margin_c: thermal_margin,
        package_power_mean_w: power_mean,
        package_power_cv: power_cv,
        thermal_occupancy: samples.iter().filter(|sample| sample.thermal_flag).count() as f64
            / count,
        prochot_occupancy: samples.iter().filter(|sample| sample.prochot_flag).count() as f64
            / count,
        power_occupancy: samples.iter().filter(|sample| sample.power_flag).count() as f64 / count,
        current_occupancy: samples.iter().filter(|sample| sample.current_flag).count() as f64
            / count,
        sustained_load: samples
            .iter()
            .all(|sample| sample.load_percent >= rules.active_load_percent),
        in_turbo_window: samples.iter().any(|sample| sample.in_turbo_window),
    })
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn stddev(values: &[f64], average: f64) -> f64 {
    (values.iter().map(|value| (value - average).powi(2)).sum::<f64>() / values.len() as f64).sqrt()
}

#[cfg(test)]
mod tests {
    use super::{
        active_processor_count, stable_windows, sustained_load_started, turbo_end,
        turbo_exclusion_ms,
    };
    use crate::diagnostics::{DiagnosticSample, rules::Ruleset};

    fn sample(index: u64) -> DiagnosticSample {
        DiagnosticSample {
            monotonic_ms: index * 1000,
            load_percent: 95.0,
            active_clock_mhz: Some(3800.0),
            base_clock_mhz: Some(3500.0),
            temperature_c: Some(92.0),
            thermal_limit_c: Some(95.0),
            package_power_w: Some(60.0),
            power_limit_w: Some(90.0),
            thermal_flag: index.is_multiple_of(5),
            prochot_flag: false,
            power_flag: false,
            current_flag: false,
            in_turbo_window: false,
        }
    }

    #[test]
    fn builds_sliding_sixty_second_windows() -> serde_json::Result<()> {
        let rules = Ruleset::v1()?;
        let samples: Vec<_> = (0..=70).map(sample).collect();
        let windows = stable_windows(&samples, &rules);
        assert!(!windows.is_empty());
        assert_eq!(windows[0].sample_count, 60);
        assert!(windows[0].thermal_occupancy > 0.0);
        Ok(())
    }

    #[test]
    fn detects_active_cores_load_start_turbo_end_and_exclusion_window() -> serde_json::Result<()> {
        let rules = Ruleset::v1()?;
        assert_eq!(active_processor_count(&[20.0, 80.0, 95.0], rules.active_load_percent), 2);
        assert!(sustained_load_started(20.0, 90.0, &rules));
        assert!(turbo_end(100.0, 80.0, 5_000, &rules));
        assert_eq!(turbo_exclusion_ms(Some(28.0), &rules), 60_000);
        assert_eq!(turbo_exclusion_ms(Some(90.0), &rules), 90_000);
        Ok(())
    }
}
