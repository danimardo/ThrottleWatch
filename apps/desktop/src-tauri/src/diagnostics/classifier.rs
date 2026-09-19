use super::{
    CoverageSignals, DiagnosticSample, platform, power,
    rules::{CoverageTier, Ruleset},
    thermal,
    windows::{StableWindow, stable_windows},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Classification {
    Normal,
    HotUnproven,
    ThermalProbable,
    ThermalConfirmed,
    PowerLimited,
    PlatformLimited,
    MixedLimit,
    Indeterminate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Boost,
    BelowBase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformSubtype {
    ChassisThermal,
    ExternalProchot,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiagnosticResult {
    pub classification: Classification,
    pub severity: Option<Severity>,
    pub coverage: CoverageTier,
    pub confidence: f64,
    pub platform: Option<PlatformSubtype>,
    pub evidence: Vec<&'static str>,
    pub alternative_causes: Vec<&'static str>,
    pub analyzed_from_ms: Option<u64>,
    pub analyzed_to_ms: Option<u64>,
    pub windows: usize,
}

pub fn diagnose(
    samples: &[DiagnosticSample],
    coverage: CoverageSignals,
    rules: &Ruleset,
) -> DiagnosticResult {
    let tier = coverage.tier();
    let windows = stable_windows(samples, rules);
    let Some(window) = windows.last().copied() else {
        return result(
            Classification::Indeterminate,
            None,
            tier,
            0.0,
            None,
            vec!["missing_stable_window"],
            vec![],
            None,
            None,
            0,
        );
    };
    if !coverage.temperature || !coverage.active_clock {
        return result(
            Classification::Indeterminate,
            None,
            tier,
            0.2,
            None,
            vec!["missing_temperature_or_clock"],
            vec![],
            Some(window.start_ms),
            Some(window.end_ms),
            windows.len(),
        );
    }
    if !window.sustained_load || window.in_turbo_window {
        let class = if thermal::temperature_is_high(&window, rules) {
            Classification::HotUnproven
        } else {
            Classification::Normal
        };
        let evidence =
            if window.in_turbo_window { vec!["turbo_window"] } else { vec!["no_sustained_load"] };
        return result(
            class,
            None,
            tier,
            confidence(tier, 0.55),
            None,
            evidence,
            vec![],
            Some(window.start_ms),
            Some(window.end_ms),
            windows.len(),
        );
    }

    let thermal_reason = window.thermal_occupancy >= rules.reason_occupancy;
    let power_reason = power::has_power_reason(&window, rules);
    let thermal_plateau = thermal::thermal_plateau(&window, rules);
    let power_plateau = power::power_plateau(&window, rules);

    let limit_values: Vec<f64> = samples.iter().filter_map(|sample| sample.power_limit_w).collect();
    let limit_trend = platform::limit_trend(&limit_values, rules);
    let (classification, subtype, evidence, alternatives) =
        if tier == CoverageTier::A && thermal_reason && power_reason {
            (Classification::MixedLimit, None, vec!["thermal_reason", "power_reason"], vec![])
        } else if tier == CoverageTier::A && thermal_reason {
            (Classification::ThermalConfirmed, None, vec!["thermal_reason"], vec![])
        } else if tier == CoverageTier::A && platform::external_prochot(&window, rules) {
            (
                Classification::PlatformLimited,
                Some(PlatformSubtype::ExternalProchot),
                vec!["external_prochot"],
                vec!["check_power_supply"],
            )
        } else if matches!(limit_trend, platform::LimitTrend::Progressive)
            && window.thermal_margin_c.is_some_and(|margin| margin > rules.thermal_high_margin_c)
        {
            (
                Classification::PlatformLimited,
                Some(PlatformSubtype::ChassisThermal),
                vec!["progressive_power_limit_drop"],
                vec!["improve_chassis_cooling"],
            )
        } else if matches!(limit_trend, platform::LimitTrend::SingleStep)
            && window.thermal_margin_c.is_some_and(|margin| margin > rules.thermal_high_margin_c)
        {
            (
                Classification::Indeterminate,
                None,
                vec!["single_power_limit_step"],
                vec!["oem_mode_change"],
            )
        } else if thermal_plateau && tier != CoverageTier::C {
            (
                Classification::ThermalProbable,
                None,
                vec!["thermal_plateau"],
                vec!["power_reason_not_measured"],
            )
        } else if tier == CoverageTier::C
            && thermal_plateau
            && window.active_clock_mhz < window.base_clock_mhz
        {
            (
                Classification::ThermalProbable,
                None,
                vec!["thermal_plateau_below_base"],
                vec!["power_reason_not_measured"],
            )
        } else if power_reason || power_plateau {
            (
                Classification::PowerLimited,
                None,
                vec![if power_reason { "power_reason" } else { "power_plateau" }],
                vec!["cooling_has_limited_effect"],
            )
        } else {
            let energy_policy = window.active_clock_mhz
                < window.base_clock_mhz
                    * (1.0
                        - rules.parameter("rules.freq_drop_vs_turbo_pct").unwrap_or(8.0) / 100.0);
            (
                Classification::Normal,
                None,
                vec!["no_limiting_evidence"],
                if energy_policy {
                    vec!["energy_policy", "eco_qos_or_epp", "power_plan"]
                } else {
                    vec![]
                },
            )
        };
    let final_severity = matches!(
        classification,
        Classification::ThermalConfirmed
            | Classification::ThermalProbable
            | Classification::PowerLimited
            | Classification::PlatformLimited
            | Classification::MixedLimit
    )
    .then(|| severity(&window, rules));
    result(
        classification,
        final_severity.flatten(),
        tier,
        confidence(tier, evidence_score(&window, classification, rules)),
        subtype,
        evidence,
        alternatives,
        Some(window.start_ms),
        Some(window.end_ms),
        windows.len(),
    )
}

fn severity(window: &StableWindow, rules: &Ruleset) -> Option<Severity> {
    let ratio = window.active_clock_mhz / window.base_clock_mhz;
    let duration_s = window.end_ms.saturating_sub(window.start_ms) as f64 / 1000.0;
    if ratio.is_finite()
        && ratio < rules.below_base_ratio
        && duration_s >= rules.below_base_duration_s
    {
        Some(Severity::BelowBase)
    } else {
        Some(Severity::Boost)
    }
}

fn evidence_score(window: &StableWindow, class: Classification, rules: &Ruleset) -> f64 {
    let duration_score = (window.sample_count as f64 / (rules.stable_window_s as f64)).min(1.0);
    let class_score = match class {
        Classification::Normal => 0.6,
        Classification::ThermalConfirmed | Classification::MixedLimit => 0.95,
        Classification::ThermalProbable
        | Classification::PowerLimited
        | Classification::PlatformLimited => 0.78,
        Classification::HotUnproven => 0.55,
        Classification::Indeterminate => 0.2,
    };
    (duration_score * 0.3 + class_score * 0.7).min(1.0)
}

fn confidence(tier: CoverageTier, score: f64) -> f64 {
    let ceiling = match tier {
        CoverageTier::A => 1.0,
        CoverageTier::B => 0.75,
        CoverageTier::C => 0.45,
    };
    score.min(ceiling)
}

#[allow(clippy::too_many_arguments)]
fn result(
    classification: Classification,
    severity: Option<Severity>,
    coverage: CoverageTier,
    confidence: f64,
    platform: Option<PlatformSubtype>,
    evidence: Vec<&'static str>,
    alternative_causes: Vec<&'static str>,
    analyzed_from_ms: Option<u64>,
    analyzed_to_ms: Option<u64>,
    windows: usize,
) -> DiagnosticResult {
    DiagnosticResult {
        classification,
        severity,
        coverage,
        confidence,
        platform,
        evidence,
        alternative_causes,
        analyzed_from_ms,
        analyzed_to_ms,
        windows,
    }
}

#[cfg(test)]
mod tests {
    use super::{Classification, Severity, diagnose};
    use crate::diagnostics::{CoverageSignals, DiagnosticSample, rules::Ruleset};

    fn sample(index: u64, thermal: bool, power: bool) -> DiagnosticSample {
        DiagnosticSample {
            monotonic_ms: index * 1000,
            load_percent: 95.0,
            active_clock_mhz: Some(3300.0),
            base_clock_mhz: Some(3500.0),
            temperature_c: Some(92.0),
            thermal_limit_c: Some(95.0),
            package_power_w: Some(60.0),
            power_limit_w: Some(80.0),
            thermal_flag: thermal,
            prochot_flag: false,
            power_flag: power,
            current_flag: false,
            in_turbo_window: false,
        }
    }

    fn coverage() -> CoverageSignals {
        CoverageSignals {
            temperature: true,
            active_clock: true,
            per_core_load: true,
            package_power: true,
            power_limit: true,
            limit_reasons: true,
        }
    }

    #[test]
    fn confirms_thermal_reason_at_level_a() -> serde_json::Result<()> {
        let rules = Ruleset::v1()?;
        let samples: Vec<_> = (0..=60).map(|index| sample(index, true, false)).collect();
        let result = diagnose(&samples, coverage(), &rules);
        assert_eq!(result.classification, Classification::ThermalConfirmed);
        assert_eq!(result.severity, Some(Severity::BelowBase));
        Ok(())
    }

    #[test]
    fn never_emits_mixed_at_level_b() -> serde_json::Result<()> {
        let rules = Ruleset::v1()?;
        let samples: Vec<_> = (0..=60).map(|index| sample(index, false, true)).collect();
        let level_b = CoverageSignals { power_limit: false, limit_reasons: false, ..coverage() };
        let result = diagnose(&samples, level_b, &rules);
        assert_ne!(result.classification, Classification::MixedLimit);
        Ok(())
    }

    #[test]
    fn reports_external_prochot_without_calling_it_cpu_thermal() -> serde_json::Result<()> {
        let rules = Ruleset::v1()?;
        let samples: Vec<_> = (0..=60)
            .map(|index| DiagnosticSample {
                thermal_flag: false,
                prochot_flag: true,
                ..sample(index, false, false)
            })
            .collect();
        let result = diagnose(&samples, coverage(), &rules);
        assert_eq!(result.classification, Classification::PlatformLimited);
        assert_eq!(result.platform, Some(super::PlatformSubtype::ExternalProchot));
        Ok(())
    }

    #[test]
    fn remains_deterministic_for_the_same_trace() -> serde_json::Result<()> {
        let rules = Ruleset::v1()?;
        let samples: Vec<_> = (0..=60).map(|index| sample(index, true, false)).collect();
        assert_eq!(diagnose(&samples, coverage(), &rules), diagnose(&samples, coverage(), &rules));
        Ok(())
    }

    #[test]
    fn uses_plateaus_only_for_probable_classes_and_keeps_power_separate() -> serde_json::Result<()>
    {
        let rules = Ruleset::v1()?;
        let thermal_samples: Vec<_> = (0..=60)
            .map(|index| DiagnosticSample { thermal_flag: false, ..sample(index, false, false) })
            .collect();
        let level_b = CoverageSignals { power_limit: false, limit_reasons: false, ..coverage() };
        let thermal = diagnose(&thermal_samples, level_b, &rules);
        assert_eq!(thermal.classification, Classification::ThermalProbable);

        let power_samples: Vec<_> = (0..=60)
            .map(|index| DiagnosticSample {
                temperature_c: Some(65.0),
                thermal_limit_c: Some(95.0),
                power_flag: false,
                ..sample(index, false, false)
            })
            .collect();
        let power = diagnose(&power_samples, level_b, &rules);
        assert_eq!(power.classification, Classification::PowerLimited);
        Ok(())
    }
}
