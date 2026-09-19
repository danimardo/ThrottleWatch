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

  it('lets an explicit language override Windows', () => {
    expect(createTranslator('en', 'es-ES').t('nav.settings')).toBe('Settings');
    expect(createTranslator('es', 'en-US').t('nav.settings')).toBe('Ajustes');
  });
});
