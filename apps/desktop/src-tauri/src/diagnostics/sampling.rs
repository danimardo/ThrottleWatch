//! Sampling policy shared by the collector runtime and settings (T071).
#![deny(clippy::unwrap_used, clippy::expect_used)]

use super::Ruleset;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplingProfile {
    LowPower,
    Normal,
    Diagnostic,
}

impl SamplingProfile {
    pub const fn key(self) -> &'static str {
        match self {
            Self::LowPower => "low_power",
            Self::Normal => "normal",
            Self::Diagnostic => "diagnostic",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        [Self::LowPower, Self::Normal, Self::Diagnostic].into_iter().find(|item| item.key() == key)
    }

    fn parameter(self) -> &'static str {
        match self {
            Self::LowPower => "sampling.interval_low_power_ms",
            Self::Normal => "sampling.interval_normal_ms",
            Self::Diagnostic => "sampling.interval_diagnostic_ms",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnBatteryMode {
    Keep,
    LowPower,
    Pause,
}

impl OnBatteryMode {
    pub const fn key(self) -> &'static str {
        match self {
            Self::Keep => "keep",
            Self::LowPower => "low_power",
            Self::Pause => "pause",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        [Self::Keep, Self::LowPower, Self::Pause].into_iter().find(|item| item.key() == key)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SamplingPlan {
    pub interval: Option<Duration>,
    pub per_core_history: bool,
}

pub fn plan(
    rules: &Ruleset,
    profile: SamplingProfile,
    on_battery: OnBatteryMode,
    ac_power: bool,
    per_core_history: bool,
) -> Option<SamplingPlan> {
    let interval_key = if ac_power {
        profile.parameter()
    } else {
        match on_battery {
            OnBatteryMode::Keep => profile.parameter(),
            OnBatteryMode::LowPower => "sampling.interval_low_power_ms",
            OnBatteryMode::Pause => {
                return Some(SamplingPlan { interval: None, per_core_history });
            }
        }
    };
    let interval_ms = rules.parameter(interval_key)?;
    if !interval_ms.is_finite() || interval_ms <= 0.0 || interval_ms > u64::MAX as f64 {
        return None;
    }
    Some(SamplingPlan {
        interval: Some(Duration::from_millis(interval_ms.round() as u64)),
        per_core_history,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_read_the_three_ruleset_intervals() {
        let rules = Ruleset::v1().unwrap_or_else(|error| panic!("ruleset: {error}"));
        assert_eq!(
            plan(&rules, SamplingProfile::LowPower, OnBatteryMode::Keep, true, false)
                .and_then(|p| p.interval),
            Some(Duration::from_secs(5))
        );
        assert_eq!(
            plan(&rules, SamplingProfile::Normal, OnBatteryMode::Keep, true, false)
                .and_then(|p| p.interval),
            Some(Duration::from_secs(1))
        );
        assert_eq!(
            plan(&rules, SamplingProfile::Diagnostic, OnBatteryMode::Keep, true, true)
                .and_then(|p| p.interval),
            Some(Duration::from_millis(500))
        );
    }

    #[test]
    fn battery_modes_are_explicit_and_preserve_core_history() {
        let rules = Ruleset::v1().unwrap_or_else(|error| panic!("ruleset: {error}"));
        let low_power =
            plan(&rules, SamplingProfile::Diagnostic, OnBatteryMode::LowPower, false, true);
        assert_eq!(low_power.and_then(|p| p.interval), Some(Duration::from_secs(5)));
        assert!(low_power.is_some_and(|p| p.per_core_history));
        let paused = plan(&rules, SamplingProfile::Normal, OnBatteryMode::Pause, false, true);
        assert_eq!(paused, Some(SamplingPlan { interval: None, per_core_history: true }));
    }

    #[test]
    fn keep_mode_does_not_change_profile_on_battery() {
        let rules = Ruleset::v1().unwrap_or_else(|error| panic!("ruleset: {error}"));
        let plan = plan(&rules, SamplingProfile::Diagnostic, OnBatteryMode::Keep, false, false);
        assert_eq!(plan.and_then(|p| p.interval), Some(Duration::from_millis(500)));
    }
}
