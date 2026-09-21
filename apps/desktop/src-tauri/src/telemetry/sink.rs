//! Executes what the [`SessionRecorder`](super::recorder::SessionRecorder) decided against
//! storage: the only place where the recorder's pure actions become rows (sessions, frames,
//! limitations, the frozen report).
#![deny(clippy::unwrap_used, clippy::expect_used)]

use super::live::CpuInfo;
use super::recorder::RecorderAction;
use crate::storage::Storage;

/// The processor a session belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuIdentity {
    pub id: String,
    pub vendor: String,
    pub display_name: String,
    pub logical_processors: i64,
    pub hybrid: bool,
}

/// The processor the collector reported, or a plainly named placeholder before any catalog
/// arrived (a session never carries an invented model).
pub fn cpu_identity(cpu: Option<&CpuInfo>) -> CpuIdentity {
    match cpu {
        Some(cpu) => CpuIdentity {
            id: Storage::cpu_id(&cpu.vendor, &cpu.display_name, cpu.logical_processors),
            vendor: cpu.vendor.clone(),
            display_name: cpu.display_name.clone(),
            logical_processors: i64::from(cpu.logical_processors),
            hybrid: cpu.hybrid,
        },
        None => CpuIdentity {
            id: "cpu-unknown".to_owned(),
            vendor: "unknown".to_owned(),
            display_name: "CPU no identificada".to_owned(),
            logical_processors: 1,
            hybrid: false,
        },
    }
}

pub fn timestamp(epoch_ms: u64) -> String {
    i64::try_from(epoch_ms)
        .ok()
        .and_then(|millis| jiff::Timestamp::from_millisecond(millis).ok())
        .unwrap_or(jiff::Timestamp::UNIX_EPOCH)
        .to_string()
}

