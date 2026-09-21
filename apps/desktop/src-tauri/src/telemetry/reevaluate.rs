//! Rebuilds a session's verdict from its stored samples with the ruleset in force right now
//! (FR-073, `reevaluate_report`): only ever asked of `imported` sessions, and only shown next to
//! the original, never over it — nothing here writes to storage.
//!
//! It replays the same pieces a live session uses (`LiveEvaluator` classifies each window,
//! [`Timeline`] fuses them into limitations and a session verdict), fed from
//! [`StoredFrame`](crate::storage::StoredFrame) instead of the collector. Two things a live
//! session has and a replay cannot: the effective thermal limit (only ever known live, never
//! stored per frame, so thermal margin and B/C plateau reasoning are unavailable here) and which
//! magnitudes the catalog *advertised* (a replay only knows what a frame actually carried, which
//! is coverage from the readings themselves — a fair substitute, not the same thing).
#![deny(clippy::unwrap_used, clippy::expect_used)]

use super::catalog::known_sensor;
use super::evaluator::LiveEvaluator;
use super::timeline::{LimitSpan, Timeline};
use crate::diagnostics::events::SessionClassification;
use crate::diagnostics::rules::Ruleset;
use crate::diagnostics::{CoverageSignals, DiagnosticSample};
use crate::storage::StoredFrame;

#[derive(Debug, Clone, PartialEq)]
pub struct ReplayResult {
    /// `None` when nothing in the session could be judged at all (e.g. no frame ever carried a
    /// package load reading) — the caller must say so, never report it as `normal`.
    pub verdict: Option<SessionClassification>,
    pub events: Vec<LimitSpan>,
}

/// Replays every frame of a session through a fresh engine, in order, and returns the finished
/// session verdict and the limitations that lasted (`session.class_min_s`, same threshold a live
/// session uses).
pub fn reevaluate_frames(frames: &[StoredFrame], rules: &Ruleset) -> ReplayResult {
    let coverage = coverage_signals(frames);
    let mut evaluator = LiveEvaluator::new(rules.clone());
    let mut timeline = Timeline::default();
    for frame in frames {
        let sample = diagnostic_sample(frame);
        evaluator.observe(sample, coverage);
        let at_ms = u64::try_from(frame.monotonic_ms).unwrap_or(0);
        timeline.push(at_ms, frame.sequence, evaluator.latest());
    }
    let min_ms =
        rules.parameter("session.class_min_s").map_or(60_000, |seconds| (seconds * 1_000.0) as u64);
    ReplayResult { verdict: timeline.classify(rules), events: timeline.drain_all(min_ms) }
}

/// What the whole session ever carried, metric by metric — the best a replay can say about
/// coverage without the original catalog (which was never stored). A magnitude present in even
/// one frame counts as covered for all of it, same as a live catalog advertising a sensor whether
/// or not every sample fills it in.
fn coverage_signals(frames: &[StoredFrame]) -> CoverageSignals {
    let mut signals = CoverageSignals {
        temperature: false,
        active_clock: false,
        per_core_load: false,
        package_power: false,
        power_limit: false,
        limit_reasons: false,
    };
    for (metric, scope) in frames
        .iter()
        .flat_map(|frame| frame.values.iter())
        .filter(|(_, value, boolean)| value.is_some() || boolean.is_some())
        .filter_map(|(sensor_id, _, _)| known_sensor(sensor_id))
    {
        match (metric, scope) {
            ("temperature", "package") => signals.temperature = true,
            ("active_clock", _) => signals.active_clock = true,
            ("load", "core") => signals.per_core_load = true,
            ("power", "package") => signals.package_power = true,
            ("power_limit", "package") => signals.power_limit = true,
            ("thermal_flag" | "prochot_flag" | "power_flag" | "current_flag", _) => {
                signals.limit_reasons = true;
            }
            _ => {}
        }
    }
    signals
}

/// One frame's readings folded into a [`DiagnosticSample`]; `None` without a package load, the
/// one field the engine cannot do without. The thermal limit is always unknown here (see the
/// module note); `in_turbo_window` is [`LiveEvaluator`]'s to set, not this frame's.
fn diagnostic_sample(frame: &StoredFrame) -> Option<DiagnosticSample> {
    let mut load = None;
    let mut temperature = None;
    let mut power = None;
    let mut power_limit = None;
    let mut active_clock = None;
    let mut base_clock = None;
    let (mut thermal_flag, mut prochot_flag, mut power_flag, mut current_flag) =
        (false, false, false, false);
    for (sensor_id, value, boolean) in &frame.values {
        let Some((metric, scope)) = known_sensor(sensor_id) else { continue };
        match (metric, scope) {
            ("load", "package") => load = *value,
            ("temperature", "package") => temperature = *value,
            ("power", "package") => power = *value,
            ("power_limit", "package") => power_limit = *value,
            ("active_clock", _) if active_clock.is_none() => active_clock = *value,
            ("base_clock", _) if base_clock.is_none() => base_clock = *value,
            ("thermal_flag", _) => thermal_flag = boolean.unwrap_or(false),
            ("prochot_flag", _) => prochot_flag = boolean.unwrap_or(false),
            ("power_flag", _) => power_flag = boolean.unwrap_or(false),
            ("current_flag", _) => current_flag = boolean.unwrap_or(false),
            _ => {}
        }
    }
    Some(DiagnosticSample {
        monotonic_ms: u64::try_from(frame.monotonic_ms).unwrap_or(0),
        load_percent: load?,
        active_clock_mhz: active_clock,
        base_clock_mhz: base_clock,
        temperature_c: temperature,
        thermal_limit_c: None,
        package_power_w: power,
        power_limit_w: power_limit,
        thermal_flag,
        prochot_flag,
        power_flag,
        current_flag,
        in_turbo_window: false,
    })
}

