import { describe, expect, it } from 'vitest';
import { createTranslator, type Locale } from '../../lib/i18n';
import { narrativeFor } from './report-narrative';

const LOCALES: readonly Locale[] = ['es', 'en'];
const CLASSES = [
  'normal',
  'hot_unproven',
  'thermal_probable',
  'thermal_confirmed',
  'power_limited',
  'platform_limited',
  'mixed_limit',
  'indeterminate'
];
const raw = (key: string): string => key;

describe('report narrative', () => {
  it.each(LOCALES)(
    'has a real text in %s for every class and tier, never a raw key',
    (locale) => {
      const { t } = createTranslator(locale);
      for (const classification of CLASSES) {
        for (const coverage_tier of ['A', 'B', 'C']) {
          const narrative = narrativeFor(t, {
            classification,
            coverage_tier,
            severity: 'below_base',
            events: [{ kind: 'oem_mode_change' }]
          });
          const texts = [
            narrative.observed,
            ...narrative.alternativeCauses,
            ...narrative.cannotConclude,
            ...narrative.recommendations,
            ...(narrative.causalChain ?? []).map((node) => node.label),
            ...(narrative.causalChain ?? []).map((node) => node.value)
          ];
          for (const text of texts) {
            expect(text, `${classification}/${coverage_tier}`).not.toMatch(
              /reportNarrative\.|sessions\./
            );
            expect(text.length).toBeGreaterThan(3);
          }
        }
      }
    }
  );

  it('never recommends better cooling for a pure power limit (SC-017)', () => {
    const narrative = narrativeFor(raw, {
      classification: 'power_limited',
      coverage_tier: 'A'
    });
    expect(narrative.recommendations).not.toContain(
      'reportNarrative.recCooling'
    );
    expect(narrative.recommendations).toContain(
      'reportNarrative.recPowerLimits'
    );
    expect(narrative.cannotConclude).toContain('reportNarrative.noCoolingGain');
  });

  it('draws the causal chain only from direct reasons, and never with turbo end', () => {
    const chain = (classification: string, coverage_tier: string) =>
      narrativeFor(raw, { classification, coverage_tier, severity: 'boost' })
        .causalChain;
    expect(chain('thermal_confirmed', 'A')?.map((node) => node.kind)).toEqual([
      'load',
      'temperature',
      'clock'
    ]);
    expect(chain('mixed_limit', 'A')?.map((node) => node.kind)).toEqual([
      'load',
      'temperature',
      'power',
      'clock'
    ]);
    // A chain has 2 to 4 nodes and is not padded: outside tier A, or for a class without direct
    // evidence of a limit, there is no chain at all.
    expect(chain('thermal_confirmed', 'B')).toBeUndefined();
    expect(chain('thermal_probable', 'A')).toBeUndefined();
    expect(chain('normal', 'A')).toBeUndefined();
    for (const cls of CLASSES) {
      const nodes = chain(cls, 'A') ?? [];
      expect(
        nodes.length === 0 || (nodes.length >= 2 && nodes.length <= 4)
      ).toBe(true);
      expect(nodes.map((node) => node.id)).not.toContain('turbo_end');
    }
  });

  it('marks a frequency below the guaranteed base only when the report says so', () => {
    const clock = (severity: string) =>
      narrativeFor(raw, {
        classification: 'thermal_confirmed',
        coverage_tier: 'A',
        severity
      }).causalChain?.find((node) => node.kind === 'clock')?.value;
    expect(clock('below_base')).toBe('reportNarrative.chainBelowBase');
    expect(clock('boost')).toBe('reportNarrative.chainReduced');
  });

  it('lists the alternative causes the spec names', () => {
    expect(
      narrativeFor(raw, {
        classification: 'thermal_probable',
        coverage_tier: 'B'
      }).alternativeCauses
    ).toEqual(['reportNarrative.altSimultaneousPower']);
    const indeterminate = narrativeFor(raw, {
      classification: 'indeterminate',
      coverage_tier: 'A',
      events: [{ kind: 'oem_mode_change' }]
    });
    expect(indeterminate.alternativeCauses).toEqual([
      'reportNarrative.altWindowsPower',
      'reportNarrative.altOemMode'
    ]);
    expect(indeterminate.recommendations).toContain(
      'reportNarrative.recNoVentilationForOem'
    );
  });

  it('says a result below tier A is an inference and points to advanced access', () => {
    const narrative = narrativeFor(raw, {
      classification: 'thermal_probable',
      coverage_tier: 'C'
    });
    expect(narrative.cannotConclude).toContain(
      'reportNarrative.noDirectReasons'
    );
    expect(narrative.recommendations).toContain(
      'reportNarrative.recAdvancedAccess'
    );
    const direct = narrativeFor(raw, {
      classification: 'thermal_confirmed',
      coverage_tier: 'A'
    });
    expect(direct.cannotConclude).not.toContain(
      'reportNarrative.noDirectReasons'
    );
    expect(direct.recommendations).not.toContain(
      'reportNarrative.recAdvancedAccess'
    );
  });

  it('does not judge the coverage when the report does not carry its tier', () => {
    const narrative = narrativeFor(raw, { classification: 'thermal_probable' });
    expect(narrative.cannotConclude).not.toContain(
      'reportNarrative.noDirectReasons'
    );
    expect(narrative.recommendations).not.toContain(
      'reportNarrative.recAdvancedAccess'
    );
    expect(narrative.causalChain).toBeUndefined();
  });

  it('falls back to the generic text when the report has no known class', () => {
    expect(narrativeFor(raw, null).observed).toBe('sessions.reportObserved');
    expect(narrativeFor(raw, { classification: 'nonsense' }).causalChain).toBe(
      undefined
    );
  });
});
