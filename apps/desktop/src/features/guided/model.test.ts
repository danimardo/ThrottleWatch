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
      (key) => `«${key}»`
    );
    expect(
      checks.find((check) => check.label === '«guided.checks.power»')?.status
    ).toBe('failed');
    expect(
      checks.every((check) => check.label.startsWith('«guided.checks.'))
    ).toBe(true);
  });

  it('blocks starting on battery only when the test requires AC', () => {
    const base = {
      sensors: true,
      profile: true,
      disk_space: true,
      generator: true
    };
    expect(batteryStateOf({ ...base, ac_power: false, require_ac: true })).toBe(
      'blocked'
    );
    expect(
      batteryStateOf({ ...base, ac_power: false, require_ac: false })
    ).toBe('ok');
    expect(batteryStateOf({ ...base, ac_power: true, require_ac: true })).toBe(
      'ok'
    );
    expect(batteryStateOf(null)).toBe('ok');
  });

  it('calculates progress without inventing a duration', () => {
    expect(percentRemaining(50, 50)).toBe(50);
    expect(percentRemaining(50, null)).toBeUndefined();
  });
});
