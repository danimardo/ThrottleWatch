import type { Classification } from '../../design-system/lib/classification';

type Translate = (key: string) => string;

const CLASSIFICATIONS: readonly Classification[] = [
  'normal',
  'hot_unproven',
  'thermal_probable',
  'thermal_confirmed',
  'power_limited',
  'platform_limited',
  'mixed_limit',
  'indeterminate'
];

const LABEL_KEYS: Record<Classification, string> = {
  normal: 'sessions.classificationNormal',
  hot_unproven: 'sessions.classificationHotUnproven',
  thermal_probable: 'sessions.classificationThermalProbable',
  thermal_confirmed: 'sessions.classificationThermalConfirmed',
  power_limited: 'sessions.classificationPowerLimited',
  platform_limited: 'sessions.classificationPlatformLimited',
  mixed_limit: 'sessions.classificationMixedLimit',
  indeterminate: 'sessions.classificationIndeterminate'
};

export function classificationOf(value: unknown): Classification {
  return CLASSIFICATIONS.find((item) => item === value) ?? 'indeterminate';
}

export function classificationLabelFor(t: Translate, value: unknown): string {
  return t(LABEL_KEYS[classificationOf(value)]);
}