#[cfg(test)]
mod tests {
    use super::reevaluate_frames;
    use crate::diagnostics::Classification;
    use crate::diagnostics::rules::Ruleset;
    use crate::storage::StoredFrame;

    fn rules() -> Ruleset {
        Ruleset::v1().unwrap_or_else(|error| panic!("ruleset: {error}"))
    }

    fn frame(sequence: i64, second: i64, thermal: bool) -> StoredFrame {
        StoredFrame {
            sequence,
            monotonic_ms: second * 1_000,
            values: vec![
                ("cpu.package.load".to_owned(), Some(98.0), None),
                ("cpu.package.temp".to_owned(), Some(94.0), None),
                ("cpu.package.clock".to_owned(), Some(3_400.0), None),
                ("host.base_clock".to_owned(), Some(3_600.0), None),
                ("cpu.package.power".to_owned(), Some(60.0), None),
                ("cpu.package.power_limit".to_owned(), Some(90.0), None),
                ("cpu.core.1.load".to_owned(), Some(98.0), None),
                ("msr/thermal_flag".to_owned(), None, thermal.then_some(true)),
            ],
        }
    }

    #[test]
    fn an_empty_session_has_no_verdict_at_all() {
        let result = reevaluate_frames(&[], &rules());
        assert!(result.verdict.is_none());
        assert!(result.events.is_empty());
    }

    #[test]
    fn a_frame_without_a_load_reading_cannot_be_judged_and_is_skipped() {
        let frame = StoredFrame {
            sequence: 0,
            monotonic_ms: 0,
            values: vec![("cpu.package.temp".to_owned(), Some(80.0), None)],
        };
        let result = reevaluate_frames(std::slice::from_ref(&frame), &rules());
        assert!(result.verdict.is_none());
    }

    #[test]
    fn a_sustained_thermal_session_replays_to_the_same_verdict_a_live_one_would_reach() {
        let frames: Vec<_> = (0..200_i64).map(|second| frame(second, second, true)).collect();
        let result = reevaluate_frames(&frames, &rules());
        let verdict = result.verdict.unwrap_or_else(|| panic!("a verdict is expected"));
        assert_eq!(verdict.classification, Classification::ThermalConfirmed);
        assert_eq!(result.events.len(), 1);
        assert_eq!(result.events[0].kind.storage_key(), "thermal");
    }

    #[test]
    fn without_limit_reason_flags_the_same_trace_is_never_confirmed() {
        let frames: Vec<_> = (0..200_i64).map(|second| frame(second, second, false)).collect();
        let result = reevaluate_frames(&frames, &rules());
        let verdict = result.verdict.unwrap_or_else(|| panic!("a verdict is expected"));
        assert_ne!(verdict.classification, Classification::ThermalConfirmed);
    }

    #[test]
    fn without_a_clock_reading_at_all_no_window_can_form_and_nothing_is_fabricated() {
        // The classifier needs a clock to build a stable window at all (`windows.rs`); without
        // one, a reevaluation must say "no verdict", never invent `indeterminate` or `normal`.
        let frames: Vec<_> = (0..5_i64)
            .map(|second| StoredFrame {
                sequence: second,
                monotonic_ms: second * 1_000,
                values: vec![
                    ("cpu.package.load".to_owned(), Some(98.0), None),
                    ("cpu.package.temp".to_owned(), Some(94.0), None),
                ],
            })
            .collect();
        let result = reevaluate_frames(&frames, &rules());
        assert!(result.verdict.is_none());
    }

    #[test]
    fn a_session_with_only_the_minimum_fields_is_still_judged_but_never_confirmed() {
        // Load, temperature and clock (no power/power-limit/per-core coverage, no flags): a
        // window can form, but the missing tier-A signals must keep it out of `thermal_confirmed`.
        let frames: Vec<_> = (0..200_i64)
            .map(|second| StoredFrame {
                sequence: second,
                monotonic_ms: second * 1_000,
                values: vec![
                    ("cpu.package.load".to_owned(), Some(98.0), None),
                    ("cpu.package.temp".to_owned(), Some(94.0), None),
                    ("cpu.package.clock".to_owned(), Some(3_400.0), None),
                    ("host.base_clock".to_owned(), Some(3_600.0), None),
                ],
            })
            .collect();
        let result = reevaluate_frames(&frames, &rules());
        let verdict = result.verdict.unwrap_or_else(|| panic!("a verdict is expected"));
        assert_ne!(verdict.classification, Classification::ThermalConfirmed);
    }
}
