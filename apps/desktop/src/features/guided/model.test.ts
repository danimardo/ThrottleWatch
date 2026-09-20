import { describe, expect, it } from 'vitest';
import { percentRemaining, preflightChecks, toDiagnosticPhase } from './model';

describe('guided model', () => {
  it('maps backend phases to design-system phases', () => {
    expect(toDiagnosticPhase('steady_load')).toBe('steadyLoad');
    expect(toDiagnosticPhase('safety_stop')).toBe('safetyStop');
  });

  it('turns preflight failures into visible checks', () => {
    const checks = preflightChecks({
      sensors: true,
      ac_power: false,
      profile: true,
      disk_space: true,
      generator: true,
      require_ac: true
    });
    expect(checks.find((check) => check.label === 'Alimentación')?.status).toBe(
      'failed'
    );
  });

  it('calculates progress without inventing a duration', () => {
    expect(percentRemaining(50, 50)).toBe(50);
    expect(percentRemaining(50, null)).toBeUndefined();
  });
});
