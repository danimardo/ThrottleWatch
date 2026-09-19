import type { Classification } from '../../design-system/lib/classification';
import type { LiveSnapshot } from '../../lib/bridge/schemas';

export type CoverageTier = 'A' | 'B' | 'C';
export type Freshness = 'fresh' | 'stale' | 'disconnected';
export type AdvancedAccess = 'not_needed' | 'available' | 'installable' | 'denied' | 'error';

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
  coolingPotential?: { lowPercent: number; highPercent: number; method: 'power_headroom' };
}

export const DEMO_SNAPSHOT: DashboardSnapshot = {
  cpuLabel: 'CPU no detectada todavía',
  topologyLabel: 'Esperando catálogo del colector',
  powerLabel: 'Fuente desconocida',
  collectorState: 'disconnected',
  collectorLabel: 'Colector desconectado',
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
  confidenceLabel: 'Confianza máxima alcanzable: baja'
};

export function formatNumber(value: number | null, digits = 0): string {
  if (value === null || !Number.isFinite(value)) return 'No disponible';
  return value.toLocaleString('es-ES', { maximumFractionDigits: digits, minimumFractionDigits: digits });
}

export function marginText(snapshot: DashboardSnapshot): string {
  if (snapshot.temperatureC === null || snapshot.thermalLimitC === null) return 'Límite térmico no disponible';
  return `${formatNumber(snapshot.thermalLimitC - snapshot.temperatureC)} °C hasta el límite`;
}

export function fromLiveSnapshot(value: LiveSnapshot): DashboardSnapshot {
  const collectorState = value.collector_state === 'running'
    ? 'fresh'
    : value.collector_state === 'degraded' || value.collector_state === 'restarting'
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
      ? { lowPercent: value.cooling_potential.low_percent, highPercent: value.cooling_potential.high_percent, method: value.cooling_potential.method }
      : undefined
  };
}
