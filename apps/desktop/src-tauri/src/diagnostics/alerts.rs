//! Deterministic alert rules for the tray/notification layer (T069).
//!
//! This module has no Windows or Tauri dependency. It turns a stream of already classified
//! observations into at most one notification per episode, applying the persistence and cooldown
//! parameters from `ruleset-v1`. The platform adapter is responsible for rendering the returned
//! event and for handling a later `notification:opened` action.
#![deny(clippy::unwrap_used, clippy::expect_used)]

use super::Ruleset;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlertKind {
    ThermalConfirmed,
    PowerLimited,
    PlatformLimited,
    CollectorLost,
    GuidedFinished,
}

impl AlertKind {
    pub const ALL: [Self; 5] = [
        Self::ThermalConfirmed,
        Self::PowerLimited,
        Self::PlatformLimited,
        Self::CollectorLost,
        Self::GuidedFinished,
    ];

    pub const fn key(self) -> &'static str {
        match self {
            Self::ThermalConfirmed => "thermal_confirmed",
            Self::PowerLimited => "power_limited",
            Self::PlatformLimited => "platform_limited",
            Self::CollectorLost => "collector_lost",
            Self::GuidedFinished => "guided_finished",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertEvent {
    pub kind: AlertKind,
    pub at_ms: u64,
    pub session_id: Option<String>,
    pub event_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AlertObservation<'a> {
    pub at_ms: u64,
    pub classification: Option<&'a str>,
    pub severity: Option<&'a str>,
    pub certainty: Option<&'a str>,
    pub collector_state: &'a str,
    pub guided_finished: bool,
    pub notifications_enabled: bool,
    pub silenced: bool,
    pub session_id: Option<&'a str>,
    pub event_id: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, Default)]
struct Episode {
    started_at_ms: Option<u64>,
    raised: bool,
    last_raised_at_ms: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct AlertEngine {
    persistence_ms: u64,
    cooldown_ms: u64,
    episodes: [Episode; AlertKind::ALL.len()],
}

impl AlertEngine {
    pub fn from_ruleset(rules: &Ruleset) -> Option<Self> {
        let persistence_s = rules.parameter("alerts.min_persistence_s")?;
        let cooldown_min = rules.parameter("alerts.cooldown_min")?;
        if !persistence_s.is_finite()
            || persistence_s < 0.0
            || !cooldown_min.is_finite()
            || cooldown_min < 0.0
        {
            return None;
        }
        Some(Self {
            persistence_ms: seconds_to_ms(persistence_s)?,
            cooldown_ms: seconds_to_ms(cooldown_min * 60.0)?,
            episodes: [Episode::default(); AlertKind::ALL.len()],
        })
    }

    #[cfg(test)]
    fn with_thresholds(persistence_ms: u64, cooldown_ms: u64) -> Self {
        Self { persistence_ms, cooldown_ms, episodes: [Episode::default(); AlertKind::ALL.len()] }
    }

    /// A limitation or a lost collector must persist (`alerts.min_persistence_s`) before it is
    /// worth interrupting someone; the end of a guided test is an event and is told at once.
    const fn persistence_for(&self, kind: AlertKind) -> u64 {
        match kind {
            AlertKind::GuidedFinished => 0,
            _ => self.persistence_ms,
        }
    }

