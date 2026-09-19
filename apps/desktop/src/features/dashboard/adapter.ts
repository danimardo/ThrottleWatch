import type { Classification } from '../../design-system/lib/classification';
import type { DashboardSnapshot, Translate } from './model';

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

const labelKeys: Record<Classification, string> = {
  normal: 'dashboard.noLimit',
  hot_unproven: 'dashboard.highTemperature',
  thermal_probable: 'dashboard.probableThermal',
  thermal_confirmed: 'dashboard.confirmedThermal',
  power_limited: 'dashboard.powerLimit',
  platform_limited: 'dashboard.equipmentLimit',
  mixed_limit: 'dashboard.mixedLimit',
  indeterminate: 'dashboard.insufficientData'
};

export function toStatusHeroView(
  snapshot: DashboardSnapshot,
  t: Translate,
  locale = 'es-ES'
): StatusHeroView {
  const ringValue =
    snapshot.temperatureC === null
      ? '—'
      : `${String(Math.round(snapshot.temperatureC))}°`;
  const ringCaption =
    snapshot.thermalLimitC === null
      ? t('dashboard.limitUnavailable')
      : `${locale === 'es-ES' ? 'Límite' : 'Limit'} ${String(Math.round(snapshot.thermalLimitC))}°`;
  const ringPercent =
    snapshot.temperatureC !== null && snapshot.thermalLimitC !== null
      ? Math.min(
          100,
          Math.max(0, (snapshot.temperatureC / snapshot.thermalLimitC) * 100)
        )
      : 0;
  const performance = snapshot.coolingPotential
    ? {
        label: t('dashboard.coolingPotential'),
        rangeText: `+${String(snapshot.coolingPotential.lowPercent)}–${String(snapshot.coolingPotential.highPercent)} ${t('dashboard.percent')}`
      }
    : null;
  return {
    ringValue,
    ringCaption,
    ringPercent,
    classification: snapshot.classification,
    classificationLabel: t(labelKeys[snapshot.classification]),
    evidenceLine: `${snapshot.confidenceLabel} · ${snapshot.collectorLabel}`,
    performance,
    noPotentialText: t('dashboard.notQuantifiable'),
    severity: snapshot.severity
  };
}
