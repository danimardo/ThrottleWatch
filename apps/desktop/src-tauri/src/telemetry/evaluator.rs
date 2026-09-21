//! Feeds the diagnostic engine from the live samples: keeps the recent window, marks the turbo
//! exclusion window after a load start (FR-077) and re-classifies on every sample. Pure and
//! synchronous, so a trace replays to the same result (NFR-009).
#![deny(clippy::unwrap_used, clippy::expect_used)]

use crate::diagnostics::windows::{sustained_load_started, turbo_exclusion_ms};
use crate::diagnostics::{CoverageSignals, DiagnosticResult, DiagnosticSample, Ruleset, diagnose};
use std::collections::VecDeque;

/// The buffer keeps this many stable windows of history: the classifier judges the latest window
/// and the power-limit trend needs a longer tail.
const HORIZON_WINDOWS: u64 = 3;

#[derive(Debug, Clone)]
pub struct LiveEvaluator {
    rules: Ruleset,
    samples: VecDeque<DiagnosticSample>,
    turbo_until_ms: Option<u64>,
    previous_load: Option<f64>,
    latest: Option<DiagnosticResult>,
}

impl LiveEvaluator {
    pub fn new(rules: Ruleset) -> Self {
        Self {
            rules,
            samples: VecDeque::new(),
            turbo_until_ms: None,
            previous_load: None,
            latest: None,
        }
    }

    /// The result of the last evaluation; `None` until the first sample arrives.
    pub fn latest(&self) -> Option<&DiagnosticResult> {
        self.latest.as_ref()
    }

    /// Forgets the history (a new catalog or a collector restart breaks the continuity of the trace).
    pub fn reset(&mut self) {
        self.samples.clear();
        self.turbo_until_ms = None;
        self.previous_load = None;
        self.latest = None;
    }

    /// Adds one sample (when the collector delivered a usable one) and classifies the recent window
    /// with the coverage in force right now (FR-090).
    pub fn observe(&mut self, sample: Option<DiagnosticSample>, coverage: CoverageSignals) {
        if let Some(mut sample) = sample {
            // Without the moment the load started (the app opened on a loaded machine) the turbo
            // window is assumed to start now: better an unproven verdict than a false accusation.
            let started = self.previous_load.is_none_or(|previous| {
                sustained_load_started(previous, sample.load_percent, &self.rules)
            });
            if started && sample.load_percent >= self.rules.active_load_percent {
                self.turbo_until_ms =
                    Some(sample.monotonic_ms.saturating_add(turbo_exclusion_ms(None, &self.rules)));
            }
            self.previous_load = Some(sample.load_percent);
            sample.in_turbo_window =
                self.turbo_until_ms.is_some_and(|until| sample.monotonic_ms < until);
            let horizon_ms =
                self.rules.stable_window_s.saturating_mul(1000).saturating_mul(HORIZON_WINDOWS);
            while self.samples.front().is_some_and(|oldest| {
                sample.monotonic_ms.saturating_sub(oldest.monotonic_ms) > horizon_ms
            }) {
                self.samples.pop_front();
            }
            self.samples.push_back(sample);
        }
        if self.samples.is_empty() {
            self.latest = None;
            return;
        }
        let window: Vec<DiagnosticSample> = self.samples.iter().copied().collect();
        self.latest = Some(diagnose(&window, coverage, &self.rules));
    }
}

#[cfg(test)]
mod tests {
    use super::LiveEvaluator;
    use crate::diagnostics::{Classification, CoverageSignals, DiagnosticSample, Ruleset};

    fn evaluator() -> LiveEvaluator {
        LiveEvaluator::new(Ruleset::v1().unwrap_or_else(|error| panic!("ruleset: {error}")))
    }

    fn full() -> CoverageSignals {
        CoverageSignals {
            temperature: true,
            active_clock: true,
            per_core_load: true,
            package_power: true,
            power_limit: true,
            limit_reasons: true,
        }
    }

    fn loaded(second: u64, thermal: bool) -> DiagnosticSample {
        DiagnosticSample {
            monotonic_ms: second * 1000,
            load_percent: 98.0,
            active_clock_mhz: Some(3400.0),
            base_clock_mhz: Some(3600.0),
            temperature_c: Some(94.0),
            thermal_limit_c: Some(95.0),
            package_power_w: Some(60.0),
            power_limit_w: Some(90.0),
            thermal_flag: thermal,
            prochot_flag: false,
            power_flag: false,
            current_flag: false,
            in_turbo_window: false,
        }
    }

    #[test]
    fn nothing_is_classified_before_the_first_sample() {
        let mut evaluator = evaluator();
        evaluator.observe(None, full());
        assert!(evaluator.latest().is_none());
    }

    #[test]
    fn a_sustained_thermal_limit_is_confirmed_only_after_the_turbo_window() {
        let mut evaluator = evaluator();
        for second in 0..=200 {
            evaluator.observe(Some(loaded(second, true)), full());
            if second == 40 {
                assert_ne!(
                    evaluator.latest().map(|result| result.classification),
                    Some(Classification::ThermalConfirmed),
                    "the first minute of a load is the turbo window and never accuses the cooling"
                );
            }
        }
        assert_eq!(
            evaluator.latest().map(|result| result.classification),
            Some(Classification::ThermalConfirmed)
        );
    }

    #[test]
    fn without_limit_reasons_the_same_trace_is_never_confirmed() {
        let mut evaluator = evaluator();
        for second in 0..=200 {
            evaluator.observe(Some(loaded(second, false)), full());
        }
        assert_ne!(
            evaluator.latest().map(|result| result.classification),
            Some(Classification::ThermalConfirmed)
        );
    }

    #[test]
    fn a_missing_sample_does_not_erase_the_history_but_a_reset_does() {
        let mut evaluator = evaluator();
        evaluator.observe(Some(loaded(0, false)), full());
        evaluator.observe(None, full());
        assert!(evaluator.latest().is_some());
        evaluator.reset();
        assert!(evaluator.latest().is_none());
    }

    #[test]
    fn coverage_degrading_mid_trace_stops_tier_a_only_figures_from_that_instant() {
        // FR-090: "el motor DEBE evaluar cada ventana con el nivel vigente en ella (las cifras
        // que exigen nivel A se detienen desde ese instante)" — a provider failing mid-session
        // (service stopped, read denied, provider exception) must not keep confirming a
        // tier-A-only verdict once the coverage passed to `observe` reflects the loss.
        let mut evaluator = evaluator();
        for second in 0..=150 {
            evaluator.observe(Some(loaded(second, true)), full());
        }
        assert_eq!(
            evaluator.latest().map(|result| result.classification),
            Some(Classification::ThermalConfirmed),
            "sanity check: full coverage past the turbo window confirms the limit"
        );

        let mut degraded = full();
        degraded.limit_reasons = false;
        evaluator.observe(Some(loaded(151, true)), degraded);

        assert_ne!(
            evaluator.latest().map(|result| result.classification),
            Some(Classification::ThermalConfirmed),
            "the very next window, evaluated with the degraded coverage in force, must not \
             confirm a tier-A-only verdict just because the rolling buffer still holds \
             full-coverage samples from before the failure"
        );
    }

    #[test]
    fn the_buffer_keeps_a_bounded_history() {
        let mut evaluator = evaluator();
        for second in 0..2_000 {
            evaluator.observe(Some(loaded(second, false)), full());
        }
        assert!(evaluator.samples.len() <= 181, "kept {}", evaluator.samples.len());
    }
}
