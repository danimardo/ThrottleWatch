use super::clock::SampleQuality;
use crate::diagnostics::Ruleset;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitReason {
    Start,
    Gap,
    MaxDuration,
    Resume,
    CollectorRestart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportState {
    Provisional,
    Frozen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionTransition {
    Continue,
    Split(SplitReason),
}

#[derive(Debug, Clone, Copy)]
pub struct PassiveSessionTracker {
    started_ms: u64,
    last_sample_ms: Option<u64>,
    report_state: ReportState,
    max_passive_ms: u64,
    gap_limit_ms: u64,
}

impl PassiveSessionTracker {
    /// A session starts with the sampling, after a gap longer than `session.gap_s` and, at most,
    /// every `session.max_h` of continuous duration (FR-067); both limits come from `ruleset-v1`.
    pub fn start(started_ms: u64, rules: &Ruleset) -> Option<Self> {
        let gap_s = rules.parameter("session.gap_s")?;
        let max_h = rules.parameter("session.max_h")?;
        Some(Self {
            started_ms,
            last_sample_ms: None,
            report_state: ReportState::Provisional,
            max_passive_ms: (max_h * 3_600_000.0) as u64,
            gap_limit_ms: (gap_s * 1_000.0) as u64,
        })
    }

    pub const fn last_sample_ms(&self) -> Option<u64> {
        self.last_sample_ms
    }

    pub fn observe(&mut self, timestamp_ms: u64, quality: SampleQuality) -> SessionTransition {
        let transition = if timestamp_ms.saturating_sub(self.started_ms) >= self.max_passive_ms {
            SessionTransition::Split(SplitReason::MaxDuration)
        } else if let Some(last) = self.last_sample_ms {
            if timestamp_ms.saturating_sub(last) > self.gap_limit_ms
                || quality == SampleQuality::Gap
            {
                SessionTransition::Split(SplitReason::Gap)
            } else {
                SessionTransition::Continue
            }
        } else {
            SessionTransition::Continue
        };
        self.last_sample_ms = Some(timestamp_ms);
        transition
    }

    pub fn report_state(&self) -> ReportState {
        self.report_state
    }

    pub fn freeze_report(&mut self) {
        self.report_state = ReportState::Frozen;
    }
}

#[cfg(test)]
mod tests {
    use super::{PassiveSessionTracker, ReportState, SessionTransition, SplitReason};
    use crate::diagnostics::Ruleset;
    use crate::telemetry::clock::SampleQuality;

    fn tracker(started_ms: u64) -> PassiveSessionTracker {
        let rules = Ruleset::v1().unwrap_or_else(|error| panic!("ruleset: {error}"));
        PassiveSessionTracker::start(started_ms, &rules)
            .unwrap_or_else(|| panic!("ruleset must carry the session parameters"))
    }

    #[test]
    fn splits_after_a_gap_and_keeps_live_report_provisional() {
        let mut tracker = tracker(0);
        assert_eq!(tracker.observe(1000, SampleQuality::Complete), SessionTransition::Continue);
        assert_eq!(
            tracker.observe(61_001, SampleQuality::Complete),
            SessionTransition::Split(SplitReason::Gap)
        );
        assert_eq!(tracker.report_state(), ReportState::Provisional);
        tracker.freeze_report();
        assert_eq!(tracker.report_state(), ReportState::Frozen);
    }

    #[test]
    fn splits_at_twenty_four_hours() {
        let mut tracker = tracker(0);
        assert_eq!(
            tracker.observe(24 * 60 * 60 * 1000, SampleQuality::Complete),
            SessionTransition::Split(SplitReason::MaxDuration)
        );
    }
}
