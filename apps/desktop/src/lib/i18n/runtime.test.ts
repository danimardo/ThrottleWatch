import { get } from 'svelte/store';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { BridgeTransport } from '../bridge';
import { getTranslator, localeStore, refreshLocale } from './runtime';

const answering = (value: unknown): BridgeTransport => ({
  invoke: () => Promise.resolve(value),
  listen: () => Promise.resolve(() => undefined)
});

const failing: BridgeTransport = {
  invoke: () => Promise.reject(new Error('the backend is not there')),
  listen: () => Promise.resolve(() => undefined)
};

afterEach(() => {
  vi.unstubAllGlobals();
  localeStore.set('en');
});

describe('the interface language', () => {
  it('is whatever Rust says, and that overrides what the browser reports', async () => {
    // The browser says English, as the WebView did on a machine whose display language was English
    // while the person's Windows list ranked Spanish first. Rust knows the list; the browser did not.
    vi.stubGlobal('navigator', { language: 'en-US', languages: ['en-US'] });

    expect(await refreshLocale(answering('es'))).toBe('es');
    expect(get(localeStore)).toBe('es');
    expect(getTranslator().t('nav.settings')).toBe('Ajustes');
  });

  it('changes when the choice changes, without restarting', async () => {
    await refreshLocale(answering('es'));
    expect(getTranslator().t('nav.settings')).toBe('Ajustes');

    await refreshLocale(answering('en'));
    expect(getTranslator().t('nav.settings')).toBe('Settings');
  });

  it('falls back to the first supported language of the browser when Rust cannot answer', async () => {
    vi.stubGlobal('navigator', {
      language: 'fr-FR',
      languages: ['fr-FR', 'es-ES', 'en-US']
    });

    expect(await refreshLocale(failing)).toBe('es');
  });

  it('does not trust an answer that is not a language it has', async () => {
    vi.stubGlobal('navigator', { language: 'en-US', languages: ['en-US'] });

    // Not `es` or `en`: the schema refuses it and the browser's list decides instead.
    expect(await refreshLocale(answering('klingon'))).toBe('en');
  });
});
