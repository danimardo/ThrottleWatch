use super::{CoverageTier, classifier::Classification, rules::Ruleset};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PowerHeadroomInput {
    pub package_power_w: f64,
    pub power_limit_w: f64,
    pub active_clock_mhz: f64,
    pub base_clock_mhz: f64,
    pub turbo_clock_mhz: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PotentialBand {
    pub lower_percent: u8,
    pub upper_percent: u8,
    pub method: &'static str,
    pub confidence: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PotentialResult {
    Quantified(PotentialBand),
    Qualitative { label: &'static str, confidence: f64 },
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct CoolingPotentialDto {
    pub low_percent: u8,
    pub high_percent: u8,
    pub method: &'static str,
}

pub fn serializable_potential(result: PotentialResult) -> Option<CoolingPotentialDto> {
    match result {
        PotentialResult::Quantified(band) if band.method == "power_headroom" => {
            Some(CoolingPotentialDto {
                low_percent: band.lower_percent,
                high_percent: band.upper_percent,
                method: band.method,
            })
        }
        PotentialResult::Quantified(_)
        | PotentialResult::Qualitative { .. }
        | PotentialResult::Unavailable => None,
    }
}

pub fn power_headroom(
    input: PowerHeadroomInput,
    classification: Classification,
    coverage: CoverageTier,
    in_turbo_window: bool,
    rules: &Ruleset,
) -> PotentialResult {
    if coverage != CoverageTier::A
        || in_turbo_window
        || !matches!(
            classification,
            Classification::ThermalConfirmed
                | Classification::ThermalProbable
                | Classification::PlatformLimited
                | Classification::MixedLimit
        )
        || !valid_input(input)
    {
        return if coverage != CoverageTier::A
            && input.active_clock_mhz.is_finite()
            && input.base_clock_mhz.is_finite()
            && input.active_clock_mhz < input.base_clock_mhz
            && !in_turbo_window
        {
            PotentialResult::Qualitative { label: "probably_notable", confidence: 0.45 }
        } else {
            PotentialResult::Unavailable
        };
    }

    let power_gain = (input.power_limit_w / input.package_power_w).cbrt() - 1.0;
    let turbo_cap = input.turbo_clock_mhz / input.active_clock_mhz - 1.0;
    let gain = power_gain.max(0.0).min(turbo_cap.max(0.0));
    let low_factor = rules.parameter("potential.range_low_factor").unwrap_or(0.5);
    let rounded = round_outward(
        (gain * 100.0 * low_factor).clamp(0.0, 100.0),
        (gain * 100.0).clamp(0.0, 100.0),
    );
    PotentialResult::Quantified(PotentialBand {
        lower_percent: rounded.0,
        upper_percent: rounded.1,
        method: "power_headroom",
        confidence: 0.75,
    })
}

fn valid_input(input: PowerHeadroomInput) -> bool {
    [
        input.package_power_w,
        input.power_limit_w,
        input.active_clock_mhz,
        input.base_clock_mhz,
        input.turbo_clock_mhz,
    ]
    .iter()
    .all(|value| value.is_finite() && *value > 0.0)
        && input.power_limit_w >= input.package_power_w
        && input.turbo_clock_mhz >= input.base_clock_mhz
}

fn round_outward(lower_value: f64, upper_value: f64) -> (u8, u8) {
    let lower = (lower_value / 5.0).floor() * 5.0;
    let upper = (upper_value / 5.0).ceil() * 5.0;
    (lower.clamp(0.0, 100.0) as u8, upper.clamp(0.0, 100.0) as u8)
}

#[cfg(test)]
mod tests {
    use super::{PotentialResult, PowerHeadroomInput, power_headroom, serializable_potential};
    use crate::diagnostics::{CoverageTier, classifier::Classification, rules::Ruleset};

    fn input() -> PowerHeadroomInput {
        PowerHeadroomInput {
            package_power_w: 70.0,
            power_limit_w: 100.0,
            active_clock_mhz: 3_500.0,
            base_clock_mhz: 3_000.0,
            turbo_clock_mhz: 4_000.0,
        }
    }

    #[test]
    fn quantifies_only_level_a_outside_turbo() -> serde_json::Result<()> {
        let rules = Ruleset::v1()?;
        let result = power_headroom(
            input(),
            Classification::ThermalConfirmed,
            CoverageTier::A,
            false,
            &rules,
        );
        assert!(matches!(result, PotentialResult::Quantified(_)));
        assert_eq!(
            power_headroom(
                input(),
                Classification::ThermalConfirmed,
                CoverageTier::A,
                true,
                &rules
            ),
            PotentialResult::Unavailable
        );
        Ok(())
    }

    #[test]
    fn gives_only_a_qualitative_low_confidence_result_without_level_a_inputs()
    -> serde_json::Result<()> {
        let rules = Ruleset::v1()?;
        let below_base = PowerHeadroomInput { active_clock_mhz: 2_800.0, ..input() };
        assert!(matches!(
            power_headroom(
                below_base,
                Classification::ThermalProbable,
                CoverageTier::B,
                false,
                &rules
            ),
            PotentialResult::Qualitative { .. }
        ));
        Ok(())
    }

    #[test]
    fn never_serializes_a_band_without_the_declared_method() -> serde_json::Result<()> {
        let rules = Ruleset::v1()?;
        let result = power_headroom(
            input(),
            Classification::ThermalConfirmed,
            CoverageTier::A,
            false,
            &rules,
        );
        assert!(serializable_potential(result).is_some());
        assert!(serializable_potential(PotentialResult::Unavailable).is_none());
        Ok(())
    }

    #[test]
    fn applies_turbo_and_power_negative_cases_and_outward_rounding() -> serde_json::Result<()> {
        let rules = Ruleset::v1()?;
        let band = power_headroom(
            input(),
            Classification::ThermalConfirmed,
            CoverageTier::A,
            false,
            &rules,
        );
        if let PotentialResult::Quantified(value) = band {
            assert_eq!(value.lower_percent % 5, 0);
            assert_eq!(value.upper_percent % 5, 0);
            assert!(value.lower_percent <= value.upper_percent);
        } else {
            panic!("expected quantified potential");
        }
        assert_eq!(
            power_headroom(input(), Classification::PowerLimited, CoverageTier::A, false, &rules),
            PotentialResult::Unavailable
        );
        assert_eq!(
            power_headroom(
                input(),
                Classification::ThermalConfirmed,
                CoverageTier::A,
                true,
                &rules
            ),
            PotentialResult::Unavailable
        );
        Ok(())
    }

    #[test]
    fn outward_rounding_properties_hold_for_a_matrix_of_valid_inputs() -> serde_json::Result<()> {
        let rules = Ruleset::v1()?;
        let low_factor = rules.parameter("potential.range_low_factor").unwrap_or(0.5);
        for package_power_w in [35.0, 70.0, 105.0] {
            for power_limit_w in [package_power_w, package_power_w * 1.1, package_power_w * 1.8] {
                for active_clock_mhz in [2_000.0, 3_000.0, 3_500.0, 4_000.0] {
                    let input = PowerHeadroomInput {
                        package_power_w,
                        power_limit_w,
                        active_clock_mhz,
                        base_clock_mhz: active_clock_mhz * 0.8,
                        turbo_clock_mhz: active_clock_mhz * 1.25,
                    };
                    let result = power_headroom(
                        input,
                        Classification::ThermalConfirmed,
                        CoverageTier::A,
                        false,
                        &rules,
                    );
                    let PotentialResult::Quantified(band) = result else {
                        panic!("valid level-A input must produce a quantified band");
                    };
                    let power_gain = (power_limit_w / package_power_w).cbrt() - 1.0;
                    let turbo_cap = input.turbo_clock_mhz / active_clock_mhz - 1.0;
                    let gain = power_gain.max(0.0).min(turbo_cap.max(0.0));
                    let raw_low = (gain * 100.0 * low_factor).clamp(0.0, 100.0);
                    let raw_high = (gain * 100.0).clamp(0.0, 100.0);
                    assert!(f64::from(band.lower_percent) <= raw_low + f64::EPSILON);
                    assert!(f64::from(band.upper_percent) + f64::EPSILON >= raw_high);
                    assert_eq!(band.lower_percent % 5, 0);
                    assert_eq!(band.upper_percent % 5, 0);
                    assert!(band.lower_percent <= band.upper_percent);
                }
            }
        }
        Ok(())
    }
}
