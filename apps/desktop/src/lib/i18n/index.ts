import es from './locales/es.json';
import en from './locales/en.json';

export type LanguageMode = 'system' | 'es' | 'en';
export type Locale = 'es' | 'en';
export type CatalogValue = string | Catalog;
export type Catalog = { readonly [key: string]: CatalogValue };

export const catalogs: Record<Locale, Catalog> = { es, en };

export function resolveLocale(
  mode: LanguageMode,
  windowsLocale: string
): Locale {
  if (mode === 'es') return 'es';
  if (mode === 'en') return 'en';
  const normalized = windowsLocale.trim().toLowerCase().replace('_', '-');
  const language = normalized.split('-')[0] ?? '';
  return ['es', 'ca', 'gl', 'eu', 'ast', 'an'].includes(language) ? 'es' : 'en';
}

export function createTranslator(
  mode: LanguageMode = 'system',
  windowsLocale = 'en-US'
): { locale: Locale; t: (key: string) => string } {
  const locale = resolveLocale(mode, windowsLocale);
  const catalog = catalogs[locale];
  return {
    locale,
    t: (key) => {
      const value = key
        .split('.')
        .reduce<CatalogValue | undefined>((current, segment) => {
          if (current === undefined || typeof current === 'string')
            return undefined;
          return current[segment];
        }, catalog);
      if (typeof value !== 'string')
        throw new Error(`Missing catalog key: ${key}`);
      return value;
    }
  };
}

export function catalogKeys(catalog: Catalog, prefix = ''): string[] {
  const keys: string[] = [];
  for (const key of Object.keys(catalog)) {
    const value = catalog[key];
    const fullKey = prefix === '' ? key : `${prefix}.${key}`;
    if (typeof value === 'string') {
      keys.push(fullKey);
      continue;
    }
    if (typeof value === 'object') {
      keys.push(...catalogKeys(value, fullKey));
      continue;
    }
  }
  return keys;
}
