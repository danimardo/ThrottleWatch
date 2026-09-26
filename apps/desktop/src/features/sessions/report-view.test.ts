import { describe, expect, it } from 'vitest';
import { createTranslator } from '../../lib/i18n';
import {
  eventLines,
  evidenceLines,
  impactText,
  noImpactReason
} from './report-view';

const t = (key: string): string => `«${key}»`;

describe('report view', () => {
  it('names limit events instead of printing their internal codes', () => {
    const lines = eventLines(t, {
      events: [
        { kind: 'thermal', start_ms: 1000, end_ms: 13_000 },
        { kind: 'turbo_end', start_ms: 0, end_ms: 0 },
        { kind: 'something_new', start_ms: 0, end_ms: 5000 }
      ]
    });
    expect(lines).toEqual([
      '«sessions.eventThermal» · 12 «common.seconds»',
      '«sessions.eventTurboEnd» · 0 «common.seconds»',
      'something_new · 5 «common.seconds»'
    ]);
    expect(eventLines(t, null)).toEqual([]);
    expect(eventLines(t, { events: 'x' })).toEqual([]);
  });

  it('reports confidence, coverage and analysed interval before the events', () => {
    const lines = evidenceLines((key) => key, {
      confidence: 'medium',
      coverage_tier: 'B',
      analyzed_start_ms: 10_000,
      analyzed_end_ms: 70_000,
      events: [{ kind: 'power', start_ms: 0, end_ms: 2000 }]
    });
    expect(lines).toEqual([
      'sessions.reportConfidenceLine',
      'sessions.reportCoverageLine',
      'sessions.reportIntervalLine',
      'sessions.eventPower · 2 common.seconds'
    ]);
  });

  it('shows only the two figures the spec allows, and never raw JSON', () => {
    expect(impactText(t, null)).toBeUndefined();
    expect(impactText(t, { classification: 'normal' })).toBeUndefined();
    const cooling = impactText((key) => key, {
      cooling_potential: {
        low_percent: 5,
        high_percent: 15,
        method: 'power_headroom',
        inputs: { power_w: 41 }
      }
    });
    expect(cooling).toBe('sessions.reportCoolingImpact');
    expect(cooling).not.toContain('{');
    const guided = impactText((key) => `${key}:{percent}`, {
      guided_result: { relative_percent: 91.6 }
    });
    expect(guided).toBe('sessions.reportGuidedImpact:92');
  });

  it.each(['es', 'en'] as const)(
    'fills every placeholder of the real catalog texts in %s',
    (locale) => {
      const { t } = createTranslator(locale);
      const cooling = impactText(t, {
        cooling_potential: { low_percent: 5, high_percent: 15 }
      });
      expect(cooling).toContain('15');
      const guided = impactText(t, { guided_result: { relative_percent: 92 } });
      expect(guided).toContain('92');
      const lines = evidenceLines(t, {
        confidence: 'high',
        coverage_tier: 'A',
        analyzed_start_ms: 0,
        analyzed_end_ms: 60_000,
        events: [
          'thermal',
          'power',
          'platform',
          'mixed',
          'turbo_end',
          'oem_mode_change'
        ].map((kind) => ({ kind, start_ms: 0, end_ms: 1000 }))
      });
      expect(lines).toHaveLength(9);
      for (const text of [cooling, guided, ...lines]) {
        expect(text).not.toMatch(/[{}]/);
        expect(text).not.toMatch(/\b(sessions|common)\./);
      }
    }
  );

  it('explains that a pure power limit has no cooling figure', () => {
    expect(noImpactReason(t, { classification: 'power_limited' })).toBe(
      '«sessions.reportNoImpactPower»'
    );
    expect(noImpactReason(t, { classification: 'thermal_confirmed' })).toBe(
      '«sessions.reportNoImpactReason»'
    );
    expect(noImpactReason(t, null)).toBe('«sessions.reportNoImpactReason»');
  });
});
