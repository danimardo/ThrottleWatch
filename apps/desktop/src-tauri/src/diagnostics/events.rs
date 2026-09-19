use super::classifier::Severity;
use super::classifier::{Classification, DiagnosticResult};
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

pub fn classify_session(items: &[TimedClassification], sustained_ms: u64) -> SessionClassification {
    let mut durations = BTreeMap::new();
    for item in items {
        let duration = item.end_ms.saturating_sub(item.start_ms);
        *durations.entry(classification_key(item.classification)).or_insert(0) += duration;
    }
    let indeterminate_ms = durations.get("indeterminate").copied().unwrap_or(0);
    let limit = items
        .iter()
        .filter(|item| is_limiting(item.classification))
        .filter_map(|item| {
            let duration = item.end_ms.saturating_sub(item.start_ms);
            (duration >= 60_000 && duration.saturating_mul(10) >= sustained_ms)
                .then_some((item, duration))
        })
        .max_by_key(|(item, duration)| {
            (u8::from(item.severity == Some(Severity::BelowBase)), *duration)
        });
    let classification = if indeterminate_ms.saturating_mul(2) > sustained_ms {
        Classification::Indeterminate
    } else if let Some((item, _)) = limit {
        item.classification
    } else if durations.contains_key("hot_unproven") {
        Classification::HotUnproven
    } else {
        Classification::Normal
    };
    let severity = items
        .iter()
        .filter(|item| {
            item.classification == classification && item.severity == Some(Severity::BelowBase)
        })
        .map(|item| item.end_ms.saturating_sub(item.start_ms))
        .sum::<u64>()
        .ge(&30_000)
        .then_some(Severity::BelowBase);
    SessionClassification {
        classification,
        severity,
        class_durations_ms: durations,
        analyzed_from_ms: items.first().map(|item| item.start_ms),
        analyzed_to_ms: items.last().map(|item| item.end_ms),
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

fn classification_key(classification: Classification) -> String {
    format!("{classification:?}").to_lowercase()
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

fn event_kind(classification: Classification) -> Option<EventKind> {
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
        );
        assert_eq!(summary.classification, Classification::PowerLimited);
        assert_eq!(summary.severity, Some(Severity::BelowBase));
        assert_eq!(summary.analyzed_to_ms, Some(120_000));
    }

    #[test]
    fn keeps_informational_markers_separate_from_limiting_events() {
        let marker = DiagnosticMarker { kind: InformationalMarker::TurboEnd, at_ms: 5_000 };
        let (events, markers) = fuse_with_markers(&[], &[marker.clone()]);
        assert!(events.is_empty());
        assert_eq!(markers, vec![marker]);
    }

    #[test]
    fn emits_an_oem_marker_for_a_single_limit_step() {
        let rules = crate::diagnostics::rules::Ruleset::v1().expect("valid rules");
        let marker = oem_mode_change_marker(Some(100.0), Some(85.0), &[100.0, 85.0], 5_000, &rules);
        assert_eq!(marker.map(|value| value.kind), Some(InformationalMarker::OemModeChange));
    }
}
