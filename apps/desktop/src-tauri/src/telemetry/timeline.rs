//! The classified history of one session: the engine's verdict on each interval, merged into
//! segments, from which the persistent `limit_event`s and the frozen report are derived (T042,
//! FR-067). Pure: the recorder feeds it and reads it, storage never sees the engine.
#![deny(clippy::unwrap_used, clippy::expect_used)]

use crate::diagnostics::classifier::{Classification, DiagnosticResult, Severity};
use crate::diagnostics::events::{
    EventKind, SessionClassification, TimedClassification, classify_session, event_kind,
};
use crate::diagnostics::rules::Ruleset;

/// Evidence codes of a verdict that says nothing about the session's load: an idle machine, the
/// turbo exclusion window, or no window to judge yet.
const NOT_SUSTAINED: [&str; 3] = ["no_sustained_load", "turbo_window", "missing_stable_window"];

#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub classification: Classification,
    pub severity: Option<Severity>,
    pub start_ms: u64,
    pub end_ms: u64,
    pub start_seq: i64,
    pub end_seq: i64,
    /// Whether the interval counts as time under sustained load (the denominator of the report).
    pub sustained: bool,
}

/// A limitation that lasted: consecutive segments of the same event kind.
#[derive(Debug, Clone, PartialEq)]
pub struct LimitSpan {
    pub kind: EventKind,
    pub start_seq: i64,
    pub end_seq: i64,
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Debug, Clone, Default)]
pub struct Timeline {
    segments: Vec<Segment>,
    last: Option<(u64, i64)>,
    finished_spans: usize,
}

impl Timeline {
    /// Adds the verdict in force after the sample at `at_ms` (session-relative) with sequence
    /// `seq`. It covers the interval since the previous sample; a missing verdict, a machine with
    /// nothing to judge yet, or an idle stretch adds no interval.
    pub fn push(&mut self, at_ms: u64, seq: i64, verdict: Option<&DiagnosticResult>) {
        let previous = self.last.replace((at_ms, seq));
        let (Some((from_ms, from_seq)), Some(verdict)) = (previous, verdict) else { return };
        if verdict.evidence.contains(&"missing_stable_window") {
            return;
        }
        let sustained = !verdict.evidence.iter().any(|code| NOT_SUSTAINED.contains(code));
        if verdict.classification == Classification::Normal && !sustained {
            return;
        }
        if let Some(tail) = self.segments.last_mut()
            && tail.classification == verdict.classification
            && tail.severity == verdict.severity
            && tail.sustained == sustained
            && tail.end_ms == from_ms
        {
            tail.end_ms = at_ms;
            tail.end_seq = seq;
            return;
        }
        self.segments.push(Segment {
            classification: verdict.classification,
            severity: verdict.severity,
            start_ms: from_ms,
            end_ms: at_ms,
            start_seq: from_seq,
            end_seq: seq,
            sustained,
        });
    }

    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    fn spans(&self) -> Vec<LimitSpan> {
        let mut spans: Vec<LimitSpan> = Vec::new();
        for segment in &self.segments {
            let Some(kind) = event_kind(segment.classification) else { continue };
            if let Some(tail) = spans.last_mut()
                && tail.kind == kind
                && tail.end_ms == segment.start_ms
            {
                tail.end_ms = segment.end_ms;
                tail.end_seq = segment.end_seq;
                continue;
            }
            spans.push(LimitSpan {
                kind,
                start_seq: segment.start_seq,
                end_seq: segment.end_seq,
                start_ms: segment.start_ms,
                end_ms: segment.end_ms,
            });
        }
        spans
    }

    /// The limitations that can no longer grow, lasting at least `min_ms`, not returned before:
    /// what to persist while the session is still running.
    pub fn drain_finished(&mut self, min_ms: u64) -> Vec<LimitSpan> {
        let spans = self.spans();
        let still_growing = usize::from(
            spans.last().is_some_and(|span| self.last.is_some_and(|(at, _)| span.end_ms >= at)),
        );
        let finished_until = spans.len().saturating_sub(still_growing);
        self.take_spans(&spans, finished_until, min_ms)
    }

    /// Everything not yet returned, including a limitation still in progress: the session ended.
    pub fn drain_all(&mut self, min_ms: u64) -> Vec<LimitSpan> {
        let spans = self.spans();
        let all = spans.len();
        self.take_spans(&spans, all, min_ms)
    }

    fn take_spans(&mut self, spans: &[LimitSpan], until: usize, min_ms: u64) -> Vec<LimitSpan> {
        let start = self.finished_spans.min(until);
        self.finished_spans = until;
        spans[start..until]
            .iter()
            .filter(|span| span.end_ms.saturating_sub(span.start_ms) >= min_ms)
            .cloned()
            .collect()
    }

