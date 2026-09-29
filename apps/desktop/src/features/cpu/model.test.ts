import { describe, expect, it } from 'vitest';
import { coreTemperatureView } from './model';

const t = (key: string): string =>
  ({
    'dashboard.celsius': '°C',
    'common.noValue': '—',
    'cpu.generalTemperature': '{value} (general)'
  })[key] ?? key;

describe('coreTemperatureView', () => {
  it('prefers the core\'s own temperature', () => {
    expect(coreTemperatureView(t, 61.4, 68)).toEqual({ label: '61 °C', general: false });
  });

  it('repeats the whole processor\'s temperature, marked as general, when a core has none', () => {
    expect(coreTemperatureView(t, null, 68.2)).toEqual({
      label: '68 °C (general)',
      general: true
    });
    expect(coreTemperatureView(t, undefined, 68.2).general).toBe(true);
  });

  it('shows a dash only when there is no temperature at all', () => {
    expect(coreTemperatureView(t, null, null)).toEqual({ label: '—', general: false });
  });
});
