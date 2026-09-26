import { describe, expect, it } from 'vitest';
import {
  catalogKeys,
  catalogs,
  createTranslator,
  resolveLocale
} from './index';

describe('catalogs', () => {
  it('keeps Spanish and English key sets identical', () => {
    expect(catalogKeys(catalogs.es).sort()).toEqual(
      catalogKeys(catalogs.en).sort()
    );
  });

  it.each([
    ['es-ES', 'es'],
    ['ca-AD', 'es'],
    ['gl-ES', 'es'],
    ['eu-ES', 'es'],
    ['ast-ES', 'es'],
    ['an-ES', 'es'],
    ['pt-PT', 'en'],
    ['de-DE', 'en']
  ] as const)('resolves %s to %s in system mode', (windowsLocale, expected) => {
    expect(resolveLocale('system', windowsLocale)).toBe(expected);
  });

  it.each([
    // The person's own ranking decides: the first language the application has.
    [['es-ES', 'en-US'], 'es'],
    [['en-US', 'es-ES'], 'en'],
    // An unsupported language is skipped, not taken as the answer.
    [['fr-FR', 'es-ES'], 'es'],
    [['de-DE', 'en-GB'], 'en'],
    // Nothing we have: English.
    [['fr-FR'], 'en'],
    [['pt-PT', 'de'], 'en'],
    [[], 'en']
  ] as const)(
    'resolves the list %j to %s in system mode',
    (languages, expected) => {
      expect(resolveLocale('system', languages)).toBe(expected);
    }
  );

  it('never lets the list override an explicit choice', () => {
    expect(resolveLocale('es', ['en-US'])).toBe('es');
    expect(resolveLocale('en', ['es-ES'])).toBe('en');
  });

  it('lets an explicit language override Windows', () => {
    expect(createTranslator('en', 'es-ES').t('nav.settings')).toBe('Settings');
    expect(createTranslator('es', 'en-US').t('nav.settings')).toBe('Ajustes');
  });
});
