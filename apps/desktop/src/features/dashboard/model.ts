import type { Classification } from '../../design-system/lib/classification';
import type { LiveSnapshot } from '../../lib/bridge/schemas';
import { catalogs } from '../../lib/i18n';

export type CoverageTier = 'A' | 'B' | 'C';
export type Freshness = 'fresh' | 'stale' | 'disconnected';
export type AdvancedAccess =
  | 'not_needed'
  | 'available'
  | 'installable'
  | 'upgradable'
  | 'denied'
  | 'error'
  | 'capped_by_vendor';
export type Translate = (key: string) => string;

/**
 * The maximum-confidence line and the power line, from the same `native.live.*` texts Rust uses for
 * the snapshot. The live events used to replace them with strings written in the component
 * (`Maximum reachable confidence: low`, `Battery 64 %`, and the raw `ac`), so a coverage or power
 * change turned the Spanish label Rust had produced into English.
 */
export function confidenceLabelFor(
  t: Translate,
  ceiling: 'low' | 'medium' | 'high'
): string {
  return t(`native.live.confidence.${ceiling}`);
}

export function powerLabelFor(
  t: Translate,
  source: 'ac' | 'battery' | 'unknown',
  batteryPercent: number | null | undefined
): string {
  if (source === 'battery') {
    const base = t('native.live.power.battery');
    return batteryPercent === undefined || batteryPercent === null
      ? base
      : `${base} ${String(batteryPercent)} %`;
  }
  return t(
    source === 'ac' ? 'native.live.power.ac' : 'native.live.power.unknown'
  );
}

/**
 * The reasons Rust gives a coverage row that is not available: whatever is under `coverage` in the
 * catalogs, so there is one list and it is the texts themselves. (A test in `i18n.rs` makes sure
 * every reason the backend can emit is in there.)
 */
const coverageCatalog = catalogs.en.coverage;
export const COVERAGE_REASON_KEYS: readonly string[] =
  typeof coverageCatalog === 'object'
    ? Object.keys(coverageCatalog).map((name) => `coverage.${name}`)
    : [];

/**
 * The text of a row's reason. The row carries a catalog *key*, and the matrix used to print it as
 * it came (`coverage.temperature_missing`). A key that is not one of ours — a newer backend, say —
 * gets the generic "not available" rather than the key itself.
 */
export function coverageReasonLabel(
  t: Translate,
  key: string | null | undefined
): string {
  return key !== null && key !== undefined && COVERAGE_REASON_KEYS.includes(key)
    ? t(key)
    : t('dashboard.unavailable');
}

/** Catalog key of every collector state the backend can report (`collector_state`). */
export const COLLECTOR_LABEL_KEYS: Readonly<Record<string, string>> = {
  starting: 'dashboard.collectorStarting',
  running: 'dashboard.collectorRunning',
  degraded: 'dashboard.collectorDegraded',
  restarting: 'dashboard.collectorRestarting',
  stopped: 'dashboard.collectorStopped',
  failed: 'dashboard.collectorFailed'
};

export interface DashboardSnapshot {
  cpuLabel: string;
  topologyLabel: string;
  powerLabel: string;
  collectorState: Freshness;
  collectorLabel: string;
  classification: Classification;
  severity?: 'boost' | 'below_base';
  temperatureC: number | null;
  thermalLimitC: number | null;
  loadPercent: number | null;
  activeClockMhz: number | null;
  baseClockMhz: number | null;
  packagePowerW: number | null;
  powerLimitW: number | null;
  coverage: CoverageTier;
  advancedAccess: AdvancedAccess;
  confidenceLabel: string;
  coolingPotential?: {
    lowPercent: number;
    highPercent: number;
    method: 'power_headroom';
  };
}

export const DEMO_SNAPSHOT: DashboardSnapshot = {
  cpuLabel: 'CPU not detected yet',
  topologyLabel: 'Waiting for collector catalog',
  powerLabel: 'Unknown power source',
  collectorState: 'disconnected',
  collectorLabel: 'Collector disconnected',
  classification: 'indeterminate',
  temperatureC: null,
  thermalLimitC: null,
  loadPercent: null,
  activeClockMhz: null,
  baseClockMhz: null,
  packagePowerW: null,
  powerLimitW: null,
  coverage: 'C',
  advancedAccess: 'installable',
  confidenceLabel: 'Maximum reachable confidence: low'
};

