#![deny(clippy::unwrap_used, clippy::expect_used)]

use crate::access::{self, AccessRequest, AccessRequestResult};
use crate::diagnostics::{AdvancedAccess, ConfidenceCeiling, CoverageSignals, CoverageTier};
use crate::storage::AppState;
use crate::telemetry::snapshot::Freshness;
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

#[derive(Debug, Clone, Serialize)]
pub struct CoverageDto {
    pub tier: CoverageTierDto,
    pub confidence_ceiling: ConfidenceDto,
    pub advanced_access: AdvancedAccessDto,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum CoverageTierDto {
    A,
    B,
    C,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfidenceDto {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdvancedAccessDto {
    NotNeeded,
    Available,
    Installable,
    Denied,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommandError {
    pub code: &'static str,
    pub message_key: &'static str,
}

impl CommandError {
    fn operation_failed() -> Self {
        Self { code: "LOW_LEVEL_ACCESS_OPERATION_FAILED", message_key: "access.operation_failed" }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CoverageMatrixDto {
    pub tier: CoverageTierDto,
    pub confidence_ceiling: ConfidenceDto,
    pub advanced_access: AdvancedAccessDto,
    pub rows: Vec<CoverageRowDto>,
    pub conclusion_key: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct CoverageRowDto {
    pub id: &'static str,
    pub label: &'static str,
    pub available: bool,
    pub quality: &'static str,
    pub quality_label: &'static str,
    pub source_label: &'static str,
    pub reason_key: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveSnapshotDto {
    pub captured_at_ms: u64,
    pub freshness: &'static str,
    pub age_ms: u64,
    pub temperature_c: Option<f64>,
    pub thermal_limit_c: Option<f64>,
    pub thermal_margin_c: Option<f64>,
    pub load_percent: Option<f64>,
    pub active_clock_mhz: Option<f64>,
    pub base_clock_mhz: Option<f64>,
    pub package_power_w: Option<f64>,
    pub power_limit_w: Option<f64>,
    pub classification: Option<&'static str>,
    pub severity: Option<&'static str>,
    pub cpu_label: &'static str,
    pub topology_label: &'static str,
    pub power_label: &'static str,
    pub collector_state: &'static str,
    pub coverage: CoverageDto,
    pub confidence_label: &'static str,
    pub active_cores: Option<u16>,
    pub platform_kind: Option<&'static str>,
    pub in_turbo_window: bool,
}

#[tauri::command]
pub fn get_coverage(state: State<'_, AppState>) -> CoverageMatrixDto {
    let enabled = state
        .storage
        .lock()
        .map(|storage| storage.advanced_access_enabled().unwrap_or(true))
        .unwrap_or(true);
    coverage_matrix(enabled)
}

fn coverage_matrix(advanced_access_enabled: bool) -> CoverageMatrixDto {
    let signals = CoverageSignals {
        temperature: false,
        active_clock: false,
        per_core_load: false,
        package_power: false,
        power_limit: false,
        limit_reasons: false,
    };
    let summary = signals.summary();
    CoverageMatrixDto {
        tier: tier(summary.tier),
        confidence_ceiling: confidence(summary.confidence_ceiling),
        advanced_access: if advanced_access_enabled {
            access(summary.advanced_access)
        } else {
            AdvancedAccessDto::Denied
        },
        rows: vec![
            row("temperature", "direct", "Directa", "CPU package", "coverage.temperature_missing"),
            row("active_clock", "derived", "Derivada", "PDH", "coverage.active_clock_missing"),
            row("power", "direct", "Directa", "CPU package", "coverage.power_missing"),
            row(
                "thermal_flag",
                "direct",
                "Directa",
                "MSR/SMU",
                "coverage.advanced_access_required",
            ),
        ],
        conclusion_key: "coverage.conclusion.c",
    }
}

#[tauri::command]
pub fn recheck_coverage(state: State<'_, AppState>) -> CoverageMatrixDto {
    get_coverage(state)
}

#[tauri::command]
pub fn get_live_snapshot(state: State<'_, AppState>) -> LiveSnapshotDto {
    let coverage = CoverageSignals {
        temperature: false,
        active_clock: false,
        per_core_load: false,
        package_power: false,
        power_limit: false,
        limit_reasons: false,
    };
    let summary = coverage.summary();
    LiveSnapshotDto {
        captured_at_ms: 0,
        freshness: freshness(Freshness::Disconnected),
        age_ms: 0,
        temperature_c: None,
        thermal_limit_c: None,
        thermal_margin_c: None,
        load_percent: None,
        active_clock_mhz: None,
        base_clock_mhz: None,
        package_power_w: None,
        power_limit_w: None,
        classification: Some("indeterminate"),
        severity: None,
        cpu_label: "CPU no detectada todavía",
        topology_label: "Esperando catálogo del colector",
        power_label: "Fuente desconocida",
        collector_state: "stopped",
        coverage: CoverageDto {
            tier: tier(summary.tier),
            confidence_ceiling: confidence(summary.confidence_ceiling),
            advanced_access: if state
                .storage
                .lock()
                .map(|storage| storage.advanced_access_enabled().unwrap_or(true))
                .unwrap_or(true)
            {
                access(summary.advanced_access)
            } else {
                AdvancedAccessDto::Denied
            },
        },
        confidence_label: "Confianza máxima alcanzable: baja",
        active_cores: None,
        platform_kind: None,
        in_turbo_window: false,
    }
}

#[tauri::command]
pub fn request_low_level_access(
    app: AppHandle,
    state: State<'_, AppState>,
    request: AccessRequest,
) -> Result<AccessRequestResult, CommandError> {
    let manifest = access::manifest().map_err(|_| CommandError::operation_failed())?;
    let resource_dir = app.path().resource_dir().map_err(|_| CommandError::operation_failed())?;
    let installer = access::installer_path(&resource_dir, &manifest);
    access::verify_installer(&installer, &manifest)
        .map_err(|_| CommandError::operation_failed())?;
    let launcher = std::env::current_exe().map_err(|_| CommandError::operation_failed())?;
    access::launch_installer_and_register_task(&installer, &launcher)
        .map_err(|_| CommandError::operation_failed())?;
    let guard = state.storage.lock().map_err(|_| CommandError::operation_failed())?;
    guard.set_advanced_access_enabled(true).map_err(|_| CommandError::operation_failed())?;
    Ok(AccessRequestResult {
        state: "install_requested",
        action: request.action,
        reboot_may_be_required: true,
    })
}

#[tauri::command]
pub fn disable_advanced_access(
    state: State<'_, AppState>,
) -> Result<CoverageMatrixDto, CommandError> {
    let guard = state.storage.lock().map_err(|_| CommandError::operation_failed())?;
    guard.set_advanced_access_enabled(false).map_err(|_| CommandError::operation_failed())?;
    Ok(coverage_matrix(false))
}

fn row(
    id: &'static str,
    quality: &'static str,
    quality_label: &'static str,
    source: &'static str,
    reason: &'static str,
) -> CoverageRowDto {
    CoverageRowDto {
        id,
        label: id,
        available: false,
        quality,
        quality_label,
        source_label: source,
        reason_key: Some(reason),
    }
}

fn tier(value: CoverageTier) -> CoverageTierDto {
    match value {
        CoverageTier::A => CoverageTierDto::A,
        CoverageTier::B => CoverageTierDto::B,
        CoverageTier::C => CoverageTierDto::C,
    }
}

fn confidence(value: ConfidenceCeiling) -> ConfidenceDto {
    match value {
        ConfidenceCeiling::Low => ConfidenceDto::Low,
        ConfidenceCeiling::Medium => ConfidenceDto::Medium,
        ConfidenceCeiling::High => ConfidenceDto::High,
    }
}

fn access(value: AdvancedAccess) -> AdvancedAccessDto {
    match value {
        AdvancedAccess::NotNeeded => AdvancedAccessDto::NotNeeded,
        AdvancedAccess::Available => AdvancedAccessDto::Available,
        AdvancedAccess::Installable => AdvancedAccessDto::Installable,
        AdvancedAccess::Denied => AdvancedAccessDto::Denied,
        AdvancedAccess::Error => AdvancedAccessDto::Error,
    }
}

fn freshness(value: Freshness) -> &'static str {
    match value {
        Freshness::Fresh => "fresh",
        Freshness::Stale => "stale",
        Freshness::Disconnected => "disconnected",
    }
}
