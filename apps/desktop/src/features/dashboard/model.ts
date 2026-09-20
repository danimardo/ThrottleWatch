import type { Classification } from '../../design-system/lib/classification';
import type { LiveSnapshot } from '../../lib/bridge/schemas';

export type CoverageTier = 'A' | 'B' | 'C';
export type Freshness = 'fresh' | 'stale' | 'disconnected';
export type AdvancedAccess =
  'not_needed' | 'available' | 'installable' | 'denied' | 'error';
export type Translate = (key: string) => string;

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
