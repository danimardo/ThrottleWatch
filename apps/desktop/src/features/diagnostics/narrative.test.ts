import { describe, expect, it } from 'vitest';
import { toNarrativeView } from './narrative';

describe('diagnostic narrative adapter', () => {
  it('exposes coverage, confidence and interval to every consumer', () => {
    const view = toNarrativeView({
      classification: 'thermal_confirmed',
      coverage: 'A',
      confidence: 0.9,
      severity: 'below_base',
      evidence: ['thermal_reason', 'below_base'],
      alternativeCauses: [],
      analyzedFromMs: 0,
      analyzedToMs: 60_000
    });
    expect(view.evidenceLine).toContain('cobertura A');
    expect(view.evidenceLine).toContain('intervalo 60 s');
    expect(view.causalChain).toHaveLength(2);
  });

  it('does not invent a causal chain for an informational turbo marker', () => {
    const view = toNarrativeView({
      classification: 'normal',
      coverage: 'B',
      confidence: 0.6,
      evidence: ['turbo_end'],
      alternativeCauses: []
    });
    expect(view.causalChain).toBeUndefined();
  });
});
