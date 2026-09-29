export type Translate = (key: string) => string;

export interface CoreTemperatureView {
  label: string;
  /** True when the figure is the whole processor's, repeated on a core that has none of its own. */
  general: boolean;
}

/**
 * The temperature shown on one core. Some processors (Ryzen among them) only report a temperature for
 * the whole chip: instead of a dash on every core, that figure is repeated and marked as general so it
 * is not read as the core's own.
 */
export function coreTemperatureView(
  t: Translate,
  ownC: number | null | undefined,
  generalC: number | null | undefined
): CoreTemperatureView {
  const degrees = (value: number): string =>
    `${String(Math.round(value))} ${t('dashboard.celsius')}`;
  if (ownC !== null && ownC !== undefined) {
    return { label: degrees(ownC), general: false };
  }
  if (generalC !== null && generalC !== undefined) {
    return {
      label: t('cpu.generalTemperature').replace('{value}', degrees(generalC)),
      general: true
    };
  }
  return { label: t('common.noValue'), general: false };
}
