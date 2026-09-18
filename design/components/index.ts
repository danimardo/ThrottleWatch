export { default as Button } from './Button.svelte';
export { default as Switch } from './Switch.svelte';
export { default as SegmentedControl } from './SegmentedControl.svelte';
export { default as Select } from './Select.svelte';
export { default as Dialog } from './Dialog.svelte';
export { default as OptionRow } from './OptionRow.svelte';
export { default as Tooltip } from './Tooltip.svelte';
export { default as ProgressBar } from './ProgressBar.svelte';
export { default as Banner } from './Banner.svelte';
export { default as EmptyState } from './EmptyState.svelte';
export { default as OnboardingProgress } from './OnboardingProgress.svelte';
export { default as OnboardingSlide } from './OnboardingSlide.svelte';
export { default as OnboardingFlow } from './OnboardingFlow.svelte';
export type { OnboardingStepContent, OnboardingDetectionContent, DetectionStatus } from './OnboardingFlow.svelte';

export { default as SessionCard } from './SessionCard.svelte';
export type { SessionStatus } from './SessionCard.svelte';
export { default as SessionsScreen } from './SessionsScreen.svelte';
export type { SessionsListStatus, SessionsScreenSession } from './SessionsScreen.svelte';

export { default as CoreCell } from './CoreCell.svelte';
export type { CoreGroupKind, CoreMetricMode, CoreReading } from './CoreCell.svelte';
export { default as CpuTopologyMap } from './CpuTopologyMap.svelte';
export type { CoreGroup } from './CpuTopologyMap.svelte';
export { default as CpuAdvancedTable } from './CpuAdvancedTable.svelte';
export type {
  CoreTableRow,
  CoreTableSortColumn,
  CoreTableSortDirection,
  CoreTableColumnLabels
} from './CpuAdvancedTable.svelte';
export { default as CpuScreen } from './CpuScreen.svelte';

export { default as AnalysisChart } from './AnalysisChart.svelte';
export type {
  AnalysisTrackKind,
  AnalysisEventKind,
  AnalysisPoint,
  AnalysisTrack,
  AnalysisEvent
} from './AnalysisChart.svelte';
export { default as AnalysisScreen } from './AnalysisScreen.svelte';
export type {
  AnalysisStatus,
  AnalysisEvidenceItem,
  AnalysisEvidence
} from './AnalysisScreen.svelte';

export { default as ReportScreen } from './ReportScreen.svelte';
export type { ReportComparisonMetric } from './ReportScreen.svelte';

export { default as GuidedDiagnosticScreen } from './GuidedDiagnosticScreen.svelte';
export type {
  DiagnosticPhase,
  BatteryState,
  PreflightCheck,
  DiagnosticLiveReading,
  DiagnosticResult,
  WhatWillHappenItem
} from './GuidedDiagnosticScreen.svelte';

export { default as SettingsScreen } from './SettingsScreen.svelte';
export type {
  SettingsOption,
  SettingsUpdateStatus,
  SettingsSensorStatus,
  SettingsDangerStatus,
  SettingsCloseAction,
  SettingsSafetyLimit,
  SettingsGeneralSection,
  SettingsLanguageSection,
  SettingsAppearanceSection,
  SettingsMonitoringSection,
  SettingsTraySection,
  SettingsPrivacySection,
  SettingsSensorsSection,
  SettingsDiagnosticsSection,
  SettingsUpdatesSection,
  SettingsAboutLink,
  SettingsAboutSection,
  SettingsRiskZoneSection
} from './SettingsScreen.svelte';

export { default as TitleBar } from './TitleBar.svelte';
export { default as NavigationItem } from './NavigationItem.svelte';
export { default as BottomBar } from './BottomBar.svelte';
export type { BottomBarItem, BottomBarMenu, BottomBarMenuItem } from './BottomBar.svelte';

export { default as CoverageMatrix } from './CoverageMatrix.svelte';
export type { CoverageRow, CoverageQuality, CoverageColumnLabels } from './CoverageMatrix.svelte';
export { default as ContextStrip } from './ContextStrip.svelte';
export type { CollectorState } from './ContextStrip.svelte';
export { default as ExportDialog } from './ExportDialog.svelte';
export type { ExportFormat } from './ExportDialog.svelte';
export { default as ImportResultDialog } from './ImportResultDialog.svelte';
export type { ImportStatus } from './ImportResultDialog.svelte';
export { default as FirstCloseDialog } from './FirstCloseDialog.svelte';
export { default as CloseBlockedDialog } from './CloseBlockedDialog.svelte';
export type { CloseBlockedReason } from './CloseBlockedDialog.svelte';
export { default as WhatsNewCards } from './WhatsNewCards.svelte';
export type { WhatsNewCard } from './WhatsNewCards.svelte';
export { default as TechnicalSummary } from './TechnicalSummary.svelte';
export { default as LicensesScreen } from './LicensesScreen.svelte';
export type { LicenseEntry } from './LicensesScreen.svelte';
export type { AdvancedAccessState } from '../lib/access';
export { default as ToolbarButton } from './ToolbarButton.svelte';
export { default as StatusHero } from './StatusHero.svelte';
export { default as StatWidget } from './StatWidget.svelte';
export { default as StatusChip } from './StatusChip.svelte';
export { default as CausalRail } from './CausalRail.svelte';
export type { CausalNode } from './CausalRail.svelte';

export { default as StatusIcon } from '../icons/StatusIcon.svelte';
export { default as NavIcon } from '../icons/NavIcon.svelte';
export type { NavIconKind } from '../icons/NavIcon.svelte';

export { default as OnboardingWelcome } from '../illustrations/OnboardingWelcome.svelte';
export { default as OnboardingSignals } from '../illustrations/OnboardingSignals.svelte';
export { default as OnboardingConclusions } from '../illustrations/OnboardingConclusions.svelte';
export { default as OnboardingPrivacy } from '../illustrations/OnboardingPrivacy.svelte';
export { default as OnboardingThisComputer } from '../illustrations/OnboardingThisComputer.svelte';

export {
  CLASSIFICATION_META,
  OPPOSED_CLASSIFICATIONS,
  type Classification,
  type ClassificationMeta,
  type StatusIconKind
} from '../lib/classification';

export {
  THEMES,
  COLOR_TOKENS,
  SPACING_TOKENS,
  RADIUS_TOKENS,
  TYPE_STYLES,
  TONE_TOKENS,
  BREAKPOINTS,
  FONT_SANS,
  type Theme,
  type Tone,
  type TypeStyleName
} from '../tokens/tokens';

export { createWidthTracker, tierForWidth, type WidthTier } from '../lib/responsive.svelte';
