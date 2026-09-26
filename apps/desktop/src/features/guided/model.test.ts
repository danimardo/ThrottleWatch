import { describe, expect, it } from 'vitest';
import {
  batteryStateOf,
  percentRemaining,
  preflightChecks,
  toDiagnosticPhase
} from './model';

describe('guided model', () => {
  it('maps backend phases to design-system phases', () => {
    expect(toDiagnosticPhase('steady_load')).toBe('steadyLoad');
    expect(toDiagnosticPhase('safety_stop')).toBe('safetyStop');
  });

  it('turns preflight failures into visible checks', () => {
    const checks = preflightChecks(
      {
        sensors: true,
        ac_power: false,
        profile: true,
        disk_space: true,
        generator: true,
        require_ac: true
      },
      (key) => `«${key}»`,
      true
    );
    expect(
      checks.find((check) => check.label === '«guided.checks.power»')?.status
    ).toBe('failed');
    expect(
      checks.every((check) => check.label.startsWith('«guided.checks.'))
    ).toBe(true);
  });

  it('blocks starting on battery only when the person asked for AC, and warns otherwise', () => {
    const base = {
      sensors: true,
      profile: true,
      disk_space: true,
      generator: true,
      require_ac: true
    };
    expect(batteryStateOf({ ...base, ac_power: false }, true)).toBe('blocked');
    expect(batteryStateOf({ ...base, ac_power: false }, false)).toBe('warning');
    expect(batteryStateOf({ ...base, ac_power: true }, true)).toBe('ok');
    expect(batteryStateOf(null, true)).toBe('ok');
  });

  it('does not fail the power check on battery when AC is not required', () => {
    const value = {
      sensors: true,
      ac_power: false,
      profile: true,
      disk_space: true,
      generator: true,
      require_ac: true
    };
    const power = (requireAc: boolean) =>
      preflightChecks(value, (key) => key, requireAc).find(
        (check) => check.label === 'guided.checks.power'
      )?.status;
    expect(power(true)).toBe('failed');
    expect(power(false)).toBe('ok');
  });

  it('calculates progress without inventing a duration', () => {
    expect(percentRemaining(50, 50)).toBe(50);
    expect(percentRemaining(50, null)).toBeUndefined();
  });
});
