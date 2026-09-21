//! Turns the live sample stream into passive sessions (FR-067, FR-065): opens a session when
//! sampling starts, splits it after a gap, a resume, a collector restart or 24 h, records every
//! frame, and on closing reduces the engine's verdicts into limitations and a session verdict.
//! It only produces [`RecorderAction`]s; whoever owns storage executes them, so this is pure and
//! replays deterministically.
#![deny(clippy::unwrap_used, clippy::expect_used)]

use super::clock::SampleQuality;
use super::session::{PassiveSessionTracker, SessionTransition, SplitReason};
use super::timeline::Timeline;
use crate::diagnostics::classifier::DiagnosticResult;
use crate::diagnostics::events::SessionClassification;
use crate::diagnostics::rules::{CoverageTier, Ruleset};
use crate::storage::StoredSampleValue;

#[derive(Debug, Clone, PartialEq)]
pub enum RecorderAction {
    Open {
        session_id: String,
        split_reason: &'static str,
        started_epoch_ms: u64,
    },
    Frame {
        session_id: String,
        sequence: i64,
        at_ms: i64,
        duration_ms: i64,
        values: Vec<StoredSampleValue>,
    },
    Event {
        session_id: String,
        kind: &'static str,
        start_sequence: i64,
        end_sequence: i64,
    },
    Close {
        session_id: String,
        ended_epoch_ms: u64,
        duration_ms: i64,
        tier: CoverageTier,
        verdict: Option<SessionClassification>,
    },
}

/// One collector sample as the recorder needs it.
pub struct RecorderInput<'a> {
    pub epoch_ms: u64,
    pub values: Vec<StoredSampleValue>,
    /// The engine's verdict after this sample; `None` while there is nothing to judge.
    pub verdict: Option<&'a DiagnosticResult>,
    pub tier: CoverageTier,
    pub quality: SampleQuality,
    /// The machine resumed from sleep or hibernation since the previous sample.
    pub resumed: bool,
}

struct Open {
    id: String,
    started_ms: u64,
    last_frame_ms: u64,
    sequence: i64,
    tracker: PassiveSessionTracker,
    timeline: Timeline,
    tier: CoverageTier,
}

pub struct SessionRecorder {
    rules: Ruleset,
    /// A limitation shorter than this is a blip, not an event (`session.class_min_s`).
    event_min_ms: u64,
    open: Option<Open>,
    next_reason: SplitReason,
    counter: u64,
}

impl SessionRecorder {
    pub fn new(rules: Ruleset) -> Option<Self> {
        let event_min_ms = (rules.parameter("session.class_min_s")? * 1_000.0) as u64;
        rules.parameter("session.gap_s")?;
        rules.parameter("session.max_h")?;
        Some(Self { rules, event_min_ms, open: None, next_reason: SplitReason::Start, counter: 0 })
    }

    pub fn open_session_id(&self) -> Option<&str> {
        self.open.as_ref().map(|open| open.id.as_str())
    }

    pub const fn has_open_session(&self) -> bool {
        self.open.is_some()
    }

    /// The collector restarted or the machine woke: the current session ends and the next sample
    /// opens another one for `reason`.
    pub fn interrupt(&mut self, reason: SplitReason, now_epoch_ms: u64) -> Vec<RecorderAction> {
        let actions = self.close(now_epoch_ms);
        self.next_reason = reason;
        actions
    }

    /// Forgets the session in progress without writing anything: its rows were just deleted
    /// (Eliminar todos mis datos / Restablecer), so the next sample opens a fresh one.
    pub fn abandon(&mut self) {
        self.open = None;
        self.next_reason = SplitReason::Start;
    }

    /// Closes the session in progress (the application is exiting or the collector is gone).
    pub fn close(&mut self, now_epoch_ms: u64) -> Vec<RecorderAction> {
        let Some(mut open) = self.open.take() else { return Vec::new() };
        let mut actions: Vec<RecorderAction> = open
            .timeline
            .drain_all(self.event_min_ms)
            .into_iter()
            .map(|span| RecorderAction::Event {
                session_id: open.id.clone(),
                kind: span.kind.storage_key(),
                start_sequence: span.start_seq,
                end_sequence: span.end_seq,
            })
            .collect();
        let ended = open.last_frame_ms.max(open.started_ms).min(now_epoch_ms.max(open.started_ms));
        actions.push(RecorderAction::Close {
            session_id: open.id.clone(),
            ended_epoch_ms: ended,
            duration_ms: i64::try_from(ended.saturating_sub(open.started_ms)).unwrap_or(i64::MAX),
            tier: open.tier,
            verdict: open.timeline.classify(&self.rules),
        });
        actions
    }

