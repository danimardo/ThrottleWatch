use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum CoverageTier {
    A,
    B,
    C,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Ruleset {
    pub version: u32,
    pub active_load_percent: f64,
    pub sustained_load_start_percent: f64,
    pub thermal_high_margin_c: f64,
    pub thermal_critical_margin_c: f64,
    pub thermal_plateau_stddev_c: f64,
    pub power_plateau_cv: f64,
    pub reason_occupancy: f64,
    pub below_base_ratio: f64,
    pub below_base_duration_s: f64,
    pub stable_window_s: u64,
    pub stable_step_s: u64,
    pub turbo_min_window_s: u64,
    pub turbo_end_drop_ratio: f64,
    pub turbo_end_window_s: u64,
    pub platform_limit_drop_ratio: f64,
    pub platform_progressive_window_s: u64,
    #[serde(default)]
    pub parameter_ids: Vec<String>,
    #[serde(default)]
    pub parameters: BTreeMap<String, f64>,
    #[serde(default)]
    pub temperature_priority: Vec<String>,
    #[serde(default)]
    pub explainable_codes: Vec<String>,
    #[serde(default)]
    pub classification_rules: Vec<ClassificationRule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ClassificationRule {
    pub id: String,
    pub classification: String,
    pub explain_code: String,
}

impl Ruleset {
    pub fn v1() -> Result<Self, serde_json::Error> {
        serde_json::from_str(include_str!("../../resources/ruleset-v1.json"))
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.version == 0 || self.stable_window_s == 0 || self.stable_step_s == 0 {
            return Err("ruleset timing values must be positive");
        }
        if !(0.0..=100.0).contains(&self.active_load_percent)
            || !(0.0..=100.0).contains(&self.reason_occupancy)
            || !(0.0..=1.0).contains(&self.below_base_ratio)
        {
            return Err("ruleset percentages are outside their valid ranges");
        }
        if self.parameter_ids.is_empty() {
            return Err("ruleset must enumerate parameter identifiers");
        }
        if self.temperature_priority.is_empty()
            || self.explainable_codes.is_empty()
            || self.classification_rules.is_empty()
            || self
                .classification_rules
                .iter()
                .any(|rule| !self.explainable_codes.iter().any(|code| code == &rule.explain_code))
        {
            return Err("ruleset must define temperature priority, explainable codes and rules");
        }
        Ok(())
    }

    pub fn parameter(&self, id: &str) -> Option<f64> {
        self.parameters.get(id).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::Ruleset;

    #[test]
    fn loads_versioned_ruleset() -> serde_json::Result<()> {
        let rules = Ruleset::v1()?;
        assert_eq!(rules.version, 1);
        assert!(rules.stable_window_s > 0);
        assert!(rules.parameter_ids.iter().any(|id| id == "thermal.critical_margin_c"));
        assert!(rules.validate().is_ok());
        assert_eq!(
            rules.classification_rules.first().map(|rule| rule.id.as_str()),
            Some("rule-1-indeterminate")
        );
        let required = [
            "thermal.margin_warn_c",
            "thermal.margin_critical_c",
            "thermal.abs_warn_c",
            "thermal.abs_critical_c",
            "load.active_core_util_pct",
            "load.start_threshold_pct",
            "window.stable_s",
            "window.step_s",
            "window.turbo_min_s",
            "turbo.end_power_drop_pct",
            "turbo.end_drop_window_s",
            "plateau.thermal_margin_c",
            "plateau.thermal_stddev_c",
            "plateau.power_cv_pct",
            "rules.reason_occupancy_pct",
            "rules.chassis_limit_drop_pct",
            "rules.chassis_plateau_drop_pct",
            "rules.chassis_min_steps",
            "rules.chassis_min_span_s",
            "rules.oem_step_window_s",
            "rules.power_margin_a_c",
            "rules.power_margin_b_c",
            "rules.freq_drop_vs_turbo_pct",
            "severity.below_base_ratio",
            "severity.min_duration_s",
            "session.indeterminate_share_pct",
            "session.class_min_s",
            "session.class_min_share_pct",
            "session.gap_s",
            "session.max_h",
            "confidence.medium_min",
            "confidence.high_min",
            "potential.range_low_factor",
            "potential.tier_barely_max_pct",
            "potential.tier_moderate_max_pct",
            "potential.round_step_pct",
            "guided.preflight_max_s",
            "guided.rest_s",
            "guided.warmup_s",
            "guided.load_short_s",
            "guided.load_standard_s",
            "guided.load_long_s",
            "guided.recovery_s",
            "guided.measure_head_s",
            "guided.measure_tail_s",
            "guided.stop_over_limit_c",
            "guided.stop_over_limit_samples",
            "guided.stop_low_freq_ratio",
            "guided.stop_low_freq_s",
            "guided.stop_missing_sensor_samples",
            "guided.stop_generator_timeout_s",
            "guided.min_free_disk_mb",
            "alerts.min_persistence_s",
            "alerts.cooldown_min",
            "sampling.interval_low_power_ms",
            "sampling.interval_normal_ms",
            "sampling.interval_diagnostic_ms",
            "glass.degrade_fps",
            "glass.degrade_window_s",
            "glass.degrade_idle_cpu_pct",
            "glass.degrade_idle_window_s",
            "glass.restore_fps",
            "glass.restore_window_s",
            "storage.retry_s",
            "logging.frontend_max_per_min",
            "logging.detailed_hours",
            "collector.max_invalid_messages",
            "collector.max_restarts",
            "collector.restart_window_min",
            "collector.stall_intervals",
            "collector.parent_check_s",
            "potential.max_percent",
        ];
        assert!(required.iter().all(|id| rules.parameter_ids.iter().any(|actual| actual == id)));
        assert!(required.iter().all(|id| rules.parameter(id).is_some()));
        Ok(())
    }
}
