//! FR-075: when storage stops accepting writes (a full disk, most commonly), sampling must keep
//! going in memory, the interface must warn persistently, and the system must retry without any
//! destructive action on what is already stored. This module is the pure retry/backlog policy —
//! no I/O, no `rusqlite`, no `AppHandle` — so the cadence and ordering guarantees are unit-tested
//! directly; [`crate::commands::live`] supplies the actual write attempt.
#![deny(clippy::unwrap_used, clippy::expect_used)]

use std::collections::VecDeque;

use super::recorder::RecorderAction;

/// What changed as a result of [`WriteBacklog::submit`], if anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transition {
    /// Storage was healthy before and after this call.
    None,
    /// A write just failed for the first time; the interface should start warning.
    EnteredDegraded,
    /// The backlog just fully drained after being degraded; the interface should stop warning.
    Recovered,
}

/// Queues actions that failed to write and retries them, oldest first, no more often than
/// `retry_ms`. A later action is never applied before an earlier one that is still queued —
/// storage stays chronologically consistent even while catching up.
pub struct WriteBacklog {
    pending: VecDeque<RecorderAction>,
    degraded: bool,
    retry_ms: u64,
    last_attempt_ms: u64,
}

impl WriteBacklog {
    pub fn new(retry_ms: u64) -> Self {
        Self { pending: VecDeque::new(), degraded: false, retry_ms, last_attempt_ms: 0 }
    }

    pub fn is_degraded(&self) -> bool {
        self.degraded
    }

    /// Number of actions still waiting to reach storage.
    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    /// Queues `actions` (if any) and, unless still within the retry cadence of a previous
    /// failure, attempts to drain the backlog through `write_one` — oldest first, stopping at
    /// the first failure so nothing is skipped or reordered. `write_one` returns whether that
    /// single action reached storage.
    pub fn submit(
        &mut self,
        now_ms: u64,
        actions: &[RecorderAction],
        mut write_one: impl FnMut(&RecorderAction) -> bool,
    ) -> Transition {
        self.pending.extend(actions.iter().cloned());
        if self.pending.is_empty() {
            return Transition::None;
        }
        if self.degraded && now_ms.saturating_sub(self.last_attempt_ms) < self.retry_ms {
            return Transition::None;
        }
        self.last_attempt_ms = now_ms;
        let was_degraded = self.degraded;
        while let Some(action) = self.pending.front() {
            if write_one(action) {
                self.pending.pop_front();
            } else {
                self.degraded = true;
                return if was_degraded { Transition::None } else { Transition::EnteredDegraded };
            }
        }
        self.degraded = false;
        if was_degraded { Transition::Recovered } else { Transition::None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(sequence: i64) -> RecorderAction {
        RecorderAction::Frame {
            session_id: "s1".to_owned(),
            sequence,
            at_ms: sequence,
            duration_ms: 1_000,
            values: vec![],
        }
    }

    #[test]
    fn a_write_that_succeeds_never_becomes_degraded() {
        let mut backlog = WriteBacklog::new(60_000);
        let transition = backlog.submit(0, &[frame(1)], |_| true);
        assert_eq!(transition, Transition::None);
        assert!(!backlog.is_degraded());
        assert_eq!(backlog.pending_len(), 0);
    }

    #[test]
    fn a_failing_write_enters_degraded_exactly_once_and_keeps_the_action_queued() {
        let mut backlog = WriteBacklog::new(60_000);
        let first = backlog.submit(0, &[frame(1)], |_| false);
        assert_eq!(first, Transition::EnteredDegraded);
        assert!(backlog.is_degraded());
        assert_eq!(backlog.pending_len(), 1);

        // Same millisecond, still within the retry window: no second write attempt at all, so
        // no repeated EnteredDegraded and no wasted syscall against a full disk.
        let second = backlog.submit(0, &[frame(2)], |_| panic!("must not retry within retry_ms"));
        assert_eq!(second, Transition::None);
        assert_eq!(backlog.pending_len(), 2, "the new action still queues even without a retry");
    }

    #[test]
    fn recovery_drains_the_whole_backlog_oldest_first_and_reports_once() {
        let mut backlog = WriteBacklog::new(60_000);
        backlog.submit(0, &[frame(1)], |_| false);
        backlog.submit(0, &[frame(2)], |_| panic!("still within retry_ms"));

        let mut applied = Vec::new();
        let transition = backlog.submit(60_000, &[frame(3)], |action| {
            if let RecorderAction::Frame { sequence, .. } = action {
                applied.push(*sequence);
            }
            true
        });
        assert_eq!(transition, Transition::Recovered);
        assert!(!backlog.is_degraded());
        assert_eq!(backlog.pending_len(), 0);
        assert_eq!(applied, vec![1, 2, 3], "oldest first, nothing reordered or skipped");
    }

    #[test]
    fn a_retry_that_fails_again_stays_degraded_without_a_second_transition() {
        let mut backlog = WriteBacklog::new(60_000);
        backlog.submit(0, &[frame(1)], |_| false);
        let retry = backlog.submit(60_000, &[frame(2)], |_| false);
        assert_eq!(retry, Transition::None, "already degraded — no repeated notice");
        assert!(backlog.is_degraded());
        assert_eq!(backlog.pending_len(), 2, "nothing lost across a failed retry");
    }

    #[test]
    fn a_partial_recovery_stops_at_the_first_action_still_failing() {
        let mut backlog = WriteBacklog::new(60_000);
        backlog.submit(0, &[frame(1), frame(2)], |_| false);
        // Only the oldest action would succeed this time; the rest of the disk is still full.
        let transition = backlog.submit(60_000, &[], |action| {
            matches!(action, RecorderAction::Frame { sequence: 1, .. })
        });
        assert_eq!(transition, Transition::None, "still degraded, no false recovery");
        assert!(backlog.is_degraded());
        assert_eq!(backlog.pending_len(), 1, "the succeeding action left the queue");
    }

    #[test]
    fn submitting_nothing_while_healthy_never_attempts_a_write() {
        let mut backlog = WriteBacklog::new(60_000);
        let transition = backlog.submit(0, &[], |_| panic!("nothing to write"));
        assert_eq!(transition, Transition::None);
        assert!(!backlog.is_degraded());
    }

    #[test]
    fn a_zero_retry_interval_retries_on_every_call() {
        let mut backlog = WriteBacklog::new(0);
        backlog.submit(0, &[frame(1)], |_| false);
        let transition = backlog.submit(0, &[], |_| true);
        assert_eq!(transition, Transition::Recovered);
    }
}
