import './app.css';
import { mount } from 'svelte';
import App from './App.svelte';

const target = document.getElementById('app');
if (target === null) {
  throw new Error('Application root is missing');
}

mount(App, { target });
