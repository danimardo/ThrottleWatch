import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [svelte()],
  test: {
    name: 'component',
    environment: 'jsdom',
    include: ['src/**/*.component.test.ts'],
    exclude: ['e2e/**', 'node_modules/**']
  }
});