export function createDemoSnapshot(t: Translate): DashboardSnapshot {
  return {
    ...DEMO_SNAPSHOT,
    cpuLabel: t('dashboard.demoCpu'),
    topologyLabel: t('dashboard.demoTopology'),
    powerLabel: t('dashboard.demoPower'),
    collectorLabel: t('dashboard.demoCollector'),
    confidenceLabel: t('dashboard.confidenceLow')
  };
}

export function formatNumber(
  value: number | null,
  digits = 0,
  locale = 'es-ES',
  unavailable = 'Unavailable'
): string {
  if (value === null || !Number.isFinite(value)) return unavailable;
  return value.toLocaleString(locale, {
    maximumFractionDigits: digits,
    minimumFractionDigits: digits
  });
}

export function marginText(
  snapshot: DashboardSnapshot,
  t: Translate,
  locale = 'es-ES'
): string {
  if (snapshot.temperatureC === null || snapshot.thermalLimitC === null) {
    return t('dashboard.limitUnavailable');
  }
  const suffix = locale === 'es-ES' ? 'hasta el límite' : 'to limit';
  return `${formatNumber(snapshot.thermalLimitC - snapshot.temperatureC, 0, locale, t('dashboard.unavailableShort'))} ${t('dashboard.celsius')} ${suffix}`;
}

export function fromLiveSnapshot(value: LiveSnapshot): DashboardSnapshot {
  const collectorState =
    value.collector_state === 'running'
      ? 'fresh'
      : value.collector_state === 'degraded' ||
          value.collector_state === 'restarting'
        ? 'stale'
        : 'disconnected';
  return {
    cpuLabel: value.cpu_label,
    topologyLabel: value.topology_label,
    powerLabel: value.power_label,
    collectorState,
    collectorLabel: value.collector_state,
    classification: value.classification ?? 'indeterminate',
    severity: value.severity ?? undefined,
    temperatureC: value.temperature_c,
    thermalLimitC: value.thermal_limit_c,
    loadPercent: value.load_percent,
    activeClockMhz: value.active_clock_mhz,
    baseClockMhz: value.base_clock_mhz,
    packagePowerW: value.package_power_w,
    powerLimitW: value.power_limit_w,
    coverage: value.coverage.tier,
    advancedAccess: value.coverage.advanced_access,
    confidenceLabel: value.confidence_label,
    coolingPotential: value.cooling_potential
      ? {
          lowPercent: value.cooling_potential.low_percent,
          highPercent: value.cooling_potential.high_percent,
          method: value.cooling_potential.method
        }
      : undefined
  };
}

const COVERAGE_ROW_IDS = ['temperature', 'active_clock', 'power', 'thermal_flag'];
const COVERAGE_SOURCES: Readonly<Record<string, string>> = {
  'CPU package': 'cpu_package',
  LibreHardwareMonitor: 'librehardwaremonitor',
  'MSR/SMU': 'msr_smu',
  PDH: 'pdh'
};

export interface CoverageRowView {
  id: string;
  label: string;
  available: boolean;
  quality: 'direct' | 'derived' | 'substitute';
  qualityLabel: string;
  sourceLabel: string;
  reasonLabel: string;
}

/**
 * One row of the coverage table, in the person's words. The backend sends identifiers (`active_clock`)
 * and, for quality, a Spanish word; both used to reach the screen as they were — in the wrong
 * language, and after "Comprobar de nuevo" with the raw reason key too.
 */
export function coverageRowView(
  t: Translate,
  row: {
    id: string;
    available: boolean;
    quality?: string;
    source_label?: string | null;
    reason_key?: string | null;
  }
): CoverageRowView {
  const quality =
    row.quality === 'derived' || row.quality === 'substitute'
      ? row.quality
      : 'direct';
  const source = row.source_label ?? undefined;
  const sourceKey = source === undefined ? undefined : COVERAGE_SOURCES[source];
  return {
    id: row.id,
    label: t(
      `dashboard.coverageRow.${COVERAGE_ROW_IDS.includes(row.id) ? row.id : 'unknown'}`
    ),
    available: row.available,
    quality,
    qualityLabel: t(`dashboard.coverageQuality.${quality}`),
    sourceLabel:
      source === undefined
        ? t('dashboard.unavailable')
        : sourceKey === undefined
          ? source
          : t(`dashboard.coverageSource.${sourceKey}`),
    reasonLabel: coverageReasonLabel(t, row.reason_key)
  };
}
