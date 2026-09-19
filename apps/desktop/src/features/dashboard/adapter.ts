import type { Classification } from '../../design-system/lib/classification';
import type { DashboardSnapshot } from './model';

export interface StatusHeroView {
  ringValue: string;
  ringCaption: string;
  ringPercent: number;
  classification: Classification;
  classificationLabel: string;
  evidenceLine: string;
  performance: { label: string; rangeText: string } | null;
  noPotentialText: string;
  severity?: 'boost' | 'below_base';
}

const labels: Record<Classification, string> = {
  normal: 'NO LIMITATION DETECTED',
  hot_unproven: 'HIGH TEMPERATURE',
  thermal_probable: 'PROBABLE THERMAL LIMIT',
  thermal_confirmed: 'CONFIRMED THERMAL LIMIT',
  power_limited: 'POWER LIMIT',
  platform_limited: 'EQUIPMENT LIMIT',
  mixed_limit: 'MIXED LIMIT',
  indeterminate: 'INSUFFICIENT DATA'
};

export function toStatusHeroView(snapshot: DashboardSnapshot): StatusHeroView {
  const ringValue = snapshot.temperatureC === null ? '—' : `${String(Math.round(snapshot.temperatureC))}°`;
  const ringCaption = snapshot.thermalLimitC === null ? 'Limit unavailable' : `Limit ${String(Math.round(snapshot.thermalLimitC))}°`;
  const ringPercent = snapshot.temperatureC !== null && snapshot.thermalLimitC !== null
    ? Math.min(100, Math.max(0, (snapshot.temperatureC / snapshot.thermalLimitC) * 100))
    : 0;
  const performance = snapshot.coolingPotential
    ? {
        label: 'Cooling potential',
        rangeText: `+${String(snapshot.coolingPotential.lowPercent)}–${String(snapshot.coolingPotential.highPercent)} %`
      }
    : null;
  return {
    ringValue,
    ringCaption,
    ringPercent,
    classification: snapshot.classification,
    classificationLabel: labels[snapshot.classification],
    evidenceLine: `${snapshot.confidenceLabel} · ${snapshot.collectorLabel}`,
    performance,
    noPotentialText: 'Not quantifiable for this equipment',
    severity: snapshot.severity
  };
}
