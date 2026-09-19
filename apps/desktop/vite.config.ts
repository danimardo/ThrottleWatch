import { defineConfig, loadEnv } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig(({ mode, command }) => {
  const env = loadEnv(mode, '.', 'PUBLIC_');
  if (
    command === 'build' &&
    mode === 'production' &&
    env.PUBLIC_LOG_LEVEL !== undefined
  ) {
    throw new Error(
      'PUBLIC_LOG_LEVEL no puede existir en una compilación de producción'
    );
  }
  return { plugins: [svelte()], envPrefix: 'PUBLIC_' };
});
