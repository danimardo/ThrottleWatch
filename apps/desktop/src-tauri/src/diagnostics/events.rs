use super::classifier::Severity;
use super::classifier::{Classification, DiagnosticResult};
use super::rules::Ruleset;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    Thermal,
    Power,
    Current,
    Platform,
    Mixed,
    TurboEnd,
    OemModeChange,
}

impl EventKind {
    /// The `limit_event.kind` stored and shown by the analysis view.
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Thermal => "thermal",
            Self::Power | Self::Current => "power",
            Self::Platform => "platform",
            Self::Mixed => "mixed",
            Self::TurboEnd => "turbo_end",
            Self::OemModeChange => "oem_mode_change",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LimitEvent {
    pub kind: EventKind,
    pub start_ms: u64,
    pub end_ms: u64,
    pub confidence: f64,
    pub evidence: Vec<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InformationalMarker {
    TurboEnd,
    OemModeChange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticMarker {
    pub kind: InformationalMarker,
    pub at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimedClassification {
    pub classification: Classification,
    pub severity: Option<Severity>,
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SessionClassification {
    pub classification: Classification,
    pub severity: Option<Severity>,
    pub class_durations_ms: BTreeMap<String, u64>,
    pub analyzed_from_ms: Option<u64>,
    pub analyzed_to_ms: Option<u64>,
}

/// Classifies a whole session from its stable windows (spec § «Clasificación de una sesión»):
/// `indeterminate` when it dominates the time under sustained load; otherwise the limiting class
/// with the most **accumulated** time, if it adds up to `session.class_min_s` and
/// `session.class_min_share_pct` of that load (ties: `below_base`, then the table order);
/// otherwise `hot_unproven` if any window was hot, else `normal`. Every threshold is read from
/// `ruleset-v1`.
pub fn classify_session(
    items: &[TimedClassification],
    sustained_ms: u64,
    rules: &Ruleset,
) -> SessionClassification {
    let indeterminate_share_pct =
        rules.parameter("session.indeterminate_share_pct").unwrap_or(50.0);
    let class_min_ms = (rules.parameter("session.class_min_s").unwrap_or(60.0) * 1_000.0) as u64;
    let class_min_share_pct = rules.parameter("session.class_min_share_pct").unwrap_or(10.0);
    let below_base_min_ms = (rules.below_base_duration_s * 1_000.0) as u64;

    let mut durations = BTreeMap::new();
    let mut by_class: BTreeMap<&'static str, (Classification, u64, u64)> = BTreeMap::new();
    for item in items {
        let duration = item.end_ms.saturating_sub(item.start_ms);
        *durations.entry(item.classification.key().to_owned()).or_insert(0) += duration;
        let entry =
            by_class.entry(item.classification.key()).or_insert((item.classification, 0, 0));
        entry.1 += duration;
        if item.severity == Some(Severity::BelowBase) {
            entry.2 += duration;
        }
    }
    let share_of_load = |milliseconds: u64| milliseconds as f64 * 100.0;
    let indeterminate_ms = durations.get("indeterminate").copied().unwrap_or(0);
    let limit = by_class
        .values()
        .filter(|(class, total, _)| {
            is_limiting(*class)
                && *total >= class_min_ms
                && share_of_load(*total) >= sustained_ms as f64 * class_min_share_pct
        })
        .max_by_key(|(class, total, below_base)| {
            (*total, u8::from(*below_base > 0), std::cmp::Reverse(table_order(*class)))
        })
        .map(|(class, _, _)| *class);
    let classification = if sustained_ms > 0
        && share_of_load(indeterminate_ms) > sustained_ms as f64 * indeterminate_share_pct
    {
        Classification::Indeterminate
    } else if let Some(class) = limit {
        class
    } else if durations.contains_key("hot_unproven") {
        Classification::HotUnproven
    } else {
        Classification::Normal
    };
    let severity = by_class
        .get(classification.key())
        .is_some_and(|(_, _, below_base)| *below_base >= below_base_min_ms)
        .then_some(Severity::BelowBase);
    SessionClassification {
        classification,
        severity,
        class_durations_ms: durations,
        analyzed_from_ms: items.first().map(|item| item.start_ms),
        analyzed_to_ms: items.last().map(|item| item.end_ms),
    }
}

/// Position of a limiting class in the rule table (spec § «Reglas de clasificación»): the tie-break.
const fn table_order(classification: Classification) -> u8 {
    match classification {
        Classification::MixedLimit => 3,
        Classification::ThermalConfirmed => 4,
        Classification::ThermalProbable => 5,
        Classification::PlatformLimited => 6,
        Classification::PowerLimited => 8,
        _ => u8::MAX,
    }
}

fn is_limiting(classification: Classification) -> bool {
    matches!(
        classification,
        Classification::ThermalConfirmed
            | Classification::ThermalProbable
            | Classification::PowerLimited
            | Classification::PlatformLimited
            | Classification::MixedLimit
    )
}

pub fn fuse_classifications(items: &[TimedClassification]) -> Vec<LimitEvent> {
    let mut output: Vec<LimitEvent> = Vec::new();
    for item in items {
        let Some(kind) = event_kind(item.classification) else { continue };
        if let Some(previous) = output.last_mut()
            && previous.kind == kind
            && previous.end_ms >= item.start_ms
        {
            previous.end_ms = previous.end_ms.max(item.end_ms);
            continue;
        }
        output.push(LimitEvent {
            kind,
            start_ms: item.start_ms,
            end_ms: item.end_ms,
            confidence: 0.0,
            evidence: Vec::new(),
        });
    }
    output
}

pub fn fuse_with_markers(
    items: &[TimedClassification],
    markers: &[DiagnosticMarker],
) -> (Vec<LimitEvent>, Vec<DiagnosticMarker>) {
    (fuse_classifications(items), markers.to_vec())
}

pub fn markers_from_samples(
    samples: &[super::DiagnosticSample],
    rules: &super::rules::Ruleset,
) -> Vec<DiagnosticMarker> {
    let mut markers = Vec::new();
    for pair in samples.windows(2) {
        let previous = pair[0];
        let current = pair[1];
        let elapsed = current.monotonic_ms.saturating_sub(previous.monotonic_ms);
        if super::windows::turbo_end(
            previous.package_power_w.unwrap_or(0.0),
            current.package_power_w.unwrap_or(0.0),
            elapsed,
            rules,
        ) {
            markers.push(DiagnosticMarker {
                kind: InformationalMarker::TurboEnd,
                at_ms: current.monotonic_ms,
            });
        }
    }
    markers
}

pub fn oem_mode_change_marker(
    first_limit_w: Option<f64>,
    last_limit_w: Option<f64>,
    limit_values: &[f64],
    at_ms: u64,
    rules: &super::rules::Ruleset,
) -> Option<DiagnosticMarker> {
    let dropped = first_limit_w.zip(last_limit_w).is_some_and(|(first, last)| {
        first > 0.0 && (first - last) / first >= rules.platform_limit_drop_ratio
    });
    (dropped
        && super::platform::limit_trend(limit_values, rules)
            == super::platform::LimitTrend::SingleStep)
        .then_some(DiagnosticMarker { kind: InformationalMarker::OemModeChange, at_ms })
}

pub fn event_kind(classification: Classification) -> Option<EventKind> {
    match classification {
        Classification::ThermalConfirmed | Classification::ThermalProbable => {
            Some(EventKind::Thermal)
        }
        Classification::PowerLimited => Some(EventKind::Power),
        Classification::PlatformLimited => Some(EventKind::Platform),
        Classification::MixedLimit => Some(EventKind::Mixed),
        _ => None,
    }
}

pub fn event_for_result(result: &DiagnosticResult) -> Option<LimitEvent> {
    let kind = match result.classification {
        Classification::ThermalConfirmed | Classification::ThermalProbable => EventKind::Thermal,
        Classification::PowerLimited => EventKind::Power,
        Classification::PlatformLimited => EventKind::Platform,
        Classification::MixedLimit => EventKind::Mixed,
        _ => return None,
    };
    Some(LimitEvent {
        kind,
        start_ms: result.analyzed_from_ms?,
        end_ms: result.analyzed_to_ms?,
        confidence: result.confidence,
        evidence: result.evidence.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        DiagnosticMarker, EventKind, InformationalMarker, TimedClassification, classify_session,
        event_for_result, fuse_classifications, fuse_with_markers, oem_mode_change_marker,
    };
    use crate::diagnostics::{
        CoverageTier, DiagnosticResult, Severity, classifier::Classification,
    };

    fn rules() -> crate::diagnostics::rules::Ruleset {
        crate::diagnostics::rules::Ruleset::v1().unwrap_or_else(|error| panic!("ruleset: {error}"))
    }

    fn item(
        classification: Classification,
        severity: Option<Severity>,
        start_ms: u64,
        end_ms: u64,
    ) -> TimedClassification {
        TimedClassification { classification, severity, start_ms, end_ms }
    }

    #[test]
    fn a_class_is_judged_by_its_accumulated_time_not_by_a_single_window() {
        // Three separate 25 s thermal spells add up to 75 s of a 300 s load (25 %): a limitation.
        let items = [
            item(Classification::ThermalConfirmed, None, 0, 25_000),
            item(Classification::Normal, None, 25_000, 100_000),
            item(Classification::ThermalConfirmed, None, 100_000, 125_000),
            item(Classification::Normal, None, 125_000, 200_000),
            item(Classification::ThermalConfirmed, None, 200_000, 225_000),
            item(Classification::Normal, None, 225_000, 300_000),
        ];
        let summary = classify_session(&items, 300_000, &rules());
        assert_eq!(summary.classification, Classification::ThermalConfirmed);
        assert_eq!(summary.class_durations_ms.get("thermal_confirmed"), Some(&75_000));
    }

    #[test]
    fn a_short_or_marginal_class_does_not_decide_the_session() {
        // 45 s is under `session.class_min_s`; 60 s of a 1 h load is under the 10 % share.
        let short = [item(Classification::ThermalConfirmed, None, 0, 45_000)];
        assert_eq!(
            classify_session(&short, 45_000, &rules()).classification,
            Classification::Normal
        );
        let marginal = [item(Classification::PowerLimited, None, 0, 60_000)];
        assert_eq!(
            classify_session(&marginal, 3_600_000, &rules()).classification,
            Classification::Normal
        );
    }

    #[test]
    fn hot_windows_without_a_limitation_make_the_session_hot_unproven() {
        let items = [
            item(Classification::HotUnproven, None, 0, 30_000),
            item(Classification::Normal, None, 30_000, 90_000),
        ];
        let summary = classify_session(&items, 60_000, &rules());
        assert_eq!(summary.classification, Classification::HotUnproven);
        assert!(summary.class_durations_ms.contains_key("hot_unproven"));
    }

    #[test]
    fn indeterminate_wins_when_it_dominates_the_load_and_ties_prefer_below_base() {
        let mostly_unknown = [
            item(Classification::Indeterminate, None, 0, 100_000),
            item(Classification::ThermalConfirmed, None, 100_000, 160_000),
        ];
        assert_eq!(
            classify_session(&mostly_unknown, 160_000, &rules()).classification,
            Classification::Indeterminate
        );
        let tie = [
            item(Classification::ThermalConfirmed, None, 0, 60_000),
            item(Classification::PowerLimited, Some(Severity::BelowBase), 60_000, 120_000),
        ];
        let summary = classify_session(&tie, 120_000, &rules());
        assert_eq!(summary.classification, Classification::PowerLimited);
        assert_eq!(summary.severity, Some(Severity::BelowBase));
    }

    #[test]
    fn durations_are_keyed_like_the_report_and_the_bridge() {
        let items = [item(Classification::HotUnproven, None, 0, 10_000)];
        let keys: Vec<_> =
            classify_session(&items, 0, &rules()).class_durations_ms.keys().cloned().collect();
        assert_eq!(keys, vec!["hot_unproven".to_owned()]);
    }

    #[test]
    fn maps_only_limiting_classifications_to_events() {
        let result = DiagnosticResult {
            classification: Classification::PowerLimited,
            severity: None,
            coverage: CoverageTier::A,
            confidence: 0.8,
            platform: None,
            evidence: vec!["power_reason"],
            alternative_causes: vec![],
            analyzed_from_ms: Some(0),
            analyzed_to_ms: Some(60_000),
            windows: 1,
        };
        assert_eq!(event_for_result(&result).map(|event| event.kind), Some(EventKind::Power));
    }

    #[test]
    fn fuses_adjacent_windows_of_the_same_class() {
        let events = fuse_classifications(&[
            TimedClassification {
                classification: Classification::PowerLimited,
                severity: Some(Severity::BelowBase),
                start_ms: 0,
                end_ms: 60_000,
            },
            TimedClassification {
                classification: Classification::PowerLimited,
                severity: Some(Severity::BelowBase),
                start_ms: 60_000,
                end_ms: 120_000,
            },
            TimedClassification {
                classification: Classification::ThermalProbable,
                severity: None,
                start_ms: 120_000,
                end_ms: 180_000,
            },
        ]);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].end_ms, 120_000);
    }

    #[test]
    fn classifies_a_session_by_accumulated_time_and_keeps_the_interval() {
        let summary = classify_session(
            &[
                TimedClassification {
                    classification: Classification::PowerLimited,
                    severity: Some(Severity::BelowBase),
                    start_ms: 0,
                    end_ms: 60_000,
                },
                TimedClassification {
                    classification: Classification::Normal,
                    severity: None,
                    start_ms: 60_000,
                    end_ms: 120_000,
                },
            ],
            120_000,
            &rules(),
        );
        assert_eq!(summary.classification, Classification::PowerLimited);
        assert_eq!(summary.severity, Some(Severity::BelowBase));
        assert_eq!(summary.analyzed_to_ms, Some(120_000));
    }

    #[test]
    fn keeps_informational_markers_separate_from_limiting_events() {
        let marker = DiagnosticMarker { kind: InformationalMarker::TurboEnd, at_ms: 5_000 };
        let (events, markers) = fuse_with_markers(&[], std::slice::from_ref(&marker));
        assert!(events.is_empty());
        assert_eq!(markers, vec![marker]);
    }

    #[test]
    fn emits_an_oem_marker_for_a_single_limit_step() -> serde_json::Result<()> {
        let rules = crate::diagnostics::rules::Ruleset::v1()?;
        let marker = oem_mode_change_marker(Some(100.0), Some(85.0), &[100.0, 85.0], 5_000, &rules);
        assert_eq!(marker.map(|value| value.kind), Some(InformationalMarker::OemModeChange));
        Ok(())
    }
}
