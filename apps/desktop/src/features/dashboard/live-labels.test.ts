import { describe, expect, it } from 'vitest';
import {
  coverageMatrixSchema,
  powerContextEventSchema
} from '../../lib/bridge/schemas';
import { createTranslator, type Locale } from '../../lib/i18n';
import {
  confidenceLabelFor,
  COVERAGE_REASON_KEYS,
  coverageReasonLabel,
  powerLabelFor
} from './model';

const LOCALES: readonly Locale[] = ['es', 'en'];

/**
 * The live events used to overwrite the labels Rust had already localized with strings written in
 * the component: `Maximum reachable confidence: low` (English, and the raw enum value at that) and
 * `Battery 64 %`, with the raw `ac` as the power label. Rust builds the same labels from the shared
 * catalog (`native.live.*`), so the interface uses the same texts.
 */
describe('the live confidence and power labels', () => {
  it.each(LOCALES)('name every confidence ceiling in %s', (locale) => {
    const { t } = createTranslator(locale);
    for (const ceiling of coverageMatrixSchema.shape.confidence_ceiling
      .options) {
      const label = confidenceLabelFor(t, ceiling);
      expect(label, ceiling).not.toBe('');
      expect(label, 'the raw enum value is not a label').not.toBe(ceiling);
      expect(label).not.toContain('native.live');
    }
  });

  it('is Spanish in Spanish and English in English', () => {
    expect(confidenceLabelFor(createTranslator('es').t, 'low')).toBe(
      'Confianza máxima alcanzable: baja'
    );
    expect(confidenceLabelFor(createTranslator('en').t, 'low')).toBe(
      'Maximum achievable confidence: low'
    );
  });

  it('never shows the raw power source, and adds the battery level when there is one', () => {
    const es = createTranslator('es').t;
    const en = createTranslator('en').t;
    expect(powerLabelFor(es, 'ac', null)).toBe('Corriente alterna');
    expect(powerLabelFor(en, 'ac', null)).toBe('AC power');
    expect(powerLabelFor(es, 'battery', 64)).toBe('Batería 64 %');
    expect(powerLabelFor(en, 'battery', undefined)).toBe('Battery');
    for (const source of powerContextEventSchema.shape.source.options) {
      for (const locale of LOCALES) {
        expect(
          powerLabelFor(createTranslator(locale).t, source, null)
        ).not.toBe(source);
      }
    }
  });
});

describe('the coverage matrix reasons', () => {
  it.each(LOCALES)(
    'have a text in %s for every reason the backend emits',
    (locale) => {
      const { t } = createTranslator(locale);
      for (const key of COVERAGE_REASON_KEYS) {
        const label = coverageReasonLabel(t, key);
        expect(label, key).not.toBe(key);
        expect(label, key).not.toMatch(/^coverage\./);
      }
    }
  );

  it('falls back to a generic text, never to a raw key, for one it does not know', () => {
    const { t } = createTranslator('es');
    expect(coverageReasonLabel(t, 'coverage.something_new')).toBe(
      'No disponible'
    );
    expect(coverageReasonLabel(t, null)).toBe('No disponible');
  });
});