    pub fn observe(&mut self, input: RecorderInput<'_>) -> Vec<RecorderAction> {
        let mut actions = Vec::new();
        let split = match self.open.as_mut() {
            Some(open) => {
                let transition = open.tracker.observe(input.epoch_ms, input.quality);
                match transition {
                    SessionTransition::Split(reason) => Some(reason),
                    SessionTransition::Continue if input.resumed => Some(SplitReason::Resume),
                    SessionTransition::Continue => None,
                }
            }
            None => None,
        };
        if let Some(reason) = split {
            actions.extend(self.close(input.epoch_ms));
            self.next_reason = reason;
        }
        if self.open.is_none() {
            let Some(tracker) = PassiveSessionTracker::start(input.epoch_ms, &self.rules) else {
                return actions;
            };
            self.counter += 1;
            let id = format!("passive-{}-{}", input.epoch_ms, self.counter);
            actions.push(RecorderAction::Open {
                session_id: id.clone(),
                split_reason: split_key(self.next_reason),
                started_epoch_ms: input.epoch_ms,
            });
            self.next_reason = SplitReason::Start;
            self.open = Some(Open {
                id,
                started_ms: input.epoch_ms,
                last_frame_ms: input.epoch_ms,
                sequence: -1,
                tracker,
                timeline: Timeline::default(),
                tier: input.tier,
            });
        }
        let Some(open) = self.open.as_mut() else { return actions };
        let at_ms = input.epoch_ms.saturating_sub(open.started_ms);
        let duration_ms = input.epoch_ms.saturating_sub(open.last_frame_ms);
        open.sequence += 1;
        open.last_frame_ms = input.epoch_ms;
        open.tier = input.tier;
        actions.push(RecorderAction::Frame {
            session_id: open.id.clone(),
            sequence: open.sequence,
            at_ms: i64::try_from(at_ms).unwrap_or(i64::MAX),
            duration_ms: i64::try_from(duration_ms).unwrap_or(i64::MAX),
            values: input.values,
        });
        open.timeline.push(at_ms, open.sequence, input.verdict);
        for span in open.timeline.drain_finished(self.event_min_ms) {
            actions.push(RecorderAction::Event {
                session_id: open.id.clone(),
                kind: span.kind.storage_key(),
                start_sequence: span.start_seq,
                end_sequence: span.end_seq,
            });
        }
        actions
    }
}

const fn split_key(reason: SplitReason) -> &'static str {
    match reason {
        SplitReason::Start => "start",
        SplitReason::Gap => "gap",
        SplitReason::MaxDuration => "max_duration",
        SplitReason::Resume => "resume",
        SplitReason::CollectorRestart => "collector_restart",
    }
}

#[cfg(test)]
mod tests {
    use super::{RecorderAction, RecorderInput, SessionRecorder};
    use crate::diagnostics::classifier::{Classification, DiagnosticResult, Severity};
    use crate::diagnostics::rules::{CoverageTier, Ruleset};
    use crate::telemetry::clock::SampleQuality;
    use crate::telemetry::session::SplitReason;

    fn recorder() -> SessionRecorder {
        let rules = Ruleset::v1().unwrap_or_else(|error| panic!("ruleset: {error}"));
        SessionRecorder::new(rules).unwrap_or_else(|| panic!("ruleset carries the session block"))
    }

    fn verdict(classification: Classification, evidence: Vec<&'static str>) -> DiagnosticResult {
        DiagnosticResult {
            classification,
            severity: (classification == Classification::ThermalConfirmed)
                .then_some(Severity::BelowBase),
            coverage: CoverageTier::A,
            confidence: 0.8,
            platform: None,
            evidence,
            alternative_causes: vec![],
            analyzed_from_ms: None,
            analyzed_to_ms: None,
            windows: 1,
        }
    }

