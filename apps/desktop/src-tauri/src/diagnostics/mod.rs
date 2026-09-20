#![deny(clippy::unwrap_used, clippy::expect_used)]

pub mod analysis;
pub mod classifier;
pub mod disk_space;
pub mod events;
pub mod guided;
pub mod platform;
pub mod potential;
pub mod power;
pub mod rules;
pub mod thermal;
pub mod topology;
pub mod windows;

pub use classifier::{Classification, DiagnosticResult, Severity};
pub use rules::{CoverageTier, Ruleset};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiagnosticSample {
    pub monotonic_ms: u64,
    pub load_percent: f64,
    pub active_clock_mhz: Option<f64>,
    pub base_clock_mhz: Option<f64>,
    pub temperature_c: Option<f64>,
    pub thermal_limit_c: Option<f64>,
    pub package_power_w: Option<f64>,
    pub power_limit_w: Option<f64>,
    pub thermal_flag: bool,
    pub prochot_flag: bool,
    pub power_flag: bool,
    pub current_flag: bool,
    pub in_turbo_window: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerContext {
    Ac,
    Battery,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoverageSignals {
    pub temperature: bool,
    pub active_clock: bool,
    pub per_core_load: bool,
    pub package_power: bool,
    pub power_limit: bool,
    pub limit_reasons: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfidenceCeiling {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoverageSummary {
    pub tier: CoverageTier,
    pub confidence_ceiling: ConfidenceCeiling,
    pub advanced_access: AdvancedAccess,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvancedAccess {
    NotNeeded,
    Available,
    Installable,
    Denied,
    Error,
}

impl CoverageSignals {
    pub const fn tier(self) -> CoverageTier {
        if self.temperature
            && self.active_clock
            && self.per_core_load
            && self.package_power
            && self.power_limit
            && self.limit_reasons
        {
            CoverageTier::A
        } else if self.temperature && self.active_clock && self.per_core_load && self.package_power
        {
            CoverageTier::B
        } else {
            CoverageTier::C
        }
    }

    pub const fn summary(self) -> CoverageSummary {
        let tier = self.tier();
        CoverageSummary {
            tier,
            confidence_ceiling: match tier {
                CoverageTier::A => ConfidenceCeiling::High,
                CoverageTier::B => ConfidenceCeiling::Medium,
                CoverageTier::C => ConfidenceCeiling::Low,
            },
            advanced_access: match tier {
                CoverageTier::A => AdvancedAccess::NotNeeded,
                CoverageTier::B | CoverageTier::C => AdvancedAccess::Installable,
            },
        }
    }
}

pub fn diagnose(
    samples: &[DiagnosticSample],
    coverage: CoverageSignals,
    ruleset: &Ruleset,
) -> DiagnosticResult {
    classifier::diagnose(samples, coverage, ruleset)
}

#[cfg(test)]
mod tests {
    use super::{CoverageSignals, CoverageTier};

    #[test]
    fn maps_signal_sets_to_coverage_tiers() {
        let complete = CoverageSignals {
            temperature: true,
            active_clock: true,
            per_core_load: true,
            package_power: true,
            power_limit: true,
            limit_reasons: true,
        };
        assert_eq!(complete.tier(), CoverageTier::A);
        assert_eq!(
            CoverageSignals { power_limit: false, limit_reasons: false, ..complete }.tier(),
            CoverageTier::B
        );
        assert_eq!(CoverageSignals { package_power: false, ..complete }.tier(), CoverageTier::C);
        assert!(!matches!(
            CoverageSignals { package_power: false, ..complete }.summary().advanced_access,
            super::AdvancedAccess::NotNeeded
        ));
    }
}