    pub fn observe(&mut self, observation: AlertObservation<'_>) -> Vec<AlertEvent> {
        let mut raised = Vec::new();
        for kind in AlertKind::ALL {
            let active = is_active(kind, observation);
            let persistence_ms = self.persistence_for(kind);
            let episode = &mut self.episodes[index(kind)];
            if !active {
                episode.started_at_ms = None;
                episode.raised = false;
                continue;
            }
            let started_at = *episode.started_at_ms.get_or_insert(observation.at_ms);
            if episode.raised || observation.at_ms.saturating_sub(started_at) < persistence_ms {
                continue;
            }
            if episode
                .last_raised_at_ms
                .is_some_and(|last| observation.at_ms.saturating_sub(last) < self.cooldown_ms)
            {
                continue;
            }
            episode.raised = true;
            episode.last_raised_at_ms = Some(observation.at_ms);
            if observation.notifications_enabled && !observation.silenced {
                raised.push(AlertEvent {
                    kind,
                    at_ms: observation.at_ms,
                    session_id: observation.session_id.map(str::to_owned),
                    event_id: observation.event_id.map(str::to_owned),
                });
            }
        }
        raised
    }
}

/// Whether `hour` (local, 0–23) falls inside the quiet period `{start, end}` (hours as strings,
/// e.g. `{"start": "22", "end": "07"}`). A period that ends before it starts wraps past midnight.
/// `null`, malformed or equal bounds mean no quiet period.
pub fn in_quiet_period(period: &serde_json::Value, hour: u8) -> bool {
    let bound = |key: &str| {
        period
            .get(key)
            .and_then(serde_json::Value::as_str)
            .and_then(|value| value.trim().parse::<u8>().ok())
            .filter(|value| *value < 24)
    };
    let (Some(start), Some(end)) = (bound("start"), bound("end")) else { return false };
    match start.cmp(&end) {
        std::cmp::Ordering::Equal => false,
        std::cmp::Ordering::Less => (start..end).contains(&hour),
        std::cmp::Ordering::Greater => hour >= start || hour < end,
    }
}

fn seconds_to_ms(seconds: f64) -> Option<u64> {
    let milliseconds = (seconds * 1_000.0).round();
    if milliseconds.is_finite() && milliseconds >= 0.0 && milliseconds <= u64::MAX as f64 {
        Some(milliseconds as u64)
    } else {
        None
    }
}

fn index(kind: AlertKind) -> usize {
    match kind {
        AlertKind::ThermalConfirmed => 0,
        AlertKind::PowerLimited => 1,
        AlertKind::PlatformLimited => 2,
        AlertKind::CollectorLost => 3,
        AlertKind::GuidedFinished => 4,
    }
}

fn is_active(kind: AlertKind, observation: AlertObservation<'_>) -> bool {
    match kind {
        AlertKind::ThermalConfirmed => limitation_is_active(observation, "thermal_confirmed"),
        AlertKind::PowerLimited => limitation_is_active(observation, "power_limited"),
        AlertKind::PlatformLimited => limitation_is_active(observation, "platform_limited"),
        AlertKind::CollectorLost => matches!(observation.collector_state, "stopped" | "failed"),
        AlertKind::GuidedFinished => observation.guided_finished,
    }
}

fn limitation_is_active(observation: AlertObservation<'_>, classification: &str) -> bool {
    observation.classification == Some(classification)
        && observation.severity == Some("below_base")
        && observation.certainty == Some("observed")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn thermal(at_ms: u64) -> AlertObservation<'static> {
        AlertObservation {
            at_ms,
            classification: Some("thermal_confirmed"),
            severity: Some("below_base"),
            certainty: Some("observed"),
            collector_state: "running",
            notifications_enabled: true,
            session_id: Some("session-1"),
            event_id: Some("event-1"),
            ..AlertObservation::default()
        }
    }

    #[test]
    fn a_finished_guided_test_is_told_at_once_not_after_the_persistence_window() {
        let mut engine = AlertEngine::with_thresholds(90_000, 1_800_000);
        let events = engine.observe(AlertObservation {
            at_ms: 5,
            collector_state: "running",
            guided_finished: true,
            notifications_enabled: true,
            ..AlertObservation::default()
        });
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, AlertKind::GuidedFinished);
    }

    #[test]
    fn the_quiet_period_wraps_past_midnight_and_ignores_garbage() {
        use serde_json::json;
        let night = json!({"start": "22", "end": "07"});
        for hour in [22, 23, 0, 3, 6] {
            assert!(in_quiet_period(&night, hour), "{hour}");
        }
        for hour in [7, 12, 21] {
            assert!(!in_quiet_period(&night, hour), "{hour}");
        }
        let day = json!({"start": "09", "end": "17"});
        assert!(in_quiet_period(&day, 9) && !in_quiet_period(&day, 17));
        for none in [
            json!(null),
            json!({"start": "22", "end": "22"}),
            json!({"start": "x", "end": "7"}),
            json!({"start": "25", "end": "7"}),
            json!({"start": "22"}),
        ] {
            assert!(!in_quiet_period(&none, 23), "{none}");
        }
    }

