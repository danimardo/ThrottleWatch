import es from './locales/es.json';
import en from './locales/en.json';

export type LanguageMode = 'system' | 'es' | 'en';
export type Locale = 'es' | 'en';
export type CatalogValue = string | Catalog;
export type Catalog = { readonly [key: string]: CatalogValue };

export const catalogs: Record<Locale, Catalog> = { es, en };

const SPANISH_FAMILY = ['es', 'ca', 'gl', 'eu', 'ast', 'an'];

/** The interface language of one language tag, if the application has it. */
function localeOf(tag: string): Locale | undefined {
  const language =
    tag.trim().toLowerCase().replace('_', '-').split('-')[0] ?? '';
  if (SPANISH_FAMILY.includes(language)) return 'es';
  if (language === 'en') return 'en';
  return undefined;
}

/**
 * `system` takes the **first** language of the person's ranked list that the application has, so a
 * person who lists Spanish before English gets Spanish and one who lists French then English gets
 * English; English when nothing matches. The same rule as `i18n.rs` (`resolve`). The interface
 * normally does not apply it itself — it asks Rust (`get_effective_locale`) — and keeps this only
 * for when Rust cannot answer, such as outside the Tauri window.
 */
export function resolveLocale(
  mode: LanguageMode,
  languages: string | readonly string[]
): Locale {
  if (mode === 'es') return 'es';
  if (mode === 'en') return 'en';
  const list = typeof languages === 'string' ? [languages] : languages;
  for (const tag of list) {
    const found = localeOf(tag);
    if (found !== undefined) return found;
  }
  return 'en';
}

export function createTranslator(
  mode: LanguageMode = 'system',
  windowsLocale: string | readonly string[] = 'en-US'
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