pub fn execute(
    storage: &Storage,
    cpu: &CpuIdentity,
    actions: &[RecorderAction],
) -> rusqlite::Result<()> {
    for action in actions {
        match action {
            RecorderAction::Open { session_id, split_reason, started_epoch_ms } => {
                let started_at = timestamp(*started_epoch_ms);
                storage.upsert_cpu(
                    &cpu.id,
                    &cpu.vendor,
                    &cpu.display_name,
                    cpu.logical_processors,
                    cpu.hybrid,
                    &started_at,
                )?;
                storage.create_session(session_id, &cpu.id, split_reason, &started_at)?;
            }
            RecorderAction::Frame { session_id, sequence, at_ms, duration_ms, values } => {
                storage.insert_sample(session_id, *sequence, *at_ms, *duration_ms, values)?;
            }
            RecorderAction::Event { session_id, kind, start_sequence, end_sequence } => {
                storage.insert_limit_event(session_id, kind, *start_sequence, *end_sequence)?;
            }
            RecorderAction::Close { session_id, ended_epoch_ms, duration_ms, tier, verdict } => {
                storage.set_session_coverage_tier(
                    session_id,
                    match tier {
                        crate::diagnostics::CoverageTier::A => "A",
                        crate::diagnostics::CoverageTier::B => "B",
                        crate::diagnostics::CoverageTier::C => "C",
                    },
                )?;
                storage.finish_session(
                    session_id,
                    "completed",
                    &timestamp(*ended_epoch_ms),
                    *duration_ms,
                    None,
                )?;
                storage.freeze_report(session_id, verdict.as_ref())?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{cpu_identity, execute};
    use crate::diagnostics::classifier::{Classification, DiagnosticResult, Severity};
    use crate::diagnostics::rules::{CoverageTier, Ruleset};
    use crate::storage::{Storage, StoredSampleValue};
    use crate::telemetry::clock::SampleQuality;
    use crate::telemetry::recorder::{RecorderInput, SessionRecorder};

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

    fn value(id: &str, number: f64) -> StoredSampleValue {
        StoredSampleValue {
            sensor_id: id.to_owned(),
            value: Some(number),
            boolean: None,
            quality: "direct".to_owned(),
        }
    }

    /// A stream replays into storage exactly as it would live: a real session, its frames, its
    /// limitation and a frozen report that says what the engine concluded.
    #[test]
    fn a_thermal_run_ends_up_as_a_session_with_frames_an_event_and_a_frozen_report()
    -> rusqlite::Result<()> {
        let storage = Storage::in_memory()?;
        let rules = Ruleset::v1().unwrap_or_else(|error| panic!("ruleset: {error}"));
        let mut recorder = SessionRecorder::new(rules)
            .unwrap_or_else(|| panic!("ruleset carries the session block"));
        let cpu = cpu_identity(None);
        let normal = verdict(Classification::Normal, vec!["stable"]);
        let thermal = verdict(Classification::ThermalConfirmed, vec!["thermal_reason"]);
        let start = 1_790_000_000_000_u64;

        let mut actions = Vec::new();
        for second in 0..150_u64 {
            let current = match second {
                0..=9 | 130.. => &normal,
                _ => &thermal,
            };
            actions.extend(recorder.observe(RecorderInput {
                epoch_ms: start + second * 1_000,
                values: vec![value("cpu.package.temp", 60.0 + second as f64 / 10.0)],
                verdict: Some(current),
                tier: CoverageTier::A,
                quality: SampleQuality::Complete,
                resumed: false,
            }));
        }
        actions.extend(recorder.close(start + 150_000));
        execute(&storage, &cpu, &actions)?;

        let sessions = storage.list_sessions(10)?;
        assert_eq!(sessions.len(), 1);
        let session = &sessions[0];
        assert_eq!((session.kind.as_str(), session.status.as_str()), ("passive", "completed"));
        assert_eq!(session.coverage_tier, "A");
        assert_eq!(session.frame_count, 150);
        assert_eq!(session.started_at, "2026-09-21T14:13:20Z");
        assert!(session.ended_at.is_some() && session.duration_ms == Some(149_000));
        assert_eq!(session.report_classification.as_deref(), Some("thermal_confirmed"));
        assert_eq!(storage.active_session_id()?, None, "a closed session frees the active slot");

        let events = storage.analysis_events(&session.session_id, 0, 200_000)?;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, "thermal");
        assert!(events[0].start_ms >= 9_000 && events[0].end_ms >= 129_000);
        let points = storage.analysis_points(&session.session_id, 0, 200_000)?;
        assert_eq!(points.len(), 150, "frames are placed relative to the session start");
        assert_eq!(points[0].monotonic_ms, 0);
        Ok(())
    }

    #[test]
    fn a_session_nobody_could_evaluate_is_reported_as_indeterminate() -> rusqlite::Result<()> {
        let storage = Storage::in_memory()?;
        let rules = Ruleset::v1().unwrap_or_else(|error| panic!("ruleset: {error}"));
        let mut recorder = SessionRecorder::new(rules)
            .unwrap_or_else(|| panic!("ruleset carries the session block"));
        let mut actions = Vec::new();
        for second in 0..5_u64 {
            actions.extend(recorder.observe(RecorderInput {
                epoch_ms: 1_790_000_000_000 + second * 1_000,
                values: vec![],
                verdict: None,
                tier: CoverageTier::C,
                quality: SampleQuality::Complete,
                resumed: false,
            }));
        }
        actions.extend(recorder.close(1_790_000_005_000));
        execute(&storage, &cpu_identity(None), &actions)?;
        let report = storage
            .list_sessions(1)?
            .into_iter()
            .next()
            .and_then(|session| session.report_classification);
        assert_eq!(report.as_deref(), Some("indeterminate"));
        Ok(())
    }

    #[test]
    fn the_cpu_is_the_one_the_collector_reported() {
        let cpu = super::CpuInfo {
            vendor: "amd".to_owned(),
            display_name: "Ryzen 5 2600X".to_owned(),
            logical_processors: 12,
            hybrid: false,
            virtualized: false,
        };
        let identity = cpu_identity(Some(&cpu));
        assert_eq!(identity.display_name, "Ryzen 5 2600X");
        assert_eq!(identity.id, Storage::cpu_id("amd", "Ryzen 5 2600X", 12));
        assert_eq!(cpu_identity(None).id, "cpu-unknown");
    }
}