    #[test]
    fn waits_for_persistence_and_emits_once_per_episode() {
        let mut engine = AlertEngine::with_thresholds(90_000, 1_800_000);
        assert!(engine.observe(thermal(0)).is_empty());
        assert!(engine.observe(thermal(89_999)).is_empty());
        let events = engine.observe(thermal(90_000));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, AlertKind::ThermalConfirmed);
        assert!(engine.observe(thermal(180_000)).is_empty());
    }

    #[test]
    fn boost_and_inferred_limitations_never_raise_alerts() {
        let mut engine = AlertEngine::with_thresholds(0, 0);
        for observation in [
            AlertObservation {
                classification: Some("thermal_confirmed"),
                severity: Some("boost"),
                certainty: Some("observed"),
                collector_state: "running",
                notifications_enabled: true,
                ..AlertObservation::default()
            },
            AlertObservation {
                classification: Some("power_limited"),
                severity: Some("below_base"),
                certainty: Some("inferred"),
                collector_state: "running",
                notifications_enabled: true,
                ..AlertObservation::default()
            },
        ] {
            assert!(engine.observe(observation).is_empty());
        }
    }

    #[test]
    fn cooldown_applies_after_an_episode_ends() {
        let mut engine = AlertEngine::with_thresholds(0, 1_800_000);
        assert_eq!(engine.observe(thermal(0)).len(), 1);
        assert!(
            engine
                .observe(AlertObservation {
                    at_ms: 1,
                    collector_state: "running",
                    ..AlertObservation::default()
                })
                .is_empty()
        );
        assert!(engine.observe(thermal(1_000_000)).is_empty());
        assert_eq!(engine.observe(thermal(1_800_001)).len(), 1);
    }

    #[test]
    fn collector_loss_and_guided_completion_are_distinct_alerts() {
        let mut engine = AlertEngine::with_thresholds(0, 0);
        let events = engine.observe(AlertObservation {
            at_ms: 10,
            collector_state: "failed",
            guided_finished: true,
            notifications_enabled: true,
            ..AlertObservation::default()
        });
        assert_eq!(
            events.iter().map(|event| event.kind).collect::<Vec<_>>(),
            vec![AlertKind::CollectorLost, AlertKind::GuidedFinished]
        );
    }

    #[test]
    fn disabled_or_silenced_notifications_still_close_the_episode() {
        let mut engine = AlertEngine::with_thresholds(0, 0);
        assert!(
            engine
                .observe(AlertObservation { notifications_enabled: false, ..thermal(0) })
                .is_empty()
        );
        assert!(
            engine
                .observe(AlertObservation {
                    at_ms: 1,
                    collector_state: "running",
                    ..AlertObservation::default()
                })
                .is_empty()
        );
        assert_eq!(
            engine
                .observe(AlertObservation { at_ms: 2, notifications_enabled: true, ..thermal(2) })
                .len(),
            1
        );
    }

    #[test]
    fn a_brief_limitation_episode_does_not_raise_after_it_clears() {
        let mut engine = AlertEngine::with_thresholds(90_000, 1_800_000);
        assert!(engine.observe(thermal(0)).is_empty());
        assert!(engine.observe(thermal(89_999)).is_empty());
        assert!(
            engine
                .observe(AlertObservation {
                    at_ms: 90_000,
                    collector_state: "running",
                    ..AlertObservation::default()
                })
                .is_empty()
        );
        assert!(engine.observe(thermal(179_999)).is_empty());
        assert!(engine.observe(thermal(269_998)).is_empty());
        assert_eq!(engine.observe(thermal(270_000)).len(), 1);
    }
}