    /// The session's verdict (spec § «Clasificación de una sesión»); `None` when no interval was
    /// ever evaluated, which the report must say instead of inventing «normal».
    pub fn classify(&self, rules: &Ruleset) -> Option<SessionClassification> {
        if self.segments.is_empty() {
            return None;
        }
        let items: Vec<TimedClassification> = self
            .segments
            .iter()
            .map(|segment| TimedClassification {
                classification: segment.classification,
                severity: segment.severity,
                start_ms: segment.start_ms,
                end_ms: segment.end_ms,
            })
            .collect();
        let sustained_ms = self
            .segments
            .iter()
            .filter(|segment| segment.sustained)
            .map(|segment| segment.end_ms.saturating_sub(segment.start_ms))
            .sum();
        Some(classify_session(&items, sustained_ms, rules))
    }
}

#[cfg(test)]
mod tests {
    use super::Timeline;
    use crate::diagnostics::classifier::{Classification, DiagnosticResult, Severity};
    use crate::diagnostics::rules::{CoverageTier, Ruleset};

    fn rules() -> Ruleset {
        Ruleset::v1().unwrap_or_else(|error| panic!("ruleset: {error}"))
    }

    fn verdict(
        classification: Classification,
        severity: Option<Severity>,
        evidence: Vec<&'static str>,
    ) -> DiagnosticResult {
        DiagnosticResult {
            classification,
            severity,
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

    fn thermal() -> DiagnosticResult {
        verdict(Classification::ThermalConfirmed, Some(Severity::BelowBase), vec!["thermal_reason"])
    }

    #[test]
    fn consecutive_verdicts_of_one_class_merge_into_a_single_segment() {
        let mut timeline = Timeline::default();
        for second in 0..=10_i64 {
            timeline.push(second as u64 * 1_000, second, Some(&thermal()));
        }
        assert_eq!(timeline.segments.len(), 1);
        assert_eq!(timeline.segments[0].start_ms, 0);
        assert_eq!(timeline.segments[0].end_ms, 10_000);
        assert_eq!((timeline.segments[0].start_seq, timeline.segments[0].end_seq), (0, 10));
    }

    #[test]
    fn an_idle_or_unjudged_machine_adds_no_time_to_the_report() {
        let mut timeline = Timeline::default();
        let idle = verdict(Classification::Normal, None, vec!["no_sustained_load"]);
        let unjudged = verdict(Classification::Indeterminate, None, vec!["missing_stable_window"]);
        for second in 0..5_i64 {
            timeline.push(second as u64 * 1_000, second, Some(&idle));
            timeline.push(second as u64 * 1_000 + 500, second, Some(&unjudged));
            timeline.push(second as u64 * 1_000 + 900, second, None);
        }
        assert!(timeline.is_empty());
        assert_eq!(
            timeline.classify(&rules()),
            None,
            "nothing evaluated: no verdict, not «normal»"
        );
    }

    #[test]
    fn a_limitation_becomes_an_event_only_once_it_has_ended_and_lasted() {
        let mut timeline = Timeline::default();
        let normal = verdict(Classification::Normal, None, vec!["stable"]);
        let mut second = 0_i64;
        let mut push = |timeline: &mut Timeline, result: &DiagnosticResult, count: i64| {
            for _ in 0..count {
                timeline.push(second as u64 * 1_000, second, Some(result));
                second += 1;
            }
        };
        push(&mut timeline, &normal, 10);
        push(&mut timeline, &thermal(), 90);
        assert!(timeline.drain_finished(60_000).is_empty(), "still growing: not persisted yet");
        push(&mut timeline, &normal, 5);
        let events = timeline.drain_finished(60_000);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind.storage_key(), "thermal");
        assert!(events[0].end_ms - events[0].start_ms >= 60_000);
        assert!(timeline.drain_finished(60_000).is_empty(), "never returned twice");
    }

    #[test]
    fn a_short_blip_is_not_an_event_and_the_end_of_the_session_flushes_the_open_one() {
        let mut timeline = Timeline::default();
        let normal = verdict(Classification::Normal, None, vec!["stable"]);
        for second in 0..5_i64 {
            timeline.push(second as u64 * 1_000, second, Some(&normal));
        }
        for second in 5..15_i64 {
            timeline.push(second as u64 * 1_000, second, Some(&thermal()));
        }
        for second in 15..20_i64 {
            timeline.push(second as u64 * 1_000, second, Some(&normal));
        }
        assert!(timeline.drain_finished(60_000).is_empty(), "10 s is a blip");

        let mut open = Timeline::default();
        for second in 0..100_i64 {
            open.push(second as u64 * 1_000, second, Some(&thermal()));
        }
        assert_eq!(open.drain_all(60_000).len(), 1, "closing the session persists the open one");
    }

    #[test]
    fn a_sustained_thermal_session_is_classified_from_its_segments() {
        let mut timeline = Timeline::default();
        for second in 0..200_i64 {
            timeline.push(second as u64 * 1_000, second, Some(&thermal()));
        }
        let summary = timeline
            .classify(&rules())
            .unwrap_or_else(|| panic!("a session with evaluated time has a verdict"));
        assert_eq!(summary.classification, Classification::ThermalConfirmed);
        assert_eq!(summary.severity, Some(Severity::BelowBase));
        assert_eq!(summary.analyzed_from_ms, Some(0));
        assert_eq!(summary.analyzed_to_ms, Some(199_000));
    }
}
