use super::clock::SampleQuality;

const MAX_PASSIVE_MS: u64 = 24 * 60 * 60 * 1000;
const GAP_LIMIT_MS: u64 = 60 * 1000;

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
}

impl PassiveSessionTracker {
    pub fn start(started_ms: u64) -> Self {
        Self { started_ms, last_sample_ms: None, report_state: ReportState::Provisional }
    }

    pub fn observe(&mut self, timestamp_ms: u64, quality: SampleQuality) -> SessionTransition {
        let transition = if timestamp_ms.saturating_sub(self.started_ms) >= MAX_PASSIVE_MS {
            SessionTransition::Split(SplitReason::MaxDuration)
        } else if let Some(last) = self.last_sample_ms {
            if timestamp_ms.saturating_sub(last) > GAP_LIMIT_MS || quality == SampleQuality::Gap {
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
    use crate::telemetry::clock::SampleQuality;

    #[test]
    fn splits_after_a_gap_and_keeps_live_report_provisional() {
        let mut tracker = PassiveSessionTracker::start(0);
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
        let mut tracker = PassiveSessionTracker::start(0);
        assert_eq!(
            tracker.observe(24 * 60 * 60 * 1000, SampleQuality::Complete),
            SessionTransition::Split(SplitReason::MaxDuration)
        );
    }
}