    fn input(epoch_ms: u64, verdict: Option<&DiagnosticResult>) -> RecorderInput<'_> {
        RecorderInput {
            epoch_ms,
            values: vec![],
            verdict,
            tier: CoverageTier::A,
            quality: SampleQuality::Complete,
            resumed: false,
        }
    }

    fn kinds(actions: &[RecorderAction]) -> Vec<&'static str> {
        actions
            .iter()
            .map(|action| match action {
                RecorderAction::Open { .. } => "open",
                RecorderAction::Frame { .. } => "frame",
                RecorderAction::Event { .. } => "event",
                RecorderAction::Close { .. } => "close",
            })
            .collect()
    }

    #[test]
    fn the_first_sample_opens_a_session_and_every_sample_is_a_numbered_frame() {
        let mut recorder = recorder();
        let first = recorder.observe(input(1_000_000, None));
        assert_eq!(kinds(&first), ["open", "frame"]);
        let RecorderAction::Open { split_reason, started_epoch_ms, .. } = &first[0] else {
            panic!("open expected")
        };
        assert_eq!((*split_reason, *started_epoch_ms), ("start", 1_000_000));

        let second = recorder.observe(input(1_001_000, None));
        assert_eq!(kinds(&second), ["frame"]);
        let RecorderAction::Frame { sequence, at_ms, duration_ms, .. } = &second[0] else {
            panic!("frame expected")
        };
        assert_eq!((*sequence, *at_ms, *duration_ms), (1, 1_000, 1_000));
    }

    #[test]
    fn a_gap_closes_the_session_and_opens_the_next_one_with_that_reason() {
        let mut recorder = recorder();
        recorder.observe(input(0, None));
        recorder.observe(input(1_000, None));
        let actions = recorder.observe(input(1_000 + 61_000, None));
        assert_eq!(kinds(&actions), ["close", "open", "frame"]);
        let RecorderAction::Close { ended_epoch_ms, duration_ms, .. } = &actions[0] else {
            panic!("close expected")
        };
        assert_eq!((*ended_epoch_ms, *duration_ms), (1_000, 1_000), "it ended at its last sample");
        let RecorderAction::Open { split_reason, .. } = &actions[1] else { panic!("open") };
        assert_eq!(*split_reason, "gap");
    }

    #[test]
    fn a_resume_and_a_collector_restart_split_the_session_too() {
        let mut recorder = recorder();
        recorder.observe(input(0, None));
        let mut resumed = input(1_000, None);
        resumed.resumed = true;
        let actions = recorder.observe(resumed);
        assert_eq!(kinds(&actions), ["close", "open", "frame"]);
        let RecorderAction::Open { split_reason, .. } = &actions[1] else { panic!("open") };
        assert_eq!(*split_reason, "resume");

        let closed = recorder.interrupt(SplitReason::CollectorRestart, 2_000);
        assert_eq!(kinds(&closed), ["close"]);
        let reopened = recorder.observe(input(3_000, None));
        let RecorderAction::Open { split_reason, .. } = &reopened[0] else { panic!("open") };
        assert_eq!(*split_reason, "collector_restart");
    }

    #[test]
    fn the_session_closes_at_the_final_coverage_tier() {
        let mut recorder = recorder();
        recorder.observe(input(0, None));
        let mut degraded = input(5_000, None);
        degraded.tier = CoverageTier::B;
        recorder.observe(degraded);
        let closed = recorder.close(6_000);
        let RecorderAction::Close { tier, .. } = closed.last().unwrap_or_else(|| panic!("close"))
        else {
            panic!("close expected")
        };
        assert_eq!(*tier, CoverageTier::B);
    }

    #[test]
    fn a_sustained_limitation_is_recorded_as_an_event_and_decides_the_session() {
        let mut recorder = recorder();
        let normal = verdict(Classification::Normal, vec!["stable"]);
        let thermal = verdict(Classification::ThermalConfirmed, vec!["thermal_reason"]);
        let mut all = Vec::new();
        for second in 0..10_u64 {
            all.extend(recorder.observe(input(second * 1_000, Some(&normal))));
        }
        for second in 10..130_u64 {
            all.extend(recorder.observe(input(second * 1_000, Some(&thermal))));
        }
        for second in 130..140_u64 {
            all.extend(recorder.observe(input(second * 1_000, Some(&normal))));
        }
        let events: Vec<_> = all
            .iter()
            .filter_map(|action| match action {
                RecorderAction::Event { kind, start_sequence, end_sequence, .. } => {
                    Some((*kind, *start_sequence, *end_sequence))
                }
                _ => None,
            })
            .collect();
        assert_eq!(events.len(), 1, "{events:?}");
        assert_eq!(events[0].0, "thermal");
        assert!(events[0].2 - events[0].1 >= 60);

        let closed = recorder.close(140_000);
        let RecorderAction::Close { verdict, .. } =
            closed.last().unwrap_or_else(|| panic!("close"))
        else {
            panic!("close expected")
        };
        let verdict =
            verdict.as_ref().unwrap_or_else(|| panic!("an evaluated session has a verdict"));
        assert_eq!(verdict.classification, Classification::ThermalConfirmed);
    }

    #[test]
    fn an_abandoned_session_writes_nothing_and_the_next_sample_starts_a_new_one() {
        let mut recorder = recorder();
        recorder.observe(input(0, None));
        recorder.abandon();
        assert!(!recorder.has_open_session());
        assert!(recorder.close(1_000).is_empty());
        let actions = recorder.observe(input(2_000, None));
        assert_eq!(kinds(&actions), ["open", "frame"]);
    }

    #[test]
    fn a_session_with_nothing_evaluated_closes_without_a_verdict() {
        let mut recorder = recorder();
        recorder.observe(input(0, None));
        recorder.observe(input(1_000, None));
        let closed = recorder.close(2_000);
        let RecorderAction::Close { verdict, .. } = &closed[0] else { panic!("close expected") };
        assert!(verdict.is_none());
        assert!(!recorder.has_open_session());
        assert!(recorder.close(3_000).is_empty(), "closing twice does nothing");
    }
}
