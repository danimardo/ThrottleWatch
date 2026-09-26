import { get, writable } from 'svelte/store';
import { invokeValidated, type BridgeTransport } from '../bridge';
import { commandResponseSchemas } from '../bridge/schemas';
import { createTranslator, resolveLocale, type Locale } from './index';

/**
 * The interface language in force. Rust decides it (`get_effective_locale`: the stored
 * `locale.mode`, and for `system` the first language in Windows' ranked list that the application
 * has) and the interface asks.
 *
 * Every component used to build its own translator with `createTranslator('system',
 * navigator.language)`: the stored choice was never read, so picking Spanish saved `"es"` and the
 * screen stayed as it was, and `system` depended on one value from the WebView — which is also what
 * Rust's own texts disagreed with, hence an interface half English and half Spanish.
 */
export const localeStore = writable<Locale>('en');

/** What the browser's own language list says, for when Rust cannot answer (outside the app). */
function browserLocale(): Locale {
  if (typeof navigator === 'undefined') return 'en';
  const list =
    navigator.languages.length > 0 ? navigator.languages : [navigator.language];
  return resolveLocale('system', list);
}

/** Asks Rust for the language in force and adopts it; every screen is rebuilt on a change. */
export async function refreshLocale(
  transport?: BridgeTransport
): Promise<Locale> {
  const result = await invokeValidated(
    'get_effective_locale',
    undefined,
    commandResponseSchemas.get_effective_locale,
    transport
  );
  const locale = result.ok ? result.value : browserLocale();
  localeStore.set(locale);
  return locale;
}

/** The translator for the language in force *now*. Screens are rebuilt when it changes. */
export function getTranslator(): ReturnType<typeof createTranslator> {
  return createTranslator(get(localeStore));
}
