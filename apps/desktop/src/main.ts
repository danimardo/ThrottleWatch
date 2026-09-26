import './app.css';
import { mount } from 'svelte';
import App from './App.svelte';
import { refreshLocale } from './lib/i18n/runtime';

const target = document.getElementById('app');
if (target === null) {
  throw new Error('Application root is missing');
}

// The language is known before anything is drawn: asked of Rust first, so the interface never
// flashes in one language and then switches to the other.
void refreshLocale().then(() => mount(App, { target }));
